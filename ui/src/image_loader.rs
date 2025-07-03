//! Image loading and caching functionality for GooglePicz UI.

use api_client;
use iced::widget::image::Handle;
use reqwest;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use futures::future;
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Error)]
pub enum ImageLoaderError {
    #[error("network error: {0}")]
    Request(String),
    #[error("io error: {0}")]
    Io(String),
}

#[derive(Debug, Clone)]
pub struct ImageLoader {
    cache_dir: PathBuf,
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl ImageLoader {
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::new();
        Self {
            cache_dir,
            client,
            semaphore: Arc::new(Semaphore::new(4)),
        }
    }

    pub async fn load_thumbnail(
        &self,
        media_id: &str,
        base_url: &str,
    ) -> Result<Handle, ImageLoaderError> {
        let _permit = self.semaphore.acquire().await.expect("semaphore");
        // Create thumbnail URL (150x150 pixels)
        let thumbnail_url = format!("{}=w150-h150-c", base_url);

        // Check if cached on disk
        let cache_path = self
            .cache_dir
            .join("thumbnails")
            .join(format!("{}.jpg", media_id));

        if cache_path.exists() {
            let handle = Handle::from_path(&cache_path);
            return Ok(handle);
        }

        // Download thumbnail
        let response = self
            .client
            .get(&thumbnail_url)
            .send()
            .await
            .map_err(|e| ImageLoaderError::Request(e.to_string()))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ImageLoaderError::Request(e.to_string()))?;

        // Ensure cache directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ImageLoaderError::Io(e.to_string()))?;
        }

        // Save to cache
        fs::write(&cache_path, &bytes)
            .await
            .map_err(|e| ImageLoaderError::Io(e.to_string()))?;

        // Create handle
        let handle = Handle::from_path(&cache_path);

        Ok(handle)
    }

    pub async fn load_full_image(
        &self,
        media_id: &str,
        base_url: &str,
    ) -> Result<Handle, ImageLoaderError> {
        let _permit = self.semaphore.acquire().await.expect("semaphore");
        let full_url = format!("{}=d", base_url);
        let cache_path = self
            .cache_dir
            .join("full")
            .join(format!("{}.jpg", media_id));

        if cache_path.exists() {
            return Ok(Handle::from_path(&cache_path));
        }

        let response = self
            .client
            .get(&full_url)
            .send()
            .await
            .map_err(|e| ImageLoaderError::Request(e.to_string()))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ImageLoaderError::Request(e.to_string()))?;

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ImageLoaderError::Io(e.to_string()))?;
        }

        fs::write(&cache_path, &bytes)
            .await
            .map_err(|e| ImageLoaderError::Io(e.to_string()))?;

        Ok(Handle::from_path(&cache_path))
    }

    #[allow(dead_code)]
    pub fn get_cached_thumbnail(&self, _media_id: &str) -> Option<Handle> {
        None // Since we are not caching in memory anymore
    }

    #[allow(dead_code)]
    pub async fn preload_thumbnails(&self, media_items: &[api_client::MediaItem], count: usize) {
        let futures = media_items
            .iter()
            .take(count)
            .map(|item| async move {
                if let Err(e) = self.load_thumbnail(&item.id, &item.base_url).await {
                    tracing::error!("Failed to preload thumbnail for {}: {}", &item.id, e);
                }
            });
        futures::future::join_all(futures).await;
    }
}

#[cfg(test)]
mod tests {
    use super::ImageLoader;
    use httpmock::prelude::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_thumbnail() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/thumb.jpg=w150-h150-c");
            then.status(200).body("img");
        });
        let dir = tempdir().unwrap();
        let loader = ImageLoader::new(dir.path().to_path_buf());
        let url = format!("{}/thumb.jpg", server.url(""));
        let _ = loader.load_thumbnail("1", &url).await.unwrap();
        assert!(dir.path().join("thumbnails/1.jpg").exists());
        mock.assert();
    }
}
