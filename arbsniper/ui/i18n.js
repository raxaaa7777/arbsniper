// i18n.js — словари RU/EN для ArbSniper UI. Подключается до app.js.
"use strict";

const I18N = {
  ru: {
    "app.name": "ArbSniper",
    "app.tagline": "радар P2P-спредов",
    "spot.ref": "Спот-ориентир",
    "alert.title": "ЖИВОЕ ОКНО",
    "tab.radar": "Радар",
    "tab.calc": "Калькулятор",
    "tab.settings": "Настройки",
    "radar.scanning": "Сканирую витрины…",
    "radar.live": "Live",
    "radar.scan": "Скан",
    "radar.windows": "Окна спреда",
    "radar.windows.hint": "Лучшие пары «где купить → где продать». Прибыль считается на объём из настроек, за вычетом комиссий.",
    "radar.windows.empty": "Пока нет окон выше порога. Радар продолжает сканировать.",
    "radar.books": "Витрины по биржам",
    "venue.buybest": "лучшая цена покупки",
    "venue.sellbest": "лучшая цена продажи",
    "venue.stale": "нет данных",
    "venue.adcount": "объявлений",
    "calc.title": "Калькулятор сделки",
    "calc.buy": "Цена покупки (фиат за USDT)",
    "calc.sell": "Цена продажи",
    "calc.vol": "Объём (USDT)",
    "calc.fees": "Комиссии круга, %",
    "calc.gross": "Валовый спред",
    "calc.net": "Чистый спред",
    "calc.capital": "Нужно фиата",
    "calc.profit": "Чистая прибыль",
    "calc.perround": "на круг",
    "calc.journal": "Журнал сигналов",
    "calc.log.empty": "Пока тихо. Живые окна появятся здесь с историей.",
    "set.radar": "Радар",
    "set.fiat": "Фиат",
    "set.source": "Источник данных",
    "set.source.demo": "Demo (встроенная витрина)",
    "set.source.live": "Live (Binance + Bybit P2P)",
    "set.poll": "Период скана, сек",
    "set.minspread": "Порог сигнала, %",
    "set.trade": "Сделка",
    "set.volume": "Расчётный объём, USDT",
    "set.fees": "Комиссии круга, %",
    "set.alerts": "Сигналы",
    "set.sound": "Звук уведомлений",
    "set.tray": "Счётчик живых окон в трее",
    "set.hotkey": "Глобальный хоткей Alt+S показывает/прячет окно из любого места.",
    "set.about": "О программе",
    "set.about.text": "ArbSniper — не биржа и не кошелёк: он только находит окна. Решение и риск всегда за вами. Live-витрины открываются с домашнего IP; из облачных IP биржи закрывают доступ (WAF).",
    "err.badinput": "Проверь поля: цены и объём должны быть числами больше нуля.",
    "log.hour": "ч",
  },

  en: {
    "app.name": "ArbSniper",
    "app.tagline": "P2P spread radar",
    "spot.ref": "Spot reference",
    "alert.title": "LIVE WINDOW",
    "tab.radar": "Radar",
    "tab.calc": "Calculator",
    "tab.settings": "Settings",
    "radar.scanning": "Scanning books…",
    "radar.live": "Live",
    "radar.scan": "Scan",
    "radar.windows": "Spread windows",
    "radar.windows.hint": "Best “where to buy → where to sell” pairs. Profit is computed on the settings volume, net of fees.",
    "radar.windows.empty": "No windows above threshold yet. The radar keeps scanning.",
    "radar.books": "Venue books",
    "venue.buybest": "best buy price",
    "venue.sellbest": "best sell price",
    "venue.stale": "no data",
    "venue.adcount": "ads",
    "calc.title": "Deal calculator",
    "calc.buy": "Buy price (fiat per USDT)",
    "calc.sell": "Sell price",
    "calc.vol": "Volume (USDT)",
    "calc.fees": "Round-trip fees, %",
    "calc.gross": "Gross spread",
    "calc.net": "Net spread",
    "calc.capital": "Fiat needed",
    "calc.profit": "Net profit",
    "calc.perround": "per round",
    "calc.journal": "Signal journal",
    "calc.log.empty": "Quiet so far. Live windows will show up here with history.",
    "set.radar": "Radar",
    "set.fiat": "Fiat",
    "set.source": "Data source",
    "set.source.demo": "Demo (built-in book)",
    "set.source.live": "Live (Binance + Bybit P2P)",
    "set.poll": "Scan interval, s",
    "set.minspread": "Signal threshold, %",
    "set.trade": "Trade",
    "set.volume": "Estimated volume, USDT",
    "set.fees": "Round-trip fees, %",
    "set.alerts": "Alerts",
    "set.sound": "Alert sound",
    "set.tray": "Live window counter in tray",
    "set.hotkey": "Global shortcut Alt+S shows/hides the window from anywhere.",
    "set.about": "About",
    "set.about.text": "ArbSniper is not an exchange and not a wallet: it only finds windows. The decision and the risk are always yours. Live books open from a home IP; cloud IPs are blocked by the venues' WAF.",
    "err.badinput": "Check the fields: prices and volume must be positive numbers.",
    "log.hour": "h",
  },
};

let LANG = "ru";

/** Установить язык и перевести DOM (элементы с data-i18n). */
function setLang(lang) {
  LANG = I18N[lang] ? lang : "ru";
  const dict = I18N[LANG];
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (dict[key] !== undefined) el.textContent = dict[key];
  });
  document.documentElement.setAttribute("lang", LANG);
  document.querySelectorAll(".lang-switch button").forEach((b) => {
    b.classList.toggle("active", b.id === "lang-" + LANG);
  });
}

/** Перевести строку по ключу с подстановками {x}. */
function t(key, subs) {
  let s = I18N[LANG][key] !== undefined ? I18N[LANG][key] : key;
  if (subs) {
    Object.keys(subs).forEach((k) => {
      s = s.replace("{" + k + "}", String(subs[k]));
    });
  }
  return s;
}
