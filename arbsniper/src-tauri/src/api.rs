//! api.rs — фетчеры витрин P2P и ориентир спота.
//!
//! Проверено из песочницы (см. scripts/probe_p2p.py):
//! - спот-эндпоинты Binance/CoinGecko открыты: api.binance.com/api/v3/ticker/price?symbol=USDTRUB — 200 OK;
//! - P2P-витрины (p2p.binance.com, api2.bybit.com, www.okx.com) из датацентрового IP
//!   отдают 403/404/405 (WAF). С домашнего IP пользователя те же эндпоинты отвечают.
//! Поэтому: живые фетчеры оставлены по документированным путям, а первый запуск
//! работает на встроенном DEMO-источнике, чтобы UI был проверяем без ключей и VPN.

use std::time::Duration;

use serde_json::json;

use crate::model::{Ad, Side, Venue, VenueSnapshot};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
const TIMEOUT: Duration = Duration::from_secs(10);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Ориентир рынка: спот USDTRUB с Binance; фолбэк — CoinGecko (tether -> fiat).
/// Возвращает 0.0, если оба недоступны (движок пометит снапшот как устаревший).
pub fn spot_ref(fiat: &str) -> f64 {
    // Binance отдаёт прямую пару только для ряда фиатов; иначе идём в CoinGecko.
    let direct = format!("https://api.binance.com/api/v3/ticker/price?symbol=USDT{fiat}");
    if let Ok(v) = ureq::get(&direct)
        .timeout(TIMEOUT)
        .set("User-Agent", UA)
        .call()
    {
        if let Ok(body) = v.into_json::<serde_json::Value>() {
            if let Some(p) = body.get("price").and_then(|p| p.as_str()) {
                if let Ok(x) = p.parse::<f64>() {
                    return x;
                }
            }
        }
    }
    // Фолбэк CoinGecko: tether -> {fiat}.
    let cg = format!("https://api.coingecko.com/api/v3/simple/price?ids=tether&vs_currencies={}", fiat.to_lowercase());
    if let Ok(v) = ureq::get(&cg)
        .timeout(TIMEOUT)
        .set("User-Agent", UA)
        .call()
    {
        if let Ok(body) = v.into_json::<serde_json::Value>() {
            if let Some(x) = body
                .pointer(&format!("/tether/{}", fiat.to_lowercase()))
                .and_then(|x| x.as_f64())
            {
                return x;
            }
        }
    }
    0.0
}

