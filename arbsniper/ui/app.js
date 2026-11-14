// app.js — логика ArbSniper UI: радар спредов, витрины, калькулятор, настройки.
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ---------- Табы ----------
document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((x) => x.classList.remove("active"));
    document.querySelectorAll(".tab-page").forEach((x) => x.classList.remove("active"));
    tab.classList.add("active");
    document.getElementById("tab-" + tab.dataset.tab).classList.add("active");
  });
});

// ---------- Язык ----------
function bindLangButtons() {
  document.getElementById("lang-ru").addEventListener("click", () => { setLang("ru"); pushSettings(); });
  document.getElementById("lang-en").addEventListener("click", () => { setLang("en"); pushSettings(); });
}

// ---------- Радар: тик движка ----------
function fmtNum(v, digits) {
  if (v === null || v === undefined || isNaN(v)) return "—";
  return Number(v).toLocaleString("en-US", { maximumFractionDigits: digits });
}

function renderTick(payload) {
  const snaps = payload.snapshots || [];
  const wins = payload.windows || [];

  // Спот-ориентир: из первого непустого снапшота.
  const spot = snaps.map((s) => s.spot_ref).find((v) => v > 0);
  setText("spot-ref", spot ? fmtNum(spot, 2) : "—");

  renderWindows(wins);
  renderVenues(snaps);
  setStatus(true);
}

function renderWindows(wins) {
  const box = document.getElementById("windows");
  const empty = document.getElementById("windows-empty");
  box.innerHTML = "";
  const s = currentSettings;
  const minPct = s ? s.min_spread_pct : 1.5;
  const live = wins.filter((w) => w.spread_pct >= minPct);
  empty.classList.toggle("hidden", wins.length > 0);
  wins.slice(0, 8).forEach((w) => {
    const el = document.createElement("div");
    el.className = "win" + (w.spread_pct >= minPct ? " live" : "");
    el.innerHTML =
      '<div class="win-route">' +
      '<span class="price">' + escapeHtml(w.buy_venue_label || w.buy_venue) + '</span> ' +
      '<span class="price">' + fmtNum(w.buy_price, 2) + '</span> ' +
      '<span class="arrow">→</span> ' +
      '<span class="price">' + fmtNum(w.sell_price, 2) + '</span> ' +
      '<span>' + escapeHtml(w.sell_venue_label || w.sell_venue) + "</span>" +
      "</div>" +
      '<div class="win-stats">' +
      "<span>" + t("venue.buybest") + ": <b>" + fmtNum(w.buy_price, 2) + "</b></span>" +
      "<span>vol ≤ <b>" + fmtNum(w.max_volume_usdt, 0) + " USDT</b></span>" +
      "<span>fee <b>" + fmtNum(w.fees_pct, 2) + "%</b></span>" +
      "</div>";
    const profit = document.createElement("div");
    profit.className = "win-profit";
    const pct = document.createElement("div");
    pct.className = "pct";
    pct.textContent = "+" + fmtNum(w.spread_pct, 2) + "%";
    const fiat = document.createElement("div");
    fiat.className = "fiat";
    fiat.textContent = "≈ " + fmtNum(w.net_profit_fiat, 0) + " " + (s ? s.fiat : "");
    profit.append(pct, fiat);
    el.appendChild(profit);
    box.appendChild(el);
  });
}

function renderVenues(snaps) {
  const box = document.getElementById("venues");
  box.innerHTML = "";
  snaps.forEach((snap) => {
    const el = document.createElement("div");
    el.className = "venue-card";
    const head = document.createElement("div");
    head.className = "venue-head";
    const label = snap.venue_label || snap.venue;
    if (snap.ts_ms === 0) {
      head.innerHTML = "<b>" + escapeHtml(label) + "</b><span class='stale'>" + t("venue.stale") + "</span>";
    } else {
      const buyBest = snap.buy_ads.length ? Math.min(...snap.buy_ads.map((a) => a.price)) : null;
      const sellBest = snap.sell_ads.length ? Math.max(...snap.sell_ads.map((a) => a.price)) : null;
      head.innerHTML =
        "<b>" + escapeHtml(label) + "</b>" +
        "<span>" + t("venue.buybest") + " <b>" + fmtNum(buyBest, 2) + "</b> · " +
        t("venue.sellbest") + " <b>" + fmtNum(sellBest, 2) + "</b></span>";
    }
    el.appendChild(head);
    // Верх книги: 3 лучших с каждой стороны.
    const buys = snap.buy_ads.slice(0, 3).map((a) => bookRow(a, "buy"));
    const sells = snap.sell_ads.slice(0, 3).map((a) => bookRow(a, "sell"));
    el.append(...buys, ...sells);
    box.appendChild(el);
  });
}

