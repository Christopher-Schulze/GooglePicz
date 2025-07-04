use criterion::{criterion_group, criterion_main, Criterion};
use cache::CacheManager;
use api_client::{MediaItem, MediaMetadata, VideoMetadata, Album};
use tempfile::NamedTempFile;

fn sample_media_item(id: &str) -> MediaItem {
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

fn sample_media_item_with_mime(id: &str, mime: &str) -> MediaItem {
    MediaItem {
        id: id.to_string(),
        description: None,
        product_url: "http://example.com".into(),
        base_url: "http://example.com/base".into(),
        mime_type: mime.into(),
        media_metadata: MediaMetadata {
            creation_time: "2023-01-01T00:00:00Z".into(),
            width: "1".into(),
            height: "1".into(),
            video: None,
        },
        filename: format!("{}.jpg", id),
    }
}

fn sample_album(id: &str) -> Album {
    Album {
        id: id.to_string(),
        title: Some("Album".into()),
        product_url: None,
        is_writeable: None,
        media_items_count: None,
        cover_photo_base_url: None,
        cover_photo_media_item_id: None,
    }
}

fn sample_media_item_with_camera(id: &str, model: &str) -> MediaItem {
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
            video: Some(VideoMetadata {
                camera_make: None,
                camera_model: Some(model.into()),
                fps: None,
                status: None,
            }),
        },
        filename: format!("{}.jpg", id),
    }
}

fn bench_load_all(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..1000u32 {
        let item = sample_media_item(&i.to_string());
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("load_all_1000", |b| {
        b.iter(|| {
            let _ = cache.get_all_media_items().unwrap();
        })
    });
}

fn bench_load_all_10k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let item = sample_media_item(&i.to_string());
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("load_all_10k", |b| {
        b.iter(|| {
            let _ = cache.get_all_media_items().unwrap();
        })
    });
}

fn bench_load_all_100k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..100_000u32 {
        let item = sample_media_item(&i.to_string());
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("load_all_100k", |b| {
        b.iter(|| {
            let _ = cache.get_all_media_items().unwrap();
        })
    });
}

fn bench_mime_type_query(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let mime = if i % 2 == 0 { "image/jpeg" } else { "video/mp4" };
        let item = sample_media_item_with_mime(&i.to_string(), mime);
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("mime_type_query", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_mime_type("image/jpeg").unwrap();
        })
    });
}

fn bench_album_query(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for a in 0..10u32 {
        let album = sample_album(&format!("album{}", a));
        cache.insert_album(&album).unwrap();
    }
    for i in 0..10_000u32 {
        let album_id = format!("album{}", i % 10);
        let item = sample_media_item(&i.to_string());
        cache.insert_media_item(&item).unwrap();
        cache.associate_media_item_with_album(&item.id, &album_id).unwrap();
    }
    c.bench_function("album_query", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_album("album0").unwrap();
        })
    });
}

fn bench_camera_model_query(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let model = if i % 2 == 0 { "Canon" } else { "Nikon" };
        let item = sample_media_item_with_camera(&i.to_string(), model);
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("camera_model_query", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_camera_model("Canon").unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_load_all,
    bench_load_all_10k,
    bench_load_all_100k,
    bench_camera_model_query,
    bench_mime_type_query,
    bench_album_query
);
criterion_main!(benches);

