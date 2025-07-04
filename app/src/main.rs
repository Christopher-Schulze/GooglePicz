//! Main application entry point for GooglePicz.

use auth::{authenticate, ensure_access_token_valid};
use clap::Parser;
use std::path::PathBuf;
use sync::Syncer;
use tokio::fs;
use tokio::task::LocalSet;
use tokio::time::Duration;
use tracing::{error, info};
use tracing_appender::rolling;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;
#[cfg(feature = "console")]
use console_subscriber;
use ui;
mod config;

#[derive(Parser, Debug)]
#[command(name = "googlepicz", about = "Google Photos Desktop Client")]
struct Cli {
    /// Override log level (e.g. info, debug)
    #[arg(long)]
    log_level: Option<String>,
    /// Override OAuth redirect port
    #[arg(long)]
    oauth_redirect_port: Option<u16>,
    /// Override number of thumbnails to preload
    #[arg(long)]
    thumbnails_preload: Option<usize>,
    /// Override sync interval in minutes
    #[arg(long)]
    sync_interval_minutes: Option<u64>,
    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,
    /// Enable tokio console for debugging
    #[arg(long)]
    debug_console: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let overrides = config::AppConfigOverrides {
        log_level: cli.log_level.clone(),
        oauth_redirect_port: cli.oauth_redirect_port,
        thumbnails_preload: cli.thumbnails_preload,
        sync_interval_minutes: cli.sync_interval_minutes,
        debug_console: cli.debug_console,
    };
    let cfg = config::AppConfig::load_from(cli.config.clone()).apply_overrides(&overrides);
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".googlepicz");
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = rolling::daily(&log_dir, "googlepicz.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(cfg.log_level.clone()))
        .with_writer(std::io::stdout.and(file_writer))
        .init();

    if cfg.debug_console {
        #[cfg(feature = "console")]
        {
            let _ = std::env::var("TOKIO_CONSOLE");
            console_subscriber::init();
        }
    }

    let local = LocalSet::new();
    local.run_until(main_inner(cfg)).await
}

async fn main_inner(cfg: config::AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("🚀 Starting GooglePicz - Google Photos Manager");

    // Ensure environment variables are set for client ID and secret
    if std::env::var("GOOGLE_CLIENT_ID").is_err() || std::env::var("GOOGLE_CLIENT_SECRET").is_err()
    {
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

    // Check if we have a valid token, refreshing if necessary
    let needs_auth = match ensure_access_token_valid().await {
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
        match authenticate(cfg.oauth_redirect_port).await {
            Ok(_) => info!("✅ Authentication successful!"),
            Err(e) => {
                error!("❌ Authentication failed: {}", e);
                error!("💡 Please ensure your GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET are correct and you have internet access.");
                return Ok(());
            }
        }
    }

    // Always ensure we have a valid token before continuing
    if let Err(e) = ensure_access_token_valid().await {
        error!("❌ Failed to validate access token: {}", e);
        return Ok(());
    }

    info!("🔄 Initializing synchronization...");
    match Syncer::new(&db_path).await {
        Ok(mut syncer) => {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let preload = cfg.thumbnails_preload;

            let interval = Duration::from_secs(cfg.sync_interval_minutes * 60);

            info!("📥 Starting synchronization...");
            if ensure_access_token_valid().await.is_ok() {
                if let Err(e) = syncer.sync_media_items(Some(tx.clone()), None).await {
                    error!("❌ Synchronization failed: {}", e);
                }
            } else {
                error!("❌ Cannot synchronize without a valid access token");
            }

            let (handle, shutdown, err_rx) = if ensure_access_token_valid().await.is_ok() {
                syncer.start_periodic_sync(interval, tx)
            } else {
                error!("❌ Cannot start periodic sync without a valid token");
                syncer.start_periodic_sync(interval, tx)
            };

            let ui_thread = std::thread::spawn(move || {
                if let Err(e) = ui::run(Some(rx), Some(err_rx), preload) {
                    error!("UI error: {}", e);
                }
            });

            if let Err(e) = ui_thread.join() {
                error!("UI thread panicked: {:?}", e);
            }
            let _ = shutdown.send(());
            let _ = handle.await;
        }
        Err(e) => {
            error!("❌ Failed to initialize syncer: {}", e);
            error!("💡 The UI will still start, but photos may not be available until sync is working.");
            ui::run(None, None, cfg.thumbnails_preload)?;
        }
    }

    Ok(())
}
