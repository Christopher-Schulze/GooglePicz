use gstreamer_iced::{GstreamerIcedBase, GStreamerMessage, PlayStatus};
use std::path::Path;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_sample_video_plays() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("videos")
        .join("sample.mp4");
    let url = gstreamer_iced::reexport::url::Url::from_file_path(&path).unwrap();
    let mut player = GstreamerIcedBase::new_url(&url, false).expect("create player");
    player.update(GStreamerMessage::PlayStatusChanged(PlayStatus::Playing));
    for _ in 0..20 {
        player.update(GStreamerMessage::Update);
        if player.frame_data().is_some() {
            return;
        }
        sleep(Duration::from_millis(100)).await;
    }
    panic!("no frame decoded");
}
