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
| `app_startup` | n/a | ~15 ms |
| `full_sync` | n/a | ~30 ms |
| `thumbnail_load_50` | 50 | ~350 ms |
| `thumbnail_load_500` | 500 | ~3.2 s |
| `thumbnail_load_5000` | 5,000 | ~32 s |
| `query_text_fts_10k` | 10,000 | ~2 ms |
| `ui_startup` | n/a | ~200 ms |
| `ui_memory` | n/a | ~110 MB |

The numbers show that loading the entire cache scales linearly while common
queries remain below a few milliseconds. Loading thumbnails also scales with the
requested count and reaches roughly 32&nbsp;s when fetching 5,000 previews.
Keeping the item count modest helps startup time and full synchronizations
finish quickly.

The new `query_text_fts_10k` benchmark demonstrates the effect of using the
full text search table. It completes in about 2&nbsp;ms compared to roughly
7&nbsp;ms with the generic LIKE query. Instrumentation of the GUI startup via
`tokio-console` shows an initialization time around 200&nbsp;ms with a memory
footprint of about 110&nbsp;MB.
