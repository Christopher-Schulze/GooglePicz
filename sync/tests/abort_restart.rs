use sync::{Syncer, SyncProgress, SyncTaskError};
use serial_test::serial;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration, pause, advance};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_periodic_sync_abort_and_restart() {
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
    // first build syncer with mock to create db
    std::env::set_var("MOCK_API_CLIENT", "1");
    let file = NamedTempFile::new().unwrap();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let mut syncer = Syncer::new(file.path()).await.unwrap();
            // drop mock to force failures
            std::env::remove_var("MOCK_API_CLIENT");
            let (p_tx, _p_rx) = mpsc::unbounded_channel();
            let (e_tx, mut e_rx) = mpsc::unbounded_channel::<SyncTaskError>();
            pause();
            let (handle, _shutdown) = syncer.start_periodic_sync(
                Duration::from_secs(1),
                p_tx,
                e_tx,
                None,
                None,
                None,
            );
            advance(Duration::from_secs(40)).await; // enough for 5 failures
            let result = handle.await.unwrap();
            assert!(matches!(result, Err(SyncTaskError::Aborted(_))));
            // ensure Aborted sent
            let err = timeout(Duration::from_secs(5), e_rx.recv()).await.unwrap().unwrap();
            assert!(matches!(err, SyncTaskError::Aborted(_)));
            // restart with working API
            std::env::set_var("MOCK_API_CLIENT", "1");
            let mut syncer2 = Syncer::new(file.path()).await.unwrap();
            let (p_tx2, mut p_rx2) = mpsc::unbounded_channel();
            let (e_tx2, _e_rx2) = mpsc::unbounded_channel();
            let (handle2, shutdown2) = syncer2.start_periodic_sync(
                Duration::from_secs(1),
                p_tx2,
                e_tx2,
                None,
                None,
                None,
            );
            advance(Duration::from_secs(1)).await;
            let started = timeout(Duration::from_secs(5), p_rx2.recv()).await.unwrap();
            assert!(matches!(started, Some(SyncProgress::Started)));
            let _ = shutdown2.send(());
            let _ = handle2.await;
        })
        .await;
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
    std::env::remove_var("MOCK_API_CLIENT");
}
