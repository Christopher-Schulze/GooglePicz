use sync::{Syncer, SyncTaskError};
use serial_test::serial;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tempfile::tempdir;
use ui::{GooglePiczUI, Message};
use iced::Application;

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn test_token_refresh_task_reports_error() {
    std::env::set_var("MOCK_KEYRING", "1");
    // No refresh token so refresh will fail
    let (err_tx, mut err_rx) = mpsc::unbounded_channel::<SyncTaskError>();
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

#[test]
#[serial]
fn test_token_refresh_error_forwarded_to_ui() {
    std::env::set_var("MOCK_KEYRING", "1");
    let (err_tx, mut err_rx) = mpsc::unbounded_channel::<SyncTaskError>();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let err = rt.block_on(async {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async move {
                let (handle, shutdown) =
                    Syncer::start_token_refresh_task(Duration::from_millis(10), err_tx);
                let err = timeout(Duration::from_secs(5), err_rx.recv()).await.unwrap().unwrap();
                let _ = shutdown.send(());
                let _ = handle.await;
                err
            })
            .await
    });
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();
    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::SyncError(err));
    assert!(ui.error_count() > 0);
    std::env::remove_var("MOCK_KEYRING");
}
