//! Main application entry point for GooglePicz.

use auth::{authenticate, get_access_token};
use sync::Syncer;
use ui;
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info};

mod config;
use config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load().unwrap_or_default();
    let level = config
        .log_level
        .parse::<tracing::Level>()
        .unwrap_or(tracing::Level::INFO);
    tracing_subscriber::fmt().with_max_level(level).init();

    info!("🚀 Starting GooglePicz - Google Photos Manager");
    
    // Ensure environment variables are set for client ID and secret
    if std::env::var("GOOGLE_CLIENT_ID").is_err() || std::env::var("GOOGLE_CLIENT_SECRET").is_err() {
        error!("❌ Error: GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables must be set.");
        error!("📝 Please visit https://console.developers.google.com/ to create OAuth 2.0 credentials.");
        error!("💡 Set them using:");
        error!("   export GOOGLE_CLIENT_ID=your_client_id");
        error!("   export GOOGLE_CLIENT_SECRET=your_client_secret");
        return Ok(());
    }

    // Setup cache directory
    let cache_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".googlepicz");
    
    let db_path = cache_dir.join("cache.sqlite");
    
    // Ensure the directory exists
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).await?;
        info!("📁 Cache directory: {:?}", parent);
    }

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
        Ok(syncer) => {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let ui_thread = std::thread::spawn(move || {
                if let Err(e) = ui::run(Some(rx)) {
                    error!("UI error: {}", e);
                }
            });

            info!("📥 Starting synchronization...");
            if let Err(e) = syncer.sync_media_items(Some(tx)).await {
                error!("❌ Synchronization failed: {}", e);
            }

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