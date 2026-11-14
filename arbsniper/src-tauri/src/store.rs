//! store.rs — настройки и журнал сигналов: один JSON в app_config_dir.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::model::{LogEntry, Settings};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Store {
    pub settings: Settings,
    pub log: Vec<LogEntry>,
}

/// Максимум записей журнала в памяти и на диске.
const LOG_CAP: usize = 200;

impl Store {
    pub fn load(path: &PathBuf) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Store::default(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), String> {
        let dir = path.parent().ok_or("no store dir")?;
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())
    }

    /// Добавить запись в журнал (свежие сверху), обрезая хвост до LOG_CAP.
    pub fn push_log(&mut self, entry: LogEntry) {
        self.log.insert(0, entry);
        let n = self.log.len();
        if n > LOG_CAP {
            self.log.truncate(LOG_CAP);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_log_cap() {
        let p = std::env::temp_dir().join(format!("arbsniper-test-{}.json", std::process::id()));
        let mut s = Store::default();
        s.settings.fiat = "RUB".into();
        for i in 0..(LOG_CAP + 20) {
            s.push_log(LogEntry { ts_ms: i as u64, text: format!("w{i}"), spread_pct: 1.0 });
        }
        assert_eq!(s.log.len(), LOG_CAP);
        assert_eq!(s.log[0].text, format!("w{}", LOG_CAP + 19));
        s.save(&p).unwrap();
        let loaded = Store::load(&p);
        assert_eq!(loaded.settings.fiat, "RUB");
        assert_eq!(loaded.log.len(), LOG_CAP);
        std::fs::remove_file(&p).ok();
    }
}
