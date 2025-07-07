use assert_cmd::prelude::*;
use tempfile::tempdir;
use std::process::Command;
use cache::CacheManager;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_REFRESH_TOKEN", "test");

    let dir = tempdir().expect("dir");
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    CacheManager::new(&db).unwrap();

    let img_path = dir.path().join("dummy.jpg");
    std::fs::write(&img_path, [0u8; 4]).unwrap();

    Command::cargo_bin("sync_cli")
        .unwrap()
        .env("MOCK_API_CLIENT", "1")
        .env("MOCK_KEYRING", "1")
        .env("MOCK_REFRESH_TOKEN", "test")
        .env("HOME", dir.path())
        .args(&["upload-item", img_path.to_str().unwrap(), "Test"])
        .assert()
        .success();

    let cache = CacheManager::new(&db).unwrap();
    let items = cache.get_all_media_items().unwrap();
    assert_eq!(items.len(), 1);
}
