use face_recognition::FaceRecognizer;
use base64::Engine;
use api_client::{MediaItem, MediaMetadata};
use cache::CacheManager;

const SAMPLE_IMAGE_B64: &str = include_str!("./face_image.b64");

fn prepare_sample_file() -> std::path::PathBuf {
    let engine = base64::engine::general_purpose::STANDARD;
    let data = engine
        .decode(SAMPLE_IMAGE_B64.replace('\n', "").as_bytes())
        .expect("decode");
    let path = std::env::temp_dir().join("face_sample_roundtrip.jpg");
    std::fs::write(&path, &data).expect("write");
    path
}

#[test]
fn test_detect_and_cache_roundtrip() {
    let img = prepare_sample_file();
    let item = MediaItem {
        id: "r1".into(),
        description: None,
        product_url: String::new(),
        base_url: format!("file://{}", img.display()),
        mime_type: "image/jpeg".into(),
        media_metadata: MediaMetadata {
            creation_time: "2024-01-01T00:00:00Z".into(),
            width: "200".into(),
            height: "200".into(),
            video: None,
        },
        filename: "sample.jpg".into(),
    };

    let cache_file = tempfile::NamedTempFile::new().expect("tmpfile");
    let cache = CacheManager::new(cache_file.path()).expect("cache");
    cache.insert_media_item(&item).expect("insert item");

    let rec = FaceRecognizer::new();
    let faces = rec
        .detect_and_cache_faces(&cache, &item)
        .expect("detect");
    assert!(!faces.is_empty());

    let stored = cache.get_faces(&item.id).expect("get").unwrap();
    assert_eq!(stored.len(), faces.len());
}
