use cache::{CacheManager, CacheError};
use tempfile::NamedTempFile;
use api_client::{MediaItem, MediaMetadata};
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashSet;

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
            video: None,
        },
        filename: format!("{}.jpg", id),
    }
}

#[test]
fn export_and_import_roundtrip() {
    let file = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(file.path()).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    let export_file = NamedTempFile::new().unwrap();
    cache.export_media_items(export_file.path()).unwrap();
    cache.clear_cache().unwrap();
    cache.import_media_items(export_file.path()).unwrap();

    let items = cache.get_all_media_items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, item.id);
}

#[test]
fn test_new_applies_migrations() {
    let file = NamedTempFile::new().unwrap();
    let _ = CacheManager::new(file.path()).unwrap();
    let conn = Connection::open(file.path()).unwrap();
    let version: i64 = conn
        .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 13);
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

#[test]
fn test_cover_photo_fk_sets_null() {
    let file = NamedTempFile::new().unwrap();
    let cm = CacheManager::new(file.path()).unwrap();
    let item = sample_item("1");
    cm.insert_media_item(&item).unwrap();
    let album = api_client::Album {
        id: "a1".into(),
        title: Some("Album".into()),
        product_url: None,
        is_writeable: None,
        media_items_count: Some("1".into()),
        cover_photo_base_url: None,
        cover_photo_media_item_id: Some(item.id.clone()),
    };
    cm.insert_album(&album).unwrap();
    cm.delete_media_item(&item.id).unwrap();
    let conn = Connection::open(file.path()).unwrap();
    let mut stmt = conn
        .prepare("SELECT cover_photo_media_item_id FROM albums WHERE id = ?1")
        .unwrap();
    let val: Option<String> = stmt
        .query_row([album.id.as_str()], |row| row.get(0))
        .unwrap();
    assert!(val.is_none());
}

#[test]
fn test_explain_date_range_uses_index() {
    let file = NamedTempFile::new().unwrap();
    let _ = CacheManager::new(file.path()).unwrap();
    let conn = Connection::open(file.path()).unwrap();
    let mut stmt = conn
        .prepare(
            "EXPLAIN QUERY PLAN SELECT m.id FROM media_items m JOIN media_metadata md ON m.id = md.media_item_id WHERE md.creation_time >= ?1 AND md.creation_time <= ?2",
        )
        .unwrap();
    let plan: String = stmt.query_row([0i64, 0i64], |row| row.get(3)).unwrap();
    assert!(plan.contains("idx_media_metadata_creation_time"), "plan was {}", plan);
}

#[test]
fn test_explain_album_filter_uses_index() {
    let file = NamedTempFile::new().unwrap();
    let _ = CacheManager::new(file.path()).unwrap();
    let conn = Connection::open(file.path()).unwrap();
    let mut stmt = conn
        .prepare(
            "EXPLAIN QUERY PLAN SELECT m.id FROM media_items m JOIN album_media_items ami ON m.id = ami.media_item_id JOIN media_metadata md ON m.id = md.media_item_id WHERE ami.album_id = ?1",
        )
        .unwrap();
    let plan: String = stmt.query_row(["a"], |row| row.get(3)).unwrap();
    assert!(plan.contains("INDEX"), "plan was {}", plan);
}

#[test]
fn test_explain_mime_filter_uses_index() {
    let file = NamedTempFile::new().unwrap();
    let _ = CacheManager::new(file.path()).unwrap();
    let conn = Connection::open(file.path()).unwrap();
    let mut stmt = conn
        .prepare(
            "EXPLAIN QUERY PLAN SELECT m.id FROM media_items m \
             JOIN media_metadata md ON m.id = md.media_item_id \
             WHERE m.mime_type = ?1",
        )
        .unwrap();
    let plan: String = stmt.query_row(["image/jpeg"], |row| row.get(3)).unwrap();
    assert!(plan.contains("idx_media_items_mime_type"), "plan was {}", plan);
}

#[tokio::test]
async fn test_async_wrappers() {
    let file = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(file.path()).unwrap();
    let item = sample_item("async1");
    cache.insert_media_item_async(item.clone()).await.unwrap();
    let items = cache.get_all_media_items_async().await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, item.id);
}

#[test]
fn test_poisoned_mutex_returns_error() {
    let file = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(file.path()).unwrap();
    let cache_clone = cache.clone();
    let _ = std::panic::catch_unwind(|| {
        let _guard = cache_clone.lock_conn().unwrap();
        panic!("boom");
    });
    let result = cache.get_all_media_items();
    assert!(matches!(result, Err(CacheError::Other(_))));
}

#[tokio::test]
async fn test_poisoned_mutex_returns_error_async() {
    let file = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(file.path()).unwrap();
    let cache_clone = cache.clone();
    let _ = std::panic::catch_unwind(|| {
        let _guard = cache_clone.lock_conn().unwrap();
        panic!("boom");
    });
    let result = cache.get_all_media_items_async().await;
    assert!(matches!(result, Err(CacheError::Other(_))));
}

#[test]
fn test_get_media_items_by_description() {
    let file = NamedTempFile::new().unwrap();
    let cm = CacheManager::new(file.path()).unwrap();
    let mut item1 = sample_item("1");
    item1.description = Some("cat picture".into());
    cm.insert_media_item(&item1).unwrap();
    let mut item2 = sample_item("2");
    item2.description = Some("dog photo".into());
    cm.insert_media_item(&item2).unwrap();

    let results = cm.get_media_items_by_description("cat").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, item1.id);
}

#[test]
fn test_get_media_items_by_text() {
    let file = NamedTempFile::new().unwrap();
    let cm = CacheManager::new(file.path()).unwrap();
    let mut item1 = sample_item("1");
    item1.description = Some("foo".into());
    item1.filename = "bar.jpg".into();
    cm.insert_media_item(&item1).unwrap();
    let mut item2 = sample_item("2");
    item2.description = Some("bar".into());
    item2.filename = "foo.png".into();
    cm.insert_media_item(&item2).unwrap();

    let results = cm.get_media_items_by_text("foo").unwrap();
    assert_eq!(results.len(), 2);
    let ids: HashSet<_> = results.iter().map(|i| i.id.as_str()).collect();
    assert!(ids.contains("1") && ids.contains("2"));
}

#[test]
fn test_query_media_items_combined() {
    let file = NamedTempFile::new().unwrap();
    let cm = CacheManager::new(file.path()).unwrap();
    let mut item1 = sample_item("1");
    item1.media_metadata.creation_time = "2023-01-02T00:00:00Z".into();
    item1.media_metadata.video = Some(api_client::VideoMetadata {
        camera_make: Some("Canon".into()),
        camera_model: Some("EOS".into()),
        fps: None,
        status: None,
    });
    cm.insert_media_item(&item1).unwrap();
    {
        let conn = cm.conn.lock().unwrap();
        conn.execute(
            "UPDATE media_items SET is_favorite = 1 WHERE id = ?1",
            params![item1.id],
        )
        .unwrap();
    }
    let mut item2 = sample_item("2");
    item2.media_metadata.creation_time = "2023-02-01T00:00:00Z".into();
    item2.media_metadata.video = Some(api_client::VideoMetadata {
        camera_make: Some("Nikon".into()),
        camera_model: Some("D5".into()),
        fps: None,
        status: None,
    });
    cm.insert_media_item(&item2).unwrap();
    let start = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2023, 1, 31, 23, 59, 59).unwrap();
    let results = cm
        .query_media_items(Some("EOS"), Some(start), Some(end), Some(true))
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, item1.id);
}
