use assert_cmd::prelude::*;
use api_client::ApiClient;
use cache::CacheManager;
use tempfile::TempDir;
use std::process::Command;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_REFRESH_TOKEN", "test");

    let dir = TempDir::new().expect("dir");
    let home = dir.path();
    let base = home.join(".googlepicz");
    std::fs::create_dir_all(&base).expect("create base");
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).expect("cache");

    let client = ApiClient::new("token".into());
    let album = client.create_album("Holiday").await.expect("album");
    cache.insert_album(&album).expect("insert album");

    let mut cmd = Command::cargo_bin("sync_cli").expect("bin");
    cmd.env("MOCK_API_CLIENT", "1")
        .env("MOCK_KEYRING", "1")
        .env("MOCK_REFRESH_TOKEN", "test")
        .env("HOME", home)
        .args(&["rename-album", &album.id, "Renamed"])
        .assert()
        .success();

    let cache = CacheManager::new(&db).expect("cache2");
    let albums = cache.get_all_albums().expect("albums");
    assert_eq!(albums[0].title.as_deref(), Some("Renamed"));
}
