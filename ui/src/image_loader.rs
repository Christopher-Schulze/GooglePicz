//! Image loading and caching functionality for GooglePicz UI.

use api_client;
use iced::widget::image::Handle;
use reqwest;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct ImageLoader {
    cache_dir: PathBuf,
    client: reqwest::Client,
}

impl ImageLoader {
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::new();
        Self { cache_dir, client }
    }

    pub async fn load_thumbnail(
        &self,
        media_id: &str,
        base_url: &str,
    ) -> Result<Handle, Box<dyn std::error::Error>> {
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
        let response = self.client.get(&thumbnail_url).send().await?;
        let bytes = response.bytes().await?;

        // Ensure cache directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Save to cache
        fs::write(&cache_path, &bytes).await?;

        // Create handle
        let handle = Handle::from_path(&cache_path);

        Ok(handle)
    }

    pub async fn load_full_image(
        &self,
        media_id: &str,
        base_url: &str,
    ) -> Result<Handle, Box<dyn std::error::Error>> {
        let full_url = format!("{}=d", base_url);
        let cache_path = self
            .cache_dir
            .join("full")
            .join(format!("{}.jpg", media_id));

        if cache_path.exists() {
            return Ok(Handle::from_path(&cache_path));
        }

        let response = self.client.get(&full_url).send().await?;
        let bytes = response.bytes().await?;

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&cache_path, &bytes).await?;

        Ok(Handle::from_path(&cache_path))
    }

    #[allow(dead_code)]
    pub fn get_cached_thumbnail(&self, _media_id: &str) -> Option<Handle> {
        None // Since we are not caching in memory anymore
    }

    #[allow(dead_code)]
    pub async fn preload_thumbnails(&self, media_items: &[api_client::MediaItem], count: usize) {
        for item in media_items.iter().take(count) {
            if let Err(e) = self.load_thumbnail(&item.id, &item.base_url).await {
                tracing::error!("Failed to preload thumbnail for {}: {}", &item.id, e);
            }
        }
    }
}
