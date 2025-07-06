//! Synchronization module for Google Photos data.

use api_client::ApiClient;
use auth::ensure_access_token_valid;
use cache::CacheManager;
use chrono::{DateTime, Datelike, Utc};
use serde_json::json;
#[cfg(feature = "face-recognition")]
use face_recognition::FaceRecognizer;
use thiserror::Error;
use std::path::{Path, PathBuf};
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
    state_path: PathBuf,
    detect_faces: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SyncState {
    page_token: Option<String>,
    total_synced: u64,
    last_success: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum SyncProgress {
    Started,
    ItemSynced(u64),
    Retrying(u64),
    Finished(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncErrorCode {
    Auth,
    Network,
    Cache,
    Other,
}

#[derive(Debug, Clone, Error)]
pub enum SyncTaskError {
    #[error("Periodic sync failed [{code:?}]: {message}")]
    PeriodicSyncFailed { code: SyncErrorCode, message: String },
    #[error("Token refresh failed [{code:?}]: {message}")]
    TokenRefreshFailed { code: SyncErrorCode, message: String },
    #[error("Task aborted: {0}")]
    Aborted(String),
    #[error("Restart attempt {0}")]
    RestartAttempt(u32),
    #[error("{code:?}: {message}")]
    Other { code: SyncErrorCode, message: String },
    #[error("Status update ({last_synced}): {message}")]
    Status { last_synced: DateTime<Utc>, message: String },
}

impl Syncer {
    fn forward<T: Clone>(tx: &Option<mpsc::UnboundedSender<T>>, value: T) {
        if let Some(t) = tx {
            let _ = t.send(value);
        }
    }

    fn load_state(&self) -> Result<SyncState, SyncError> {
        match std::fs::read_to_string(&self.state_path) {
            Ok(data) => serde_json::from_str(&data).map_err(|e| {
                tracing::error!(error = ?e, "Failed to parse state file");
                SyncError::Other(format!("Failed to parse state file: {}", e))
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SyncState::default()),
            Err(e) => {
                tracing::error!(error = ?e, "Failed to read state file");
                Err(SyncError::Other(format!("Failed to read state file: {}", e)))
            }
        }
    }

    fn save_state(&self, state: &SyncState) -> Result<(), SyncError> {
        let data = serde_json::to_string(state).map_err(|e| {
            tracing::error!(error = ?e, "Failed to serialize state");
            SyncError::Other(format!("Failed to serialize state: {}", e))
        })?;
        std::fs::write(&self.state_path, data).map_err(|e| {
            tracing::error!(error = ?e, "Failed to write state file");
            SyncError::Other(format!("Failed to write state file: {}", e))
        })
    }
    #[cfg_attr(feature = "trace-spans", tracing::instrument)]
    pub async fn new(db_path: &Path) -> Result<Self, SyncError> {
        let access_token = ensure_access_token_valid().await.map_err(|e| {
            SyncError::AuthenticationError(format!("Failed to get access token: {}", e))
        })?;

        let api_client = ApiClient::new(access_token);

        let cache_manager = CacheManager::new(db_path)
            .map_err(|e| SyncError::CacheError(format!("Failed to create cache manager: {}", e)))?;

        let mut state_path = db_path.to_path_buf();
        state_path.set_extension("state.json");

        Ok(Syncer {
            api_client,
            cache_manager,
            state_path,
            detect_faces: false,
        })
    }

    pub fn set_face_detection(&mut self, enable: bool) {
        self.detect_faces = enable;
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, progress, error)))]
    pub async fn sync_media_items(
        &mut self,
        progress: Option<mpsc::UnboundedSender<SyncProgress>>,
        error: Option<mpsc::UnboundedSender<SyncTaskError>>,
        ui_progress: Option<mpsc::UnboundedSender<SyncProgress>>,
        ui_error: Option<mpsc::UnboundedSender<SyncTaskError>>,
    ) -> Result<(), SyncError> {
        tracing::info!("Starting media item synchronization...");
        if let Some(tx) = &progress {
            if let Err(e) = tx.send(SyncProgress::Started) {
                if let Some(err_tx) = &error {
                    let _ = err_tx.send(SyncTaskError::Other {
                        code: SyncErrorCode::Other,
                        message: format!("Failed to send progress: {}", e),
                    });
                }
                Self::forward(&ui_error, SyncTaskError::Other {
                    code: SyncErrorCode::Other,
                    message: format!("Failed to send progress: {}", e),
                });
            }
        }
        Self::forward(&ui_progress, SyncProgress::Started);
        let status = SyncTaskError::Status {
            last_synced: Utc::now(),
            message: "Sync started".into(),
        };
        if let Some(tx) = &error {
            let _ = tx.send(status.clone());
        }
        Self::forward(&ui_error, status.clone());
        let mut state = self.load_state().map_err(|e| {
            let msg = format!("Failed to load state: {}", e);
            if let Some(tx) = &error {
                let _ = tx.send(SyncTaskError::Other { code: SyncErrorCode::Other, message: msg.clone() });
            }
            Self::forward(&ui_error, SyncTaskError::Other { code: SyncErrorCode::Other, message: msg.clone() });
            SyncError::Other(msg)
        })?;
        let mut page_token: Option<String> = state.page_token.clone();
        let mut total_synced = state.total_synced;

        let last_sync = match self.cache_manager.get_last_sync_async().await {
            Ok(ts) => ts,
            Err(e) => {
                let msg = format!("Failed to get last sync time: {}", e);
                if let Some(tx) = &error {
                    if let Err(send_err) = tx.send(SyncTaskError::Other {
                        code: SyncErrorCode::Cache,
                        message: msg.clone(),
                    }) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
                }
                Self::forward(&ui_error, SyncTaskError::Other {
                    code: SyncErrorCode::Cache,
                    message: msg.clone(),
                });
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
                    if let Err(send_err) = tx.send(SyncTaskError::Other {
                        code: SyncErrorCode::Auth,
                        message: msg.clone(),
                    }) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
                }
                Self::forward(&ui_error, SyncTaskError::Other {
                    code: SyncErrorCode::Auth,
                    message: msg.clone(),
                });
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
                        if let Err(send_err) = tx.send(SyncTaskError::Other {
                            code: SyncErrorCode::Network,
                            message: msg.clone(),
                        }) {
                            tracing::error!("Failed to forward error: {}", send_err);
                        }
                    }
                    Self::forward(&ui_error, SyncTaskError::Other {
                        code: SyncErrorCode::Network,
                        message: msg.clone(),
                    });
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
                            if let Err(send_err) = tx.send(SyncTaskError::Other {
                                code: SyncErrorCode::Cache,
                                message: msg.clone(),
                            }) {
                                tracing::error!("Failed to forward error: {}", send_err);
                            }
                        }
                        Self::forward(&ui_error, SyncTaskError::Other {
                            code: SyncErrorCode::Cache,
                            message: msg.clone(),
                        });
                        SyncError::CacheError(msg)
                    })?;
                total_synced += 1;
                if let Some(tx) = &progress {
                    if let Err(e) = tx.send(SyncProgress::ItemSynced(total_synced)) {
                        if let Some(err) = &error {
                            if let Err(send_err) = err.send(SyncTaskError::Other {
                                code: SyncErrorCode::Other,
                                message: format!("Failed to send progress update: {}", e),
                            }) {
                                tracing::error!("Failed to forward error: {}", send_err);
                            }
                        }
                    }
                }
                Self::forward(&ui_progress, SyncProgress::ItemSynced(total_synced));
                if total_synced % 50 == 0 {
                    let status = SyncTaskError::Status {
                        last_synced: Utc::now(),
                        message: format!("Synced {total_synced} items"),
                    };
                    if let Some(tx) = &error {
                        let _ = tx.send(status.clone());
                    }
                    Self::forward(&ui_error, status);
                }

                #[cfg(feature = "face-recognition")]
                if self.detect_faces {
                    let cache = self.cache_manager.clone();
                    let item_clone = item.clone();
                    let err_tx = error.clone();
                    let ui_err = ui_error.clone();
                    tokio::task::spawn_blocking(move || {
                        let rec = face_recognition::FaceRecognizer::new();
                        if let Err(e) = rec.detect_and_cache_faces(&cache, &item_clone, true) {
                            let msg = format!("Face detection failed: {}", e);
                            if let Some(tx) = &err_tx {
                                let _ = tx.send(SyncTaskError::Other {
                                    code: SyncErrorCode::Other,
                                    message: msg.clone(),
                                });
                            }
                            if let Some(tx) = &ui_err {
                                let _ = tx.send(SyncTaskError::Other {
                                    code: SyncErrorCode::Other,
                                    message: msg.clone(),
                                });
                            }
                            tracing::error!(error = ?e, "Face detection failed");
                        }
                    })
                    .await
                    .ok();
                }
            }

            tracing::info!("Synced {} media items so far.", total_synced);

            state.page_token = next_page_token.clone();
            state.total_synced = total_synced;
            if let Err(e) = self.save_state(&state) {
                tracing::error!(error = ?e, "Failed to save state");
                if let Some(tx) = &error {
                    let _ = tx.send(SyncTaskError::Other { code: SyncErrorCode::Other, message: e.to_string() });
                }
                Self::forward(&ui_error, SyncTaskError::Other { code: SyncErrorCode::Other, message: e.to_string() });
            }

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
                    if let Err(send_err) = err.send(SyncTaskError::Other {
                        code: SyncErrorCode::Other,
                        message: format!("Failed to send progress update: {}", e),
                    }) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
                }
            }
        }
        Self::forward(&ui_progress, SyncProgress::Finished(total_synced));
        state.page_token = None;
        state.total_synced = total_synced;
        state.last_success = Some(Utc::now());
        if let Err(e) = self.save_state(&state) {
            tracing::warn!(error = ?e, "Failed to update state file");
        }
        self.cache_manager
            .update_last_sync_async(Utc::now())
            .await
            .map_err(|e| {
                let msg = format!("Failed to update last sync: {}", e);
                if let Some(tx) = &error {
                    if let Err(send_err) = tx.send(SyncTaskError::Other {
                        code: SyncErrorCode::Cache,
                        message: msg.clone(),
                    }) {
                        tracing::error!("Failed to forward error: {}", send_err);
                    }
                }
                Self::forward(&ui_error, SyncTaskError::Other {
                    code: SyncErrorCode::Cache,
                    message: msg.clone(),
                });
                SyncError::CacheError(msg)
            })?;
        let status = SyncTaskError::Status {
            last_synced: Utc::now(),
            message: format!("Sync completed: {total_synced} items"),
        };
        if let Some(tx) = &error {
            let _ = tx.send(status.clone());
        }
        Self::forward(&ui_error, status);
        Ok(())
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, progress_tx, error_tx)))]
    pub fn start_periodic_sync(
        self,
        interval: Duration,
        progress_tx: mpsc::UnboundedSender<SyncProgress>,
        error_tx: mpsc::UnboundedSender<SyncTaskError>,
        status_tx: Option<mpsc::UnboundedSender<SyncTaskError>>,
        ui_progress_tx: Option<mpsc::UnboundedSender<SyncProgress>>,
        ui_error_tx: Option<mpsc::UnboundedSender<SyncTaskError>>,
    ) -> (JoinHandle<Result<(), SyncTaskError>>, oneshot::Sender<()>) {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let forward_err = error_tx.clone();
        let forward_ui_err = ui_error_tx.clone();
        let forward_status = status_tx.clone();

        let sync_task = spawn_local(async move {
            let mut syncer = self;
            let mut backoff = 1u64;
            let mut failures: u32 = 0;
            const MAX_FAILURES: u32 = 5;
            let mut state = syncer.load_state().unwrap_or_default();
            let mut last_success = if let Some(ts) = state.last_success {
                ts
            } else {
                match syncer.cache_manager.get_last_sync_async().await {
                    Ok(ts) => ts,
                    Err(_) => DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH),
                }
            };
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Periodic sync task shutting down");
                        return Ok(());
                    }
                    result = async {
                        if let Err(e) =
                            syncer
                                .sync_media_items(
                                    Some(progress_tx.clone()),
                                    Some(error_tx.clone()),
                                    ui_progress_tx.clone(),
                                    ui_error_tx.clone(),
                                )
                                .await
                        {
                            let code = match e {
                                SyncError::AuthenticationError(_) => SyncErrorCode::Auth,
                                SyncError::ApiClientError(_) => SyncErrorCode::Network,
                                SyncError::CacheError(_) => SyncErrorCode::Cache,
                                SyncError::Other(_) => SyncErrorCode::Other,
                            };
                            let msg = format!(
                                "{} | last_success: {}",
                                e,
                                last_success.to_rfc3339()
                            );
                            if let Err(send_err) = error_tx.send(SyncTaskError::PeriodicSyncFailed {
                                code,
                                message: msg.clone(),
                            }) {
                                tracing::error!(error = ?send_err, "Failed to forward periodic sync error");
                            }
                            Self::forward(&ui_error_tx, SyncTaskError::PeriodicSyncFailed {
                                code,
                                message: msg.clone(),
                            });
                            let status = SyncTaskError::Status {
                                last_synced: last_success,
                                message: msg.clone(),
                            };
                            if ui_error_tx
                                .as_ref()
                                .map(|u| u.same_channel(&error_tx))
                                .unwrap_or(false)
                            {
                                let _ = error_tx.send(status.clone());
                            } else {
                                let _ = error_tx.send(status.clone());
                                Self::forward(&ui_error_tx, status);
                            }
                            failures += 1;
                            if let Some(tx) = &sync_status_tx {
                                let _ = tx.send(SyncTaskError::RestartAttempt(failures));
                            }
                            let wait = backoff.min(300);
                            if let Err(send_err) = error_tx.send(SyncTaskError::RestartAttempt(failures)) {
                                tracing::error!(error = ?send_err, "Failed to forward restart attempt");
                            }
                            Self::forward(&ui_error_tx, SyncTaskError::RestartAttempt(failures));
                            if failures > 3 {
                                tracing::error!(?e, attempts = failures, backoff = wait, "Periodic sync failed");
                            } else {
                                tracing::warn!(?e, attempts = failures, backoff = wait, "Periodic sync failed");
                            }
                            backoff = (backoff * 2).min(300);
                            if let Err(send_err) = progress_tx.send(SyncProgress::Retrying(wait)) {
                                tracing::error!(error = ?send_err, "Failed to send retry progress");
                                let _ = error_tx.send(SyncTaskError::Other {
                                    code: SyncErrorCode::Other,
                                    message: format!("Failed to send progress update: {}", send_err),
                                });
                            }
                            Self::forward(&ui_progress_tx, SyncProgress::Retrying(wait));
                            if failures >= MAX_FAILURES {
                                let abort_msg = format!("periodic sync aborted after {} failures", failures);
                                tracing::error!("{}", abort_msg);
                                let abort_err = SyncTaskError::Aborted(abort_msg.clone());
                                let _ = error_tx.send(abort_err.clone());
                                Self::forward(&ui_error_tx, abort_err.clone());
                                if let Some(tx) = &sync_status_tx {
                                    let _ = tx.send(abort_err.clone());
                                }
                                failures = 0;
                                backoff = (backoff * 2).min(300);
                                sleep(Duration::from_secs(backoff)).await;
                            } else {
                                sleep(Duration::from_secs(wait)).await;
                            }
                        } else {
                            last_success = Utc::now();
                            state.last_success = Some(last_success);
                            let _ = syncer.save_state(&state);
                            backoff = 1;
                            failures = 0;
                            if let Some(tx) = &sync_status_tx {
                                let _ = tx.send(SyncTaskError::Status {
                                    last_synced: last_success,
                                    message: "Sync completed".into(),
                                });
                            }
                            let status = SyncTaskError::Status {
                                last_synced: last_success,
                                message: "Sync completed".into(),
                            };
                            if ui_error_tx
                                .as_ref()
                                .map(|u| u.same_channel(&error_tx))
                                .unwrap_or(false)
                            {
                                let _ = error_tx.send(status.clone());
                            } else {
                                let _ = error_tx.send(status.clone());
                                Self::forward(&ui_error_tx, status);
                            }
                            sleep(interval).await;
                        }
                        Ok::<(), SyncTaskError>(())
                    } => match result {
                        Ok(()) => {},
                        Err(e) => return Err(e),
                    }
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), SyncTaskError>(())
        });

        let handle = spawn_local(async move {
            match sync_task.await {
                Ok(res) => {
                    if let Err(ref e) = res {
                        let _ = forward_err.send(e.clone());
                        Syncer::forward(&forward_ui_err, e.clone());
                        if let Some(tx) = &forward_status {
                            let _ = tx.send(e.clone());
                        }
                    }
                    res
                }
                Err(join_err) => {
                    let msg = format!("task join error: {}", join_err);
                    let err = SyncTaskError::Other { code: SyncErrorCode::Other, message: msg };
                    let _ = forward_err.send(err.clone());
                    Syncer::forward(&forward_ui_err, err.clone());
                    if let Some(tx) = &forward_status {
                        let _ = tx.send(err.clone());
                    }
                    Err(err)
                }
            }
        });
        (handle, shutdown_tx)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub fn start_token_refresh_task(
        interval: Duration,
        error_tx: mpsc::UnboundedSender<SyncTaskError>,
        ui_error_tx: Option<mpsc::UnboundedSender<SyncTaskError>>,
    ) -> (JoinHandle<Result<(), SyncTaskError>>, oneshot::Sender<()>) {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let handle = spawn_local(async move {
            let mut interval = interval;
            let mut last_success = Utc::now();
            let mut failures: u32 = 0;
            const MAX_FAILURES: u32 = 5;
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Token refresh task shutting down");
                        return Ok(());
                    }
                    result = async {
                        sleep(interval).await;
                        if let Err(e) = ensure_access_token_valid().await {
                            let _code = match &e {
                                auth::AuthError::Keyring(_) => "keyring",
                                auth::AuthError::OAuth(_) => "oauth",
                                auth::AuthError::Other(_) => "other",
                            };
                            let msg = format!(
                                "{} | last_success: {}",
                                e,
                                last_success.to_rfc3339()
                            );
                            tracing::error!(error = ?e, "Token refresh failed");
                            let err_variant = SyncTaskError::TokenRefreshFailed {
                                code: match &e {
                                    auth::AuthError::Keyring(_) | auth::AuthError::OAuth(_) | auth::AuthError::Other(_) => SyncErrorCode::Auth,
                                },
                                message: msg.clone(),
                            };
                            if let Err(send_err) = error_tx.send(err_variant.clone())
                            {
                                tracing::error!(error = ?send_err, "Failed to forward token refresh error");
                            }
                            Self::forward(&ui_error_tx, err_variant.clone());
                            let status = SyncTaskError::Status {
                                last_synced: last_success,
                                message: msg.clone(),
                            };
                            let _ = error_tx.send(status.clone());
                            if ui_error_tx
                                .as_ref()
                                .map(|u| u.same_channel(&error_tx))
                                .unwrap_or(false)
                            {
                                // already sent via error_tx
                            } else {
                                Self::forward(&ui_error_tx, status);
                            }
                            failures += 1;
                            if let Err(send_err) = error_tx.send(SyncTaskError::RestartAttempt(failures)) {
                                tracing::error!(error = ?send_err, "Failed to forward restart attempt");
                            }
                            Self::forward(&ui_error_tx, SyncTaskError::RestartAttempt(failures));
                            if failures >= MAX_FAILURES {
                                let abort_msg = format!("token refresh aborted after {} failures", failures);
                                tracing::error!("{}", abort_msg);
                                let _ = error_tx.send(SyncTaskError::Aborted(abort_msg.clone()));
                                Self::forward(&ui_error_tx, SyncTaskError::Aborted(abort_msg.clone()));
                                failures = 0;
                                interval = Duration::from_secs((interval.as_secs() * 2).min(300));
                                sleep(interval).await;
                            }
                        } else {
                            last_success = Utc::now();
                            failures = 0;
                            let status = SyncTaskError::Status {
                                last_synced: last_success,
                                message: "Token refreshed".into(),
                            };
                            let _ = error_tx.send(status.clone());
                            if ui_error_tx
                                .as_ref()
                                .map(|u| u.same_channel(&error_tx))
                                .unwrap_or(false)
                            {
                                // already sent via error_tx
                            } else {
                                Self::forward(&ui_error_tx, status);
                            }
                        }
                        Ok::<(), SyncTaskError>(())
                    } => match result {
                        Ok(()) => {},
                        Err(e) => return Err(e),
                    }
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), SyncTaskError>(())
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
        let result = syncer.sync_media_items(None, None, None, None).await;
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
