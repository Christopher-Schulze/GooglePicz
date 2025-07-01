use std::path::PathBuf;

pub struct AppConfig {
    pub log_level: String,
    pub oauth_redirect_port: u16,
    pub thumbnails_preload: usize,
    pub sync_interval_minutes: u64,
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
        let oauth_redirect_port = cfg.get_int("oauth_redirect_port").unwrap_or(8080) as u16;
        let thumbnails_preload = cfg.get_int("thumbnails_preload").unwrap_or(20) as usize;
        let sync_interval_minutes = cfg.get_int("sync_interval_minutes").unwrap_or(5) as u64;

        Self {
            log_level,
            oauth_redirect_port,
            thumbnails_preload,
            sync_interval_minutes,
        }
    }
}
