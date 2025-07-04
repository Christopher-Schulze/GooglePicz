# GooglePicz Documentation

## 📋 Overview
GooglePicz is a native Google Photos client being developed in Rust. The application focuses on performance, security, and user experience. The project is structured as a Rust workspace with multiple crates.

## 🚧 Project Status: Early Development

**Note**: GooglePicz is an **experimental** project. The information in this documentation reflects the current state and is subject to change as development progresses. Planned features include video playback, advanced search and face recognition.

## 🏗️ Architecture

### Main Application
- **app**: Central entry point that coordinates all modules.

### Modules
- **auth**: Implements OAuth2 flow with secure token management
- **api_client**: Provides interface to Google Photos API
- **ui**: Handles the user interface (Iced Framework)
- **cache**: Manages local media cache (SQLite)
- **sync**: Handles synchronization with Google Photos
- **packaging**: Handles application packaging
- **face_recognition**: Planned module for detecting faces in media items

### Face Recognition Module (planned)
The optional `face_recognition` crate defines placeholder functions to detect
faces in a `MediaItem`. When built with the `cache` feature the results can be
stored in the local cache. Enabling the `ui` feature will later allow the user
interface to display tagged faces once real detection logic is implemented.

### Crate Interactions

```
            +-------+            
            |  app  |
            +---+---+
                |
                v
            +---+---+
            |   ui  |
            +---+---+
                |
                v
+-----------+     +-------------+
|   sync    |<--->| api_client  |
+-----------+     +-------------+
      |
      v
   +-----+
   |cache|
   +-----+
      ^
      |
   +-----+
   | auth|
   +-----+
```

The `app` crate launches the UI and coordinates other modules. During startup, the UI triggers the OAuth flow in the `auth` crate to obtain an access token. The `sync` crate uses this token through `api_client` to fetch photos and album data, storing the results via the `cache` crate. The UI then queries the cache to render thumbnails and albums, while sync continues to update the cache in the background.

## 🛠️ Technologies

### Core Technologies
- **Language**: Rust 1.70+
- **UI Framework**: Iced (wgpu backend)
- **Async Runtime**: Tokio
- **Database**: SQLite3
- **HTTP Client**: reqwest
- **OAuth2**: oauth2, google-photos1
- **Image Processing**: image-rs

### Dependencies
```toml
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
oauth2 = "4.4"
google-photos1 = "0.1"
rusqlite = "0.29"
dirs = "5.0"
```

## 🚀 Current Implementation Status

### Core Components
- [x] Basic project structure
- [x] Rust workspace setup
- [x] Module separation
- [ ] Complete API integration
- [ ] Full UI implementation

### Authentication
- [x] OAuth2 flow structure
- [ ] Token refresh handling
- [ ] Secure credential storage

### UI Components
- [x] Basic window setup
- [ ] Photo grid view
- [ ] Album management
- [ ] Settings panel

## 🧪 Testing Strategy (Planned)

### Unit Testing
- [ ] Core functionality tests
- [ ] API client tests
- [ ] Cache layer tests

### Integration Testing
- [ ] Authentication flow
- [ ] Photo synchronization
- [ ] UI interactions

### E2E Tests
End-to-end tests live under `tests/e2e` and exercise higher level workflows.
Mocks for the API client and keyring (`MOCK_API_CLIENT`, `MOCK_KEYRING`) avoid
network access. The scenarios cover starting a sync, creating albums and
querying the cache via search helpers.

## 📦 Build & Development

### Prerequisites
- Rust 1.70 or later
- Cargo
- SQLite development files

### Building
```bash
# Build in debug mode
cargo build

# Build for release
cargo build --release
```

### Development Workflow
```bash
# Format code
cargo fmt

# Run linter
cargo clippy --all -- -D warnings

# Run tests
cargo test
```
Run `cargo fmt` and `cargo clippy --all -- -D warnings` locally before committing
your changes to ensure consistent formatting and catch linter warnings.

## 🌎 Configuration

Details about required environment variables and optional `AppConfig` settings have been consolidated in [docs/CONFIGURATION.md](CONFIGURATION.md). Refer to that document for a full list of keys and examples for setting up OAuth credentials.

### Packaging installers
Follow these steps to produce release artifacts:

