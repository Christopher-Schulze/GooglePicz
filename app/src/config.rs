use std::path::PathBuf;

pub struct AppConfig {
    pub log_level: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let mut builder = config::Config::builder();
        if let Some(home) = dirs::home_dir() {
            let path: PathBuf = home.join(".googlepicz").join("config");
            builder = builder.add_source(config::File::from(path).required(false));
        }
        let cfg = builder.build().unwrap_or_default();
        let log_level = cfg
            .get_string("log_level")
            .unwrap_or_else(|_| "info".to_string());
        Self { log_level }
    }
}
