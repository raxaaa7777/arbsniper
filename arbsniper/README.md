# ⌖ ArbSniper — радар P2P-спредов

**Настольный (Windows/macOS) радар arbitrage-окон между P2P-витринами бирж:
где купить USDT дешевле и где продать дороже — до того, как окно закроется.**

Не торговый бот. Не кошелёк. Прицел: он находит окна, где зарабатывают,
а решение — за вами.

---

## Зачем

P2P-арбитраж — покупка USDT по 90.00 на одной витрине и продажа по 92.00 на
другой — сегодня делается руками: десять вкладок, F5, калькулятор, молитва.
Окно живёт минуты: пока вы пересчитываете, цена ушла. Причина, по которой за
это платят: **спред — это буквально деньги на столе**, и скорость забора
решает всё.

ArbSniper делает рутину машиной: сканирует витрины Binance P2P и Bybit P2P,
считает чистый спред за вычетом комиссий, уведомляет о живых окнах и ведёт
журнал сигналов. Обычная практика: +1–3% на круг; при обороте 1000 USDT это
20–60 фиата за круг, 5–10 кругов в день на живых окнах — см. `docs/STRATEGY.md`
(там же честный список рисков и почему это не «кнопка деньги»).

## Что умеет (v0.1)

| Блок | Что делает |
| --- | --- |
| **Радар** | Фоновый цикл (по умолчанию 30 сек) снимает P2P-витрины, строит книги обеих сторон, находит лучшие пары «купить A → продать B». Живые окна выше порога — баннер + нативное уведомление + звук. |
| **Спот-ориентир** | Живой спот USDTRUB с Binance (фолбэк CoinGecko) в шапке — видно, насколько P2P-цены оторваны от рынка. |
| **Калькулятор** | Цена покупки/продажи, объём, комиссии → валовый и чистый спред, потребный фиат, чистая прибыль на круг. |
| **Трей** | Счётчик живых окон прямо в иконке. Разворачивание по Alt+S из любого места. |
| **Данные** | Demo-источник (встроенная витрина, всегда отвечает) и Live (реальные эндпоинты Binance/Bybit P2P — открываются с домашнего IP). |

## Важно про Live-доступ (проверено)

Из песочницы и любых облачных IP биржевые P2P-витрины закрыты WAF'ом
(Binance 403, Bybit 404, OKX 405) — это защита от датацентрового скрапинга.
**С домашнего IP пользователя те же эндпоинты отвечают нормально.** Поэтому
v0.1 стартует в Demo-режиме (UI и математика полностью проверяемы), а Live
переключается в настройках одним движением. Спот-ориентир работает из_anywhere.

## Сборка

Rust 1.77+, Node 20+, Tauri 2 системные зависимости
([Windows](https://tauri.app/start/prerequisites/#microsoft-c-build-tools-visual-studio-build-tools) /
[macOS](https://tauri.app/start/prerequisites/#xcode-command-line-tools)):

```bash
npm install -g @tauri-apps/cli@^2
cargo tauri dev      # dev-режим
cargo tauri build   # NSIS (Windows) + DMG (macOS)
```

UI — статический HTML/JS в `ui/`, фронтенд-сборки нет (`frontendDist -> ui/`).

## Структура

```
arbsniper/
├── ui/                    # интерфейс (HTML/CSS/JS, RU+EN, тёмный неон)
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs         # окно, трей-бейдж, Alt+S, цикл радара, команды
│   │   ├── model.rs       # объявления, витрины, окна, настройки
│   │   ├── api.rs         # фетчеры Binance/Bybit P2P + спот-ориентир + demo
│   │   ├── engine.rs      # спред-математика (чистый профит за вычетом комиссий)
│   │   └── store.rs        # настройки+журнал (локальный JSON)
│   ├── tauri.conf.json    # CSP: только p2p.binance.com, api2.bybit.com, okx, coingecko, api.binance.com
│   └── capabilities/      # минимальные разрешения
└── docs/
    ├── STRATEGY.md        # как на этом зарабатывают + риски
    └── ROADMAP.md         # план и монетизация
```

## Принципы

1. **Только чтение** — никаких ключей биржи, никаких ордеров: экран и будильник, не рука.
2. **Локально** — настройки и журнал в JSON на вашем диске; наружу только GET/POST витрин.
3. **Честная математика** — комиссии и объём в формуле до того, как вы увидите «прибыль».

## Лицензия

MIT.

---

# ⌖ ArbSniper — P2P spread radar (EN)

A Windows/macOS desktop radar for arbitrage windows between P2P order books:
where to buy USDT cheaper and where to sell it higher — **before the window closes.**
Not a trading bot, not a wallet: it finds windows, you decide.

Typical practice: +1–3% per round; see `docs/STRATEGY.md` for the math and the
honest risk list. The background cycle scans Binance/Bybit P2P books, computes
net-of-fee spreads, alerts on live windows (banner, native notification, sound),
keeps a signal journal, shows a live USDTRUB spot reference, and puts a live-window
counter into the tray icon (Alt+S summons it from anywhere).

**Live access note (verified):** from datacenter IPs the P2P endpoints are
WAF-blocked (Binance 403 / Bybit 404 / OKX 405); from a home IP they respond.
v0.1 therefore starts in Demo mode (built-in book, always answers — UI and math
fully testable) and switches to Live in Settings with one click. The spot
reference works from anywhere.

**Build:** Rust 1.77+, Node 20+, Tauri 2 deps → `cargo tauri dev` / `cargo tauri build`
(NSIS + DMG). Static UI, no frontend build step. MIT license.
