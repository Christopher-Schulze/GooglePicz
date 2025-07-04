use ui::{GooglePiczUI, Message};
use iced::Application;
use tempfile::tempdir;
use api_client::{MediaItem, MediaMetadata};
use serial_test::serial;

fn sample_item() -> MediaItem {
    MediaItem {
        id: "1".to_string(),
        description: None,
        product_url: "http://example.com".into(),
        base_url: "http://example.com/base".into(),
        mime_type: "image/jpeg".into(),
        media_metadata: MediaMetadata {
            creation_time: "2023-01-01T00:00:00Z".into(),
            width: "1".into(),
            height: "1".into(),
        },
        filename: "1.jpg".into(),
    }
}

#[test]
#[serial]
fn test_initial_state() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    assert_eq!(ui.photo_count(), 0);
    assert_eq!(ui.album_count(), 0);
    assert_eq!(ui.state_debug(), "Grid");
}

#[test]
#[serial]
fn test_select_and_close_photo() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let item = sample_item();

    let _ = ui.update(Message::SelectPhoto(item.clone()));
    assert!(ui.state_debug().starts_with("SelectedPhoto"));

    let _ = ui.update(Message::ClosePhoto);
    assert_eq!(ui.state_debug(), "Grid");
}

#[test]
#[serial]
fn test_dismiss_error() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::SyncError("err".into()));
    assert_eq!(ui.error_count(), 1);
    let _ = ui.update(Message::DismissError(0));
    assert_eq!(ui.error_count(), 0);
}

#[test]
#[serial]
fn test_sync_error_added() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::SyncError("boom".into()));
    assert!(ui.error_count() > 0);
}
