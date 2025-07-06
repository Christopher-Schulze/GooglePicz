//! Image loading and caching functionality for GooglePicz UI.

use api_client;
use iced::widget::image::Handle;
use reqwest;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use futures::StreamExt;
use tracing::Instrument;
use std::time::Instant;
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Error, PartialEq)]
pub enum ImageLoaderError {
    #[error("network error: {0}")]
    Network(String),
    #[error("timeout")]
    Timeout,
    #[error("not found")]
    NotFound,
    #[error("io error: {0}")]
    Io(String),
    #[error("semaphore closed")]
    SemaphoreClosed,
}

#[derive(Debug, Clone)]
pub struct ImageLoader {
    cache_dir: PathBuf,
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl ImageLoader {
    pub fn new(cache_dir: PathBuf, threads: usize) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build client");
        Self::with_client(cache_dir, client, threads)
    }

    pub fn with_client(cache_dir: PathBuf, client: reqwest::Client, threads: usize) -> Self {
        Self {
            cache_dir,
            client,
            semaphore: Arc::new(Semaphore::new(threads)),
        }
    }

    /// Return path to the cache directory used by this loader
    pub fn cache_dir(&self) -> PathBuf {
        self.cache_dir.clone()
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn load_thumbnail(
        &self,
        media_id: &str,
        base_url: &str,
    ) -> Result<Handle, ImageLoaderError> {
        #[cfg(feature = "trace-spans")]
        let span = tracing::info_span!("load_thumbnail", id = %media_id);
        #[cfg(feature = "trace-spans")]
        let _enter = span.enter();
        let start = Instant::now();
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| ImageLoaderError::SemaphoreClosed)?;
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
            .map_err(|e| {
                if e.is_timeout() {
                    ImageLoaderError::Timeout
                } else {
                    ImageLoaderError::Network(e.to_string())
                }
            })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ImageLoaderError::NotFound);
        }
        if !response.status().is_success() {
            return Err(ImageLoaderError::Network(format!(
                "HTTP {}",
                response.status()
            )));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ImageLoaderError::Timeout
                } else {
                    ImageLoaderError::Network(e.to_string())
                }
            })?;

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

        tracing::info!("thumbnail_time_ms" = %start.elapsed().as_millis(), "id" = media_id);
        Ok(handle)
    }

    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self)))]
    pub async fn load_full_image(
        &self,
        media_id: &str,
        base_url: &str,
    ) -> Result<Handle, ImageLoaderError> {
        #[cfg(feature = "trace-spans")]
        let span = tracing::info_span!("load_full_image", id = %media_id);
        #[cfg(feature = "trace-spans")]
        let _enter = span.enter();
        let start = Instant::now();
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| ImageLoaderError::SemaphoreClosed)?;
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
            .map_err(|e| {
                if e.is_timeout() {
                    ImageLoaderError::Timeout
                } else {
                    ImageLoaderError::Network(e.to_string())
                }
            })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ImageLoaderError::NotFound);
        }
        if !response.status().is_success() {
            return Err(ImageLoaderError::Network(format!(
                "HTTP {}",
                response.status()
            )));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ImageLoaderError::Timeout
                } else {
                    ImageLoaderError::Network(e.to_string())
                }
            })?;

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ImageLoaderError::Io(e.to_string()))?;
        }

        fs::write(&cache_path, &bytes)
            .await
            .map_err(|e| ImageLoaderError::Io(e.to_string()))?;

        tracing::info!("full_image_time_ms" = %start.elapsed().as_millis(), "id" = media_id);
        Ok(Handle::from_path(&cache_path))
    }

    #[allow(dead_code)]
    pub fn get_cached_thumbnail(&self, _media_id: &str) -> Option<Handle> {
        None // Since we are not caching in memory anymore
    }

    #[allow(dead_code)]
    #[cfg_attr(feature = "trace-spans", tracing::instrument(skip(self, media_items)))]
    pub async fn preload_thumbnails(&self, media_items: &[api_client::MediaItem], count: usize) {
        let start = Instant::now();
        let stream = futures::stream::iter(media_items.iter().take(count));
        stream
            .for_each_concurrent(None, |item| {
                let span = tracing::info_span!("preload_thumbnail", id = %item.id);
                async move {
                    if let Err(e) = self.load_thumbnail(&item.id, &item.base_url).await {
                        tracing::error!("Failed to preload thumbnail for {}: {}", &item.id, e);
                    }
                }
                .instrument(span)
            })
            .await;
        tracing::info!("preload_time_ms" = %start.elapsed().as_millis(), "count" = count);
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
        let loader = ImageLoader::new(dir.path().to_path_buf(), 4);
        let url = format!("{}/thumb.jpg", server.url(""));
        let _ = loader.load_thumbnail("1", &url).await.unwrap();
        assert!(dir.path().join("thumbnails/1.jpg").exists());
        mock.assert();
    }
}
