use tempfile::TempDir;
use auth::authenticate;
use sync::Syncer;
use cache::CacheManager;
use ui::{GooglePiczUI, Message};

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");

    let dir = TempDir::new().expect("dir");
    std::env::set_var("HOME", dir.path());
    let cache_dir = dir.path().join(".googlepicz");
    std::fs::create_dir_all(&cache_dir).expect("cache dir");
    let db_path = cache_dir.join("cache.sqlite");

    authenticate(1).await.expect("auth");
    let mut syncer = Syncer::new(&db_path).await.expect("syncer");
    syncer
        .sync_media_items(None, None, None, None)
        .await
        .expect("sync");
    drop(syncer);

    let cache = CacheManager::new(&db_path).expect("cache");
    let items = cache.get_all_media_items().expect("items");
    assert!(!items.is_empty());

    let (mut ui, _) = GooglePiczUI::new((None, None, 0, cache_dir.clone()));
    assert_eq!(ui.error_count(), 0);
    let _ = ui.update(Message::PhotosLoaded(Ok(items)));
    assert_eq!(ui.error_count(), 0);
    assert!(ui.photo_count() > 0);
}
