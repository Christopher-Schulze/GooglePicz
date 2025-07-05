# Changelog

## 2025-06-29 (Fortsetzung)
- **Workspace Restrukturierung:**
  - Erstellung einer dedizierten `app`-Crate für die Hauptanwendung
  - Verschiebung der Build-Konfiguration in die jeweiligen Crates
  - Korrektur der Abhängigkeiten zwischen den Crates
  - Entfernung ungültiger Konfigurationen aus der Root-Cargo.toml

## 2025-06-29
- **Project Audit & Repair:**
  - Conducted a full codebase review.
  - Repaired multiple corrupted `Cargo.toml` files (`auth`, `api_client`, `sync`, `packaging`) by deleting and recreating them.
  - Corrected invalid main `Cargo.toml` structure, removing duplicate `[package]` sections and organizing it into a valid workspace definition.
  - Standardized all crate package names and dependencies across the workspace, removing the `googlepicz_` prefix.
  - Fixed invalid `use` statements in `main.rs`, `api_client/src/lib.rs`, `sync/src/lib.rs`, and `ui/src/lib.rs` to reflect the corrected crate names.
  - Aligned `KEYRING_SERVICE_NAME` in `auth/src/lib.rs` with the project name for consistency.
  - Refactored `ui/src/image_loader.rs` to remove the in-memory cache and mutable state from `load_thumbnail`, resolving potential concurrency issues.
  - Created `docs/TODO123.md` to outline next steps and potential improvements.
## 2024-07-29
- Initial project setup: Created Rust workspace and module folders (`auth`, `api_client`, `ui`, `cache`, `sync`, `packaging`).
- Configured main `Cargo.toml` for workspace and added global dependencies and release profiles.
- Configured `Cargo.toml` for `ui` module with `iced` and `wgpu` dependencies.
- Configured `Cargo.toml` for `auth` module with `oauth2`, `keyring`, and `tokio` dependencies.
- Configured `Cargo.toml` for `api_client` module with `google-photos1` and `tokio` dependencies.
- Configured `Cargo.toml` for `cache` module with `rusqlite` and `tokio` dependencies.
- Configured `Cargo.toml` for `sync` module with `tokio` and local module dependencies (`auth`, `api_client`, `cache`).
- Configured `Cargo.toml` for `packaging` module with `tokio` and `cargo-bundle-licenses` dependencies.
- Created initial `Changelog.md` and `DOCUMENTATION.md` files in `docs/` folder.
- Implemented `auth` module: OAuth2 authentication flow with secure token storage.
- Implemented `api_client` module: Google Photos API interaction for media items.
- Implemented `cache` module: SQLite-based caching for media items.
- Implemented `sync` module: Synchronization logic between API and local cache.
- Implemented `ui` module: Basic UI structure using `iced`.
- Implemented `packaging` module: Placeholder functions for bundling licenses and building release binaries.
- Created `src/main.rs` as the main application entry point.
- Updated main `Cargo.toml` to define `src/main.rs` as a binary target.
- Updated `DOCUMENTATION.md` to reflect implemented modules and `main.rs` creation.

## 2024-07-30
- Added `dirs` dependency and local dependencies (`googlepicz_auth`, `googlepicz_sync`, `googlepicz_ui`) to the main `Cargo.toml`.
- Created new file `/Users/christopher/CODE/GooglePicz/ui/src/image_loader.rs` defining an `ImageLoader` struct for downloading and caching image thumbnails.
- Updated `/Users/christopher/CODE/GooglePicz/ui/src/lib.rs` to integrate the new `image_loader` module (`mod image_loader;` and `use image_loader::ImageLoader;`).
- Extended `Message` enumeration and `GooglePiczUI` struct in `/Users/christopher/CODE/GooglePicz/ui/src/lib.rs` to support thumbnail loading (`ThumbnailLoaded`, `LoadThumbnail` variants, `ImageLoader` and `thumbnails` HashMap).
- Integrated `ImageLoader` initialization and thumbnail cache into `ui/src/lib.rs`.
- Extended `update` method in `ui/src/lib.rs` to handle `ThumbnailLoaded` and `LoadThumbnail` messages, store loaded thumbnails, and initiate thumbnail loading after photos are loaded.
- Adjusted `view` method in `ui/src/lib.rs` to display loaded thumbnails or placeholders and trigger thumbnail loading.
- Corrected a duplicate `eprintln!` call in `Message::PhotosLoaded` handler in `ui/src/lib.rs`.
- Removed a redundant closing brace (`}}`) in the `update` method in `ui/src/lib.rs` to fix a syntax error.
- Created `targetpicture.md` in `docs/` detailing the final vision and architecture of the project.

## 2025-07-05
- Fixed compilation errors in `sync` crate by returning `Ok(())` from async blocks.
- Handled unused `Command` warning in `ui` crate.
- Workspace builds successfully with `cargo check --all` and `cargo build`.
