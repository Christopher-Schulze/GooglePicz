use sync::{Syncer, SyncTaskError};
use serial_test::serial;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_periodic_sync_repeated_failures_reported() {
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
    // build syncer with mocked API first
    std::env::set_var("MOCK_API_CLIENT", "1");
    let file = NamedTempFile::new().unwrap();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let mut syncer = Syncer::new(file.path()).await.unwrap();
            // drop mock so periodic sync fails
            std::env::remove_var("MOCK_API_CLIENT");
            let (prog_tx, _prog_rx) = mpsc::unbounded_channel();
            let (err_tx, _err_rx) = mpsc::unbounded_channel::<SyncTaskError>();
            let (status_tx, mut status_rx) = mpsc::unbounded_channel::<u32>();
            let (handle, shutdown) = syncer.start_periodic_sync(
                Duration::from_millis(10),
                prog_tx,
                err_tx,
                Some(status_tx),
            );
            let first = timeout(Duration::from_secs(5), status_rx.recv()).await.unwrap().unwrap();
            let second = timeout(Duration::from_secs(5), status_rx.recv()).await.unwrap().unwrap();
            assert!(first == 1 && second >= 2);
            let _ = shutdown.send(());
            let _ = handle.await;
        })
        .await;
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
}