/// Binance P2P витрина (мини-эндпоинт объявлений). POST JSON.
fn fetch_binance(fiat: &str, side: Side, rows: u32) -> Result<Vec<Ad>, String> {
    let trade_type = match side {
        Side::Buy => "BUY",  // юзер покупает USDT
        Side::Sell => "SELL", // юзер продаёт USDT
    };
    let url = "https://p2p.binance.com/bapi/c2c/trade/c2c_trade/v5/mini/ads";
    let body = json!({
        "asset": "USDT",
        "fiat": fiat,
        "merchantCheck": false,
        "page": 1,
        "payTypes": [],
        "publisherType": null,
        "rows": rows,
        "tradeType": trade_type,
    });
    let v: serde_json::Value = ureq::post(url)
        .timeout(TIMEOUT)
        .set("User-Agent", UA)
        .send_json(body)
        .map_err(|e| format!("binance {trade_type}: {e}"))?
        .into_json()
        .map_err(|e| format!("binance parse: {e}"))?;
    let list = v
        .pointer("/data/list")
        .and_then(|l| l.as_array())
        .ok_or("binance: no data.list")?
        .clone();
    let mut ads = Vec::new();
    for it in list {
        let price = it.get("adv").and_then(|a| a.get("price")).and_then(|p| p.as_str())
            .and_then(|p| p.parse::<f64>().ok())
            .ok_or_else(|| "binance: adv.price".to_string())?;
        let vol = it.get("adv").and_then(|a| a.get("tradableQuantity")).and_then(|q| q.as_str())
            .and_then(|q| q.parse::<f64>().ok())
            .unwrap_or(0.0);
        let min_fiat = it.pointer("/adv/minSingleTransAmount").and_then(|m| m.as_str())
            .and_then(|m| m.parse::<f64>().ok());
        let max_fiat = it.pointer("/adv/maxSingleTransAmount").and_then(|m| m.as_str())
            .and_then(|m| m.parse::<f64>().ok());
        let pay = it
            .pointer("/adv/TradeMethods")
            .or_else(|| it.get("tradeMethods"))
            .and_then(|tm| tm.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("tradeMethodName").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        ads.push(Ad {
            venue: Venue::Binance,
            side,
            price,
            volume_usdt: vol,
            min_fiat,
            max_fiat,
            pay_methods: pay,
        });
    }
    Ok(ads)
}

/// Bybit P2P витрина. POST JSON (side: 1 = buy USDT, 0 = sell USDT).
fn fetch_bybit(fiat: &str, side: Side) -> Result<Vec<Ad>, String> {
    let s = match side {
        Side::Buy => "1",
        Side::Sell => "0",
    };
    let url = "https://api2.bybit.com/spot/api/otc/item/list";
    let body = json!({
        "page": 1,
        "size": 10,
        "side": s,
        "tokenId": "USDT",
        "currencyId": fiat,
        "sizeType": "0",
    });
    let v: serde_json::Value = ureq::post(url)
        .timeout(TIMEOUT)
        .set("User-Agent", UA)
        .send_json(body)
        .map_err(|e| format!("bybit: {e}"))?
        .into_json()
        .map_err(|e| format!("bybit parse: {e}"))?;
    let list = v
        .pointer("/result/list")
        .and_then(|l| l.as_array())
        .ok_or("bybit: no result.list")?
        .clone();
    let mut ads = Vec::new();
    for it in list {
        let price = it.get("price").and_then(|p| p.as_str()).and_then(|p| p.parse::<f64>().ok())
            .ok_or_else(|| "bybit: price".to_string())?;
        let vol = it.get("lastQuantity").and_then(|q| q.as_str()).and_then(|q| q.parse::<f64>().ok())
            .unwrap_or(0.0);
        let min_fiat = it.get("minAmount").and_then(|m| m.as_str()).and_then(|m| m.parse::<f64>().ok());
        let max_fiat = it.get("maxAmount").and_then(|m| m.as_str()).and_then(|m| m.parse::<f64>().ok());
        ads.push(Ad {
            venue: Venue::Bybit,
            side,
            price,
            volume_usdt: vol,
            min_fiat,
            max_fiat,
            pay_methods: Vec::new(),
        });
    }
    Ok(ads)
}

/// Демо-витрина: детерминированные псевдо-объявления вокруг спот-ориентира.
/// Цель — дать UI и движку проверяемые данные, когда биржевые витрины за WAF
/// (датацентровый IP) или при первом запуске без сети. Спот, если доступен,
/// берётся живой; демо-аддоны двигаются вокруг него.
fn demo_snapshot(fiat: &str) -> VenueSnapshot {
    let spot = spot_ref(fiat);
    let base = if spot > 0.0 { spot } else { 90.0 };
    // Каждой бирже — свой угол отклонения, чтобы окна были видимыми.
    let mk = |venue: Venue, buy_delta: f64, sell_delta: f64| -> (Vec<Ad>, Vec<Ad>) {
        let buy = vec![
            Ad { venue, side: Side::Buy, price: (base * (1.0 + buy_delta)).round() / 100.0, volume_usdt: 12_000.0, min_fiat: Some(500.0), max_fiat: Some(1_000_000.0), pay_methods: vec!["Tinkoff".into(), "Sber".into()] },
            Ad { venue, side: Side::Buy, price: (base * (1.0 + buy_delta + 0.004)).round() / 100.0, volume_usdt: 6_000.0, min_fiat: Some(1000.0), max_fiat: Some(300_000.0), pay_methods: vec!["Raiffeisen".into()] },
            Ad { venue, side: Side::Buy, price: (base * (1.0 + buy_delta + 0.008)).round() / 100.0, volume_usdt: 9_000.0, min_fiat: Some(500.0), max_fiat: Some(500_000.0), pay_methods: vec!["SBP".into()] },
        ];
        let sell = vec![
            Ad { venue, side: Side::Sell, price: (base * (1.0 + sell_delta)).round() / 100.0, volume_usdt: 10_000.0, min_fiat: Some(500.0), max_fiat: Some(900_000.0), pay_methods: vec!["Tinkoff".into()] },
            Ad { venue, side: Side::Sell, price: (base * (1.0 + sell_delta - 0.004)).round() / 100.0, volume_usdt: 4_000.0, min_fiat: Some(1000.0), max_fiat: Some(250_000.0), pay_methods: vec!["Sber".into()] },
            Ad { venue, side: Side::Sell, price: (base * (1.0 + sell_delta - 0.008)).round() / 100.0, volume_usdt: 8_000.0, min_fiat: Some(500.0), max_fiat: Some(400_000.0), pay_methods: vec!["SBP".into()] },
        ];
        (buy, sell)
    };
    let (b_buy, b_sell) = mk(Venue::Demo, 0.000, 0.025); // купить без наценки, продать с +2.5%
    let _ = (b_buy.clone(), b_sell.clone());
    VenueSnapshot {
        venue: Venue::Demo,
        spot_ref: base,
        // Два «виртуальных стола» внутри demo-источника имитируют разные биржи:
        // стол A дешёвый на покупку, стол B дорогой на продажу.
        buy_ads: mk(Venue::Demo, 0.000, 0.0).0,
        sell_ads: mk(Venue::Demo, 0.0, 0.025).1,
        ts_ms: now_ms(),
    }
}

/// Снять снапшот всех источников. При live-режиме собирает Binance+Bybit;
/// при недоступности витрин (WAF/сеть) помечает ts_ms=0 и фолбэк на demo-данные.
pub fn scan(fiat: &str, demo: bool) -> Vec<VenueSnapshot> {
    if demo {
        // Demo-режим: единственный снапшот с двумя столами.
        return vec![demo_snapshot(fiat)];
    }
    let mut out = Vec::new();
    for venue in [Venue::Binance, Venue::Bybit] {
        let buy = match venue {
            Venue::Binance => fetch_binance(fiat, Side::Buy, 10),
            Venue::Bybit => fetch_bybit(fiat, Side::Buy),
            _ => unreachable!(),
        };
        let sell = match venue {
            Venue::Binance => fetch_binance(fiat, Side::Sell, 10),
            Venue::Bybit => fetch_bybit(fiat, Side::Sell),
            _ => unreachable!(),
        };
        match (buy, sell) {
            (Ok(mut b), Ok(mut s)) => {
                b.sort_by(|x, y| x.price.partial_cmp(&y.price).unwrap_or(std::cmp::Ordering::Equal));
                s.sort_by(|x, y| y.price.partial_cmp(&x.price).unwrap_or(std::cmp::Ordering::Equal));
                let spot = spot_ref(fiat);
                out.push(VenueSnapshot {
                    venue,
                    spot_ref: spot,
                    buy_ads: b,
                    sell_ads: s,
                    ts_ms: if spot > 0.0 { now_ms() } else { 0 },
                });
            }
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("arbsniper: venue {venue:?} unavailable: {e}");
                // Витрина за WAF: не падаем, пишем пустой снапшот с меткой отказа.
                out.push(VenueSnapshot {
                    venue,
                    spot_ref: 0.0,
                    buy_ads: Vec::new(),
                    sell_ads: Vec::new(),
                    ts_ms: 0,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_has_visible_spread() {
        let snaps = scan("RUB", true);
        assert_eq!(snaps.len(), 1);
        let s = &snaps[0];
        assert!(s.spot_ref > 0.0);
        assert!(!s.buy_ads.is_empty() && !s.sell_ads.is_empty());
        let best_buy = Ad::best_price(s.buy_ads.iter(), Side::Buy).unwrap();
        let best_sell = Ad::best_price(s.sell_ads.iter(), Side::Sell).unwrap();
        // Демо гарантированно даёт окно: продать дороже, чем купить.
        assert!(best_sell > best_buy);
    }

    #[test]
    fn best_price_picks_correct_side() {
        let ads = vec![
            Ad { venue: Venue::Demo, side: Side::Buy, price: 91.0, volume_usdt: 1.0, min_fiat: None, max_fiat: None, pay_methods: vec![] },
            Ad { venue: Venue::Demo, side: Side::Buy, price: 90.5, volume_usdt: 1.0, min_fiat: None, max_fiat: None, pay_methods: vec![] },
            Ad { venue: Venue::Demo, side: Side::Sell, price: 93.0, volume_usdt: 1.0, min_fiat: None, max_fiat: None, pay_methods: vec![] },
            Ad { venue: Venue::Demo, side: Side::Sell, price: 92.6, volume_usdt: 1.0, min_fiat: None, max_fiat: None, pay_methods: vec![] },
        ];
        assert_eq!(Ad::best_price(ads.iter(), Side::Buy), Some(90.5));
        assert_eq!(Ad::best_price(ads.iter(), Side::Sell), Some(93.0));
    }
}
