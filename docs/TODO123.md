# TODO and Next Steps for GooglePicz

## High Priority Tasks
- **Verify Build:** Run `cargo check --all` and `cargo build` to ensure the project compiles successfully after the configuration and path fixes.
- **Implement UI Logic:** The UI currently displays photo metadata but needs implementation for actual user interactions (e.g., selecting a photo, viewing full-resolution images, managing albums).
- **Add Album Management:** Provide UI for creating, renaming, and deleting albums within the application.

## Medium Priority Tasks
- **Optimize Caching:** The cache now uses a normalized schema with separate `media_items`, `media_metadata`, `albums`, and `album_media_items` tables for efficient SQL queries.
- **Background Sync Robustness:** The background sync in `main.rs` is started in a `tokio::spawn` task. Add more robust error handling and potentially a mechanism to communicate sync status back to the UI (e.g., via channels).
- **Add CLI Sync Command:** Provide a `--sync` flag to manually trigger the background synchronization process.
- **Performance Tuning:** Profile startup time and memory usage to better support large photo libraries.

## Low Priority Tasks
- **Code Refinements:**
  - In `ui/src/lib.rs`, the creation of the `cache_manager` can be simplified.
  - Review all `.unwrap()` and `.expect()` calls to ensure they are appropriate and won't cause panics in edge cases.
