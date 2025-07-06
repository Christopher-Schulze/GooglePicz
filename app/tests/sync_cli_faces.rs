use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;
use cache::{CacheManager, FaceData};

fn build_cmd(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("MOCK_REFRESH_TOKEN", "test");
    cmd.env("HOME", home);
    cmd
}

fn sample_item(id: &str) -> api_client::MediaItem {
    api_client::MediaItem {
        id: id.to_string(),
        description: Some("desc".into()),
        product_url: "http://example.com".into(),
        base_url: "http://example.com/base".into(),
        mime_type: "image/jpeg".into(),
        media_metadata: api_client::MediaMetadata {
            creation_time: "2023-01-01T00:00:00Z".into(),
            width: "1".into(),
            height: "1".into(),
            video: None,
        },
        filename: format!("{}.jpg", id),
    }
}

#[test]
fn export_faces_writes_file() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();
    let faces = vec![FaceData { bbox: [0,0,10,10], name: Some("a".into()) }];
    let json = serde_json::to_string(&faces).unwrap();
    cache.insert_faces(&item.id, &json).unwrap();

    let export_file = dir.path().join("faces.json");
    build_cmd(dir.path())
        .args(&["export-faces", "--file", export_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Exported faces"));

    assert!(export_file.exists());
}

#[test]
fn import_faces_populates_cache() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    let faces = vec![cache::FaceExport { media_item_id: item.id.clone(), faces: vec![FaceData { bbox: [1,1,5,5], name: None }] }];
    let file_path = dir.path().join("faces.json");
    std::fs::write(&file_path, serde_json::to_vec(&faces).unwrap()).unwrap();

    build_cmd(dir.path())
        .args(&["import-faces", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Imported faces"));

    let stored = cache.get_faces(&item.id).unwrap().unwrap();
    assert_eq!(stored.len(), 1);
}

#[test]
fn set_favorite_updates_item() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    build_cmd(dir.path())
        .args(&["set-favorite", "1", "true"])
        .assert()
        .success()
        .stdout(contains("Favorite for 1 set to true"));

    let conn = cache.conn.lock().unwrap();
    let fav: i64 = conn.query_row("SELECT is_favorite FROM media_items WHERE id = '1'", [], |r| r.get(0)).unwrap();
    assert_eq!(fav, 1);
}

#[test]
fn search_with_faces_only_lists_matches() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item1 = sample_item("1");
    let item2 = sample_item("2");
    cache.insert_media_item(&item1).unwrap();
    cache.insert_media_item(&item2).unwrap();
    let faces = vec![FaceData { bbox: [0,0,5,5], name: None }];
    let json = serde_json::to_string(&faces).unwrap();
    cache.insert_faces(&item1.id, &json).unwrap();

    build_cmd(dir.path())
        .args(&["search", "1", "--faces"])
        .assert()
        .success()
        .stdout(contains("1 - 1.jpg"))
        .stdout(contains("2 - 2.jpg").not());
}

