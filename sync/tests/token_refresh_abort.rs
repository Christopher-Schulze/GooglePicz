use sync::{Syncer, SyncTaskError};
use serial_test::serial;
use tokio::sync::mpsc;
use tokio::time::{advance, pause, Duration, timeout};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn token_refresh_aborts_after_failures() {
    std::env::set_var("MOCK_KEYRING", "1");
    let (err_tx, mut err_rx) = mpsc::unbounded_channel();
    pause();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (handle, shutdown) = Syncer::start_token_refresh_task(Duration::from_secs(1), err_tx, None, None);
            advance(Duration::from_secs(6)).await; // enough for >5 failures
            let _ = shutdown.send(());
            let _ = handle.await;
        })
        .await;
    let mut seen_abort = false;
    while let Ok(Some(err)) = timeout(Duration::from_secs(1), err_rx.recv()).await {
        if matches!(err, SyncTaskError::Aborted(_)) { seen_abort = true; break; }
    }
    assert!(seen_abort, "no Aborted error emitted");
    std::env::remove_var("MOCK_KEYRING");
}
