use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;
use cache::CacheManager;

fn build_cmd(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("MOCK_REFRESH_TOKEN", "t");
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
fn export_and_import_items() {
    let dir = tempdir().unwrap();
    let base = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&base).unwrap();
    let db = base.join("cache.sqlite");
    let cache = CacheManager::new(&db).unwrap();
    let item = sample_item("1");
    cache.insert_media_item(&item).unwrap();

    let export_file = dir.path().join("items.json");
    build_cmd(dir.path())
        .args(&["export-items", "--file", export_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Exported"));

    cache.clear_cache().unwrap();

    build_cmd(dir.path())
        .args(&["import-items", "--file", export_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Imported"));

    let items = cache.get_all_media_items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, item.id);
}
