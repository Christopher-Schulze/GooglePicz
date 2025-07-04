use cache::CacheManager;
use clap::{Parser, Subcommand};
use api_client::ApiClient;
use auth::ensure_access_token_valid;
use std::path::PathBuf;
use sync::{SyncProgress, Syncer};
use tokio::sync::mpsc;
use tracing_appender::rolling;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

#[path = "../config.rs"]
mod config;

#[derive(Parser)]
#[command(
    name = "sync_cli",
    author,
    version,
    about = "GooglePicz synchronization CLI"
)]
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
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Perform a full synchronization
    Sync,
    /// Show last sync time and cached item count
    Status,
    /// Delete all cached media items
    ClearCache,
    /// Display all cached albums
    ListAlbums,
    /// Create a new album
    CreateAlbum {
        /// Title of the new album
        title: String,
    },
    /// Delete an existing album
    DeleteAlbum {
        /// ID of the album to delete
        id: String,
    },
    /// Show statistics about cached data
    CacheStats,
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
    let base_dir = cfg.cache_path.clone();
    std::fs::create_dir_all(&base_dir)?;
    let file_appender = rolling::daily(&base_dir, "googlepicz.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(cfg.log_level.clone()))
        .with_writer(std::io::stdout.and(file_writer))
        .init();

    let db_path = base_dir.join("cache.sqlite");

    match cli.command {
        Commands::Sync => {
            let mut syncer = Syncer::new(&db_path).await?;
            let (tx, mut rx) = mpsc::unbounded_channel();
            tokio::spawn(async move {
                while let Some(p) = rx.recv().await {
                    match p {
                        SyncProgress::Started => println!("Sync started"),
                        SyncProgress::Retrying(wait) => println!("Retrying in {}s", wait),
                        SyncProgress::ItemSynced(n) => println!("Synced {} items...", n),
                        SyncProgress::Finished(total) => println!("Finished sync: {} items", total),
                    }
                }
            });
            syncer.sync_media_items(Some(tx), None).await?;
        }
        Commands::Status => {
            if !db_path.exists() {
                println!("No cache found at {:?}", db_path);
                return Ok(());
            }
            let cache = CacheManager::new(&db_path)?;
            let last = cache.get_last_sync()?;
            let count = cache.get_all_media_items()?.len();
            println!("Last sync: {}", last.to_rfc3339());
            println!("Cached items: {}", count);
        }
        Commands::ClearCache => {
            if !db_path.exists() {
                println!("No cache found at {:?}", db_path);
                return Ok(());
            }
            let cache = CacheManager::new(&db_path)?;
            cache.clear_cache()?;
            println!("Cache cleared");
        }
        Commands::ListAlbums => {
            if !db_path.exists() {
                println!("No cache found at {:?}", db_path);
                return Ok(());
            }
            let cache = CacheManager::new(&db_path)?;
            let albums = cache.get_all_albums()?;
            for album in albums {
                let title = album
                    .title
                    .clone()
                    .unwrap_or_else(|| "Untitled".to_string());
                println!("{} (id: {})", title, album.id);
            }
        }
        Commands::CreateAlbum { title } => {
            if !db_path.exists() {
                println!("No cache found at {:?}", db_path);
                return Ok(());
            }
            let token = ensure_access_token_valid().await?;
            let client = ApiClient::new(token);
            let album = client.create_album(&title).await?;
            let cache = CacheManager::new(&db_path)?;
            cache.insert_album(&album)?;
            let shown_title = album.title.unwrap_or(title);
            println!("Album created: {} (id: {})", shown_title, album.id);
        }
        Commands::DeleteAlbum { id } => {
            if !db_path.exists() {
                println!("No cache found at {:?}", db_path);
                return Ok(());
            }
            let token = ensure_access_token_valid().await?;
            let client = ApiClient::new(token);
            client.delete_album(&id).await?;
            let cache = CacheManager::new(&db_path)?;
            cache.delete_album(&id)?;
            println!("Album deleted: {}", id);
        }
        Commands::CacheStats => {
            if !db_path.exists() {
                println!("No cache found at {:?}", db_path);
                return Ok(());
            }
            let cache = CacheManager::new(&db_path)?;
            let albums = cache.get_all_albums()?.len();
            let items = cache.get_all_media_items()?.len();
            println!("Albums: {}", albums);
            println!("Media items: {}", items);
        }
    }

    Ok(())
}
