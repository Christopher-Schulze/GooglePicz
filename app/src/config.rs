use std::path::PathBuf;
use serde::Deserialize;
use config::{Config, File};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".into()
}

impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".googlepicz")
            .join("config");

        let builder = Config::builder()
            .add_source(File::from(path).required(false));

        builder.build()?.try_deserialize()
    }
}
