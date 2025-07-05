<div align="center">
  <img src="logo/image.png" alt="GooglePicz Logo" width="200" style="border-radius: 20px; box-shadow: 0 4px 8px rgba(0,0,0,0.1);">

  # 🖼️ GooglePicz

  [![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
  [![CI](https://github.com/Christopher-Schulze/GooglePicz/actions/workflows/ci.yml/badge.svg)](https://github.com/Christopher-Schulze/GooglePicz/actions/workflows/ci.yml)
  [![Project Status: WIP](https://img.shields.io/badge/status-WIP-yellow)](https://github.com/Christopher-Schulze/GooglePicz)
</div>


> A work-in-progress native Google Photos client for macOS and Windows, built with Rust for maximum performance and efficiency.

## 🚧 Project Status: Early Development

This project is **experimental** and not yet ready for production use. We're building a native desktop solution to fill the gap left by Google's lack of official desktop clients. Planned features like video playback, advanced search and face recognition are still under development.

## 🎯 Project Goals

- 🚀 Native performance with Rust
- 🔒 Secure OAuth2 authentication
- ⚡ GPU-accelerated image rendering
- 📂 Local cache for offline access
- 🎨 Cross-platform UI with Iced

### Planned Features
- Video playback
- Advanced search capabilities
- Face recognition and tagging

## 🛠️ Technical Stack

- **Language**: Rust 1.70+
- **UI Framework**: Iced (wgpu backend)
- **Storage**: SQLite
- **Authentication**: OAuth2
- **Target Platforms**: macOS & Windows

## 📦 Getting the Code

```bash
git clone https://github.com/Christopher-Schulze/GooglePicz.git
cd GooglePicz
```

## 🚀 Quick Start

1. [Create OAuth credentials](#setting-up-oauth-credentials).
2. Export the required environment variables so the application can authenticate:

```bash
export GOOGLE_CLIENT_ID=your_client_id
export GOOGLE_CLIENT_SECRET=your_client_secret
```

3. Run the GUI application:

```bash
cargo run --package googlepicz
```

   Or launch the command line interface:

```bash
cargo run --package googlepicz --bin sync_cli -- sync
```

See [docs/USER_GUIDE.md](docs/USER_GUIDE.md) for configuration options and example settings.

### Setting up OAuth Credentials

1. Sign in to the [Google Cloud Console](https://console.developers.google.com/) and create a new project.
2. Enable the **Google Photos Library API** for that project.
3. Configure an **OAuth consent screen** (External) and add your user as a tester.
4. Create new **OAuth client credentials** of type **Desktop application**.
5. Note the generated **client ID** and **client secret**.
6. Export the credentials so the application can authenticate:

```bash
export GOOGLE_CLIENT_ID="your_client_id"
export GOOGLE_CLIENT_SECRET="your_client_secret"
```

Now you can run the application as shown above.

### Token Storage

Authentication tokens are stored in the system keyring by default. If the application is compiled with the optional `file-store` feature you can persist tokens in `~/.googlepicz/tokens.json` instead by passing `--use-file-store` or setting `USE_FILE_STORE=1` before launching.

## ❓ Troubleshooting

Having trouble starting the application? Here are a few common issues:

- **Missing environment variables** – Ensure `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` are set before launching. See the configuration guide linked above.
- **OAuth redirect fails** – Check that the redirect port in your config is open and not blocked by a firewall.
- **Packaging errors** – The packager relies on external tools like `cargo deb` and `makensis`. Use the `MOCK_COMMANDS` environment variable to run packaging tests without these tools.
- **GStreamer not installed** – Build the `ui` crate with `--no-default-features` to disable the video backend.
- **Developing without network access** – Set `MOCK_API_CLIENT=1` and `MOCK_KEYRING=1` (and optionally `MOCK_ACCESS_TOKEN`/`MOCK_REFRESH_TOKEN`) to run all tests without hitting Google APIs.
- **Need more insight into async tasks?** – Set `debug_console = true` in `~/.googlepicz/config` or pass `--debug-console` to print detailed Tokio diagnostics.
- **Profiling spans** – Set `trace_spans = true` or pass `--trace-spans` and build with `--features sync/trace-spans,ui/trace-spans` to record timing data.
- **Missing system libraries on Linux** – Install `glib2.0-dev`, `gstreamer1.0-dev` and `libssl-dev` (or the equivalent packages for your distribution). See [docs/USER_GUIDE.md](docs/USER_GUIDE.md) for details. On Debian/Ubuntu run:

  ```bash
  sudo apt install glib2.0-dev gstreamer1.0-dev libssl-dev
  ```

  On Fedora/RHEL use:

  ```bash
  sudo dnf install glib2-devel gstreamer1-devel openssl-devel
  ```

  Missing these libraries can lead to build failures about unavailable headers.

## 📑 Logs and Error Reports

Runtime logs are written to `~/.googlepicz/googlepicz.log`. The UI also
stores recent error messages in `~/.googlepicz/ui_errors.log` for easier
diagnostics. Delete these files if they grow too large.
To record detailed span timings, build with the `trace-spans` feature for
each crate, for example `--features sync/trace-spans,ui/trace-spans`.

## 🏗️ Project Structure

```
GooglePicz/
├── app/          # Main application
├── auth/         # OAuth2 authentication
├── api_client/   # Google Photos API client
├── ui/           # User interface (Iced)
├── cache/        # Local SQLite cache
└── sync/         # Background synchronization
```

## 📝 Documentation

See [docs/USER_GUIDE.md](docs/USER_GUIDE.md) for configuration, optional features and build instructions.
- [Release Artifacts Guide](docs/RELEASE_ARTIFACTS.md#release-process) – how to create installers and sign them.

## Sync CLI

Run the `sync_cli` binary for manual synchronization or to inspect the local cache.
Like the GUI, it reads settings from `~/.googlepicz/config` via `AppConfig` and supports
the same command line overrides (e.g. `--log-level debug`). Available options include
`--oauth-redirect-port`, `--thumbnails-preload`, `--sync-interval-minutes`, `--config`,
`--debug-console` and `--use-file-store`.
The tool exposes subcommands for `sync`, `status`, `clear-cache`, `list-albums`,
`create-album`, `delete-album`, `rename-album`, `add-to-album`, `list-album-items`,
`cache-stats`, `list-items`, `search`, `show-item`,
`export-items`, `import-items` and `export-albums` and prints progress updates
to stdout while downloading items. The source code lives in `app/src/bin/sync_cli.rs`.

```bash
cargo run --package googlepicz --bin sync_cli -- sync
```

Synchronizes all media items and prints progress.

```bash
cargo run --package googlepicz --bin sync_cli -- --help
```

Displays the available commands.

```bash
cargo run --package googlepicz --bin sync_cli -- status
```

Displays the timestamp of the last sync along with the number of cached photos.

```bash
cargo run --package googlepicz --bin sync_cli -- list-albums
```

Lists all albums stored in the local cache.

```bash
cargo run --package googlepicz --bin sync_cli -- clear-cache
```

Removes all cached media items.

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

Renames an existing album on Google Photos and updates the cache.

```bash
cargo run --package googlepicz --bin sync_cli -- add-to-album ALBUM_ID ITEM_ID
```

Associates a cached media item with an album in the local database.

```bash
cargo run --package googlepicz --bin sync_cli -- list-album-items ALBUM_ID
```

Lists cached items that belong to the given album.

```bash
cargo run --package googlepicz --bin sync_cli -- cache-stats
```

Shows how many albums and media items are cached locally.

```bash
cargo run --package googlepicz --bin sync_cli -- list-items --limit 5
```

Lists cached media items, optionally limiting the output.

```bash
cargo run --package googlepicz --bin sync_cli -- search QUERY
```

Searches cached media items by filename or description.

```bash
cargo run --package googlepicz --bin sync_cli -- show-item ITEM_ID
```

Displays the JSON metadata for a single cached media item.

```bash
cargo run --package googlepicz --bin sync_cli -- export-items --file items.json
```

Exports all cached media items to a file.

```bash
cargo run --package googlepicz --bin sync_cli -- import-items --file items.json
```

Imports media items from a file.

```bash
cargo run --package googlepicz --bin sync_cli -- export-albums --file albums.json
```

Exports all cached albums to a file.

## Packaging & Signing

The `packager` binary produces installers for macOS, Windows and Debian-based Linux systems. On Linux you can choose the output format with `--format` (`deb`, `rpm` or `appimage`).
Signing requires a few environment variables:

- `MAC_SIGN_ID` – identity passed to `codesign` on macOS.
- `APPLE_ID` and `APPLE_PASSWORD` – credentials for notarization on macOS.
- `WINDOWS_CERT` and `WINDOWS_CERT_PASSWORD` – code signing certificate for Windows.
- `LINUX_SIGN_KEY` – GPG key ID used by `dpkg-sig` to sign the generated `.deb` (optional).

The packager also requires a few external tools to be available in your `PATH`.
See the [required tools table](docs/RELEASE_ARTIFACTS.md#required-tools) for
installation commands.

- `cargo-deb` – creates Debian packages
- `cargo-bundle` – bundles macOS apps
- `cargo-bundle-licenses` – collects license metadata
- `makensis` – part of the NSIS suite used for Windows installers

Set these variables in your shell or CI environment before running `cargo run --package packaging --bin packager`.

Examples:

```bash
# Create a Debian package
cargo run --package packaging --bin packager -- --format deb

# Build an RPM on Fedora
cargo run --package packaging --bin packager -- --format rpm

# Create an AppImage
cargo run --package packaging --bin packager -- --format appimage
```

### Creating Release Artifacts

1. Ensure all tools listed above are installed.
2. Export the signing variables needed for your platform.
3. Run `cargo run --package packaging --bin packager` from the workspace root.
4. Retrieve the generated files from `target/` (e.g. `GooglePicz-<version>-Setup.exe` or `GooglePicz-<version>.deb`).
5. See the [release process](docs/RELEASE_ARTIFACTS.md#release-process) for signing and notarization details.

## Running Tests

Unit tests mock external services using environment variables. Run `cargo test` and everything should pass without Google credentials.


## 📄 License

MIT - See [LICENSE](LICENSE) for details.
