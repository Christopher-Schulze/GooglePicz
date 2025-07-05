//! Authentication module for Google Photos API.

use keyring::Entry;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use url::Url;
use thiserror::Error;
#[cfg(feature = "file-store")]
use std::path::PathBuf;
#[cfg(feature = "file-store")]
use std::fs;
#[cfg(feature = "file-store")]
use std::collections::HashMap as FileMap;
#[cfg(feature = "file-store")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "file-store")]
use serde_json;


const KEYRING_SERVICE_NAME: &str = "GooglePicz";
const ACCESS_TOKEN_EXPIRY_KEY: &str = "access_token_expiry";
/// Seconds before expiry when we proactively refresh the token.
pub const REFRESH_MARGIN_SECS: u64 = 300;
/// Environment variable to opt into storing tokens in a file instead of the keyring.
pub const USE_FILE_STORE_ENV: &str = "USE_FILE_STORE";

static SCHEDULED_REFRESH: Lazy<Mutex<Option<JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(None));

static MOCK_STORE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(feature = "file-store")]
#[derive(Serialize, Deserialize, Default)]
struct FileTokens(FileMap<String, String>);

#[cfg(feature = "file-store")]
fn token_file_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".googlepicz")
        .join("tokens.json")
}

#[cfg(feature = "file-store")]
fn store_value_file(key: &str, value: &str) -> Result<(), AuthError> {
    let path = token_file_path();
    let mut map = if path.exists() {
        let data = fs::read_to_string(&path).map_err(|e| AuthError::Other(e.to_string()))?;
        serde_json::from_str::<FileTokens>(&data).unwrap_or_default().0
    } else {
        FileMap::new()
    };
    map.insert(key.to_string(), value.to_string());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AuthError::Other(e.to_string()))?;
    }
    let data = serde_json::to_string(&FileTokens(map)).map_err(|e| AuthError::Other(e.to_string()))?;
    fs::write(path, data).map_err(|e| AuthError::Other(e.to_string()))
}

#[cfg(feature = "file-store")]
fn get_value_file(key: &str) -> Result<Option<String>, AuthError> {
    let path = token_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path).map_err(|e| AuthError::Other(e.to_string()))?;
    let map = serde_json::from_str::<FileTokens>(&data).unwrap_or_default();
    Ok(map.0.get(key).cloned())
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Keyring error: {0}")]
    Keyring(String),
    #[error("OAuth error: {0}")]
    OAuth(String),
    #[error("Other error: {0}")]
    Other(String),
}

fn store_value(key: &str, value: &str) -> Result<(), AuthError> {
    if std::env::var("MOCK_KEYRING").is_ok() {
        let mut store = MOCK_STORE
            .lock()
            .map_err(|_| AuthError::Other("Poisoned mock store lock".into()))?;
        store.insert(key.to_string(), value.to_string());
        return Ok(());
    }
    #[cfg(feature = "file-store")]
    if std::env::var(USE_FILE_STORE_ENV).is_ok() {
        return store_value_file(key, value);
    }
    {
        let entry = Entry::new(KEYRING_SERVICE_NAME, key)
            .map_err(|e| AuthError::Keyring(e.to_string()))?;
        match entry.set_password(value) {
            Ok(_) => Ok(()),
            Err(e) => {
                #[cfg(feature = "file-store")]
                {
                    store_value_file(key, value)?;
                    Ok(())
                }
                #[cfg(not(feature = "file-store"))]
                {
                    Err(AuthError::Keyring(e.to_string()))
                }
            }
        }
    }
}