function bookRow(ad, side) {
  const row = document.createElement("div");
  row.className = "book-row " + side;
  const px = document.createElement("span");
  px.className = "px";
  px.textContent = fmtNum(ad.price, 2);
  const pay = document.createElement("span");
  pay.className = "pay";
  pay.textContent = (ad.pay_methods || []).join(", ") || "—";
  const vol = document.createElement("span");
  vol.className = "vol";
  vol.textContent = fmtNum(ad.volume_usdt, 0) + " USDT";
  row.append(px, pay, vol);
  return row;
}

// ---------- Живое окно: баннер ----------
function onAlert(w) {
  const banner = document.getElementById("alert-banner");
  const s = currentSettings;
  document.getElementById("alert-text").textContent =
    (w.buy_venue_label || w.buy_venue) + " → " + (w.sell_venue_label || w.sell_venue) +
    ": +" + fmtNum(w.spread_pct, 2) + "% (≈" + fmtNum(w.net_profit_fiat, 0) + " " + (s ? s.fiat : "") +
    " на " + fmtNum(Math.min(s ? s.trade_usdt : 1000, w.max_volume_usdt), 0) + " USDT)";
  banner.classList.remove("hidden");
  addLogEntry({ ts_ms: Date.now(), text: document.getElementById("alert-text").textContent, spread_pct: w.spread_pct });
  if (currentSettings && currentSettings.alerts_sound) beep();
}

// ---------- Журнал ----------
function addLogEntry(e) {
  const log = document.getElementById("signal-log");
  const empty = document.getElementById("log-empty");
  if (empty) empty.classList.add("hidden");
  if (!log) return;
  const li = document.createElement("li");
  const time = document.createElement("time");
  const d = new Date(e.ts_ms);
  time.textContent = d.getHours().toString().padStart(2, "0") + ":" + d.getMinutes().toString().padStart(2, "0");
  const spread = document.createElement("span");
  spread.className = "spread";
  spread.textContent = "+" + fmtNum(e.spread_pct, 2) + "%";
  const text = document.createElement("span");
  text.className = "text";
  text.textContent = e.text;
  li.append(time, spread, text);
  log.prepend(li);
  while (log.children.length > 100) log.removeChild(log.lastChild);
}

function renderLog(entries) {
  entries.forEach((e) => addLogEntry(e));
}

// ---------- Калькулятор ----------
function runCalc() {
  const buy = parseFloat(document.getElementById("calc-buy").value);
  const sell = parseFloat(document.getElementById("calc-sell").value);
  const vol = parseFloat(document.getElementById("calc-vol").value);
  const fees = parseFloat(document.getElementById("calc-fees").value);
  const out = document.getElementById("calc-result");
  if (!(buy > 0 && sell > 0 && vol > 0) || !(fees >= 0)) {
    out.innerHTML = '<p class="hint">' + t("err.badinput") + "</p>";
    return;
  }
  const grossPct = (sell - buy) / buy * 100;
  const netPct = grossPct - fees;
  const capital = buy * vol;
  const profit = capital * netPct / 100;
  const line = (k, v, cls) => '<div class="calc-line ' + (cls || "") + '"><span>' + t(k) + '</span><b>' + v + "</b></div>";
  out.innerHTML =
    line("calc.gross", "+" + fmtNum(grossPct, 2) + "%") +
    line("calc.net", (netPct >= 0 ? "+" : "") + fmtNum(netPct, 2) + "%", netPct < 0 ? "negative" : "") +
    line("calc.capital", fmtNum(capital, 0) + " " + (currentSettings ? currentSettings.fiat : "")) +
    line("calc.profit", (profit >= 0 ? "+" : "") + fmtNum(profit, 0), netPct < 0 ? "negative big" : "big") +
    '<p class="hint">' + t("calc.perround") + " · " + fmtNum(vol, 0) + " USDT</p>";
}

