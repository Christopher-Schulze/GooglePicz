use assert_cmd::prelude::*;
use tempfile::tempdir;
use std::process::Command;
use cache::CacheManager;
use api_client::ApiClient;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_REFRESH_TOKEN", "test");

    let dir = tempdir().expect("dir");
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();

    let client = ApiClient::new("token".into());
    let album = client.create_album("Old").await.unwrap();
    cache.insert_album(&album).unwrap();

    Command::cargo_bin("sync_cli")
        .unwrap()
        .env("MOCK_API_CLIENT", "1")
        .env("MOCK_KEYRING", "1")
        .env("MOCK_REFRESH_TOKEN", "test")
        .env("HOME", dir.path())
        .args(&["rename-album", &album.id, "Renamed"])
        .assert()
        .success();

    let cache = CacheManager::new(&db).unwrap();
    let albums = cache.get_all_albums().unwrap();
    assert_eq!(albums[0].title.as_deref(), Some("Renamed"));
}
