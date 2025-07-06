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

The numbers show that loading the entire cache scales linearly while common
queries remain below a few milliseconds. Keeping the item count modest helps
startup time and full synchronizations finish quickly.
