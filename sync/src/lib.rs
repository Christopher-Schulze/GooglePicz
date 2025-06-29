//! Synchronization module for Google Photos data.

use api_client::ApiClient;
use auth::ensure_access_token_valid;
use cache::{CacheError, CacheManager};
use std::path::Path;
use std::error::Error;
use std::fmt;
use tokio::time::{sleep, Duration};
use tokio::sync::mpsc;
use tokio::task::{spawn_local, JoinHandle};
use chrono::{DateTime, Utc, Datelike};
use serde_json::json;

#[derive(Debug)]
pub enum SyncError {
    AuthenticationError(String),
    ApiClientError(String),
    CacheError(String),
    Other(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SyncError::AuthenticationError(msg) => write!(f, "Authentication Error: {}", msg),
            SyncError::ApiClientError(msg) => write!(f, "API Client Error: {}", msg),
            SyncError::CacheError(msg) => write!(f, "Cache Error: {}", msg),
            SyncError::Other(msg) => write!(f, "Other Error: {}", msg),
        }
    }
}

impl Error for SyncError {}

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
        let access_token = ensure_access_token_valid().await
            .map_err(|e| SyncError::AuthenticationError(format!("Failed to get access token: {}", e)))?;

        let api_client = ApiClient::new(access_token);

        let cache_manager = CacheManager::new(db_path)
            .map_err(|e| SyncError::CacheError(format!("Failed to create cache manager: {}", e)))?;

        Ok(Syncer { api_client, cache_manager })
    }

    pub async fn sync_media_items(
        &mut self,
        progress: Option<mpsc::UnboundedSender<SyncProgress>>,
    ) -> Result<(), SyncError> {
        tracing::info!("Starting media item synchronization...");
        let mut page_token: Option<String> = None;
        let mut total_synced = 0;

        let last_sync = match self.cache_manager.get_last_sync() {
            Ok(ts) => ts,
            Err(_) => DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH),
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
            let token = ensure_access_token_valid().await
                .map_err(|e| SyncError::AuthenticationError(format!("Failed to refresh token: {}", e)))?;
            self.api_client.set_access_token(token);

            let (media_items, next_page_token) = self
                .api_client
                .search_media_items(None, 100, page_token.clone(), Some(filter.clone()))
                .await
                .map_err(|e| SyncError::ApiClientError(format!("Failed to list media items from API: {}", e)))?;

            if media_items.is_empty() {
                break;
            }

            for item in media_items {
                self.cache_manager.insert_media_item(&item)
                    .map_err(|e| SyncError::CacheError(format!("Failed to insert media item into cache: {}", e)))?;
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

        tracing::info!("Synchronization complete. Total media items synced: {}.", total_synced);
        if let Some(tx) = progress {
            let _ = tx.send(SyncProgress::Finished(total_synced));
        }
        self.cache_manager
            .update_last_sync(Utc::now())
            .map_err(|e| SyncError::CacheError(format!("Failed to update last sync: {}", e)))?;
        Ok(())
    }

    pub fn start_periodic_sync(
        self,
        interval: Duration,
        tx: mpsc::UnboundedSender<SyncProgress>,
    ) -> JoinHandle<()> {
        spawn_local(async move {
            let mut syncer = self;
            loop {
                if let Err(e) = syncer.sync_media_items(Some(tx.clone())).await {
                    tracing::error!("Periodic sync failed: {}", e);
                }
                sleep(interval).await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;

    #[tokio::test]
    async fn test_sync_media_items() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_KEYRING_refresh_token", "stored_refresh");
        std::env::set_var("MOCK_REFRESH_TOKEN", "access1");

        let server = MockServer::start();
        std::env::set_var("GOOGLE_PHOTOS_BASE_URL", server.url(""));

        server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/mediaItems:search");
                then.status(200)
                    .json_body(json!({
                        "mediaItems": [{
                            "id": "1",
                            "description": null,
                            "productUrl": "http://example.com",
                            "baseUrl": "http://example.com/base",
                            "mimeType": "image/jpeg",
                            "mediaMetadata": {"creationTime": "now", "width": "1", "height": "1"},
                            "filename": "img1.jpg"
                        }]
                    }));
            })
            .await;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();

        let mut syncer = Syncer::new(db_path).await.expect("Failed to create syncer");
        syncer.sync_media_items(None).await.unwrap();

        let items = syncer
            .cache_manager
            .get_all_media_items()
            .expect("Failed to get items");
        assert_eq!(items.len(), 1);
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("GOOGLE_PHOTOS_BASE_URL");
        std::env::remove_var("MOCK_KEYRING");
    }
}