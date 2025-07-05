pub use gstreamer_iced::{GstreamerIcedBase as VideoPlayer, GStreamerMessage, PlayStatus};
pub use gstreamer_iced::reexport::url;

pub fn start_video(url: &url::Url) -> Option<VideoPlayer> {
    if let Ok(mut player) = VideoPlayer::new_url(url, false) {
        let _ = player.update(GStreamerMessage::PlayStatusChanged(PlayStatus::Playing));
        Some(player)
    } else {
        None
    }
}
