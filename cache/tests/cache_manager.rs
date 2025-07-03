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
    assert_eq!(version, 8);
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

    // Insert sample data into all relevant tables
    let item = sample_item("1");
    cm.insert_media_item(&item).unwrap();
    let album = api_client::Album {
        id: "a1".into(),
        title: Some("Album".into()),
        product_url: None,
        is_writeable: None,
        media_items_count: None,
        cover_photo_base_url: None,
        cover_photo_media_item_id: None,
    };
    cm.insert_album(&album).unwrap();
    cm.associate_media_item_with_album(&item.id, &album.id)
        .unwrap();
    cm.update_last_sync(Utc::now()).unwrap();

    assert_eq!(cm.get_all_media_items().unwrap().len(), 1);
    assert_eq!(cm.get_all_albums().unwrap().len(), 1);
    assert_eq!(
        cm.get_media_items_by_album(&album.id).unwrap().len(),
        1
    );

    cm.clear_cache().unwrap();

    // Verify all tables are empty and last_sync reset
    let conn = Connection::open(file.path()).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM media_items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM media_metadata", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM albums", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM album_media_items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let ts: String = conn
        .query_row("SELECT timestamp FROM last_sync WHERE id = 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ts, "1970-01-01T00:00:00Z");
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
