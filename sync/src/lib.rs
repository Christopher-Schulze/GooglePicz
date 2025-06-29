//! Synchronization module for Google Photos data.

use api_client::{ApiClient, ApiClientError, Filters, DateFilter, DateRange, Date};
use auth::{ensure_access_token_valid, get_access_token};
use cache::{CacheManager, CacheError};
use std::path::Path;
use std::error::Error;
use std::fmt;
use tokio::time::{sleep, Duration};
use chrono::{DateTime, Utc, Datelike};
use tokio::sync::mpsc;
use tokio::task::{spawn_local, JoinHandle};

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

        let last_sync = self.cache_manager
            .get_last_sync()
            .map_err(|e| SyncError::CacheError(format!("Failed to read last sync: {}", e)))?;

        let filters = if let Some(ts) = &last_sync {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                let dt = dt.with_timezone(&Utc);
                Some(Filters {
                    date_filter: Some(DateFilter {
                        ranges: vec![DateRange {
                            start_date: Date {
                                year: dt.year(),
                                month: dt.month(),
                                day: dt.day(),
                            },
                            end_date: None,
                        }],
                    }),
                })
            } else {
                None
            }
        } else {
            None
        };

        loop {
            let token = ensure_access_token_valid().await
                .map_err(|e| SyncError::AuthenticationError(format!("Failed to refresh token: {}", e)))?;
            self.api_client.set_access_token(token);

            let (media_items, next_page_token) = self.api_client
                .search_media_items(None, 100, page_token.clone(), filters.clone())
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
            .set_last_sync(&Utc::now().to_rfc3339())
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
    use auth::{authenticate, get_access_token};
    use tempfile::NamedTempFile;

    #[tokio::test]
    #[ignore] // Requires manual authentication and environment variables
    async fn test_sync_media_items() {
        // Ensure GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET are set in your environment
        // and you have authenticated at least once.
        // For testing, you might need to call `authenticate().await` or ensure a valid token exists.
        // For a real application, you'd have a proper token management system.

        // Attempt to authenticate if no token is found
        if get_access_token().is_err() {
            tracing::error!("No access token found. Attempting to authenticate...");
            authenticate().await.expect("Failed to authenticate for sync test");
        }

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path();

        let mut syncer = Syncer::new(db_path).await.expect("Failed to create syncer");
        let result = syncer.sync_media_items(None).await;
        assert!(result.is_ok(), "Synchronization failed: {:?}", result.err());

        let all_cached_items = syncer.cache_manager.get_all_media_items().expect("Failed to get all cached items");
        tracing::info!("Total items in cache after sync: {}", all_cached_items.len());
        assert!(!all_cached_items.is_empty());
    }
}