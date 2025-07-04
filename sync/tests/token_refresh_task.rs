use sync::Syncer;
use serial_test::serial;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_token_refresh_task_reports_error() {
    std::env::set_var("MOCK_KEYRING", "1");
    // No refresh token so refresh will fail
    let (err_tx, mut err_rx) = mpsc::unbounded_channel();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (handle, shutdown) =
                Syncer::start_token_refresh_task(Duration::from_millis(10), err_tx);
            let err = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap();
            assert!(err.is_some());
            let _ = shutdown.send(());
            let _ = handle.await;
        })
        .await;
    std::env::remove_var("MOCK_KEYRING");
}
