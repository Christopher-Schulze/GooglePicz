use std::path::PathBuf;

pub struct AppConfig {
    pub log_level: String,
    pub oauth_redirect_port: u16,
    pub thumbnails_preload: usize,
    pub sync_interval_minutes: u64,
    pub debug_console: bool,
    pub cache_path: PathBuf,
}

pub struct AppConfigOverrides {
    pub log_level: Option<String>,
    pub oauth_redirect_port: Option<u16>,
    pub thumbnails_preload: Option<usize>,
    pub sync_interval_minutes: Option<u64>,
    pub debug_console: bool,
}

impl AppConfig {
    pub fn load_from(path: Option<PathBuf>) -> Self {
        let mut builder = config::Config::builder();
        let path = match path {
            Some(p) => p,
            None => dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".googlepicz")
                .join("config"),
        };
        builder = builder.add_source(config::File::from(path).required(false));
        let cfg = builder.build().unwrap_or_default();

        let log_level = cfg
            .get_string("log_level")
            .unwrap_or_else(|_| "info".to_string());
        let oauth_redirect_port = cfg.get_int("oauth_redirect_port").unwrap_or(8080) as u16;
        let thumbnails_preload = cfg.get_int("thumbnails_preload").unwrap_or(20) as usize;
        let sync_interval_minutes = cfg.get_int("sync_interval_minutes").unwrap_or(5) as u64;
        let debug_console = cfg.get_bool("debug_console").unwrap_or(false);
        let cache_path = cfg
            .get_string("cache_path")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".googlepicz")
            });

        Self {
            log_level,
            oauth_redirect_port,
            thumbnails_preload,
            sync_interval_minutes,
            debug_console,
            cache_path,
        }
    }

    pub fn apply_overrides(mut self, ov: &AppConfigOverrides) -> Self {
        if let Some(l) = &ov.log_level {
            self.log_level = l.clone();
        }
        if let Some(p) = ov.oauth_redirect_port {
            self.oauth_redirect_port = p;
        }
        if let Some(t) = ov.thumbnails_preload {
            self.thumbnails_preload = t;
        }
        if let Some(s) = ov.sync_interval_minutes {
            self.sync_interval_minutes = s;
        }
        if ov.debug_console {
            self.debug_console = true;
        }
        self
    }
}
