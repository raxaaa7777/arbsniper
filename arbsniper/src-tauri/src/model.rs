//! model.rs — общие типы ArbSniper: объявления, книги ордеров P2P, спред-окна.

use serde::{Deserialize, Serialize};

/// Биржа-источник.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Venue {
    Binance,
    Bybit,
    Okx,
    Demo,
}

impl Venue {
    pub fn label(&self) -> &'static str {
        match self {
            Venue::Binance => "Binance P2P",
            Venue::Bybit => "Bybit P2P",
            Venue::Okx => "OKX P2P",
            Venue::Demo => "Demo",
        }
    }
}

/// Сторона P2P-объявления: как биржа называет сторону сделки для юзера.
/// buy  = юзер ПОКУПАЕТ USDT за фиат;
/// sell = юзер ПРОДАЁТ USDT за фиат.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Buy,
    Sell,
}

/// Одно объявление из витрины P2P.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ad {
    pub venue: Venue,
    pub side: Side,
    /// Цена за 1 USDT в фиате.
    pub price: f64,
    /// Доступный объём в USDT.
    pub volume_usdt: f64,
    /// Лимиты сделки (min-max в фиате), если биржа их отдаёт.
    pub min_fiat: Option<f64>,
    pub max_fiat: Option<f64>,
    /// Платёжные системы (если биржа отдаёт список названий).
    pub pay_methods: Vec<String>,
}

impl Ad {
    /// Верх книги нужной стороны: для покупки USDT — минимальная цена,
    /// для продажи — максимальная.
    pub fn best_price<'a>(ads: impl Iterator<Item = &'a Ad>, side: Side) -> Option<f64> {
        ads.filter(|a| a.side == side && a.price > 0.0)
            .map(|a| a.price)
            .reduce(|acc, p| if side == Side::Buy { acc.min(p) } else { acc.max(p) })
    }
}

/// Полный слепок витрины одной биржи.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueSnapshot {
    pub venue: Venue,
    /// Ориентир мид-рынка (спот USDTRUB на Binance), для нормировки.
    pub spot_ref: f64,
    pub buy_ads: Vec<Ad>,  // юзер покупает USDT: сортируем по цене ASC
    pub sell_ads: Vec<Ad>, // юзер продаёт USDT: сортируем по цене DESC
    /// UTC-метка успешного ответа (мс). 0 = последний запрос упал.
    pub ts_ms: u64,
}

/// Спред-окно: суть продукта. Где купить, где продать, сколько заработаешь.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadWindow {
    /// Где купить USDT (лучшая цена покупки).
    pub buy_venue: Venue,
    pub buy_price: f64,
    /// Где продать USDT (лучшая цена продажи).
    pub sell_venue: Venue,
    pub sell_price: f64,
    /// Спред в процентах от цены покупки.
    pub spread_pct: f64,
    /// Чистый профит в фиате на заявленный объём (за вычетом комиссий).
    pub net_profit_fiat: f64,
    /// Объём USDT, который реально можно провернуть: min(доступно у обоих).
    pub max_volume_usdt: f64,
    /// Учтённые комиссии в процентах (P2P обычно 0, но отдадим в модель).
    pub fees_pct: f64,
}

impl SpreadWindow {
    /// Живой ли спред: выше порога юзера, объём достаточный.
    pub fn is_live(&self, min_pct: f64, min_vol: f64) -> bool {
        self.spread_pct >= min_pct && self.max_volume_usdt >= min_vol
    }
}

/// Настройки радара.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String, // "ru" | "en"
    pub fiat: String,     // "RUB" | "USD" | ...
    pub demo_mode: bool, // источник данных: demo (гарантированный ответ) или живые API
    pub poll_secs: u64,  // период скана витрин
    pub min_spread_pct: f64, // порог сигнала
    pub trade_usdt: f64,      // расчётный объём сделки
    pub fee_pct: f64,         // суммарная комиссия круга, %
    pub alerts_sound: bool,
    pub tray_watch: bool, // трей-иконка ведёт счётчик живых окон
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "ru".into(),
            fiat: "RUB".into(),
            demo_mode: true,
            poll_secs: 30,
            min_spread_pct: 1.5,
            trade_usdt: 1000.0,
            fee_pct: 0.2,
            alerts_sound: true,
            tray_watch: true,
        }
    }
}

/// История сигналов для журнала UI (последние N).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ts_ms: u64,
    pub text: String,
    pub spread_pct: f64,
}
