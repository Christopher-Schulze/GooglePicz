# TODO and Next Steps for GooglePicz

## High Priority Tasks
- ~~**Verify Build:** Run `cargo check --all` and `cargo build` to ensure the project compiles successfully after the configuration and path fixes.~~
- ~~**Address Compiler Warnings:** Review and fix all current compiler warnings.~~

## Medium Priority Tasks
- ~~**Optimize Caching:**~~
  - ~~The cache now uses a normalized schema with separate `media_items`, `media_metadata`, `albums`, and `album_media_items` tables for efficient SQL queries.~~
  - ~~Added index on `is_favorite` and benchmarked loading 1000 items (~1ms).~~
- ~~**Background Sync Robustness:** The background sync in `main.rs` is started in a `tokio::spawn` task. Add more robust error handling and potentially a mechanism to communicate sync status back to the UI (e.g., via channels).~~ (commit b59bdae)
- ~~**CLI-Erweiterungen:** Zusätzliche Befehle und Optionen für das Kommandozeilenwerkzeug implementieren.~~
- ~~**Performance Tuning:** Profile startup time and memory usage to better support large photo libraries.~~ (commit 0c18b1f)
- ~~**Add CLI Integration Tests:** Ensure the command-line interface works correctly through automated tests.~~

## Low Priority Tasks
- **Code Refinements:**
  - ~~In `ui/src/lib.rs`, the creation of the `cache_manager` can be simplified.~~ (commit 21f6050)
  - ~~Review all `.unwrap()` and `.expect()` calls to ensure they are appropriate and won't cause panics in edge cases.~~
  - ~~Completed: replaced `.expect()` calls in `ui/src/image_loader.rs` with graceful error handling.~~
