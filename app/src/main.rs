//! Main application entry point for GooglePicz.

use auth::{authenticate, get_access_token};
use sync::Syncer;
use tokio::time::Duration;
use tokio::task::LocalSet;
use ui;
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::AppConfig::load();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(cfg.log_level.clone()))
        .init();

    // make credentials available for auth crate
    if !cfg.client_id.is_empty() {
        std::env::set_var("GOOGLE_CLIENT_ID", &cfg.client_id);
    }
    if !cfg.client_secret.is_empty() {
        std::env::set_var("GOOGLE_CLIENT_SECRET", &cfg.client_secret);
    }

    let local = LocalSet::new();
    local.run_until(main_inner(cfg)).await
}

async fn main_inner(cfg: config::AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("🚀 Starting GooglePicz - Google Photos Manager");
    
    // Ensure credentials are present
    if cfg.client_id.is_empty() || cfg.client_secret.is_empty() {
        error!("❌ Error: client_id and client_secret must be configured in ~/.googlepicz/config.toml");
        return Ok(());
    }

    // Setup cache directory
    let cache_dir = cfg.cache_dir.clone();
    let db_path = cache_dir.join("cache.sqlite");
    
    // Ensure the directory exists
    fs::create_dir_all(&cache_dir).await?;
    info!("📁 Cache directory: {:?}", cache_dir);

    // Check if we have a valid token
    let needs_auth = match get_access_token() {
        Ok(_) => {
            info!("✅ Found existing authentication token");
            false
        }
        Err(_) => {
            info!("🔐 No valid authentication token found");
            true
        }
    };

    // Authenticate if needed
    if needs_auth {
        info!("🔑 Starting authentication process...");
        match authenticate().await {
            Ok(_) => info!("✅ Authentication successful!"),
            Err(e) => {
                error!("❌ Authentication failed: {}", e);
                error!("💡 Please ensure your GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET are correct and you have internet access.");
                return Ok(());
            }
        }
    }

    info!("🔄 Initializing synchronization...");
    match Syncer::new(&db_path).await {
        Ok(mut syncer) => {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let ui_thread = std::thread::spawn(move || {
                if let Err(e) = ui::run(Some(rx)) {
                    error!("UI error: {}", e);
                }
            });

            let interval = Duration::from_secs(cfg.sync_interval_minutes * 60);

            info!("📥 Starting synchronization...");
            if let Err(e) = syncer.sync_media_items(Some(tx.clone())).await {
                error!("❌ Synchronization failed: {}", e);
            }

            let _handle = syncer.start_periodic_sync(interval, tx);

            ui_thread.join().expect("UI thread panicked");
        }
        Err(e) => {
            error!("❌ Failed to initialize syncer: {}", e);
            error!("💡 The UI will still start, but photos may not be available until sync is working.");
            ui::run(None)?;
        }
    }

    Ok(())
}