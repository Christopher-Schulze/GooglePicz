use ui::{GooglePiczUI, Message};
use iced::Application;
use tempfile::tempdir;
use serial_test::serial;
use api_client::{MediaItem, MediaMetadata};

fn sample_video() -> MediaItem {
    MediaItem {
        id: "v1".to_string(),
        description: None,
        product_url: "http://example.com".into(),
        base_url: "http://example.com/video".into(),
        mime_type: "video/mp4".into(),
        media_metadata: MediaMetadata {
            creation_time: "2023-01-01T00:00:00Z".into(),
            width: "1".into(),
            height: "1".into(),
            video: None,
        },
        filename: "v1.mp4".into(),
    }
}

#[test]
#[serial]
fn test_video_play_pause_stop() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let item = sample_video();

    let _ = ui.update(Message::PlayVideo(item.clone()));
    assert!(ui.state_debug().starts_with("PlayingVideo"));
    assert_eq!(ui.video_status(), "Playing");

    let _ = ui.update(Message::VideoPause);
    assert_eq!(ui.video_status(), "Paused");

    let _ = ui.update(Message::VideoPlay);
    assert_eq!(ui.video_status(), "Playing");

    let _ = ui.update(Message::VideoStop);
    assert_eq!(ui.state_debug(), "Grid");
}

#[test]
#[serial]
fn test_video_bus_end() {
    let dir = tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".googlepicz")).unwrap();

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, dir.path().join(".googlepicz")));
    let item = sample_video();
    let _ = ui.update(Message::PlayVideo(item));
    let _ = ui.update(Message::VideoEvent(ui::GStreamerMessage::BusGoToEnd));
    assert_eq!(ui.state_debug(), "Grid");
    assert_eq!(ui.video_status(), "Finished");
}
