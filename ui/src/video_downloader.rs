use futures_util::StreamExt;
use reqwest;
use std::path::Path;
use tempfile::{Builder, TempPath};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Error, PartialEq)]
pub enum VideoDownloadError {
    #[error("network error: {0}")]
    Network(String),
    #[error("io error: {0}")]
    Io(String),
}

#[derive(Debug, Clone)]
pub struct VideoDownloader {
    client: reqwest::Client,
}

impl VideoDownloader {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }

    /// Download a video incrementally and write to `path`.
    pub async fn download_progressive<P: AsRef<Path>>(
        &self,
        url: &str,
        path: P,
    ) -> Result<(), VideoDownloadError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| VideoDownloadError::Network(e.to_string()))?;

        let mut file = File::create(path.as_ref())
            .await
            .map_err(|e| VideoDownloadError::Io(e.to_string()))?;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| VideoDownloadError::Network(e.to_string()))?;
            file
                .write_all(&bytes)
                .await
                .map_err(|e| VideoDownloadError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// Download a video to a temporary file which is deleted when dropped.
    pub async fn download_to_tempfile(
        &self,
        url: &str,
        extension: &str,
    ) -> Result<TempPath, VideoDownloadError> {
        self
            .download_to_tempfile_with_progress(url, extension, None::<fn(u64, Option<u64>)>)
            .await
    }

    /// Download a video to a temporary file with optional progress callback.
    pub async fn download_to_tempfile_with_progress<F>(
        &self,
        url: &str,
        extension: &str,
        mut progress: Option<F>,
    ) -> Result<TempPath, VideoDownloadError>
    where
        F: FnMut(u64, Option<u64>),
    {
        let mut file = Builder::new()
            .suffix(extension)
            .tempfile()
            .map_err(|e| VideoDownloadError::Io(e.to_string()))?;

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| VideoDownloadError::Network(e.to_string()))?;
        let total = resp.content_length();
        let mut downloaded = 0u64;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| VideoDownloadError::Network(e.to_string()))?;
            downloaded += bytes.len() as u64;
            file.as_file_mut()
                .write_all(&bytes)
                .await
                .map_err(|e| VideoDownloadError::Io(e.to_string()))?;
            if let Some(cb) = progress.as_mut() {
                cb(downloaded, total);
            }
        }
        Ok(file.into_temp_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_download_progressive() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/video.mp4");
            then.status(200).body("video-data");
        });
        let dir = tempdir().unwrap();
        let path = dir.path().join("video.mp4");
        let dl = VideoDownloader::new();
        dl.download_progressive(&format!("{}/video.mp4", server.url("")), &path)
            .await
            .unwrap();
        let content = tokio::fs::read(&path).await.unwrap();
        assert_eq!(content, b"video-data");
        mock.assert();
    }

    #[tokio::test]
    async fn test_download_to_tempfile() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/video.mp4");
            then.status(200).body("video-data");
        });
        let dl = VideoDownloader::new();
        let temp = dl
            .download_to_tempfile(&format!("{}/video.mp4", server.url("")), ".mp4")
            .await
            .unwrap();
        let content = tokio::fs::read(&temp).await.unwrap();
        assert_eq!(content, b"video-data");
        // temp file will be removed on drop
        let path = temp.to_path_buf();
        drop(temp);
        assert!(!path.exists());
        mock.assert();
    }

    #[tokio::test]
    async fn test_download_progress_callback() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/video.mp4");
            then.status(200).body("video-data");
        });
        let dl = VideoDownloader::new();
        let mut progress_called = 0;
        let _ = dl
            .download_to_tempfile_with_progress(
                &format!("{}/video.mp4", server.url("")),
                ".mp4",
                Some(|_d, _t| progress_called += 1),
            )
            .await
            .unwrap();
        assert!(progress_called > 0);
        mock.assert();
    }
}
