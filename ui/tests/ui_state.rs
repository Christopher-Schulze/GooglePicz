use ui::{GooglePiczUI, Message, SearchMode};
use sync::SyncTaskError;
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
            video: None,
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
    let _ = ui.update(Message::SyncError(SyncTaskError::Other("err".into())));
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
    let _ = ui.update(Message::SyncError(SyncTaskError::Other("boom".into())));
    assert!(ui.error_count() > 0);
}

#[test]
#[serial]
fn test_sync_failed_message() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::SyncFailed("fail".into()));
    assert_eq!(ui.sync_status(), "Sync error");
    assert!(ui.error_count() > 0);
}

#[test]
#[serial]
fn test_sync_retry_message() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::SyncRetry(5));
    assert!(ui.sync_status().contains("Retrying"));
    assert_eq!(ui.error_count(), 0);
}

#[test]
#[serial]
fn test_rename_dialog_state() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::ShowRenameAlbumDialog("a1".into(), "Old".into()));
    assert_eq!(ui.renaming_album(), Some("a1".into()));
    assert_eq!(ui.rename_album_title(), "Old");
    let _ = ui.update(Message::CancelRenameAlbum);
    assert!(ui.renaming_album().is_none());
}

#[test]
#[serial]
fn test_delete_dialog_state() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::ShowDeleteAlbumDialog("a1".into()));
    assert_eq!(ui.deleting_album(), Some("a1".into()));
    let _ = ui.update(Message::CancelDeleteAlbum);
    assert!(ui.deleting_album().is_none());
}

#[test]
#[serial]
fn test_search_input() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let _ = ui.update(Message::SearchInputChanged("query".into()));
    assert_eq!(ui.search_query(), "query");
}

#[test]
#[serial]
fn test_search_mode() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    assert_eq!(ui.search_mode(), SearchMode::Filename);
    let _ = ui.update(Message::SearchModeChanged(SearchMode::Favoriten));
    assert_eq!(ui.search_mode(), SearchMode::Favoriten);
}
