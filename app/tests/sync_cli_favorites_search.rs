use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;
use cache::CacheManager;
use rusqlite::{Connection, params};

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
        id: id.to_string(),
        title: Some("Album".into()),
        product_url: None,
        is_writeable: None,
        media_items_count: None,
        cover_photo_base_url: None,
        cover_photo_media_item_id: None,
    }
}

#[test]
fn list_favorites_no_cache() {
    let dir = tempdir().unwrap();
    build_cmd(dir.path())
        .arg("list-favorites")
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn list_favorites_outputs_entries() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();
    let conn = Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE media_items SET is_favorite = 1 WHERE id = ?1",
        params!["1"],
    ).unwrap();

    build_cmd(dir.path())
        .arg("list-favorites")
        .assert()
        .success()
        .stdout(contains("1 - 1.jpg"));
}

#[test]
fn search_albums_no_cache() {
    let dir = tempdir().unwrap();
    build_cmd(dir.path())
        .args(&["search-albums", "Album"]) 
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn search_albums_outputs_matches() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album = sample_album("1");
    cache.insert_album(&album).unwrap();

    build_cmd(dir.path())
        .args(&["search-albums", "Album"]) 
        .assert()
        .success()
        .stdout(contains("Album (id: 1)"));
}
