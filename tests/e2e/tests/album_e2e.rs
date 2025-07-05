use api_client::ApiClient;
use cache::CacheManager;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");

    let dir = TempDir::new().expect("temp dir");
    let db = dir.path().join("cache.sqlite");
    let cache = CacheManager::new(&db).expect("init cache");

    let client = ApiClient::new("token".into());
    let album = client.create_album("New Album").await.expect("create");
    cache.insert_album(&album).expect("insert");

    let albums = cache.get_all_albums().expect("read");
    assert_eq!(albums.len(), 1);
}
