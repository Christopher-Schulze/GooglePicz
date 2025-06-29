//! Authentication module for Google Photos API.

use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl};
use keyring::Entry;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use url::Url;

const KEYRING_SERVICE_NAME: &str = "GooglePicz";

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

    println!("Opening browser for authentication: {}", authorize_url);
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

    // Store tokens securely
    let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token")?;
    entry.set_password(access_token)?;

    if let Some(refresh_token) = refresh_token {
        let entry = Entry::new(KEYRING_SERVICE_NAME, "refresh_token")?;
        entry.set_password(&refresh_token)?;
    }

    println!("Authentication successful!");
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
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn refresh_access_token() -> Result<String, Box<dyn std::error::Error>> {
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID")?);
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET")?);
    let token_url_str = std::env::var("GOOGLE_TOKEN_URL").unwrap_or_else(|_| "https://oauth2.googleapis.com/token".to_string());
    let token_url = TokenUrl::new(token_url_str)?;

    let client = BasicClient::new(
        client_id,
        Some(client_secret),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
        Some(token_url),
    );

    let refresh_token = match std::env::var("REFRESH_TOKEN") {
        Ok(tok) => tok,
        Err(_) => get_refresh_token()?.ok_or("No refresh token found")?,
    };

    let token_response = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await?;

    let access_token = token_response.access_token().secret();
    let entry = Entry::new(KEYRING_SERVICE_NAME, "access_token")?;
    entry.set_password(access_token)?;

    Ok(access_token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring::mock;
    use mocks::token_server;

    #[tokio::test]
    async fn test_refresh_access_token() {
        keyring::set_default_credential_builder(mock::default_credential_builder());
        std::env::set_var("REFRESH_TOKEN", "refresh");

        std::env::set_var("GOOGLE_CLIENT_ID", "id");
        std::env::set_var("GOOGLE_CLIENT_SECRET", "secret");

        let server = token_server("new_token");
        std::env::set_var("GOOGLE_TOKEN_URL", server.url_str("/token"));

        let result = refresh_access_token().await;
        assert!(result.is_ok(), "Refresh token failed: {:?}", result.err());
        assert_eq!(result.unwrap(), "new_token");
    }
}