//! Authentication module for Google Photos API.

use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl};
#[cfg(test)]
use mocks::{Entry, Error as KeyringError};
#[cfg(not(test))]
use keyring::{Entry, Error as KeyringError};
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use url::Url;

const KEYRING_SERVICE_NAME: &str = "GooglePicz";
const ACCESS_TOKEN_EXPIRY_KEY: &str = "access_token_expiry";

pub async fn authenticate() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID")?);
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET")?);
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?;

    let client = BasicClient::new(
        client_id,
        Some(client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new("http://127.0.0.1:8080".to_string())?);

    // PKCE code challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/photoslibrary.readonly".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    tracing::info!("Opening browser for authentication: {}", authorize_url);
    // Open the URL in the default browser
    webbrowser::open(authorize_url.as_str())?;

    // Await the redirect from the browser
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let (stream, _) = listener.accept().await?;
    let mut stream = BufReader::new(stream);

    let mut request_line = String::new();
    stream.read_line(&mut request_line).await?;

    let redirect_url = request_line.split_whitespace().nth(1).ok_or("No redirect URL found")?;
    let redirect_url = Url::parse(&format!("http://127.0.0.1:8080{}", redirect_url))?;

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
    let refresh_token = token_response.refresh_token().map(|t| t.secret().to_string());
    let expires_in = token_response.expires_in().unwrap_or_else(|| Duration::from_secs(3600));
    let expiry = SystemTime::now() + expires_in;
    let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();

    // Store tokens securely
    let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token")?;
    entry.set_password(access_token)?;
    let exp_entry = Entry::new(KEYRING_SERVICE_NAME, ACCESS_TOKEN_EXPIRY_KEY)?;
    exp_entry.set_password(&expiry_secs.to_string())?;

    if let Some(refresh_token) = refresh_token {
        let entry = Entry::new(KEYRING_SERVICE_NAME, "refresh_token")?;
        entry.set_password(&refresh_token)?;
    }

    tracing::info!("Authentication successful!");
    Ok(())
}

pub fn get_access_token() -> Result<String, Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token")?;
    Ok(entry.get_password()?)
}

pub fn get_refresh_token() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE_NAME, "refresh_token")?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

fn get_access_token_expiry() -> Result<Option<u64>, Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE_NAME, ACCESS_TOKEN_EXPIRY_KEY)?;
    match entry.get_password() {
        Ok(val) => Ok(Some(val.parse().unwrap_or(0))),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn refresh_access_token() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(mock_token) = std::env::var("MOCK_REFRESH_TOKEN") {
        let new_token = mock_token;
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();
        let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token")?;
        entry.set_password(&new_token)?;
        let exp_entry = Entry::new(KEYRING_SERVICE_NAME, ACCESS_TOKEN_EXPIRY_KEY)?;
        exp_entry.set_password(&expiry_secs.to_string())?;
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
    let expires_in = token_response.expires_in().unwrap_or_else(|| Duration::from_secs(3600));
    let expiry = SystemTime::now() + expires_in;
    let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();
    let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token")?;
    entry.set_password(access_token)?;
    let exp_entry = Entry::new(KEYRING_SERVICE_NAME, ACCESS_TOKEN_EXPIRY_KEY)?;
    exp_entry.set_password(&expiry_secs.to_string())?;

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
    use tokio;
    use mocks;

    // Note: These tests require GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET to be set
    // and may require manual interaction for the initial authentication flow.
    // They are primarily for demonstrating the functionality.


    #[tokio::test]
    #[serial_test::serial]
    async fn test_refresh_access_token() {
        mocks::setup_mock_keyring();
        std::env::set_var("MOCK_REFRESH_TOKEN", "refreshed");

        let result = refresh_access_token().await;
        assert!(result.is_ok(), "Refresh token failed: {:?}", result.err());
        let new_token = result.unwrap();
        tracing::info!("New Access Token: {}", new_token);
        assert!(!new_token.is_empty());
        assert_eq!(new_token, "refreshed");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_ensure_access_token_valid_no_refresh_needed() {
        mocks::setup_mock_keyring();
        std::env::set_var("MOCK_REFRESH_TOKEN", "unused");
        let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token").unwrap();
        entry.set_password("valid_token").unwrap();
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        let exp_entry = Entry::new(KEYRING_SERVICE_NAME, ACCESS_TOKEN_EXPIRY_KEY).unwrap();
        exp_entry.set_password(&expiry.duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()).unwrap();

        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "valid_token");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_ensure_access_token_valid_with_refresh() {
        mocks::setup_mock_keyring();
        std::env::set_var("MOCK_REFRESH_TOKEN", "new_token");
        let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token").unwrap();
        entry.set_password("old_token").unwrap();
        let expiry = SystemTime::now() - Duration::from_secs(10);
        let exp_entry = Entry::new(KEYRING_SERVICE_NAME, ACCESS_TOKEN_EXPIRY_KEY).unwrap();
        exp_entry.set_password(&expiry.duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()).unwrap();

        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "new_token");
        let stored = Entry::new(KEYRING_SERVICE_NAME, "access_token").unwrap().get_password().unwrap();
        assert_eq!(stored, "new_token");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
    }
}