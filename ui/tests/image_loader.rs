use ui::ImageLoader;
use httpmock::prelude::*;
use tempfile::tempdir;

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
    assert!(dir.path().join("thumbnails/1.jpg").exists());
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
    assert!(dir.path().join("full/1.jpg").exists());
    mock.assert_hits(1);

    loader.load_full_image("1", &url).await.unwrap();
    mock.assert_hits(1);
}
