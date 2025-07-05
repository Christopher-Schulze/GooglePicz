use api_client::ApiClient;
use cache::CacheManager;
use tempfile::TempDir;

#[tokio::main]
async fn main() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    std::env::set_var("MOCK_KEYRING", "1");

    let dir = TempDir::new().expect("temp dir");
    let db = dir.path().join("cache.sqlite");
    let cache = CacheManager::new(&db).expect("cache");

    let client = ApiClient::new("token".into());
    let album = client.create_album("Holiday").await.expect("album");
    cache.insert_album(&album).expect("insert album");

    let (items, _) = client.list_media_items(1, None).await.expect("items");
    let item = &items[0];
    cache.insert_media_item(item).expect("insert item");

    cache
        .associate_media_item_with_album(&item.id, &album.id)
        .expect("associate");

    let stored = cache
        .get_media_items_by_album(&album.id)
        .expect("get items");
    assert_eq!(stored.len(), 1);
}
