//! engine.rs — спред-математика: из снапшотов витрин собрать лучшие окна
//! «купить там → продать там» и посчитать чистый профит.

use crate::model::{Ad, Settings, Side, SpreadWindow, VenueSnapshot};

/// Найти лучшее окно между двумя снапшотами: A = где купить, B = где продать.
/// Возвращает None, если окно отрицательное или данных нет.
pub fn best_window(a: &VenueSnapshot, b: &VenueSnapshot, s: &Settings) -> Option<SpreadWindow> {
    let buy = Ad::best_price(a.buy_ads.iter(), Side::Buy)?;
    let sell = Ad::best_price(b.sell_ads.iter(), Side::Sell)?;
    if buy <= 0.0 || sell <= 0.0 {
        return None;
    }
    let gross_pct = (sell - buy) / buy * 100.0;
    // Комиссии съедают часть: у нас оба края P2P (maker 0% на большинстве бирж),
    // но перевод фиата между платёжками может стоить денег — юзер сам ставит fee_pct.
    let spread_pct = gross_pct - s.fee_pct;
    if spread_pct <= 0.0 {
        return None;
    }
    // Объём: сколько USDT реально можно провернуть по обеим сторонам.
    let vol_buy = a.buy_ads.iter().map(|ad| ad.volume_usdt).fold(0.0_f64, f64::max);
    let vol_sell = b.sell_ads.iter().map(|ad| ad.volume_usdt).fold(0.0_f64, f64::max);
    let max_volume_usdt = vol_buy.min(vol_sell).max(0.0);

    let use_vol = s.trade_usdt.min(max_volume_usdt);
    let net_profit_fiat = use_vol * buy * (spread_pct / 100.0);

    Some(SpreadWindow {
        buy_venue: a.venue,
        buy_price: buy,
        sell_venue: b.venue,
        sell_price: sell,
        spread_pct,
        net_profit_fiat,
        max_volume_usdt,
        fees_pct: s.fee_pct,
    })
}

/// Собрать все окна по всем парам снапшотов (A→B и B→A) и отсортировать по
/// чистому профиту. Живые — те, что проходят порог юзера.
pub fn find_windows(snaps: &[VenueSnapshot], s: &Settings) -> Vec<SpreadWindow> {
    let mut wins: Vec<SpreadWindow> = Vec::new();
    for i in 0..snaps.len() {
        for j in 0..snaps.len() {
            if i == j {
                // Внутри одной биржи окно тоже возможно (высокая ставка на
                // разных мерчантов), но для demo-источника столы разные — считаем.
                // Реальные venue различны, demo один — пропускаем самопару.
                continue;
            }
            if let Some(w) = best_window(&snaps[i], &snaps[j], s) {
                wins.push(w);
            }
        }
    }
    // Но demo-источник — один снапшот с двумя столами: самопару обрабатываем отдельно.
    if snaps.len() == 1 {
        let only = &snaps[0];
        if let Some(mut w) = best_window(only, only, s) {
            // В demo оба стола формально venue=Demo; пометим как есть.
            w.max_volume_usdt = w.max_volume_usdt;
            wins.push(w);
        }
    }
    wins.sort_by(|x, y| y.net_profit_fiat.partial_cmp(&x.net_profit_fiat).unwrap_or(std::cmp::Ordering::Equal));
    wins
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ad, Venue};

    fn snap(venue: Venue, buy_px: f64, sell_px: f64) -> VenueSnapshot {
        VenueSnapshot {
            venue,
            spot_ref: 90.0,
            buy_ads: vec![Ad { venue, side: Side::Buy, price: buy_px, volume_usdt: 5000.0, min_fiat: None, max_fiat: None, pay_methods: vec![] }],
            sell_ads: vec![Ad { venue, side: Side::Sell, price: sell_px, volume_usdt: 7000.0, min_fiat: None, max_fiat: None, pay_methods: vec![] }],
            ts_ms: 1,
        }
    }

    fn settings() -> Settings {
        Settings { trade_usdt: 1000.0, fee_pct: 0.2, min_spread_pct: 1.0, ..Default::default() }
    }

    #[test]
    fn window_math() {
        // Купить по 90.0, продать по 93.0: гросс 3.333%, минус fee 0.2% = 3.133%.
        // Объём 1000 USDT -> профит = 1000 * 90 * 0.031333... ≈ 2820 фиата.
        let s = settings();
        let w = best_window(&snap(Venue::Binance, 90.0, 93.0), &snap(Venue::Bybit, 89.0, 92.0), &s).unwrap();
        assert!((w.spread_pct - 3.1333).abs() < 0.001, "got {}", w.spread_pct);
        assert!((w.net_profit_fiat - 2820.0).abs() < 1.0, "got {}", w.net_profit_fiat);
        assert_eq!(w.buy_venue, Venue::Binance);
        assert_eq!(w.max_volume_usdt, 5000.0);
        assert!(w.is_live(1.0, 100.0));
    }

    #[test]
    fn negative_window_dropped() {
        let s = settings();
        // Купить 93, продать 90 — окно отрицательное.
        assert!(best_window(&snap(Venue::Binance, 93.0, 90.0), &snap(Venue::Bybit, 89.0, 92.0), &s).is_none());
    }

    #[test]
    fn sorted_by_profit() {
        let s = settings();
        let snaps = vec![
            snap(Venue::Binance, 90.0, 91.0), // окно нет (sell ниже buy в парах ниже)
            snap(Venue::Bybit, 90.0, 93.0),
        ];
        // Пары: B->Y: buy 90 sell 93 ок; Y->B: buy 90 sell 91 = 0.9% -> после fee<0, дропнут.
        let wins = find_windows(&snaps, &s);
        assert_eq!(wins.len(), 1);
        assert!((wins[0].spread_pct - 3.1333).abs() < 0.001);
    }
}
