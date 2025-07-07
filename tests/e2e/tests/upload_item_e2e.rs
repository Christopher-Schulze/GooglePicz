use assert_cmd::prelude::*;
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

    let img_path = dir.path().join("dummy.jpg");
    std::fs::write(&img_path, [0u8; 10]).expect("write img");

    let mut cmd = Command::cargo_bin("sync_cli").expect("bin");
    cmd.env("MOCK_API_CLIENT", "1")
        .env("MOCK_KEYRING", "1")
        .env("MOCK_REFRESH_TOKEN", "test")
        .env("HOME", home)
        .args(&["upload-item", img_path.to_str().unwrap()])
        .assert()
        .success();

    let cache = CacheManager::new(&db).expect("cache");
    let items = cache.get_all_media_items().expect("items");
    assert_eq!(items.len(), 1);
}
