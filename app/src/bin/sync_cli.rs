use clap::{Parser, Subcommand};
use std::path::PathBuf;
use dirs;
use tokio::sync::mpsc;
use sync::{Syncer, SyncProgress};
use cache::CacheManager;

#[derive(Parser)]
#[command(name = "sync_cli", about = "GooglePicz synchronization CLI")]
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let cache_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".googlepicz");
    let db_path = cache_dir.join("cache.sqlite");

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
    }

    Ok(())
}
