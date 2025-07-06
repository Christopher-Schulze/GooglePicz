use criterion::{criterion_group, criterion_main, Criterion};
use cache::CacheManager;
use api_client::{MediaItem, MediaMetadata, VideoMetadata};
use tempfile::NamedTempFile;
use chrono::{Utc, TimeZone};

fn item_with_meta(id: &str, model: &str, ts: &str) -> MediaItem {
    MediaItem {
        id: id.to_string(),
        description: Some("sample".into()),
        product_url: "http://example.com".into(),
        base_url: "http://example.com/base".into(),
        mime_type: "image/jpeg".into(),
        media_metadata: MediaMetadata {
            creation_time: ts.into(),
            width: "1".into(),
            height: "1".into(),
            video: Some(VideoMetadata {
                camera_make: Some("Canon".into()),
                camera_model: Some(model.into()),
                fps: None,
                status: None,
            }),
        },
        filename: format!("{}.jpg", id),
    }
}

fn bench_real_query(c: &mut Criterion) {
    let tmp = NamedTempFile::new().unwrap();
    let cache = CacheManager::new(tmp.path()).unwrap();

    for i in 0..10_000u32 {
        let (model, ts) = if i % 2 == 0 {
            ("EOS", "2023-01-02T00:00:00Z")
        } else {
            ("D5", "2023-02-01T00:00:00Z")
        };
        let item = item_with_meta(&i.to_string(), model, ts);
        cache.insert_media_item(&item).unwrap();
    }

    let start = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2023, 1, 31, 23, 59, 59).unwrap();
    c.bench_function("real_query", |b| {
        b.iter(|| {
            let _ = cache
                .query_media_items(Some("EOS"), Some(start), Some(end), None, Some("sample"))
                .unwrap();
        })
    });
}

criterion_group!(benches, bench_real_query);
criterion_main!(benches);
