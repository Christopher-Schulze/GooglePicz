# TODO and Next Steps for GooglePicz

## High Priority Tasks
- **Verify Build:** Run `cargo check --all` and `cargo build` to ensure the project compiles successfully after the configuration and path fixes.

## Medium Priority Tasks
- **Optimize Caching:** The cache now uses a normalized schema with separate `media_items`, `media_metadata`, `albums`, and `album_media_items` tables for efficient SQL queries.
- **Background Sync Robustness:** The background sync in `main.rs` is started in a `tokio::spawn` task. Add more robust error handling and potentially a mechanism to communicate sync status back to the UI (e.g., via channels).
- **Add CLI Sync Command:** Provide a `--sync` flag to manually trigger the background synchronization process.
- **CLI-Erweiterungen:** Zusätzliche Befehle und Optionen für das Kommandozeilenwerkzeug implementieren.
- **Performance Tuning:** Profile startup time and memory usage to better support large photo libraries.
- **Album umbenennen/löschen:** Option zum Umbenennen und Entfernen vorhandener Alben.

## Low Priority Tasks
- **Code Refinements:**
  - In `ui/src/lib.rs`, the creation of the `cache_manager` can be simplified.
  - Review all `.unwrap()` and `.expect()` calls to ensure they are appropriate and won't cause panics in edge cases.
