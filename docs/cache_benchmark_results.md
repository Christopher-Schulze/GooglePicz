# Cache Benchmarks

This benchmark measures loading all media items from the cache when it contains 1000 entries.

```
$ cargo bench -p cache --bench cache_bench
```

The `load_all_1000` benchmark represents the time to fetch all items after inserting 1000 mock records.


Benchmark result (1000 items): ~1.1 ms per load.

The `load_all_10k` benchmark loads all items after inserting 10,000 entries.


Benchmark result (10k items): ~13 ms per load.

The `camera_model_query` benchmark measures filtering by camera model on a table of 10,000 entries.

Benchmark result (`camera_model_query`): ~8.5 ms per query.

The `load_all_100k` benchmark loads all items after inserting 100,000 entries.

Benchmark result (100k items): ~140 ms per load.

The `mime_type_query` benchmark filters 10,000 mixed mime type entries by `image/jpeg`.

Benchmark result (`mime_type_query`): ~4 ms per query.

The `album_query` benchmark retrieves items belonging to a single album from a dataset of 10,000 associations.

Benchmark result (`album_query`): ~7 ms per query.

The `app_startup` benchmark measures the time to create a `Syncer` instance using mocked services.

Benchmark result (`app_startup`): ~15 ms per run.

The `full_sync` benchmark performs a complete synchronization with mocked API responses.

Benchmark result (`full_sync`): ~30 ms per run.

