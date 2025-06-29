//! API client module for Google Photos.

use serde::{Deserialize, Serialize};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::error::Error;
use std::fmt;
use std::path::Path;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub title: Option<String>,
    pub product_url: Option<String>,
    pub is_writeable: Option<bool>,
    pub media_items_count: Option<String>,
    pub cover_photo_base_url: Option<String>,
    pub cover_photo_media_item_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListMediaItemsResponse {
    media_items: Option<Vec<MediaItem>>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListAlbumsResponse {
    albums: Option<Vec<Album>>,
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchMediaItemsRequest {
    album_id: Option<String>,
    page_size: i32,
    page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchCreateResponse {
    new_media_item_results: Vec<NewMediaItemResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewMediaItemResult {
    media_item: Option<MediaItem>,
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
        ApiClient {
            client: reqwest::Client::new(),
            access_token,
            base_url: "https://photoslibrary.googleapis.com".to_string(),
        }
    }

    #[cfg(test)]
    pub fn with_base_url(access_token: String, base_url: String) -> Self {
        ApiClient {
            client: reqwest::Client::new(),
            access_token,
            base_url,
        }
    }

    pub fn set_access_token(&mut self, token: String) {
        self.access_token = token;
    }

    pub async fn list_media_items(&self, page_size: i32, page_token: Option<String>) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        let mut url = format!("{}/v1/mediaItems?pageSize={}", self.base_url, page_size);
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

    pub async fn list_albums(&self, page_size: i32, page_token: Option<String>) -> Result<(Vec<Album>, Option<String>), ApiClientError> {
        let mut url = format!("{}/v1/albums?pageSize={}", self.base_url, page_size);
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

        let list_response = response.json::<ListAlbumsResponse>().await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        Ok((list_response.albums.unwrap_or_default(), list_response.next_page_token))
    }

    pub async fn search_media_items(&self, album_id: Option<String>, page_size: i32, page_token: Option<String>) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        let url = format!("{}/v1/mediaItems:search", self.base_url);
        
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

    pub async fn upload_media_item(&self, path: &Path, album_id: Option<String>) -> Result<MediaItem, ApiClientError> {
        let file_name = path
            .file_name()
            .ok_or_else(|| ApiClientError::Other("Invalid file path".to_string()))?
            .to_string_lossy();
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| ApiClientError::Other(e.to_string()))?;

        let upload_url = format!("{}/v1/uploads", self.base_url);
        let response = self
            .client
            .post(&upload_url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header("X-Goog-Upload-File-Name", file_name.as_ref())
            .header("X-Goog-Upload-Protocol", "raw")
            .body(bytes)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(text));
        }

        let upload_token = response.text().await.map_err(|e| ApiClientError::RequestError(e.to_string()))?;
        self.add_media_item(&upload_token, file_name.as_ref(), album_id).await
    }

    pub async fn add_media_item(&self, upload_token: &str, file_name: &str, album_id: Option<String>) -> Result<MediaItem, ApiClientError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SimpleMediaItem<'a> {
            upload_token: &'a str,
            file_name: &'a str,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct NewMediaItem<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<String>,
            simple_media_item: SimpleMediaItem<'a>,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct BatchCreateRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            album_id: Option<String>,
            new_media_items: Vec<NewMediaItem<'a>>,
        }

        let request_body = BatchCreateRequest {
            album_id,
            new_media_items: vec![NewMediaItem {
                description: None,
                simple_media_item: SimpleMediaItem {
                    upload_token,
                    file_name,
                },
            }],
        };

        let url = format!("{}/v1/mediaItems:batchCreate", self.base_url);
        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(text));
        }

        let parsed = response
            .json::<BatchCreateResponse>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;
        let result = parsed
            .new_media_item_results
            .into_iter()
            .next()
            .ok_or_else(|| ApiClientError::Other("No result".to_string()))?;
        result.media_item.ok_or_else(|| ApiClientError::Other("No media item returned".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};
    use tempfile;

    #[test]
    fn test_parse_list_albums_response() {
        let json = r#"{
            "albums": [
                {
                    "id": "1",
                    "title": "Test Album",
                    "productUrl": "http://example.com/album/1",
                    "isWriteable": true,
                    "mediaItemsCount": "10",
                    "coverPhotoBaseUrl": "http://example.com/base.jpg",
                    "coverPhotoMediaItemId": "cover1"
                }
            ],
            "nextPageToken": "token123"
        }"#;

        let parsed: ListAlbumsResponse = serde_json::from_str(json).unwrap();
        let albums = parsed.albums.unwrap();
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].id, "1");
        assert_eq!(albums[0].title.as_deref(), Some("Test Album"));
        assert_eq!(parsed.next_page_token, Some("token123".to_string()));
    }

    #[tokio::test]
    async fn test_upload_media_item_request_format() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/uploads"))
            .and(header("X-Goog-Upload-Protocol", "raw"))
            .and(header("X-Goog-Upload-File-Name", "test.jpg"))
            .respond_with(ResponseTemplate::new(200).set_body_string("TOKEN"))
            .mount(&server)
            .await;

        let sample_response = serde_json::json!({
            "newMediaItemResults": [{
                "mediaItem": {
                    "id": "1",
                    "description": null,
                    "productUrl": "http://example.com/1",
                    "baseUrl": "http://example.com/base",
                    "mimeType": "image/jpeg",
                    "mediaMetadata": {"creationTime": "t", "width": "1", "height": "1"},
                    "filename": "test.jpg"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v1/mediaItems:batchCreate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_response))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.jpg");
        std::fs::write(&file_path, "hello world").unwrap();

        let client = ApiClient::with_base_url("token".into(), server.uri());
        let item = client.upload_media_item(&file_path, None).await.unwrap();
        assert_eq!(item.filename, "test.jpg");
    }
}