//! Main application entry point for GooglePicz.

use auth::{authenticate, get_access_token};
use sync::Syncer;
use ui;
use std::path::PathBuf;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Initialize syncer and start background sync
    println!("🔄 Initializing synchronization...");
    match Syncer::new(&db_path).await {
        Ok(syncer) => {
            // Start synchronization in a separate task
            tokio::spawn(async move {
                println!("📥 Starting background synchronization...");
                match syncer.sync_media_items().await {
                    Ok(_) => println!("✅ Initial synchronization completed"),
                    Err(e) => eprintln!("❌ Synchronization failed: {}", e),
                }
            });
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize syncer: {}", e);
            eprintln!("💡 The UI will still start, but photos may not be available until sync is working.");
        }
    }

    // Start the UI
    println!("🎨 Starting GooglePicz UI...");
    ui::run()?;

    Ok(());
}