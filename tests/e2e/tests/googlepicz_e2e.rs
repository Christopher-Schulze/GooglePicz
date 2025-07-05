use assert_cmd::cargo::cargo_bin;
use cache::CacheManager;
use tokio::{process::Command, time::{timeout, Duration}};
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    // Setup temporary home directory
    let dir = TempDir::new().expect("temp dir");

    let bin = cargo_bin("googlepicz");

    let mut child = Command::new("xvfb-run");
    child.arg("-a")
        .arg(bin)
        .arg("--sync-interval-minutes").arg("1")
        .env("MOCK_API_CLIENT", "1")
        .env("MOCK_KEYRING", "1")
        .env("MOCK_ACCESS_TOKEN", "token")
        .env("MOCK_REFRESH_TOKEN", "refresh")
        .env("GOOGLE_CLIENT_ID", "id")
        .env("GOOGLE_CLIENT_SECRET", "secret")
        .env("HOME", dir.path());
    let mut child = child.spawn().expect("spawn googlepicz");

    let _ = timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("process timed out")
        .expect("failed to wait");

    let db_path = dir.path().join(".googlepicz").join("cache.sqlite");
    let cache = CacheManager::new(&db_path).expect("open cache");
    let items = cache.get_all_media_items().expect("read items");
    assert!(!items.is_empty(), "Cache should contain media items");
}
