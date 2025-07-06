use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub log_level: String,
    pub oauth_redirect_port: u16,
    pub thumbnails_preload: usize,
    pub preload_threads: usize,
    pub sync_interval_minutes: u64,
    pub debug_console: bool,
    pub trace_spans: bool,
    pub detect_faces: bool,
    pub cache_path: PathBuf,
}

pub struct AppConfigOverrides {
    pub log_level: Option<String>,
    pub oauth_redirect_port: Option<u16>,
    pub thumbnails_preload: Option<usize>,
    pub preload_threads: Option<usize>,
    pub sync_interval_minutes: Option<u64>,
    pub debug_console: bool,
    pub trace_spans: bool,
    pub detect_faces: bool,
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
        let preload_threads = cfg.get_int("preload_threads").unwrap_or(4) as usize;
        let sync_interval_minutes = cfg.get_int("sync_interval_minutes").unwrap_or(5) as u64;
        let debug_console = cfg.get_bool("debug_console").unwrap_or(false);
        let trace_spans = cfg.get_bool("trace_spans").unwrap_or(false);
        let detect_faces = cfg.get_bool("detect_faces").unwrap_or(false);
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
            preload_threads,
            sync_interval_minutes,
            debug_console,
            trace_spans,
            detect_faces,
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
        if let Some(pt) = ov.preload_threads {
            self.preload_threads = pt;
        }
        if let Some(s) = ov.sync_interval_minutes {
            self.sync_interval_minutes = s;
        }
        if ov.debug_console {
            self.debug_console = true;
        }
        if ov.trace_spans {
            self.trace_spans = true;
        }
        if ov.detect_faces {
            self.detect_faces = true;
        }
        self
    }

    pub fn save_to(&self, path: Option<PathBuf>) -> std::io::Result<()> {
        let path = match path {
            Some(p) => p,
            None => dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".googlepicz")
                .join("config"),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = toml::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, data)
    }
}
