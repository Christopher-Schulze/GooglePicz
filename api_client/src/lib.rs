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
}

impl ApiClient {
    pub fn new(access_token: String) -> Self {
        ApiClient {
            client: reqwest::Client::new(),
            access_token,
        }
    }

    pub fn set_access_token(&mut self, token: String) {
        self.access_token = token;
    }

    pub async fn list_media_items(&self, page_size: i32, page_token: Option<String>) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        let mut url = format!("https://photoslibrary.googleapis.com/v1/mediaItems?pageSize={}", page_size);
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
        let url = "https://photoslibrary.googleapis.com/v1/mediaItems:search";
        
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