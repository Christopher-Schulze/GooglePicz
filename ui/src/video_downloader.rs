use futures_util::StreamExt;
use reqwest;
use std::path::Path;
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
}
