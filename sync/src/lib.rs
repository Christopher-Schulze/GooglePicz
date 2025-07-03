//! Synchronization module for Google Photos data.

use api_client::ApiClient;
use auth::ensure_access_token_valid;
use cache::CacheManager;
use chrono::{DateTime, Datelike, Utc};
use serde_json::json;
use thiserror::Error;
use std::path::Path;
use tokio::sync::mpsc;
use tokio::task::{spawn_local, JoinHandle};
use tokio::time::{sleep, Duration};

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Authentication Error: {0}")]
    AuthenticationError(String),
    #[error("API Client Error: {0}")]
    ApiClientError(String),
    #[error("Cache Error: {0}")]
    CacheError(String),
    #[error("Other Error: {0}")]
    Other(String),
}

pub struct Syncer {
    api_client: ApiClient,
    cache_manager: CacheManager,
}

#[derive(Debug, Clone)]
pub enum SyncProgress {
    ItemSynced(u64),
    Finished(u64),
}

impl Syncer {
    pub async fn new(db_path: &Path) -> Result<Self, SyncError> {
        let access_token = ensure_access_token_valid().await.map_err(|e| {
            SyncError::AuthenticationError(format!("Failed to get access token: {}", e))
        })?;

        let api_client = ApiClient::new(access_token);

        let cache_manager = CacheManager::new(db_path)
            .map_err(|e| SyncError::CacheError(format!("Failed to create cache manager: {}", e)))?;

        Ok(Syncer {
            api_client,
            cache_manager,
        })
    }

    pub async fn sync_media_items(
        &mut self,
        progress: Option<mpsc::UnboundedSender<SyncProgress>>,
        error: Option<mpsc::UnboundedSender<String>>,
    ) -> Result<(), SyncError> {
        tracing::info!("Starting media item synchronization...");
        let mut page_token: Option<String> = None;
        let mut total_synced = 0;

        let last_sync = match self.cache_manager.get_last_sync() {
            Ok(ts) => ts,
            Err(e) => {
                let msg = format!("Failed to get last sync time: {}", e);
                if let Some(tx) = &error {
                    let _ = tx.send(msg.clone());
                }
                DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH)
            }
        };
        let filter = json!({
            "dateFilter": {
                "ranges": [{
                    "startDate": {
                        "year": last_sync.year(),
                        "month": last_sync.month(),
                        "day": last_sync.day()
                    }
                }]
            }
        });

        loop {
            let token = ensure_access_token_valid().await.map_err(|e| {
                let msg = format!("Failed to refresh token: {}", e);
                if let Some(tx) = &error {
                    let _ = tx.send(msg.clone());
                }
                SyncError::AuthenticationError(msg)
            })?;
            self.api_client.set_access_token(token);

            let (media_items, next_page_token) = self
                .api_client
                .search_media_items(None, 100, page_token.clone(), Some(filter.clone()))
                .await
                .map_err(|e| {
                    let msg = format!("Failed to list media items from API: {}", e);
                    if let Some(tx) = &error {
                        let _ = tx.send(msg.clone());
                    }
                    SyncError::ApiClientError(msg)
                })?;

            if media_items.is_empty() {
                break;
            }

            for item in media_items {
                self.cache_manager.insert_media_item(&item).map_err(|e| {
                    let msg = format!("Failed to insert media item into cache: {}", e);
                    if let Some(tx) = &error {
                        let _ = tx.send(msg.clone());
                    }
                    SyncError::CacheError(msg)
                })?;
                total_synced += 1;
                if let Some(tx) = &progress {
                    let _ = tx.send(SyncProgress::ItemSynced(total_synced));
                }
            }

            tracing::info!("Synced {} media items so far.", total_synced);

            if next_page_token.is_none() {
                break;
            }
            page_token = next_page_token;

            // Be a good API citizen: wait a bit between pages
            sleep(Duration::from_millis(500)).await;
        }

        tracing::info!(
            "Synchronization complete. Total media items synced: {}.",
            total_synced
        );
        if let Some(tx) = progress {
            let _ = tx.send(SyncProgress::Finished(total_synced));
        }
        self.cache_manager
            .update_last_sync(Utc::now())
            .map_err(|e| {
                let msg = format!("Failed to update last sync: {}", e);
                if let Some(tx) = &error {
                    let _ = tx.send(msg.clone());
                }
                SyncError::CacheError(msg)
            })?;
        Ok(())
    }

    pub fn start_periodic_sync(
        self,
        interval: Duration,
        progress_tx: mpsc::UnboundedSender<SyncProgress>,
        error_tx: mpsc::UnboundedSender<String>,
    ) -> JoinHandle<()> {
        spawn_local(async move {
            let mut syncer = self;
            loop {
                if let Err(e) =
                    syncer
                        .sync_media_items(Some(progress_tx.clone()), Some(error_tx.clone()))
                        .await
                {
                    tracing::error!("Periodic sync failed: {}", e);
                    let _ = error_tx.send(format!("Periodic sync failed: {}", e));
                }
                sleep(interval).await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth::{authenticate, ensure_access_token_valid};
    use serial_test::serial;
    use tempfile::NamedTempFile;

    #[tokio::test]
    #[serial]
    async fn test_sync_media_items() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_ACCESS_TOKEN", "token");
        std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
        std::env::set_var("MOCK_API_CLIENT", "1");
        // Ensure GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET are set in your environment
        // and you have authenticated at least once.
        // For testing, you might need to call `authenticate().await` or ensure a valid token exists.
        // For a real application, you'd have a proper token management system.

        // Attempt to authenticate if no token is found
        if ensure_access_token_valid().await.is_err() {
            authenticate(8080)
                .await
                .expect("Failed to authenticate for sync test");
        }

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();

        let mut syncer = Syncer::new(db_path).await.expect("Failed to create syncer");
        let result = syncer.sync_media_items(None, None).await;
        assert!(result.is_ok(), "Synchronization failed: {:?}", result.err());

        let all_cached_items = syncer
            .cache_manager
            .get_all_media_items()
            .expect("Failed to get all cached items");
        tracing::info!(
            "Total items in cache after sync: {}",
            all_cached_items.len()
        );
        assert!(!all_cached_items.is_empty());
        std::env::remove_var("MOCK_KEYRING");
        std::env::remove_var("MOCK_ACCESS_TOKEN");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_API_CLIENT");
    }
}
