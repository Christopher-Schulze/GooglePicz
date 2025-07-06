# GooglePicz - Target Picture

## Vision
GooglePicz aims to be a native Google Photos application built with Rust, designed for maximum GPU utilization and a minimal footprint. The goal is to deliver a fully functional application within two weeks of development time.

## Core Functionality
- **Authentication**: Secure OAuth2 flow with browser redirect and token persistence in the system keychain.
- **API Interaction**: Seamless interaction with the Google Photos API for media item retrieval and management.
- **UI/UX**: A reactive user interface with:
    - Lazy-loading of thumbnails for efficient display.
    - GPU-accelerated image rendering for smooth performance.
    - Display of photo details (filename, dimensions, creation time).
    - Display of thumbnails (150x150 pixels) with caching mechanisms.
- **Caching**: Robust SQLite-based schema for albums and media items, supporting incremental updates to ensure data consistency and offline access.
- **Synchronization**: Background synchronization tasks (e.g., every 5 minutes) to keep local data up-to-date with Google Photos, along with pull-to-refresh feature.
- **Packaging**: Automated signing and notarization for macOS (.app) and Windows (.msi/.exe) installers to ensure easy distribution and installation.
- **Advanced Search**: Filters for filename, description, favorites, date range, MIME type and camera metadata.
- **Video Playback** *(optional)*: Uses the GStreamer backend when built with the `gstreamer` feature.
- **Face Recognition** *(optional)*: Detects faces and overlays bounding boxes. Results can be cached when the `face_recognition/cache` feature is enabled.

## Technical Architecture
The project is structured as a Rust workspace with the following modules (crates):
- **auth**: Handles OAuth2 flow and secure token storage.
- **api_client**: Provides a generated Google Photos client for asynchronous requests.
- **ui**: Manages reactive UI components, including the `image_loader` module for efficient thumbnail handling, lazy-loading, and GPU-accelerated image rendering.
- **cache**: Implements a SQLite schema for albums and media items.
- **sync**: Manages background synchronization tasks and pull-to-refresh feature.
- **packaging**: Handles automated signing and notarization for installers.

## Key Technologies
- **GUI Framework**: Iced (wgpu-backend) or Druid (statically linked for <10 MB binaries).
- **Asynchronous Runtime**: Tokio.
- **HTTP/OAuth2**: `oauth2` and `google-photos1` crates.
- **Caching**: `rusqlite` for thumbnails and metadata.
- **CLI/Installer**: `cargo-bundle` for macOS .app and Windows .msi/.exe.

## Performance Optimizations
- Zero-copy downloads directly to GPU upload (wgpu Texture).
- Thumbnail-first strategy with full-resolution on-demand loading.
- Release builds with `strip` and minimal features.

## Development Workflow
- Scaffold scripts for boilerplate code generation.
- CI job (GitHub Actions) for cross-compilation and packaging.
- Hot-reloading for UI prototyping.
## Delivery & Quality Assurance
- Sample screenshots generated in CI and stored in `docs/screenshots`.
- End-to-end smoke tests: Authentication → Album list → Thumbnail display.
- Generation of ready-to-distribute installer artifacts for both platforms.

## Configuration Highlights
The application reads `AppConfig` from `~/.googlepicz/config`. Important options
include `cache_path` for the data directory, `debug_console` to enable Tokio's
console subscriber, `trace_spans` for detailed profiling, `preload_threads` to
control thumbnail workers and `detect_faces` to run face detection. OAuth tokens can be
stored in the file system by compiling with the `auth/file-store` feature and
starting the tools with `--use-file-store` or `USE_FILE_STORE=1`.
