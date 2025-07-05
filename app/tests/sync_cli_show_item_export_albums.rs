use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;
use cache::CacheManager;
use serde_json;

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

fn sample_album(id: &str) -> api_client::Album {
    api_client::Album {
        id: id.into(),
        title: Some("Album".into()),
        product_url: None,
        is_writeable: None,
        media_items_count: None,
        cover_photo_base_url: None,
        cover_photo_media_item_id: None,
    }
}

#[test]
fn show_item_outputs_metadata() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    build_cmd(dir.path())
        .args(&["show-item", "1"])
        .assert()
        .success()
        .stdout(contains("\"id\": \"1\""));
}

#[test]
fn export_albums_writes_file() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album = sample_album("1");
    cache.insert_album(&album).unwrap();

    let export_file = dir.path().join("albums.json");
    build_cmd(dir.path())
        .args(&["export-albums", "--file", export_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Exported albums"));

    let exported: Vec<api_client::Album> = serde_json::from_reader(
        std::fs::File::open(&export_file).unwrap(),
    )
    .unwrap();
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].id, album.id);
}
#[test]
fn export_albums_exports_all_entries() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album1 = sample_album("1");
    let album2 = sample_album("2");
    cache.insert_album(&album1).unwrap();
    cache.insert_album(&album2).unwrap();

    let export_file = dir.path().join("all_albums.json");
    build_cmd(dir.path())
        .args(&["export-albums", "--file", export_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Exported albums"));

    let exported: Vec<api_client::Album> = serde_json::from_reader(
        std::fs::File::open(&export_file).unwrap(),
    )
    .unwrap();
    assert_eq!(exported.len(), 2);
}
