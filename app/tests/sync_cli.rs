use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::tempdir;
use cache::CacheManager;

// Helper function to create command with mocked environment
fn cli_command() -> Command {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("HOME", dir.path());
    cmd
}

#[test]
fn test_help() {
    cli_command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("GooglePicz"));
}

#[test]
fn test_status() {
    cli_command()
        .arg("status")
        .assert()
        .success();
}

#[test]
fn test_list_albums() {
    cli_command()
        .arg("list-albums")
        .assert()
        .success();
}

#[test]
fn test_create_album() {
    cli_command()
        .args(&["create-album", "Test"])
        .assert()
        .success();
}

#[test]
fn test_delete_album() {
    cli_command()
        .args(&["delete-album", "1"])
        .assert()
        .success();
}

#[test]
fn test_cache_stats() {
    cli_command()
        .arg("cache-stats")
        .assert()
        .success();
}

// Helper to build command with a provided HOME directory
fn cli_command_in_home(home: &std::path::Path) -> Command {
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
        },
        filename: format!("{}.jpg", id),
    }
}

#[test]
fn test_create_album_updates_cache() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    CacheManager::new(&db).unwrap();

    cli_command_in_home(dir.path())
        .args(&["create-album", "My Album"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Album created"));

    let cache = CacheManager::new(&db).unwrap();
    let albums = cache.get_all_albums().unwrap();
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].title.as_deref(), Some("My Album"));
}

#[test]
fn test_delete_album_updates_cache() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album = api_client::Album {
        id: "1".into(),
        title: Some("Album".into()),
        product_url: None,
        is_writeable: None,
        media_items_count: None,
        cover_photo_base_url: None,
        cover_photo_media_item_id: None,
    };
    cache.insert_album(&album).unwrap();

    cli_command_in_home(dir.path())
        .args(&["delete-album", "1"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Album deleted"));

    let cache = CacheManager::new(&db).unwrap();
    assert!(cache.get_all_albums().unwrap().is_empty());
}

#[test]
fn test_cache_stats_with_data() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let album = api_client::Album {
        id: "1".into(),
        title: Some("Album".into()),
        product_url: None,
        is_writeable: None,
        media_items_count: None,
        cover_photo_base_url: None,
        cover_photo_media_item_id: None,
    };
    cache.insert_album(&album).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    cli_command_in_home(dir.path())
        .arg("cache-stats")
        .assert()
        .success()
        .stdout(predicates::str::contains("Albums: 1"))
        .stdout(predicates::str::contains("Media items: 1"));
}

#[test]
fn test_sync_updates_cache() {
    let dir = tempdir().unwrap();
    cli_command_in_home(dir.path())
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("Finished sync"));

    let db = dir.path().join(".googlepicz").join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let items = cache.get_all_media_items().unwrap();
    assert!(!items.is_empty());
    let last_sync = cache.get_last_sync().unwrap();
    assert!(last_sync.timestamp() > 0);
}
