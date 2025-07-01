use clap::{Parser, Subcommand};
use std::path::PathBuf;
use dirs::home_dir;
use tokio::sync::mpsc;
use sync::{Syncer, SyncProgress};
use cache::CacheManager;
use tracing_subscriber::EnvFilter;
use tracing_appender::rolling;
use tracing_subscriber::fmt::writer::MakeWriterExt;

#[path = "../config.rs"]
mod config;

#[derive(Parser)]
#[command(name = "sync_cli", author, version, about = "GooglePicz synchronization CLI")]
struct Cli {
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let cfg = config::AppConfig::load();
    let base_dir = home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".googlepicz");
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
                        SyncProgress::ItemSynced(n) => println!("Synced {} items...", n),
                        SyncProgress::Finished(total) => println!("Finished sync: {} items", total),
                    }
                }
            });
            syncer.sync_media_items(Some(tx)).await?;
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
                let title = album.title.clone().unwrap_or_else(|| "Untitled".to_string());
                println!("{} (id: {})", title, album.id);
            }
        }
    }

    Ok(())
}
