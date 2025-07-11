#![warn(clippy::all)]
#![warn(rust_2018_idioms)]
//! API client module for Google Photos.

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<VideoMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub fps: Option<f32>,
    pub status: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAlbumRequest {
    album: NewAlbum,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewAlbum {
    title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchCreateMediaItemsResponse {
    new_media_item_results: Vec<NewMediaItemResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewMediaItemResult {
    media_item: Option<MediaItem>,
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiClientError {
    #[error("Request Error: {0}")]
    RequestError(String),
    #[error("Google API Error: {0}")]
    GoogleApiError(String),
    #[error("Other Error: {0}")]
    Other(String),
}

pub struct ApiClient {
    client: reqwest::Client,
    access_token: String,
}

impl ApiClient {
    fn mock_media_item(id: &str) -> MediaItem {
        MediaItem {
            id: id.to_string(),
            description: None,
            product_url: "http://example.com".into(),
            base_url: "http://example.com/base".into(),
            mime_type: "image/jpeg".into(),
            media_metadata: MediaMetadata {
                creation_time: "2023-01-01T00:00:00Z".into(),
                width: "1".into(),
                height: "1".into(),
                video: None,
            },
            filename: format!("{}.jpg", id),
        }
    }

    pub fn new(access_token: String) -> Self {
        ApiClient {
            client: reqwest::Client::new(),
            access_token,
        }
    }

    pub fn set_access_token(&mut self, token: String) {
        self.access_token = token;
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, page_token)))]
    pub async fn list_media_items(
        &self,
        page_size: i32,
        page_token: Option<String>,
    ) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            let items = vec![Self::mock_media_item("1"), Self::mock_media_item("2")];
            return Ok((items, None));
        }
        let mut url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems?pageSize={}",
            page_size
        );
        if let Some(token) = page_token {
            url.push_str(&format!("&pageToken={}", token));
        }

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let list_response = response
            .json::<ListMediaItemsResponse>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        Ok((
            list_response.media_items.unwrap_or_default(),
            list_response.next_page_token,
        ))
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, page_token)))]
    pub async fn list_albums(
        &self,
        page_size: i32,
        page_token: Option<String>,
    ) -> Result<(Vec<Album>, Option<String>), ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            let album = Album {
                id: "1".into(),
                title: Some("Test Album".into()),
                product_url: None,
                is_writeable: None,
                media_items_count: None,
                cover_photo_base_url: None,
                cover_photo_media_item_id: None,
            };
            return Ok((vec![album], None));
        }
        let mut url = format!(
            "https://photoslibrary.googleapis.com/v1/albums?pageSize={}",
            page_size
        );
        if let Some(token) = page_token {
            url.push_str(&format!("&pageToken={}", token));
        }

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let list_response = response
            .json::<ListAlbumsResponse>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        Ok((
            list_response.albums.unwrap_or_default(),
            list_response.next_page_token,
        ))
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, album_id, page_token, filters)))]
    pub async fn search_media_items(
        &self,
        album_id: Option<String>,
        page_size: i32,
        page_token: Option<String>,
        filters: Option<Value>,
    ) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            let items = vec![Self::mock_media_item("3")];
            return Ok((items, None));
        }
        let url = "https://photoslibrary.googleapis.com/v1/mediaItems:search";

        let request_body = SearchMediaItemsRequest {
            album_id,
            page_size,
            page_token,
            filters,
        };

        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let search_response = response
            .json::<ListMediaItemsResponse>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        Ok((
            search_response.media_items.unwrap_or_default(),
            search_response.next_page_token,
        ))
    }

    /// Create a new album with the given title.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn create_album(&self, title: &str) -> Result<Album, ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            return Ok(Album {
                id: "1".into(),
                title: Some(title.to_string()),
                product_url: None,
                is_writeable: None,
                media_items_count: None,
                cover_photo_base_url: None,
                cover_photo_media_item_id: None,
            });
        }
        let url = "https://photoslibrary.googleapis.com/v1/albums";
        let body = CreateAlbumRequest {
            album: NewAlbum {
                title: title.to_string(),
            },
        };

        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let album = response
            .json::<Album>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;
        Ok(album)
    }

    /// Rename an existing album (returns new Album object).
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn rename_album(&self, album_id: &str, title: &str) -> Result<Album, ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            return Ok(Album {
                id: album_id.to_string(),
                title: Some(title.to_string()),
                product_url: None,
                is_writeable: None,
                media_items_count: None,
                cover_photo_base_url: None,
                cover_photo_media_item_id: None,
            });
        }

        let url = format!(
            "https://photoslibrary.googleapis.com/v1/albums/{}?updateMask=title",
            album_id
        );
        let body = serde_json::json!({ "title": title });

        let response = self
            .client
            .patch(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let album = response
            .json::<Album>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;
        Ok(album)
    }

    /// Delete an album from Google Photos.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn delete_album(&self, album_id: &str) -> Result<(), ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            return Ok(());
        }

        let url = format!("https://photoslibrary.googleapis.com/v1/albums/{}", album_id);
        let response = self
            .client
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }
        Ok(())
    }

    /// Retrieve media items for a specific album using its ID.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, page_token)))]
    pub async fn get_album_media_items(
        &self,
        album_id: &str,
        page_size: i32,
        page_token: Option<String>,
    ) -> Result<(Vec<MediaItem>, Option<String>), ApiClientError> {
        self.search_media_items(Some(album_id.to_string()), page_size, page_token, None)
            .await
    }

    /// Retrieve the last modification timestamp of an album if available.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn get_album_modified_time(
        &self,
        album_id: &str,
    ) -> Result<Option<String>, ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            return Ok(Some("2023-01-02T00:00:00Z".into()));
        }

        let url = format!("https://photoslibrary.googleapis.com/v1/albums/{}", album_id);
        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let value: Value = response
            .json()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        Ok(value
            .get("updateTime")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    /// Remove a media item from the given album.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn remove_media_item_from_album(
        &self,
        album_id: &str,
        media_item_id: &str,
    ) -> Result<(), ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            return Ok(());
        }

        let url = format!(
            "https://photoslibrary.googleapis.com/v1/albums/{}:batchRemoveMediaItems",
            album_id
        );
        let body = serde_json::json!({ "mediaItemIds": [media_item_id] });

        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        Ok(())
    }

    /// Update the description metadata of a media item.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn update_media_item_description(
        &self,
        media_item_id: &str,
        description: &str,
    ) -> Result<MediaItem, ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            let mut item = Self::mock_media_item(media_item_id);
            item.description = Some(description.to_string());
            return Ok(item);
        }

        let url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems/{}?updateMask=description",
            media_item_id
        );
        let body = serde_json::json!({ "description": description });

        let response = self
            .client
            .patch(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let item = response
            .json::<MediaItem>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;
        Ok(item)
    }

    /// Upload new media data and create a media item in Google Photos.
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, data)))]
    pub async fn upload_media_item(
        &self,
        data: &[u8],
        file_name: &str,
        description: &str,
    ) -> Result<MediaItem, ApiClientError> {
        if std::env::var("MOCK_API_CLIENT").is_ok() {
            return Ok(Self::mock_media_item("uploaded"));
        }

        // Step 1: upload bytes and obtain upload token
        let upload_token = self
            .client
            .post("https://photoslibrary.googleapis.com/v1/uploads")
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header("X-Goog-Upload-File-Name", file_name)
            .header("X-Goog-Upload-Protocol", "raw")
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?
            .text()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        // Step 2: create the media item from the upload token
        let body = serde_json::json!({
            "newMediaItems": [{
                "description": description,
                "simpleMediaItem": {
                    "uploadToken": upload_token,
                    "fileName": file_name
                }
            }]
        });

        let response = self
            .client
            .post("https://photoslibrary.googleapis.com/v1/mediaItems:batchCreate")
            .header(AUTHORIZATION, format!("Bearer {}", self.access_token))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiClientError::GoogleApiError(error_text));
        }

        let result = response
            .json::<BatchCreateMediaItemsResponse>()
            .await
            .map_err(|e| ApiClientError::RequestError(e.to_string()))?;

        let media_item = result
            .new_media_item_results
            .into_iter()
            .next()
            .and_then(|r| r.media_item)
            .ok_or_else(|| ApiClientError::Other("No media item returned".into()))?;

        Ok(media_item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
    #[serial]
    async fn test_create_album_mock() {
        std::env::set_var("MOCK_API_CLIENT", "1");
        let client = ApiClient::new("token".into());
        let album = client.create_album("My Album").await.unwrap();
        assert_eq!(album.title.as_deref(), Some("My Album"));
        std::env::remove_var("MOCK_API_CLIENT");
    }

    #[tokio::test]
    #[serial]
    async fn test_rename_album_mock() {
        std::env::set_var("MOCK_API_CLIENT", "1");
        let client = ApiClient::new("token".into());
        let album = client.rename_album("1", "Renamed").await.unwrap();
        assert_eq!(album.title.as_deref(), Some("Renamed"));
        std::env::remove_var("MOCK_API_CLIENT");
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_album_mock() {
        std::env::set_var("MOCK_API_CLIENT", "1");
        let client = ApiClient::new("token".into());
        client.delete_album("1").await.unwrap();
        std::env::remove_var("MOCK_API_CLIENT");
    }
}
