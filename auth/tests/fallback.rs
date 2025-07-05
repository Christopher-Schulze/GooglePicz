use auth::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn uses_keyring_when_available() {
    std::env::set_var("MOCK_KEYRING", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "key_token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "key_refresh");
    authenticate(1).await.unwrap();
    let tok = get_access_token().unwrap();
    assert_eq!(tok, "key_token");
    assert!(std::env::var(USE_FILE_STORE_ENV).is_err());
    std::env::remove_var("MOCK_KEYRING");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
}

#[cfg(feature = "file-store")]
#[tokio::test]
#[serial]
async fn fallback_to_file_store_when_keyring_fails() {
    use tempfile::TempDir;
    let dir = TempDir::new().unwrap();
    std::env::set_var("HOME", dir.path());
    std::env::set_var("MOCK_KEYRING_FAIL", "1");
    std::env::set_var("MOCK_ACCESS_TOKEN", "file_token");
    std::env::set_var("MOCK_REFRESH_TOKEN", "file_refresh");
    authenticate(1).await.unwrap();
    let path = dir.path().join(".googlepicz").join("tokens.json");
    assert!(path.exists());
    assert_eq!(get_access_token().unwrap(), "file_token");
    assert_eq!(std::env::var(USE_FILE_STORE_ENV).unwrap(), "1");
    std::env::remove_var("MOCK_KEYRING_FAIL");
    std::env::remove_var("MOCK_ACCESS_TOKEN");
    std::env::remove_var("MOCK_REFRESH_TOKEN");
    std::env::remove_var(USE_FILE_STORE_ENV);
}
