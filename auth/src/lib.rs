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
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use url::Url;

const KEYRING_SERVICE_NAME: &str = "GooglePicz";
const ACCESS_TOKEN_EXPIRY_KEY: &str = "access_token_expiry";

static MOCK_STORE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn store_value(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("MOCK_KEYRING").is_ok() {
        MOCK_STORE
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    } else {
        let entry = Entry::new(KEYRING_SERVICE_NAME, key)?;
        entry.set_password(value)?;
        Ok(())
    }
}

fn get_value(key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if std::env::var("MOCK_KEYRING").is_ok() {
        Ok(MOCK_STORE.lock().unwrap().get(key).cloned())
    } else {
        let entry = Entry::new(KEYRING_SERVICE_NAME, key)?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}

pub async fn authenticate(redirect_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(mock_token) = std::env::var("MOCK_ACCESS_TOKEN") {
        store_value("access_token", &mock_token)?;
        if let Ok(refresh) = std::env::var("MOCK_REFRESH_TOKEN") {
            store_value("refresh_token", &refresh)?;
        }
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        store_value(
            ACCESS_TOKEN_EXPIRY_KEY,
            &expiry.duration_since(UNIX_EPOCH)?.as_secs().to_string(),
        )?;
        return Ok(());
    }
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID")?);
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET")?);
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?;

    let redirect_uri = format!("http://127.0.0.1:{}", redirect_port);

    let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        .set_redirect_uri(RedirectUrl::new(redirect_uri.clone())?);

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
    webbrowser::open(authorize_url.as_str())?;

    // Await the redirect from the browser
    let listener = TcpListener::bind(format!("127.0.0.1:{}", redirect_port)).await?;
    let (stream, _) = listener.accept().await?;
    let mut stream = BufReader::new(stream);

    let mut request_line = String::new();
    stream.read_line(&mut request_line).await?;

    let redirect_url = request_line
        .split_whitespace()
        .nth(1)
        .ok_or("No redirect URL found")?;
    let redirect_url = Url::parse(&format!("{}{}", redirect_uri, redirect_url))?;

    let code = AuthorizationCode::new(
        redirect_url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .ok_or("No authorization code found in redirect URL")?,
    );

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?;

    let access_token = token_response.access_token().secret();
    let refresh_token = token_response
        .refresh_token()
        .map(|t| t.secret().to_string());
    let expires_in = token_response
        .expires_in()
        .unwrap_or_else(|| Duration::from_secs(3600));
    let expiry = SystemTime::now() + expires_in;
    let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();

    // Store tokens securely
    store_value("access_token", access_token)?;
    store_value(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;

    if let Some(refresh_token) = refresh_token {
        store_value("refresh_token", &refresh_token)?;
    }

    tracing::info!("Authentication successful!");
    Ok(())
}

pub fn get_access_token() -> Result<String, Box<dyn std::error::Error>> {
    if let Some(val) = get_value("access_token")? {
        Ok(val)
    } else {
        Err("No access token".into())
    }
}

pub fn get_refresh_token() -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(get_value("refresh_token")?)
}

fn get_access_token_expiry() -> Result<Option<u64>, Box<dyn std::error::Error>> {
    Ok(get_value(ACCESS_TOKEN_EXPIRY_KEY)?.map(|v| v.parse().unwrap_or(0)))
}

pub async fn refresh_access_token() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(mock_token) = std::env::var("MOCK_REFRESH_TOKEN") {
        let new_token = mock_token;
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();
        store_value("access_token", &new_token)?;
        store_value(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;
        return Ok(new_token);
    }
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID")?);
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET")?);
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?;

    let client = BasicClient::new(
        client_id,
        Some(client_secret),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
        Some(token_url),
    );

    let refresh_token = get_refresh_token()?.ok_or("No refresh token found")?;

    let token_response = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await?;

    let access_token = token_response.access_token().secret();
    let expires_in = token_response
        .expires_in()
        .unwrap_or_else(|| Duration::from_secs(3600));
    let expiry = SystemTime::now() + expires_in;
    let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();
    store_value("access_token", access_token)?;
    store_value(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;

    Ok(access_token.to_string())
}

/// Ensure the stored access token is valid, refreshing it if expired.
pub async fn ensure_access_token_valid() -> Result<String, Box<dyn std::error::Error>> {
    let expiry = get_access_token_expiry()?.unwrap_or(0);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if expiry <= now + 60 {
        // expired or about to expire in the next minute
        refresh_access_token().await
    } else {
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
}
