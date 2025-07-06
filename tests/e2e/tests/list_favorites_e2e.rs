use cache::CacheManager;
use api_client::{MediaItem, MediaMetadata};
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");

    let dir = TempDir::new().expect("dir");
    let db = dir.path().join("cache.sqlite");
    let cache = CacheManager::new(&db).expect("cache");

    let item1 = MediaItem {
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
    let item2 = MediaItem {
        id: "2".into(),
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
        filename: "2.jpg".into(),
    };

    cache.insert_media_item(&item1).expect("insert1");
    cache.insert_media_item(&item2).expect("insert2");
    cache.set_favorite(&item2.id, true).expect("fav");

    let favs = cache.get_favorite_media_items().expect("favs");
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].id, item2.id);
}

