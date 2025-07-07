# Performance-Tuning

This chapter summarizes benchmark results for the cache layer.

The benchmarks can be executed with

```bash
cargo bench -p cache --bench cache_bench
```

Results on a Linux workstation:

| Benchmark | Items | Time per run |
|-----------|------:|-------------:|
| `load_all_1000` | 1,000 | ~1.1 ms |
| `load_all_10k` | 10,000 | ~13 ms |
| `camera_model_query` | 10,000 | ~8.5 ms |
| `load_all_100k` | 100,000 | ~140 ms |
| `mime_type_query` | 10,000 | ~4 ms |
| `album_query` | 10,000 | ~7 ms |
| `favorite_query` | 10,000 | ~5 ms |
| `app_startup` | n/a | ~15 ms |
| `full_sync` | n/a | ~30 ms |
| `thumbnail_load_50` | 50 | ~350 ms |
| `thumbnail_load_500` | 500 | ~3.2 s |
| `thumbnail_load_5000` | 5,000 | ~32 s |

The numbers show that loading the entire cache scales linearly while common
queries remain below a few milliseconds. Loading thumbnails also scales with the
requested count and reaches roughly 32&nbsp;s when fetching 5,000 previews.
Keeping the item count modest helps startup time and full synchronizations
finish quickly.

### Thumbnail preloading

With parallel thumbnail loading using a semaphore the `preload_thumbnails`
routine improved noticeably. Loading 5,000 thumbnails now takes roughly
**25&nbsp;s** instead of **32&nbsp;s** on the same hardware (traced with the
`preload_time_ms` span).

### UI startup metrics

With `tokio-console` active and the `trace-spans` feature enabled, the GUI
initializes in roughly **120&nbsp;ms**. Memory usage grows from about **40&nbsp;MB**
before initialization to **65&nbsp;MB** once the window is visible.

### Application startup metrics

Profiling the command-line initialization with `tokio-console` shows the
background services and UI launch complete in about **100&nbsp;ms**. Memory usage
increases from roughly **30&nbsp;MB** before initialization to **50&nbsp;MB**
once all tasks are running.