fn get_value(key: &str) -> Result<Option<String>, AuthError> {
    if std::env::var("MOCK_KEYRING").is_ok() {
        let store = MOCK_STORE
            .lock()
            .map_err(|_| AuthError::Other("Poisoned mock store lock".into()))?;
        return Ok(store.get(key).cloned());
    }
    #[cfg(feature = "file-store")]
    if std::env::var(USE_FILE_STORE_ENV).is_ok() {
        return get_value_file(key);
    }
    {
        let entry = Entry::new(KEYRING_SERVICE_NAME, key)
            .map_err(|e| AuthError::Keyring(e.to_string()))?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => {
                #[cfg(feature = "file-store")]
                {
                    Ok(get_value_file(key)?)
                }
                #[cfg(not(feature = "file-store"))]
                {
                    Ok(None)
                }
            }
            Err(e) => {
                #[cfg(feature = "file-store")]
                {
                    let val = get_value_file(key)?;
                    if val.is_some() {
                        Ok(val)
                    } else {
                        Err(AuthError::Keyring(e.to_string()))
                    }
                }
                #[cfg(not(feature = "file-store"))]
                {
                    Err(AuthError::Keyring(e.to_string()))
                }
            }
        }
    }
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub async fn authenticate(redirect_port: u16) -> Result<(), AuthError> {
    if let Ok(mock_token) = std::env::var("MOCK_ACCESS_TOKEN") {
        store_value("access_token", &mock_token)?;
        if let Ok(refresh) = std::env::var("MOCK_REFRESH_TOKEN") {
            store_value("refresh_token", &refresh)?;
        }
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        store_value(
            ACCESS_TOKEN_EXPIRY_KEY,
            &expiry
                .duration_since(UNIX_EPOCH)
                .map_err(|e| AuthError::Other(e.to_string()))?
                .as_secs()
                .to_string(),
        )?;
        return Ok(());
    }
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID").map_err(|e| AuthError::Other(e.to_string()))?);
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET").map_err(|e| AuthError::Other(e.to_string()))?);
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).map_err(|e| AuthError::OAuth(e.to_string()))?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).map_err(|e| AuthError::OAuth(e.to_string()))?;

    let redirect_uri = format!("http://127.0.0.1:{}", redirect_port);

    let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        .set_redirect_uri(
            RedirectUrl::new(redirect_uri.clone())
                .map_err(|e| AuthError::OAuth(e.to_string()))?,
        );

    // PKCE code challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, _csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/photoslibrary.readonly".to_string(),
        ))
        .set_pkce_challenge(pkce_challenge)
        .url();

    tracing::info!("Opening browser for authentication: {}", authorize_url);
    // Open the URL in the default browser
    webbrowser::open(authorize_url.as_str()).map_err(|e| AuthError::Other(e.to_string()))?;

    // Await the redirect from the browser
    let listener = TcpListener::bind(format!("127.0.0.1:{}", redirect_port)).await.map_err(|e| AuthError::Other(e.to_string()))?;
    let (stream, _) = listener.accept().await.map_err(|e| AuthError::Other(e.to_string()))?;
    let mut stream = BufReader::new(stream);

    let mut request_line = String::new();
    stream.read_line(&mut request_line).await.map_err(|e| AuthError::Other(e.to_string()))?;

    let redirect_url = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| AuthError::Other("No redirect URL found".into()))?;
    let redirect_url = Url::parse(&format!("{}{}", redirect_uri, redirect_url)).map_err(|e| AuthError::Other(e.to_string()))?;

    let code = AuthorizationCode::new(
        redirect_url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .ok_or_else(|| AuthError::Other("No authorization code found in redirect URL".into()))?,
    );

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| AuthError::OAuth(e.to_string()))?;

    let access_token = token_response.access_token().secret();
    let refresh_token = token_response
        .refresh_token()
        .map(|t| t.secret().to_string());
    let expires_in = token_response
        .expires_in()
        .unwrap_or_else(|| Duration::from_secs(3600));
    let expiry = SystemTime::now() + expires_in;
    let expiry_secs = expiry
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AuthError::Other(e.to_string()))?
        .as_secs();

    // Store tokens securely
    store_value("access_token", access_token)?;
    store_value(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;

    if let Some(refresh_token) = refresh_token {
        store_value("refresh_token", &refresh_token)?;
    }

    tracing::info!("Authentication successful!");
    Ok(())
}

pub fn get_access_token() -> Result<String, AuthError> {
    if let Some(val) = get_value("access_token")? {
        Ok(val)
    } else {
        Err(AuthError::Other("No access token".into()))
    }
}

pub fn get_refresh_token() -> Result<Option<String>, AuthError> {
    Ok(get_value("refresh_token")?)
}

fn get_access_token_expiry() -> Result<Option<u64>, AuthError> {
    Ok(get_value(ACCESS_TOKEN_EXPIRY_KEY)?.map(|v| v.parse().unwrap_or(0)))
}

fn cancel_scheduled_refresh() {
    if let Some(handle) = SCHEDULED_REFRESH.lock().unwrap().take() {
        handle.abort();
    }
}

fn schedule_token_refresh(expiry: u64) {
    cancel_scheduled_refresh();
    let when_secs = expiry.saturating_sub(REFRESH_MARGIN_SECS);
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    let delay = when_secs.saturating_sub(now_secs);
    let handle = tokio::spawn(async move {
        if delay > 0 {
            sleep(Duration::from_secs(delay)).await;
        }
        if let Err(e) = refresh_access_token().await {
            tracing::error!("Scheduled token refresh failed: {}", e);
        }
    });
    *SCHEDULED_REFRESH.lock().unwrap() = Some(handle);
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub async fn refresh_access_token() -> Result<String, AuthError> {
    if let Ok(mock_token) = std::env::var("MOCK_REFRESH_TOKEN") {
        let new_token = mock_token;
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        let expiry_secs = expiry
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AuthError::Other(e.to_string()))?
            .as_secs();
        store_value("access_token", &new_token)?;
        store_value(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;
        return Ok(new_token);
    }
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID").map_err(|e| AuthError::Other(e.to_string()))?);
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET").map_err(|e| AuthError::Other(e.to_string()))?);
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).map_err(|e| AuthError::OAuth(e.to_string()))?;

    let client = BasicClient::new(
        client_id,
        Some(client_secret),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| AuthError::OAuth(e.to_string()))?,
        Some(token_url),
    );

    let refresh_token = get_refresh_token()?.ok_or_else(|| AuthError::Other("No refresh token found".into()))?;

    let token_response = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await
        .map_err(|e| AuthError::OAuth(e.to_string()))?;

    let access_token = token_response.access_token().secret();
    let expires_in = token_response
        .expires_in()
        .unwrap_or_else(|| Duration::from_secs(3600));
    let expiry = SystemTime::now() + expires_in;
    let expiry_secs = expiry
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AuthError::Other(e.to_string()))?
        .as_secs();
    store_value("access_token", access_token)?;
    store_value(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;

    Ok(access_token.to_string())
}

