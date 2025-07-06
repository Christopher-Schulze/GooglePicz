use cache::{CacheManager, FaceData};
use api_client::{MediaItem, MediaMetadata};
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");

    let dir = TempDir::new().expect("dir");
    let db1 = dir.path().join("cache1.sqlite");
    let cache1 = CacheManager::new(&db1).expect("cache1");

    let item = MediaItem {
        id: "1".into(),
        description: None,
        product_url: String::new(),
        base_url: String::new(),
        mime_type: "image/jpeg".into(),
        media_metadata: MediaMetadata {
            creation_time: "2024-01-01T00:00:00Z".into(),
            width: "1".into(),
            height: "1".into(),
            video: None,
        },
        filename: "1.jpg".into(),
    };

    cache1.insert_media_item(&item).expect("insert item");
    let faces = vec![FaceData { bbox: [0, 0, 10, 10], name: Some("a".into()) }];
    let json = serde_json::to_string(&faces).unwrap();
    cache1.insert_faces(&item.id, &json).unwrap();

    let export_path = dir.path().join("faces.json");
    cache1.export_faces(&export_path).expect("export");

    let db2 = dir.path().join("cache2.sqlite");
    let cache2 = CacheManager::new(&db2).expect("cache2");
    cache2.insert_media_item(&item).expect("insert item2");
    cache2.import_faces(&export_path).expect("import");

    let stored = cache2.get_faces(&item.id).expect("faces").unwrap();
    assert_eq!(stored.len(), 1);
}

