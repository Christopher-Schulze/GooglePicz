use cache::CacheManager;
use tempfile::NamedTempFile;
use api_client::{MediaItem, MediaMetadata};
use chrono::Utc;
use rusqlite::Connection;

fn sample_item(id: &str) -> MediaItem {
    MediaItem {
        id: id.to_string(),
        description: Some("desc".into()),
        product_url: "http://example.com".into(),
        base_url: "http://example.com/base".into(),
        mime_type: "image/jpeg".into(),
        media_metadata: MediaMetadata {
            creation_time: "2023-01-01T00:00:00Z".into(),
            width: "1".into(),
            height: "1".into(),
        },
        filename: format!("{}.jpg", id),
    }
}

#[test]
fn test_new_applies_migrations() {
    let file = NamedTempFile::new().unwrap();
    let _ = CacheManager::new(file.path()).unwrap();
    let conn = Connection::open(file.path()).unwrap();
    let version: i64 = conn
        .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 6);
}

#[test]
fn test_insert_and_query_media_item() {
    let file = NamedTempFile::new().unwrap();
    let cm = CacheManager::new(file.path()).unwrap();
    let item = sample_item("1");
    cm.insert_media_item(&item).unwrap();
    let retrieved = cm.get_media_item("1").unwrap().unwrap();
    assert_eq!(retrieved.id, item.id);
    assert_eq!(retrieved.filename, item.filename);
}

#[test]
fn test_clear_cache() {
    let file = NamedTempFile::new().unwrap();
    let cm = CacheManager::new(file.path()).unwrap();
    cm.insert_media_item(&sample_item("1")).unwrap();
    cm.insert_media_item(&sample_item("2")).unwrap();
    assert_eq!(cm.get_all_media_items().unwrap().len(), 2);
    cm.clear_cache().unwrap();
    assert!(cm.get_all_media_items().unwrap().is_empty());
}

#[test]
fn test_update_last_sync() {
    let file = NamedTempFile::new().unwrap();
    let cm = CacheManager::new(file.path()).unwrap();
    let now = Utc::now();
    cm.update_last_sync(now).unwrap();
    let stored = cm.get_last_sync().unwrap();
    assert!(stored.timestamp() - now.timestamp() <= 1);
}
