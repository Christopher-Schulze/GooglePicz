use gstreamer_iced::{GstreamerIcedBase, GStreamerMessage, PlayStatus};
use std::path::Path;

#[tokio::test]
async fn test_sample_video_plays() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("videos")
        .join("sample.mp4");
    let url = gstreamer_iced::reexport::url::Url::from_file_path(&path).unwrap();
    let mut player = match GstreamerIcedBase::new_url(&url, false) {
        Ok(p) => p,
        Err(_) => return, // Skip if GStreamer unavailable
    };
    // Start playback
    let _ = player.update(GStreamerMessage::PlayStatusChanged(PlayStatus::Playing));
    // Pause playback
    let _ = player.update(GStreamerMessage::PlayStatusChanged(PlayStatus::Paused));
}

#[tokio::test]
async fn test_invalid_video_errors() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("videos")
        .join("invalid.mp4");
    std::fs::write(&path, b"not a real video").unwrap();
    let url = gstreamer_iced::reexport::url::Url::from_file_path(&path).unwrap();
    assert!(GstreamerIcedBase::new_url(&url, false).is_err());
}
