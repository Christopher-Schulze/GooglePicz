use sync::{Syncer, SyncProgress};
use serial_test::serial;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_periodic_sync_reports_error() {
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
    // create syncer with mocked API success
    std::env::set_var("MOCK_API_CLIENT", "1");
    let file = NamedTempFile::new().unwrap();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let mut syncer = Syncer::new(file.path()).await.unwrap();
            // remove API mocking so periodic sync fails when calling the network
            std::env::remove_var("MOCK_API_CLIENT");
            let (prog_tx, mut prog_rx) = mpsc::unbounded_channel();
            let (err_tx, mut err_rx) = mpsc::unbounded_channel();
            let (handle, shutdown) = syncer.start_periodic_sync(Duration::from_millis(10), prog_tx, err_tx);
            let start = timeout(Duration::from_secs(5), prog_rx.recv()).await.unwrap();
            assert!(matches!(start, Some(SyncProgress::Started)));
            let retry = timeout(Duration::from_secs(5), prog_rx.recv()).await.unwrap();
            assert!(matches!(retry, Some(SyncProgress::Retrying(_))));
            let err = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap();
            assert!(err.is_some());
            let _ = shutdown.send(());
            let _ = handle.await;
        })
        .await;
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
}
