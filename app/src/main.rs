#![warn(clippy::all)]
#![warn(rust_2018_idioms)]
//! Main application entry point for GooglePicz.

use auth::{authenticate, ensure_access_token_valid};
use clap::Parser;
use std::path::PathBuf;
use sync::{Syncer, SyncTaskError};
use tokio::fs;
use tokio::task::LocalSet;
use tokio::time::Duration;
use tracing::{error, info};
use tracing_appender::rolling;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;
#[cfg(feature = "trace-spans")]
use sysinfo::{System, SystemExt};
#[derive(serde::Deserialize)]
struct PrevSyncState {
    page_token: Option<String>,
    total_synced: u64,
    last_success: Option<chrono::DateTime<chrono::Utc>>,
}
#[cfg(feature = "tokio-console")]
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
    /// Override number of concurrent image preloads
    #[arg(long)]
    preload_threads: Option<usize>,
    /// Override sync interval in minutes
    #[arg(long)]
    sync_interval_minutes: Option<u64>,
    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,
    /// Enable tokio console for debugging
    #[arg(long)]
    debug_console: bool,
    /// Enable tracing spans instrumentation
    #[arg(long)]
    trace_spans: bool,
    /// Detect faces after downloading images
    #[arg(long)]
    detect_faces: bool,
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let overrides = config::AppConfigOverrides {
        log_level: cli.log_level.clone(),
        oauth_redirect_port: cli.oauth_redirect_port,
        thumbnails_preload: cli.thumbnails_preload,
        preload_threads: cli.preload_threads,
        sync_interval_minutes: cli.sync_interval_minutes,
        debug_console: cli.debug_console,
        trace_spans: cli.trace_spans,
        detect_faces: cli.detect_faces,
    };
    let cfg = config::AppConfig::load_from(cli.config.clone()).apply_overrides(&overrides);
    let log_dir = cfg.cache_path.clone();
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = rolling::daily(&log_dir, "googlepicz.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(cfg.log_level.clone()))
        .with_writer(std::io::stdout.and(file_writer))
        .init();

    if cfg.debug_console {
        #[cfg(feature = "tokio-console")]
        {
            let _ = std::env::var("TOKIO_CONSOLE");
            console_subscriber::init();
        }
    }

    let local = LocalSet::new();
    local.run_until(main_inner(cfg)).await
}

#[cfg_attr(feature = "trace-spans", tracing::instrument(skip(cfg)))]
async fn main_inner(cfg: config::AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("🚀 Starting GooglePicz - Google Photos Manager");
    #[cfg(feature = "trace-spans")]
    let start = std::time::Instant::now();
    #[cfg(feature = "trace-spans")]
    let mut sys = System::new();
    #[cfg(feature = "trace-spans")]
    sys.refresh_memory();
    #[cfg(feature = "trace-spans")]
    let mem_before = sys.used_memory();

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
    let cache_dir = cfg.cache_path.clone();

    let db_path = cache_dir.join("cache.sqlite");

    let state_path = db_path.with_extension("state.json");
    if let Ok(data) = fs::read_to_string(&state_path).await {
        if let Ok(state) = serde_json::from_str::<PrevSyncState>(&data) {
            if state.page_token.is_some() {
                let last = state
                    .last_success
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_else(|| "unknown".into());
                info!("⚠️ Previous sync interrupted after {} items. Last success at {}", state.total_synced, last);
            }
        }
    }

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
            syncer.set_face_detection(cfg.detect_faces);
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let (err_tx, err_rx) = tokio::sync::mpsc::unbounded_channel::<SyncTaskError>();
            let (status_tx, status_rx) = tokio::sync::mpsc::unbounded_channel::<SyncTaskError>();
            let preload = cfg.thumbnails_preload;

            let interval = Duration::from_secs(cfg.sync_interval_minutes * 60);

            info!("📥 Starting synchronization...");
            if ensure_access_token_valid().await.is_ok() {
                if let Err(e) = syncer
                    .sync_media_items(
                        Some(tx.clone()),
                        Some(err_tx.clone()),
                        Some(tx.clone()),
                        Some(err_tx.clone()),
                    )
                    .await
                {
                    error!("❌ Synchronization failed: {}", e);
                }
            } else {
                error!("❌ Cannot synchronize without a valid access token");
            }

            let (sync_handle, sync_shutdown) = if ensure_access_token_valid().await.is_ok() {
                syncer.start_periodic_sync(
                    interval,
                    tx.clone(),
                    err_tx.clone(),
                    Some(status_tx.clone()),
                    Some(tx.clone()),
                    Some(err_tx.clone()),
                )
            } else {
                error!("❌ Cannot start periodic sync without a valid token");
                syncer.start_periodic_sync(
                    interval,
                    tx.clone(),
                    err_tx.clone(),
                    Some(status_tx.clone()),
                    Some(tx.clone()),
                    Some(err_tx.clone()),
                )
            };

            let (refresh_handle, refresh_shutdown) =
                Syncer::start_token_refresh_task(
                    Duration::from_secs(60),
                    err_tx.clone(),
                    Some(err_tx.clone()),
                );

            #[cfg(feature = "trace-spans")]
            {
                sys.refresh_memory();
                tracing::info!(target = "app", "startup_time_ms" = start.elapsed().as_millis(),
                               "mem_before_kb" = mem_before, "mem_after_kb" = sys.used_memory());
            }


            let ui_thread = std::thread::spawn(move || {
                if let Err(e) = ui::run(
                    Some(rx),
                    Some(err_rx),
                    Some(status_rx),
                    preload,
                    cfg.preload_threads,
                    cache_dir,
                ) {
                    error!("UI error: {}", e);
                }
            });

            if let Err(e) = ui_thread.join() {
                error!("UI thread panicked: {:?}", e);
            }
            let _ = sync_shutdown.send(());
            let _ = refresh_shutdown.send(());
            let _ = sync_handle.await;
            let _ = refresh_handle.await;
        }
        Err(e) => {
            error!("❌ Failed to initialize syncer: {}", e);
            error!("💡 The UI will still start, but photos may not be available until sync is working.");
            #[cfg(feature = "trace-spans")]
            {
                sys.refresh_memory();
                tracing::info!(target = "app", "startup_time_ms" = start.elapsed().as_millis(),
                               "mem_before_kb" = mem_before, "mem_after_kb" = sys.used_memory());
            }
            ui::run(None, None, None, cfg.thumbnails_preload, cfg.preload_threads, cfg.cache_path.clone())?;
        }
    }

    Ok(())
}
