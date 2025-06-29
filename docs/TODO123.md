# TODO and Next Steps for GooglePicz

## High Priority Tasks
- **Verify Build:** Run `cargo check --all` and `cargo build` to ensure the project compiles successfully after the configuration and path fixes.
- **Implement UI Logic:** The UI currently displays photo metadata but needs implementation for actual user interactions (e.g., selecting a photo, viewing full-resolution images, managing albums).
- **Refine Error Handling:** While error enums exist, the UI should display user-friendly error messages instead of printing to `stderr`. Implement a global error state or notification system in the UI.

## Medium Priority Tasks
- **Optimize Caching:** The `cache` module stores the entire `MediaItem` as a JSON blob. This is inefficient for querying. The schema should be normalized to store individual metadata fields in their own columns to allow for faster lookups and filtering directly in SQL.
- **Implement Full Packaging:** The `packaging` module is currently a placeholder. Implement proper bundling for macOS (`.app`) and Windows (`.msi`/`.exe`) using `cargo-bundle` or other platform-specific tools.
- **Background Sync Robustness:** The background sync in `main.rs` is started in a `tokio::spawn` task. Add more robust error handling and potentially a mechanism to communicate sync status back to the UI (e.g., via channels).

## Low Priority Tasks
- **Configuration Management:** Move hardcoded values (like the number of thumbnails to preload) into a configuration file or UI setting.
- **Improve Test Coverage:** Enable and expand the ignored tests. Mock the Google Photos API and `keyring` service to allow for fully automated testing without requiring manual intervention or real credentials.
- **Code Refinements:**
  - In `ui/src/lib.rs`, the creation of the `cache_manager` can be simplified.
  - Review all `.unwrap()` and `.expect()` calls to ensure they are appropriate and won't cause panics in edge cases.
