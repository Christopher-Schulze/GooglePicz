//! Main application entry point for GooglePicz.

use auth::{authenticate, get_access_token};
use sync::Syncer;
use tokio::time::Duration;
use tokio::task::LocalSet;
use ui;
use std::path::PathBuf;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let local = LocalSet::new();
    local.run_until(main_inner()).await
}

async fn main_inner() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting GooglePicz - Google Photos Manager");
    
    // Ensure environment variables are set for client ID and secret
    if std::env::var("GOOGLE_CLIENT_ID").is_err() || std::env::var("GOOGLE_CLIENT_SECRET").is_err() {
        eprintln!("❌ Error: GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables must be set.");
        eprintln!("📝 Please visit https://console.developers.google.com/ to create OAuth 2.0 credentials.");
        eprintln!("💡 Set them using:");
        eprintln!("   export GOOGLE_CLIENT_ID=your_client_id");
        eprintln!("   export GOOGLE_CLIENT_SECRET=your_client_secret");
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
        println!("📁 Cache directory: {:?}", parent);
    }

    // Check if we have a valid token
    let needs_auth = match get_access_token() {
        Ok(_) => {
            println!("✅ Found existing authentication token");
            false
        }
        Err(_) => {
            println!("🔐 No valid authentication token found");
            true
        }
    };

    // Authenticate if needed
    if needs_auth {
        println!("🔑 Starting authentication process...");
        match authenticate().await {
            Ok(_) => println!("✅ Authentication successful!"),
            Err(e) => {
                eprintln!("❌ Authentication failed: {}", e);
                eprintln!("💡 Please ensure your GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET are correct and you have internet access.");
                return Ok(());
            }
        }
    }

    println!("🔄 Initializing synchronization...");
    match Syncer::new(&db_path).await {
        Ok(syncer) => {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let ui_thread = std::thread::spawn(move || {
                if let Err(e) = ui::run(Some(rx)) {
                    eprintln!("UI error: {}", e);
                }
            });

            let interval_minutes: u64 = std::env::var("GOOGLEPICZ_SYNC_INTERVAL_MINUTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5);
            let interval = Duration::from_secs(interval_minutes * 60);

            println!("📥 Starting synchronization...");
            if let Err(e) = syncer.sync_media_items(Some(tx.clone())).await {
                eprintln!("❌ Synchronization failed: {}", e);
            }

            let _handle = syncer.start_periodic_sync(interval, tx);

            ui_thread.join().expect("UI thread panicked");
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize syncer: {}", e);
            eprintln!("💡 The UI will still start, but photos may not be available until sync is working.");
            ui::run(None)?;
        }
    }

    Ok(())
}