use sync::{Syncer, SyncProgress, SyncTaskError};
use serial_test::serial;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_sync_media_items_reports_error() {
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
    // build syncer with mocked API for initialization
    std::env::set_var("MOCK_API_CLIENT", "1");
    let file = NamedTempFile::new().unwrap();
    let mut syncer = Syncer::new(file.path()).await.unwrap();
    // drop mock so sync_media_items fails when calling API
    std::env::remove_var("MOCK_API_CLIENT");
    let (prog_tx, mut prog_rx) = mpsc::unbounded_channel();
    let (err_tx, mut err_rx) = mpsc::unbounded_channel::<SyncTaskError>();
    let result = syncer
        .sync_media_items(Some(prog_tx), Some(err_tx), None, None, None)
        .await;
    assert!(result.is_err());
    // ensure error forwarded
    // ensure progress and error forwarded
    match timeout(Duration::from_secs(5), prog_rx.recv()).await.unwrap() {
        Some(SyncProgress::Started) => {}
        other => panic!("expected Started progress, got {:?}", other),
    }
    let err = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap();
    assert!(err.is_some());
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
}
