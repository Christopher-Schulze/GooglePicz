use sync::{Syncer, SyncProgress, SyncTaskError};
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
            let (err_tx, mut err_rx) = mpsc::unbounded_channel::<SyncTaskError>();
            let (handle, shutdown) = syncer.start_periodic_sync(Duration::from_millis(10), prog_tx, err_tx, None);
            let start = timeout(Duration::from_secs(5), prog_rx.recv()).await.unwrap();
            assert!(matches!(start, Some(SyncProgress::Started)));
            let retry = timeout(Duration::from_secs(5), prog_rx.recv()).await.unwrap();
            assert!(matches!(retry, Some(SyncProgress::Retrying(_))));
            let err1 = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap();
            let err2 = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap();
            let detail_err = [err1, err2]
                .into_iter()
                .flatten()
                .find(|e| matches!(e, SyncTaskError::PeriodicSyncFailed(_)))
                .expect("no periodic sync failure");
            if let SyncTaskError::PeriodicSyncFailed(detail) = detail_err {
                assert!(detail.contains("last_success"));
                assert!(detail.contains("code:"));
            } else {
                panic!("unexpected error variant: {:?}", detail_err);
            }
            let _ = shutdown.send(());
            let _ = handle.await;
        })
        .await;
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_periodic_sync_progress_send_failure_forwarded() {
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
            let (err_tx, mut err_rx) = mpsc::unbounded_channel::<SyncTaskError>();
            let (handle, shutdown) =
                syncer.start_periodic_sync(Duration::from_millis(10), prog_tx, err_tx, None);
            // consume the Started event then drop receiver to cause send failure later
            let start = timeout(Duration::from_secs(5), prog_rx.recv()).await.unwrap();
            assert!(matches!(start, Some(SyncProgress::Started)));
            drop(prog_rx);
            // first error is from periodic sync failing, second from progress send failure
            let first = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap();
            let second = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap();
            let primary = [first, second]
                .into_iter()
                .flatten()
                .find(|e| matches!(e, SyncTaskError::PeriodicSyncFailed(_)))
                .expect("no periodic failure");
            if let SyncTaskError::PeriodicSyncFailed(msg) = &primary {
                assert!(msg.contains("last_success"));
                assert!(msg.contains("code:"));
            }
            let _ = shutdown.send(());
            let _ = handle.await;
        })
        .await;
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
}