// ---------- Настройки ----------
let currentSettings = null;

function renderSettings(s) {
  if (!s) return;
  currentSettings = s;
  setSelect("set-fiat", s.fiat);
  setSelect("set-source", s.demo_mode ? "demo" : "live");
  setInput("set-poll", s.poll_secs);
  setInput("set-minspread", s.min_spread_pct);
  setInput("set-trade", s.trade_usdt);
  setInput("set-fees", s.fee_pct);
  setChecked("set-sound", s.alerts_sound);
  setChecked("set-tray", s.tray_watch);
}

async function pushSettings() {
  try {
    const s = await invoke("get_store");
    const st = s.settings;
    st.language = LANG;
    st.fiat = getSelect("set-fiat") || st.fiat;
    const src = getSelect("set-source");
    if (src) st.demo_mode = src === "demo";
    st.poll_secs = Math.max(5, parseFloat(getInput("set-poll")) || st.poll_secs);
    st.min_spread_pct = parseFloat(getInput("set-minspread")) || st.min_spread_pct;
    st.trade_usdt = Math.max(100, parseFloat(getInput("set-trade")) || st.trade_usdt);
    st.fee_pct = Math.max(0, parseFloat(getInput("set-fees")) || 0);
    st.alerts_sound = getChecked("set-sound");
    st.tray_watch = getChecked("set-tray");
    await invoke("set_settings", { settings: st });
    currentSettings = st;
  } catch (e) { console.warn("pushSettings failed", e); }
}

// ---------- Статус / звук ----------
function setStatus(ok) {
  const dot = document.getElementById("status-dot");
  const text = document.getElementById("status-text");
  dot.classList.toggle("pulse", ok);
  text.textContent = t("radar.scanning");
}

function beep() {
  try {
    const ctx = new (window.AudioContext || window.webkitAudioContext)();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "sine";
    osc.frequency.value = 1320;
    gain.gain.value = 0.06;
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start();
    setTimeout(() => { osc.stop(); ctx.close(); }, 220);
  } catch (e) { console.warn("beep failed", e); }
}

// ---------- Утилиты ----------
function setText(id, s) {
  const el = document.getElementById(id);
  if (el) el.textContent = s;
}
function setSelect(id, v) {
  const el = document.getElementById(id);
  if (el) el.value = v;
}
function getSelect(id) {
  const el = document.getElementById(id);
  return el ? el.value : "";
}
function setInput(id, v) {
  const el = document.getElementById(id);
  if (el) el.value = v;
}
function getInput(id) {
  const el = document.getElementById(id);
  return el ? el.value : "";
}
function setChecked(id, v) {
  const el = document.getElementById(id);
  if (el) el.checked = !!v;
}
function getChecked(id) {
  const el = document.getElementById(id);
  return el ? el.checked : false;
}
function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

// ---------- Инициализация ----------
async function init() {
  bindLangButtons();
  try {
    const s = await invoke("get_store");
    setLang(s.settings.language === "en" ? "en" : "ru");
    renderSettings(s.settings);
    currentSettings = s.settings;
    if (Array.isArray(s.log)) renderLog(s.log);
  } catch (e) {
    console.warn("init store failed", e);
    setLang("ru");
  }

  document.getElementById("btn-scan").addEventListener("click", async () => {
    try { await invoke("scan_now"); } catch (e) { console.warn("scan failed", e); }
  });

  ["calc-buy", "calc-sell", "calc-vol", "calc-fees"].forEach((id) => {
    document.getElementById(id).addEventListener("input", runCalc);
  });
  runCalc();

  document.querySelectorAll("select, input[type='number'], .switch-row input").forEach((el) => {
    el.addEventListener("change", pushSettings);
  });

  listen("radar:tick", (ev) => renderTick(ev.payload));
  listen("radar:alert", (ev) => onAlert(ev.payload));
}

document.addEventListener("DOMContentLoaded", init);
