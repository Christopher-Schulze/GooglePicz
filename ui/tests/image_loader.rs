use ui::{ImageLoader, ImageLoaderError};
use httpmock::prelude::*;
use tempfile::tempdir;
use std::time::Duration;

#[tokio::test]
async fn test_thumbnail_cached() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/img.jpg=w150-h150-c");
        then.status(200).body("thumb");
    });

    let dir = tempdir().unwrap();
    let loader = ImageLoader::new(dir.path().to_path_buf());
    let url = format!("{}/img.jpg", server.url(""));

    loader.load_thumbnail("1", &url).await.unwrap();
    let thumb_path = dir.path().join("thumbnails").join("1.jpg");
    assert!(thumb_path.exists());
    mock.assert_hits(1);

    // Second call should use cache
    loader.load_thumbnail("1", &url).await.unwrap();
    mock.assert_hits(1);
}

#[tokio::test]
async fn test_full_image_cached() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/img.jpg=d");
        then.status(200).body("full");
    });

    let dir = tempdir().unwrap();
    let loader = ImageLoader::new(dir.path().to_path_buf());
    let url = format!("{}/img.jpg", server.url(""));

    loader.load_full_image("1", &url).await.unwrap();
    let full_path = dir.path().join("full").join("1.jpg");
    assert!(full_path.exists());
    mock.assert_hits(1);

    loader.load_full_image("1", &url).await.unwrap();
    mock.assert_hits(1);
}

#[tokio::test]
async fn test_thumbnail_not_found() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/missing.jpg=w150-h150-c");
        then.status(404);
    });

    let dir = tempdir().unwrap();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let loader = ImageLoader::with_client(dir.path().to_path_buf(), client);
    let url = format!("{}/missing.jpg", server.url(""));

    let err = loader.load_thumbnail("1", &url).await.err().unwrap();
    assert_eq!(err, ImageLoaderError::NotFound);
}

#[tokio::test]
async fn test_thumbnail_timeout() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/slow.jpg=w150-h150-c");
        then.status(200).body("img").delay(Duration::from_millis(200));
    });

    let dir = tempdir().unwrap();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let loader = ImageLoader::with_client(dir.path().to_path_buf(), client);
    let url = format!("{}/slow.jpg", server.url(""));

    let err = loader.load_thumbnail("1", &url).await.err().unwrap();
    assert_eq!(err, ImageLoaderError::Timeout);
}

#[tokio::test]
async fn test_network_error() {
    let dir = tempdir().unwrap();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    let loader = ImageLoader::with_client(dir.path().to_path_buf(), client);

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let url = format!("http://{}/img.jpg", addr);
    let err = loader.load_thumbnail("1", &url).await.err().unwrap();
    match err {
        ImageLoaderError::Network(_) => (),
        other => panic!("expected network error, got {:?}", other),
    }
}