/// Ensure the stored access token is valid, refreshing it if expired.
#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub async fn ensure_access_token_valid() -> Result<String, AuthError> {
    let mut expiry = get_access_token_expiry()?.unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AuthError::Other(e.to_string()))?
        .as_secs();
    if expiry <= now + REFRESH_MARGIN_SECS {
        // expired or about to expire soon
        let token = refresh_access_token().await?;
        expiry = get_access_token_expiry()?.unwrap_or(expiry);
        schedule_token_refresh(expiry);
        Ok(token)
    } else {
        schedule_token_refresh(expiry);
        get_access_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tokio;

    // Note: These tests require GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET to be set
    // and may require manual interaction for the initial authentication flow.
    // They are primarily for demonstrating the functionality.

    #[tokio::test]
    #[serial]
    async fn test_authenticate() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_ACCESS_TOKEN", "token1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "refresh1");

        let result = authenticate(8080).await;
        assert!(result.is_ok(), "Authentication failed: {:?}", result.err());
        let token = get_access_token();
        assert!(token.is_ok());
        tracing::info!("Access Token: {}", token.unwrap());
        std::env::remove_var("MOCK_KEYRING");
        std::env::remove_var("MOCK_ACCESS_TOKEN");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
    }

    #[tokio::test]
    #[serial]
    async fn test_refresh_access_token() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "new_token");

        let result = refresh_access_token().await;
        assert!(result.is_ok(), "Refresh token failed: {:?}", result.err());
        let new_token = result.unwrap();
        tracing::info!("New Access Token: {}", new_token);
        assert!(!new_token.is_empty());
        std::env::remove_var("MOCK_KEYRING");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
    }

    #[tokio::test]
    #[serial]
    async fn test_ensure_access_token_valid_no_refresh_needed() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "unused");
        store_value("access_token", "valid_token").unwrap();
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        store_value(
            ACCESS_TOKEN_EXPIRY_KEY,
            &expiry
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        )
        .unwrap();

        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "valid_token");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial]
    async fn test_ensure_access_token_valid_with_refresh() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "new_token");
        store_value("access_token", "old_token").unwrap();
        let expiry = SystemTime::now() - Duration::from_secs(10);
        store_value(
            ACCESS_TOKEN_EXPIRY_KEY,
            &expiry
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        )
        .unwrap();

        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "new_token");
        let stored = get_access_token().unwrap();
        assert_eq!(stored, "new_token");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_access_token_missing() {
        std::env::set_var("MOCK_KEYRING", "1");
        {
            let mut store = MOCK_STORE.lock().unwrap();
            store.remove("access_token");
        }
        let result = get_access_token();
        assert!(result.is_err());
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial]
    async fn test_refresh_access_token_missing_vars() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("GOOGLE_CLIENT_ID");
        std::env::remove_var("GOOGLE_CLIENT_SECRET");
        let result = refresh_access_token().await;
        assert!(result.is_err());
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial]
    async fn test_ensure_access_token_valid_expiring_soon() {
        cancel_scheduled_refresh();
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "soon_new");
        store_value("access_token", "soon_old").unwrap();
        let expiry = SystemTime::now() + Duration::from_secs(10);
        store_value(
            ACCESS_TOKEN_EXPIRY_KEY,
            &expiry
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        )
        .unwrap();
        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "soon_new");
        cancel_scheduled_refresh();
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial]
    async fn test_scheduled_refresh_happens() {
        cancel_scheduled_refresh();
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "sched_new");
        store_value("access_token", "sched_old").unwrap();
        let expiry = SystemTime::now() + Duration::from_secs(REFRESH_MARGIN_SECS + 1);
        store_value(
            ACCESS_TOKEN_EXPIRY_KEY,
            &expiry
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        )
        .unwrap();
        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "sched_old");
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let stored = get_access_token().unwrap();
        assert_eq!(stored, "sched_new");
        cancel_scheduled_refresh();
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }

    #[cfg(feature = "file-store")]
    #[tokio::test]
    #[serial]
    async fn test_store_tokens_in_file() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::env::set_var(USE_FILE_STORE_ENV, "1");
        std::env::set_var("HOME", dir.path());
        store_value("access_token", "file_token").unwrap();
        let val = get_value("access_token").unwrap();
        assert_eq!(val.unwrap(), "file_token");
        let path = dir.path().join(".googlepicz").join("tokens.json");
        assert!(path.exists());
        std::env::remove_var(USE_FILE_STORE_ENV);
    }
}
