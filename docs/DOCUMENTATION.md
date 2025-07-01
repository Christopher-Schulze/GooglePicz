# GooglePicz Documentation

## 📋 Overview
GooglePicz is a native Google Photos client being developed in Rust. The application focuses on performance, security, and user experience. The project is structured as a Rust workspace with multiple crates.

## 🚧 Project Status: Early Development

**Note**: This project is currently in active development. The information in this documentation reflects the current state and is subject to change as development progresses.

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
cargo clippy

# Run tests
cargo test
```

## 🌎 Environment Variables

The application and packaging scripts rely on several environment variables:

- `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` – OAuth 2.0 credentials required for authentication.
- `MAC_SIGN_ID` – Signing identity used on macOS (optional).
- `APPLE_ID` and `APPLE_PASSWORD` – Credentials for notarizing macOS builds (optional).
- `WINDOWS_CERT` and `WINDOWS_CERT_PASSWORD` – Path and password for a Windows code signing certificate (optional).
- `LINUX_SIGN_KEY` – GPG key ID used to sign the generated `.deb` package (optional).
- `MOCK_REFRESH_TOKEN` – Used only for automated tests to bypass live authentication.
- `MOCK_COMMANDS` – Skips running external tools during packaging tests.

### Packaging installers
Run the packager binary to create platform specific artifacts. The version is
read from `Cargo.toml`, so each file is versioned automatically. Linux packages
are built using `cargo deb` with the `--deb-version` flag, requiring the
`cargo-deb` crate in `dev-dependencies`. The `.deb` output is renamed to include
the version just like the Windows installer.

```bash
cargo run --package packaging --bin packager
```

Generated files include `GooglePicz-<version>-Setup.exe` for Windows and
`GooglePicz-<version>.deb` for Debian-based Linux. The `.deb` file is renamed
after creation to embed the version, mirroring the Windows installer name.

## Sync CLI

In addition to the main application UI, the project provides a command line utility for manual synchronization and cache inspection. The binary lives under `app/src/bin/sync_cli.rs` and is built alongside the rest of the workspace.

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

## 📝 Next Steps
### Short-term Goals
1. Complete basic photo viewing functionality
2. Implement album management
3. Add settings and preferences

### Long-term Goals
1. Video playback support
2. Advanced search features
3. Face recognition and tagging
4. Cross-platform packaging

## ⚠️ Note
This project is under active development. Features and APIs are subject to change. Documentation will be updated as the project evolves.
- **Documentation**: `Changelog.md` and `DOCUMENTATION.md` files are maintained and updated with project progress.
