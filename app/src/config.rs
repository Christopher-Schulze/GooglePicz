use serde::{Deserialize, Serialize};
use std::path::{PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub client_id: String,
    pub client_secret: String,
    pub sync_interval_minutes: u64,
    pub cache_dir: PathBuf,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            sync_interval_minutes: 5,
            cache_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".googlepicz"),
            log_level: "info".into(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".googlepicz")
            .join("config.toml");
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<AppConfig>(&contents) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(home) = dirs::home_dir() {
            let dir = home.join(".googlepicz");
            fs::create_dir_all(&dir)?;
            let file = dir.join("config.toml");
            let data = toml::to_string_pretty(self).unwrap();
            fs::write(file, data)?;
        }
        Ok(())
    }
}
