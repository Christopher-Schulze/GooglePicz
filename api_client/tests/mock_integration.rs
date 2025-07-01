use api_client::ApiClient;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_list_and_search_media_items_mock() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    let client = ApiClient::new("token".into());
    let (items, token) = client.list_media_items(10, None).await.unwrap();
    assert_eq!(items.len(), 2);
    assert!(token.is_none());
    let (searched, _) = client
        .search_media_items(None, 10, None, None)
        .await
        .unwrap();
    assert_eq!(searched.len(), 1);
    std::env::remove_var("MOCK_API_CLIENT");
}

#[tokio::test]
#[serial]
async fn test_album_management_mock() {
    std::env::set_var("MOCK_API_CLIENT", "1");
    let client = ApiClient::new("token".into());
    let album = client.create_album("Test").await.unwrap();
    assert_eq!(album.title.as_deref(), Some("Test"));
    let album = client.rename_album(&album.id, "New").await.unwrap();
    assert_eq!(album.title.as_deref(), Some("New"));
    client.delete_album(&album.id).await.unwrap();
    std::env::remove_var("MOCK_API_CLIENT");
}
