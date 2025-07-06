use sync::{Syncer, SyncTaskError};
use serial_test::serial;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_status_messages_emitted() {
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
    std::env::set_var("MOCK_API_CLIENT", "1");
    let file = NamedTempFile::new().unwrap();
    let mut syncer = Syncer::new(file.path()).await.unwrap();
    let (err_tx, mut err_rx) = mpsc::unbounded_channel::<SyncTaskError>();
    syncer.sync_media_items(None, Some(err_tx), None, None).await.unwrap();
    let first = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap().unwrap();
    assert!(matches!(first, SyncTaskError::Status { message, .. } if message.contains("Sync started")));
    let second = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap().unwrap();
    assert!(matches!(second, SyncTaskError::Status { message, .. } if message.contains("Sync completed")));
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
    std::env::remove_var("MOCK_API_CLIENT");
}
