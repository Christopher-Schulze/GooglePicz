use cache::CacheManager;
use api_client::ApiClient;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");

    let dir = TempDir::new().expect("dir");
    let db = dir.path().join("cache.sqlite");
    let cache = CacheManager::new(&db).expect("cache");

    let client = ApiClient::new("token".into());
    let (items, _) = client
        .search_media_items(None, 10, None, None)
        .await
        .expect("fetch");
    for item in &items {
        cache.insert_media_item(item).expect("insert");
    }

    let results = cache
        .get_media_items_by_filename("3.jpg")
        .expect("search");
    assert_eq!(results.len(), 1);
}
