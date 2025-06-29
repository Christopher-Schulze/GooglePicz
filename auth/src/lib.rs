//! Authentication module for Google Photos API.

use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl};
use keyring::Entry;

fn set_secret(name: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("MOCK_KEYRING").is_ok() {
        std::env::set_var(format!("MOCK_KEYRING_{}", name), value);
        Ok(())
    } else {
        Entry::new(KEYRING_SERVICE_NAME, name)?.set_password(value)?;
        Ok(())
    }
}

fn get_secret(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if std::env::var("MOCK_KEYRING").is_ok() {
        match std::env::var(format!("MOCK_KEYRING_{}", name)) {
            Ok(v) => Ok(Some(v)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    } else {
        let entry = Entry::new(KEYRING_SERVICE_NAME, name)?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use url::Url;

const KEYRING_SERVICE_NAME: &str = "GooglePicz";
const ACCESS_TOKEN_EXPIRY_KEY: &str = "access_token_expiry";

pub async fn authenticate() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(tokens) = std::env::var("MOCK_AUTH_TOKENS") {
        let parts: Vec<_> = tokens.split(',').collect();
        let access = parts.get(0).unwrap_or(&"mock").to_string();
        let refresh = parts.get(1).map(|s| s.to_string());
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();
        set_secret("access_token", &access)?;
        set_secret(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;
        if let Some(r) = refresh {
            set_secret("refresh_token", &r)?;
        }
        return Ok(());
    }
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
    set_secret("access_token", access_token)?;
    set_secret(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;

    if let Some(refresh_token) = refresh_token {
        set_secret("refresh_token", &refresh_token)?;
    }

    tracing::info!("Authentication successful!");
    Ok(())
}

pub fn get_access_token() -> Result<String, Box<dyn std::error::Error>> {
    get_secret("access_token")?.ok_or_else(|| "NoEntry".into())
}

pub fn get_refresh_token() -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(get_secret("refresh_token")?)
}

fn get_access_token_expiry() -> Result<Option<u64>, Box<dyn std::error::Error>> {
    match get_secret(ACCESS_TOKEN_EXPIRY_KEY)? {
        Some(val) => Ok(val.parse().ok()),
        None => Ok(None),
    }
}

pub async fn refresh_access_token() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(mock_token) = std::env::var("MOCK_REFRESH_TOKEN") {
        let new_token = mock_token;
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        let expiry_secs = expiry.duration_since(UNIX_EPOCH)?.as_secs();
        set_secret("access_token", &new_token)?;
        set_secret(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;
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
    set_secret("access_token", access_token)?;
    set_secret(ACCESS_TOKEN_EXPIRY_KEY, &expiry_secs.to_string())?;

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

    // Note: These tests require GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET to be set
    // and may require manual interaction for the initial authentication flow.
    // They are primarily for demonstrating the functionality.

    #[tokio::test]
    #[serial_test::serial]
    async fn test_authenticate() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_AUTH_TOKENS", "access,refresh");

        authenticate().await.unwrap();
        assert_eq!(get_access_token().unwrap(), "access");
        assert_eq!(get_refresh_token().unwrap().unwrap(), "refresh");
        std::env::remove_var("MOCK_AUTH_TOKENS");
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_refresh_access_token() {
        std::env::set_var("MOCK_KEYRING", "1");
        set_secret("refresh_token", "stored_refresh").unwrap();
        std::env::set_var("MOCK_REFRESH_TOKEN", "new_token");

        let new_token = refresh_access_token().await.unwrap();
        assert_eq!(new_token, "new_token");
        let stored = get_access_token().unwrap();
        assert_eq!(stored, "new_token");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_ensure_access_token_valid_no_refresh_needed() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "unused");
        set_secret("access_token", "valid_token").unwrap();
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        set_secret(ACCESS_TOKEN_EXPIRY_KEY, &expiry.duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()).unwrap();

        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "valid_token");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_ensure_access_token_valid_with_refresh() {
        std::env::set_var("MOCK_KEYRING", "1");
        std::env::set_var("MOCK_REFRESH_TOKEN", "new_token");
        set_secret("access_token", "old_token").unwrap();
        let expiry = SystemTime::now() - Duration::from_secs(10);
        set_secret(ACCESS_TOKEN_EXPIRY_KEY, &expiry.duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()).unwrap();

        let token = ensure_access_token_valid().await.unwrap();
        assert_eq!(token, "new_token");
        let stored = get_access_token().unwrap();
        assert_eq!(stored, "new_token");
        std::env::remove_var("MOCK_REFRESH_TOKEN");
        std::env::remove_var("MOCK_KEYRING");
    }
}