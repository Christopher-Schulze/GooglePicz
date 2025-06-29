//! API client module for Google Photos.

use serde::{Deserialize, Serialize};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::error::Error;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    pub description: Option<String>,
    pub product_url: String,
    pub base_url: String,
    pub mime_type: String,
    pub media_metadata: MediaMetadata,
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub creation_time: String,
    pub width: String,
    pub height: String,
    // Other fields like photo, video can be added if needed
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListMediaItemsResponse {
    media_items: Option<Vec<MediaItem>>,
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchMediaItemsRequest {
    album_id: Option<String>,
    page_size: i32,
    page_token: Option<String>,
}

#[derive(Debug)]
pub enum ApiClientError {
    RequestError(String),
    GoogleApiError(String),
    Other(String),
}

impl fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiClientError::RequestError(msg) => write!(f, "Request Error: {}", msg),
            ApiClientError::GoogleApiError(msg) => write!(f, "Google API Error: {}", msg),
            ApiClientError::Other(msg) => write!(f, "Other Error: {}", msg),
        }
    }
}

impl Error for ApiClientError {}

pub struct ApiClient {
    client: reqwest::Client,
    access_token: String,
    base_url: String,
}

impl ApiClient {
    pub fn new(access_token: String) -> Self {
        Self::new_with_base_url(access_token, "https://photoslibrary.googleapis.com/v1")
    }

    pub fn new_with_base_url(access_token: String, base_url: impl Into<String>) -> Self {
        ApiClient {
            client: reqwest::Client::new(),
            access_token,
            base_url: base_url.into(),
        }
    }

    pub async fn list_media_items(&self, page_size: i32, page_token: Option<String>) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        let mut url = format!("{}/mediaItems?pageSize={}", self.base_url, page_size);
        if let Some(token) = page_token {
            url.push_str(&format!("&pageToken={}", token));
        }

        let response = self.client.get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }
        
        let list_response = response.json::<ListMediaItemsResponse>().await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        Ok((list_response.media_items.unwrap_or_default(), list_response.next_page_token))
    }

    pub async fn search_media_items(&self, album_id: Option<String>, page_size: i32, page_token: Option<String>) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        let url = format!("{}/mediaItems:search", self.base_url);
        
        let request_body = SearchMediaItemsRequest {
            album_id,
            page_size,
            page_token,
        };

        let response = self.client.post(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let search_response = response.json::<ListMediaItemsResponse>().await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        Ok((search_response.media_items.unwrap_or_default(), search_response.next_page_token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mocks::{photos_server, expect_list, expect_search};

    #[tokio::test]
    async fn test_list_media_items() {
        let server = photos_server();
        expect_list(&server);
        let client = ApiClient::new_with_base_url("token".into(), server.url_str("/v1"));

        let (items, next) = client.list_media_items(1, None).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(next.is_none());
        assert_eq!(items[0].id, "1");
    }

    #[tokio::test]
    async fn test_search_media_items() {
        let server = photos_server();
        expect_search(&server);
        let client = ApiClient::new_with_base_url("token".into(), server.url_str("/v1"));

        let (items, next) = client.search_media_items(None, 1, None).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(next.is_none());
        assert_eq!(items[0].filename, "file.jpg");
    }
}
