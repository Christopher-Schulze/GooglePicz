use sync::{Syncer, SyncProgress};
use cache::CacheManager;
use serial_test::serial;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;

#[tokio::test]
#[serial]
async fn test_sync_flow_mock() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
    let file = NamedTempFile::new().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut syncer = Syncer::new(file.path()).await.unwrap();
    syncer
        .sync_media_items(Some(tx), None, None, None, None)
        .await
        .unwrap();
    match rx.recv().await {
        Some(sync::SyncProgress::Started) => {}
        other => panic!("expected Started progress, got {:?}", other),
    }
    let cache = CacheManager::new(file.path()).unwrap();
    let items = cache.get_all_media_items().unwrap();
    assert!(!items.is_empty());
    std::env::remove_var("MOCK_API_CLIENT");
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
}
