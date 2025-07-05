use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;
use cache::CacheManager;

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
fn search_items_lists_matches() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    build_cmd(dir.path())
        .args(&["search", "1"]) 
        .assert()
        .success()
        .stdout(contains("1 - 1.jpg"));
}

#[test]
fn rename_album_updates_cache() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album = sample_album("1");
    cache.insert_album(&album).unwrap();

    build_cmd(dir.path())
        .args(&["rename-album", "1", "Renamed"])
        .assert()
        .success()
        .stdout(contains("Album renamed"));

    let cache = CacheManager::new(&db).unwrap();
    let albums = cache.get_all_albums().unwrap();
    assert_eq!(albums[0].title.as_deref(), Some("Renamed"));
}

#[test]
fn add_to_album_creates_association() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album = sample_album("1");
    cache.insert_album(&album).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    build_cmd(dir.path())
        .args(&["add-to-album", "1", "1"])
        .assert()
        .success()
        .stdout(contains("Added 1 to album 1"));

    let cache = CacheManager::new(&db).unwrap();
    let items = cache.get_media_items_by_album("1").unwrap();
    assert_eq!(items.len(), 1);
}

#[test]
fn list_album_items_outputs_entries() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album = sample_album("1");
    cache.insert_album(&album).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();
    cache.associate_media_item_with_album(&item.id, &album.id).unwrap();

    build_cmd(dir.path())
        .args(&["list-album-items", "1"])
        .assert()
        .success()
        .stdout(contains("1 - 1.jpg"));
}
