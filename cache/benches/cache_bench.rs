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

fn bench_load_all_200k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..200_000u32 {
        let item = sample_media_item(&i.to_string());
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("load_all_200k", |b| {
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

fn bench_favorite_query(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let item = sample_media_item(&i.to_string());
        cache.insert_media_item(&item).unwrap();
        if i % 2 == 0 {
            cache.set_favorite(&item.id, true).unwrap();
        }
    }
    c.bench_function("favorite_query", |b| {
        b.iter(|| {
            let _ = cache.get_favorite_media_items().unwrap();
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

fn bench_camera_make_query(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let make = if i % 2 == 0 { "Canon" } else { "Nikon" };
        let mut item = sample_media_item(&i.to_string());
        item.media_metadata.video = Some(VideoMetadata {
            camera_make: Some(make.into()),
            camera_model: None,
            fps: None,
            status: None,
        });
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("camera_make_query", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_camera_make("Canon").unwrap();
        })
    });
}

fn bench_filename_query(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let item = sample_media_item(&format!("file{}", i));
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("filename_query", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_filename("file1").unwrap();
        })
    });
}

fn bench_text_query_get_1k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..1_000u32 {
        let mut item = sample_media_item(&i.to_string());
        if i % 2 == 0 {
            item.description = Some("foo".into());
        }
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("get_text_1k", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_text("foo").unwrap();
        })
    });
}

fn bench_text_query_get_10k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let mut item = sample_media_item(&i.to_string());
        if i % 2 == 0 {
            item.description = Some("foo".into());
        }
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("get_text_10k", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_text("foo").unwrap();
        })
    });
}

fn bench_text_query_get_100k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..100_000u32 {
        let mut item = sample_media_item(&i.to_string());
        if i % 2 == 0 {
            item.description = Some("foo".into());
        }
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("get_text_100k", |b| {
        b.iter(|| {
            let _ = cache.get_media_items_by_text("foo").unwrap();
        })
    });
}

fn bench_text_query_general_1k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..1_000u32 {
        let mut item = sample_media_item(&i.to_string());
        if i % 2 == 0 {
            item.description = Some("foo".into());
        }
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("query_text_1k", |b| {
        b.iter(|| {
            let _ = cache
                .query_media_items(
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some("foo"),
                )
                .unwrap();
        })
    });
}

fn bench_text_query_general_10k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..10_000u32 {
        let mut item = sample_media_item(&i.to_string());
        if i % 2 == 0 {
            item.description = Some("foo".into());
        }
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("query_text_10k", |b| {
        b.iter(|| {
            let _ = cache
                .query_media_items(
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some("foo"),
                )
                .unwrap();
        })
    });
}

fn bench_text_query_general_100k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..100_000u32 {
        let mut item = sample_media_item(&i.to_string());
        if i % 2 == 0 {
            item.description = Some("foo".into());
        }
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("query_text_100k", |b| {
        b.iter(|| {
            let _ = cache
                .query_media_items(
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some("foo"),
                )
                .unwrap();
        })
    });
}

fn bench_text_query_general_200k(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();
    for i in 0..200_000u32 {
        let mut item = sample_media_item(&i.to_string());
        if i % 2 == 0 {
            item.description = Some("foo".into());
        }
        cache.insert_media_item(&item).unwrap();
    }
    c.bench_function("query_text_200k", |b| {
        b.iter(|| {
            let _ = cache
                .query_media_items(
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some("foo"),
                )
                .unwrap();
        })
    });
}

fn bench_sequential_insert_10k(c: &mut Criterion) {
    let items: Vec<_> = (0..10_000u32)
        .map(|i| sample_media_item(&i.to_string()))
        .collect();
    c.bench_function("sequential_insert_10k", |b| {
        b.iter(|| {
            let tmp = NamedTempFile::new().unwrap();
            let cache = CacheManager::new(tmp.path()).unwrap();
            for item in &items {
                cache.insert_media_item(item).unwrap();
            }
        })
    });
}

fn bench_batch_insert_10k(c: &mut Criterion) {
    let items: Vec<_> = (0..10_000u32)
        .map(|i| sample_media_item(&i.to_string()))
        .collect();
    c.bench_function("batch_insert_10k", |b| {
        b.iter(|| {
            let tmp = NamedTempFile::new().unwrap();
            let cache = CacheManager::new(tmp.path()).unwrap();
            cache.insert_media_items_batch(&items).unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_load_all,
    bench_load_all_10k,
    bench_load_all_100k,
    bench_load_all_200k,
    bench_camera_model_query,
    bench_camera_make_query,
    bench_filename_query,
    bench_text_query_get_1k,
    bench_text_query_get_10k,
    bench_text_query_get_100k,
    bench_text_query_general_1k,
    bench_text_query_general_10k,
    bench_text_query_general_100k,
    bench_text_query_general_200k,
    bench_mime_type_query,
    bench_album_query,
    bench_favorite_query,
    bench_sequential_insert_10k,
    bench_batch_insert_10k
);
criterion_main!(benches);

