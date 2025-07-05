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

