use face_recognition::{FaceRecognizer, FaceRecognitionError};
use api_client::{MediaItem, MediaMetadata};
use tempfile::TempDir;
use base64::Engine;

const SAMPLE_IMAGE_B64: &str = include_str!("../../face_recognition/tests/face_image.b64");

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("OPENCV_HAARCASCADE_PATH", "/nonexistent.xml");

    let engine = base64::engine::general_purpose::STANDARD;
    let data = engine.decode(SAMPLE_IMAGE_B64.replace('\n', "").as_bytes()).expect("decode");

    let dir = TempDir::new().expect("dir");
    let img_path = dir.path().join("face.jpg");
    std::fs::write(&img_path, &data).expect("write image");

    let item = MediaItem {
        id: "1".into(),
        description: None,
        product_url: String::new(),
        base_url: format!("file://{}", img_path.display()),
        mime_type: "image/jpeg".into(),
        media_metadata: MediaMetadata {
            creation_time: "2024-01-01T00:00:00Z".into(),
            width: "200".into(),
            height: "200".into(),
            video: None,
        },
        filename: "face.jpg".into(),
    };

    let recognizer = FaceRecognizer::new();
    let result = recognizer.detect_faces(&item);
    assert!(matches!(result, Err(FaceRecognitionError::ModelNotFound(_))));
}
