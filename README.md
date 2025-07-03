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

This project is currently in active development and not yet ready for production use. We're building a native desktop solution to fill the gap left by Google's lack of official desktop clients.

## 🎯 Project Goals

- 🚀 Native performance with Rust
- 🔒 Secure OAuth2 authentication
- ⚡ GPU-accelerated image rendering
- 📂 Local cache for offline access
- 🎨 Cross-platform UI with Iced

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

1. Create OAuth credentials in the [Google Cloud Console](https://console.developers.google.com/).
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

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for optional settings via `AppConfig`.

## ❓ Troubleshooting

Having trouble starting the application? Here are a few common issues:

- **Missing environment variables** – Ensure `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` are set before launching. See the configuration guide linked above.
- **OAuth redirect fails** – Check that the redirect port in your config is open and not blocked by a firewall.
- **Packaging errors** – The packager relies on external tools like `cargo deb` and `makensis`. Use the `MOCK_COMMANDS` environment variable to run packaging tests without these tools.
- **Developing without network access** – Set `MOCK_API_CLIENT=1` and `MOCK_KEYRING=1` to enable offline mode while testing.

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

See the following documents for additional details:
- [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md) – detailed technical documentation.
- [Configuration Guide](docs/CONFIGURATION.md) – lists available `AppConfig` options.
- Command line flags (e.g. `--log-level debug`) can override config values at runtime.
- [Example Config](docs/EXAMPLE_CONFIG.md) – sample `AppConfig` file.
- [Release Artifacts Guide](docs/RELEASE_ARTIFACTS.md) – how to create installers.

## Sync CLI

Run the `sync_cli` binary for manual synchronization or to inspect the local cache.
Like the GUI, it reads settings from `~/.googlepicz/config` via `AppConfig` and supports
the same command line overrides (e.g. `--log-level debug`).
The tool exposes subcommands for `sync`, `status`, `clear-cache`, `list-albums`,
`create-album`, `delete-album` and `cache-stats` and prints progress updates
to stdout while downloading items. The source code lives in
`app/src/bin/sync_cli.rs`.

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
cargo run --package googlepicz --bin sync_cli -- rename-album ALBUM_ID "New Title"
```

Renames the specified album in Google Photos and updates the local cache.

```bash
cargo run --package googlepicz --bin sync_cli -- delete-album ALBUM_ID
```

Deletes the album from Google Photos and the local cache.

```bash
cargo run --package googlepicz --bin sync_cli -- list-photos --album ALBUM_ID
```

Lists cached photos, optionally filtering by album ID. Omit `--album` to list all photos.

```bash
cargo run --package googlepicz --bin sync_cli -- cache-stats
```

Shows how many albums and media items are cached locally.

## Packaging & Signing

The `packager` binary produces installers for macOS, Windows and Debian-based Linux systems.
Signing requires a few environment variables:

- `MAC_SIGN_ID` – identity passed to `codesign` on macOS.
- `APPLE_ID` and `APPLE_PASSWORD` – credentials for notarization on macOS.
- `WINDOWS_CERT` and `WINDOWS_CERT_PASSWORD` – code signing certificate for Windows.
- `LINUX_SIGN_KEY` – GPG key ID used by `dpkg-sig` to sign the generated `.deb` (optional).

Set these variables in your shell or CI environment before running `cargo run --package packaging --bin packager`.

## Running Tests

Unit tests mock external services using environment variables. Run `cargo test` and everything should pass without Google credentials.


## 📄 License

MIT - See [LICENSE](LICENSE) for details.
