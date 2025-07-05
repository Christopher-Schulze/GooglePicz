//! Synchronization module for Google Photos data.

use api_client::ApiClient;
use auth::ensure_access_token_valid;
use cache::CacheManager;
use chrono::{DateTime, Datelike, Utc};
use serde_json::json;
use thiserror::Error;
use std::path::Path;
use tokio::sync::{mpsc, oneshot};
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
    Started,
    ItemSynced(u64),
    Retrying(u64),
    Finished(u64),
}

#[derive(Debug, Clone, Error)]
pub enum SyncTaskError {
    #[error("Periodic sync failed: {0}")]
    PeriodicSyncFailed(String),
    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),
    #[error("Other task error: {0}")]
    Other(String),
}

impl Syncer {
    #[cfg_attr(feature = "trace-spans", tracing::instrument)]
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

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, progress, error)))]
    pub async fn sync_media_items(
        &mut self,
        progress: Option<mpsc::UnboundedSender<SyncProgress>>,
        error: Option<mpsc::UnboundedSender<SyncTaskError>>,
    ) -> Result<(), SyncError> {
        tracing::info!("Starting media item synchronization...");
        if let Some(tx) = &progress {
            if let Err(e) = tx.send(SyncProgress::Started) {
                if let Some(err_tx) = &error {
                    let _ = err_tx.send(SyncTaskError::Other(format!(
                        "Failed to send progress: {}",
                        e
                    )));
                }
            }
        }
        let mut page_token: Option<String> = None;
        let mut total_synced = 0;

        let last_sync = match self.cache_manager.get_last_sync_async().await {
            Ok(ts) => ts,
            Err(e) => {
                let msg = format!("Failed to get last sync time: {}", e);
                if let Some(tx) = &error {
                    if let Err(send_err) = tx.send(SyncTaskError::Other(msg.clone())) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
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
                    if let Err(send_err) = tx.send(SyncTaskError::Other(msg.clone())) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
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
                        if let Err(send_err) = tx.send(SyncTaskError::Other(msg.clone())) {
                            tracing::error!("Failed to forward error: {}", send_err);
                        }
                    }
                    SyncError::ApiClientError(msg)
                })?;

            if media_items.is_empty() {
                break;
            }

            for item in media_items {
                self.cache_manager
                    .insert_media_item_async(item.clone())
                    .await
                    .map_err(|e| {
                        let msg = format!("Failed to insert media item into cache: {}", e);
                        if let Some(tx) = &error {
                            if let Err(send_err) = tx.send(SyncTaskError::Other(msg.clone())) {
                                tracing::error!("Failed to forward error: {}", send_err);
                            }
                        }
                        SyncError::CacheError(msg)
                    })?;
                total_synced += 1;
                if let Some(tx) = &progress {
                    if let Err(e) = tx.send(SyncProgress::ItemSynced(total_synced)) {
                        if let Some(err) = &error {
                            if let Err(send_err) = err.send(SyncTaskError::Other(format!(
                                "Failed to send progress update: {}",
                                e
                            ))) {
                                tracing::error!("Failed to forward error: {}", send_err);
                            }
                        }
                    }
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
        if let Some(tx) = &progress {
            if let Err(e) = tx.send(SyncProgress::Finished(total_synced)) {
                if let Some(err) = &error {
                    if let Err(send_err) = err.send(SyncTaskError::Other(format!(
                        "Failed to send progress update: {}",
                        e
                    ))) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
                }
            }
        }
        self.cache_manager
            .update_last_sync_async(Utc::now())
            .await
            .map_err(|e| {
                let msg = format!("Failed to update last sync: {}", e);
                if let Some(tx) = &error {
                    if let Err(send_err) = tx.send(SyncTaskError::Other(msg.clone())) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
                }
                SyncError::CacheError(msg)
            })?;
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, progress_tx, error_tx)))]
    pub fn start_periodic_sync(
        self,
        interval: Duration,
        progress_tx: mpsc::UnboundedSender<SyncProgress>,
        error_tx: mpsc::UnboundedSender<SyncTaskError>,
    ) -> (JoinHandle<()>, oneshot::Sender<()>) {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let handle = spawn_local(async move {
            let mut syncer = self;
            let mut backoff = 1u64;
            let mut last_success = match syncer.cache_manager.get_last_sync_async().await {
                Ok(ts) => ts,
                Err(e) => {
                    let msg = format!("Failed to get last sync time: {}", e);
                    if let Err(send_err) = error_tx.send(SyncTaskError::Other(msg.clone())) {
                        tracing::error!(error = ?send_err, "Failed to forward last_sync error");
                    }
                    DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH)
                }
            };
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    _ = async {
                        if let Err(e) =
                            syncer
                                .sync_media_items(Some(progress_tx.clone()), Some(error_tx.clone()))
                                .await
                        {
                            let code = match e {
                                SyncError::AuthenticationError(_) => "auth",
                                SyncError::ApiClientError(_) => "api",
                                SyncError::CacheError(_) => "cache",
                                SyncError::Other(_) => "other",
                            };
                            let msg = format!(
                                "{} | code: {} | last_success: {}",
                                e,
                                code,
                                last_success.to_rfc3339()
                            );
                            if let Err(send_err) =
                                error_tx.send(SyncTaskError::PeriodicSyncFailed(msg.clone()))
                            {
                                tracing::error!(error = ?send_err, "Failed to forward periodic sync error");
                            }
                            let wait = backoff.min(300);
                            tracing::error!(?e, backoff = wait, "Periodic sync failed");
                            backoff = (backoff * 2).min(300);
                            if let Err(send_err) = progress_tx.send(SyncProgress::Retrying(wait)) {
                                tracing::error!(error = ?send_err, "Failed to send retry progress");
                                let _ = error_tx.send(SyncTaskError::Other(format!(
                                    "Failed to send progress update: {}",
                                    send_err
                                )));
                            }
                            sleep(Duration::from_secs(wait)).await;
                        } else {
                            last_success = Utc::now();
                            backoff = 1;
                            sleep(interval).await;
                        }
                    } => {}
                }
            }
        });
        (handle, shutdown_tx)
    }

    pub fn start_token_refresh_task(
        interval: Duration,
        error_tx: mpsc::UnboundedSender<SyncTaskError>,
    ) -> (JoinHandle<()>, oneshot::Sender<()>) {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let handle = spawn_local(async move {
            let mut last_success = Utc::now();
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    _ = async {
                        sleep(interval).await;
                        if let Err(e) = ensure_access_token_valid().await {
                            let code = match &e {
                                auth::AuthError::Keyring(_) => "keyring",
                                auth::AuthError::OAuth(_) => "oauth",
                                auth::AuthError::Other(_) => "other",
                            };
                            let msg = format!(
                                "{} | code: {} | last_success: {}",
                                e,
                                code,
                                last_success.to_rfc3339()
                            );
                            tracing::error!(error = ?e, "Token refresh failed");
                            if let Err(send_err) =
                                error_tx.send(SyncTaskError::TokenRefreshFailed(msg))
                            {
                                tracing::error!(error = ?send_err, "Failed to forward token refresh error");
                            }
                        } else {
                            last_success = Utc::now();
                        }
                    } => {}
                }
            }
        });
        (handle, shutdown_tx)
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

    #[tokio::test]
    #[serial]
    async fn test_syncer_new_invalid_db_path() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "token");
        let dir = tempfile::tempdir().expect("create dir");
        let result = Syncer::new(dir.path()).await;
        assert!(result.is_err());
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }
}
