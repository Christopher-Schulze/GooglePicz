use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::NamedTempFile;
use tokio::runtime::Runtime;
use sync::Syncer;

fn bench_app_start(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("app_startup", |b| {
        b.to_async(&rt).iter(|| async {
            std::env::set_var("MOCK_KEYRING", "1");
            std::env::set_var("MOCK_ACCESS_TOKEN", "token");
            std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
            std::env::set_var("MOCK_API_CLIENT", "1");
            std::env::set_var("GOOGLE_CLIENT_ID", "id");
            std::env::set_var("GOOGLE_CLIENT_SECRET", "secret");
            let tmp = NamedTempFile::new().unwrap();
            let _ = Syncer::new(tmp.path()).await.unwrap();
        })
    });
}

fn bench_sync(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    c.bench_function("full_sync", |b| {
        b.to_async(&rt).iter(|| async {
            std::env::set_var("MOCK_KEYRING", "1");
            std::env::set_var("MOCK_ACCESS_TOKEN", "token");
            std::env::set_var("MOCK_REFRESH_TOKEN", "refresh");
            std::env::set_var("MOCK_API_CLIENT", "1");
            std::env::set_var("GOOGLE_CLIENT_ID", "id");
            std::env::set_var("GOOGLE_CLIENT_SECRET", "secret");
            let tmp = NamedTempFile::new().unwrap();
            let mut syncer = Syncer::new(tmp.path()).await.unwrap();
            syncer.sync_media_items(None, None, None, None).await.unwrap();
        })
    });
}

criterion_group!(benches, bench_app_start, bench_sync);
criterion_main!(benches);
