//! lib.rs — точка сборки ArbSniper: окно, трей со счётчиком окон, хоткей,
//! фоновый цикл скана витрин и команды UI.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod engine;
mod model;
mod store;

use std::sync::Mutex;
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;

use model::{LogEntry, Settings, SpreadWindow};
use store::Store;

/// Общее состояние.
struct AppState {
    store: Mutex<Store>,
}

impl AppState {
    fn settings(&self) -> Settings {
        self.store.lock().expect("store poisoned").settings.clone()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn store_path(app: &AppHandle) -> std::path::PathBuf {
    app.app_config_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("arbsniper-store.json")
}

/// Один цикл радара: снять витрины, найти окна, обновить трей, раздать события.
/// Возвращает число живых окон (для трей-бейджа и тестов).
fn radar_cycle(app: &AppHandle) -> usize {
    let state = app.state::<AppState>();
    let settings = state.settings();

    let mut snaps = api::scan(&settings.fiat, settings.demo_mode);
    let windows = engine::find_windows(&snaps, &settings);
    let live: Vec<SpreadWindow> = windows
        .iter()
        .filter(|w| w.is_live(settings.min_spread_pct, 100.0))
        .cloned()
        .collect();

    // Человекочитаемые лейблы для UI (у Venue их нет в сериализации событий).
    let json_windows: Vec<serde_json::Value> = windows
        .iter()
        .map(|w| {
            let mut v = serde_json::to_value(w).unwrap_or_default();
            v["buy_venue_label"] = serde_json::json!(w.buy_venue.label());
            v["sell_venue_label"] = serde_json::json!(w.sell_venue.label());
            v
        })
        .collect();
    let json_snaps: Vec<serde_json::Value> = snaps
        .iter_mut()
        .map(|s| {
            let mut v = serde_json::to_value(&*s).unwrap_or_default();
            v["venue_label"] = serde_json::json!(s.venue.label());
            v
        })
        .collect();

    // Трей-бейдж: сколько живых окон прямо сейчас.
    if settings.tray_watch {
        if let Some(tray) = app.tray_by_id("main") {
            let title = if live.is_empty() { None } else { Some(live.len().to_string()) };
            let _ = tray.set_title(title.as_deref());
        }
    }

    // События UI: полный снапшот + окна.
    let _ = app.emit("radar:tick", serde_json::json!({ "snapshots": json_snaps, "windows": json_windows }));

    // Живое окно дороже порога — сигнал: emit + нотификация + журнал.
    if let Some(best) = live.first() {
        let text = format!(
            "{} → {}: +{:.2}% (≈{:.0} {} на {:.0} USDT)",
            best.buy_venue.label(),
            best.sell_venue.label(),
            best.spread_pct,
            best.net_profit_fiat,
            settings.fiat,
            settings.trade_usdt.min(best.max_volume_usdt),
        );
        let mut payload = serde_json::to_value(best).unwrap_or_default();
        payload["buy_venue_label"] = serde_json::json!(best.buy_venue.label());
        payload["sell_venue_label"] = serde_json::json!(best.sell_venue.label());
        let _ = app.emit("radar:alert", payload);
        let _ = app
            .notification()
            .builder()
            .title("⚡ ArbSniper: живое окно")
            .body(&text)
            .show();
        let mut st = state.store.lock().expect("store poisoned");
        st.push_log(LogEntry { ts_ms: now_ms(), text, spread_pct: best.spread_pct });
        let _ = st.save(&store_path(app));
    }

    live.len()
}

/// Фоновый цикл: спит poll_secs, сканирует, повторяет. Живёт пока живо приложение.
fn spawn_radar(app: AppHandle) {
    tauri::async_runtime::spawn(move || {
        let mut first = true;
        loop {
            let secs = {
                let state = app.state::<AppState>();
                state.settings().poll_secs.max(5)
            };
            if !first {
                std::thread::sleep(Duration::from_secs(secs));
            }
            first = false;
            radar_cycle(&app);
        }
    });
}

fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn get_store(app: AppHandle) -> serde_json::Value {
    let state = app.state::<AppState>();
    let s = state.store.lock().expect("store poisoned");
    serde_json::to_value(&*s).unwrap_or(serde_json::json!({}))
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut s = state.store.lock().expect("store poisoned");
    s.settings = settings;
    s.save(&store_path(&app))
}

#[tauri::command]
fn get_log(app: AppHandle) -> serde_json::Value {
    let state = app.state::<AppState>();
    let s = state.store.lock().expect("store poisoned");
    serde_json::to_value(&s.log).unwrap_or(serde_json::json!([]))
}

/// Ручной пересканинг по кнопке (мгновенный ответ UI).
#[tauri::command]
fn scan_now(app: AppHandle) -> serde_json::Value {
    let n = radar_cycle(&app);
    serde_json::json!({ "live": n })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::init())
        .manage(AppState {
            store: Mutex::new(Store::default()),
        })
        .invoke_handler(tauri::generate_handler![
            get_store,
            set_settings,
            get_log,
            scan_now
        ])
        .setup(|app| {
            {
                let loaded = Store::load(&store_path(app.handle()));
                let state = app.state::<AppState>();
                *state.store.lock().expect("store poisoned") = loaded;
            }

            let open = MenuItem::with_id(app, "open", "Открыть ArbSniper", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = ev
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            let gs = app.global_shortcut();
            let shortcut = Shortcut::Alt(Some(tauri::keyboard::Code::KeyS));
            gs.on_shortcut(shortcut, move |app, _s, event| {
                if event.state == ShortcutState::Pressed {
                    if let Some(win) = app.get_webview_window("main") {
                        if win.is_visible().unwrap_or(false) {
                            let _ = win.hide();
                        } else {
                            show_main(app);
                        }
                    }
                }
            })?;
            gs.register(shortcut)?;

            spawn_radar(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ArbSniper: ошибка запуска");
}