1. Install the tools listed in the [required tools table](RELEASE_ARTIFACTS.md#required-tools).
2. Export any signing variables you need (`MAC_SIGN_ID`, `APPLE_ID`, etc.).
3. Run the packager from the workspace root:

   ```bash
   cargo run --package packaging --bin packager
   ```

4. Grab the generated files from `target/`:
   - `GooglePicz-<version>-Setup.exe` on Windows
   - `GooglePicz.dmg` on macOS
   - `GooglePicz-<version>.deb` on Linux

The version is read from `Cargo.toml` so artifact names are consistent across platforms.

## Sync CLI

In addition to the main application UI, the project provides a command line utility for manual synchronization and cache inspection. The binary lives under `app/src/bin/sync_cli.rs` and is built alongside the rest of the workspace. It uses `AppConfig` on startup, so options defined in `~/.googlepicz/config` apply here as well.

```bash
cargo run --package googlepicz --bin sync_cli -- sync
```

Synchronizes all media items and prints progress to stdout.

```bash
cargo run --package googlepicz --bin sync_cli -- status
```

Displays the last sync timestamp and the number of cached photos.

```bash
cargo run --package googlepicz --bin sync_cli -- list-albums
```

Lists all albums stored in the local cache.

```bash
cargo run --package googlepicz --bin sync_cli -- clear-cache
```

Clears all cached media items.

```bash
cargo run --package googlepicz --bin sync_cli -- create-album "My Album"
```

Creates a new album and stores it in the cache.

```bash
cargo run --package googlepicz --bin sync_cli -- delete-album ALBUM_ID
```

Deletes the album from Google Photos and the local cache.

```bash
cargo run --package googlepicz --bin sync_cli -- rename-album ALBUM_ID "New Title"
```

Renames an album on Google Photos and updates the local cache.

```bash
cargo run --package googlepicz --bin sync_cli -- add-to-album ALBUM_ID ITEM_ID
```

Associates a cached media item with an album.

```bash
cargo run --package googlepicz --bin sync_cli -- list-album-items ALBUM_ID
```

Lists items stored for the given album.

```bash
cargo run --package googlepicz --bin sync_cli -- search QUERY
```

Searches cached media items by filename or description.

```bash
cargo run --package googlepicz --bin sync_cli -- cache-stats
```

Displays the number of cached albums and media items.

### Background tasks

The `sync` crate exposes helpers for long running operations. `start_periodic_sync`
spawns a task that repeatedly calls `sync_media_items`. Failures trigger an
exponential backoff and each retry is reported via `SyncTaskError::RestartAttempt`.
After five consecutive failures the task emits `SyncTaskError::Aborted` and
terminates. The join handle now resolves to `Result<(), SyncTaskError>` so callers
can check why the loop ended.

`start_token_refresh_task` behaves the same but only refreshes the OAuth token.

| Variant | Meaning |
| ------- | ------- |
| `RestartAttempt(u32)` | The task will retry; the number indicates the attempt. |
| `Aborted(String)` | Too many failures or shutdown caused the task to end. |

## 📈 Performance Profiling

Enable detailed tracing spans and Tokio's console to diagnose slow operations.
Install the console viewer:

```bash
cargo install tokio-console
```

Run it in a separate terminal:

```bash
tokio-console
```

Launch the application with the profiling features enabled:

```bash
cargo run --package googlepicz --features sync/trace-spans,ui/trace-spans -- --debug-console --trace-spans
```

The console will display asynchronous task metrics while span timings are
written to `~/.googlepicz/googlepicz.log`.

## 🐳 CI Docker Image

The repository includes a `Dockerfile.ci` used to build a container image with stable Rust and the packaging tools required for CI. To build and publish the image:
 ```bash
# Build the image
docker build -f Dockerfile.ci -t ghcr.io/christopher-schulze/googlepicz-ci:latest .

# Authenticate to GHCR (if not already logged in)
echo "$CR_PAT" | docker login ghcr.io -u USERNAME --password-stdin

# Push to GitHub Container Registry
docker push ghcr.io/christopher-schulze/googlepicz-ci:latest
```

The GitHub Actions workflow references this image to ensure consistent dependencies across CI runs.

## ⚠️ Note
This project is under active development. Features and APIs are subject to change. Documentation will be updated as the project evolves.
The `Changelog.md` file tracks changes over time.
