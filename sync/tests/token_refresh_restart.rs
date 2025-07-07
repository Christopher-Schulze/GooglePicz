use sync::{Syncer, SyncTaskError};
use serial_test::serial;
use tokio::sync::mpsc;
use tokio::time::{advance, pause, Duration, timeout};

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn token_refresh_restart_attempts_reported() {
    std::env::set_var("MOCK_KEYRING", "1");
    let (err_tx, mut err_rx) = mpsc::unbounded_channel();
    pause();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let (handle, shutdown) = Syncer::start_token_refresh_task(Duration::from_secs(1), err_tx, None, None);
            advance(Duration::from_secs(2)).await; // allow a couple failures
            let _ = shutdown.send(());
            let _ = handle.await;
        })
        .await;
    let mut attempts = 0u32;
    while let Ok(Some(err)) = timeout(Duration::from_secs(1), err_rx.recv()).await {
        if let SyncTaskError::RestartAttempt(n) = err { attempts = n; }
    }
    assert!(attempts >= 1, "no restart attempt emitted");
    std::env::remove_var("MOCK_KEYRING");
}
