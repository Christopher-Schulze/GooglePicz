use criterion::{criterion_group, criterion_main, Criterion};
use cache::CacheManager;
use api_client::{MediaItem, MediaMetadata};
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

criterion_group!(benches, bench_load_all);
criterion_main!(benches);

