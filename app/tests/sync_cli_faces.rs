use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;
use cache::{CacheManager, FaceData};
use rusqlite::Connection;

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
            video: Some(api_client::VideoMetadata {
                camera_make: Some("Canon".into()),
                camera_model: Some("X1".into()),
                fps: None,
                status: None,
            }),
        },
        filename: format!("{}.jpg", id),
    }
}

#[test]
fn export_and_import_faces() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();
    let faces = vec![FaceData { bbox: [0, 0, 10, 10], name: Some("Alice".into()) }];
    let json = serde_json::to_string(&faces).unwrap();
    cache.insert_faces(&item.id, &json).unwrap();

    let export_file = dir.path().join("faces.json");
    build_cmd(dir.path())
        .args(&["export-faces", "--file", export_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Exported faces"));

    Connection::open(&db).unwrap().execute("DELETE FROM faces", []).unwrap();

    build_cmd(dir.path())
        .args(&["import-faces", "--file", export_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Imported faces"));

    let faces_back = cache.get_faces(&item.id).unwrap().unwrap();
    assert_eq!(faces_back.len(), 1);
    assert_eq!(faces_back[0].name.as_deref(), Some("Alice"));
}

#[test]
fn show_faces_outputs_json() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();
    let faces = vec![FaceData { bbox: [1, 1, 5, 5], name: Some("Bob".into()) }];
    let json = serde_json::to_string(&faces).unwrap();
    cache.insert_faces(&item.id, &json).unwrap();

    build_cmd(dir.path())
        .args(&["show-faces", "1"])
        .assert()
        .success()
        .stdout(contains("Bob"));
}

#[test]
fn set_favorite_updates_flag() {
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

    let conn = Connection::open(&db).unwrap();
    let val: i64 = conn
        .query_row(
            "SELECT is_favorite FROM media_items WHERE id = ?1",
            [&item.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(val, 1);
}

#[test]
fn search_with_camera_model_filter() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();

    let mut item1 = sample_item("1");
    item1.filename = "a.jpg".into();
    cache.insert_media_item(&item1).unwrap();

    let mut item2 = sample_item("2");
    item2.filename = "b.jpg".into();
    if let Some(ref mut vid) = item2.media_metadata.video {
        vid.camera_model = Some("X2".into());
    }
    cache.insert_media_item(&item2).unwrap();

    build_cmd(dir.path())
        .args(&["search", "jpg", "--camera-model", "X1"])
        .assert()
        .success()
        .stdout(contains("a.jpg"))
        .stdout(contains("b.jpg").not());
}


