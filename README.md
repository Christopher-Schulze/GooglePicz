<div align="center">
  <img src="logo/image.png" alt="GooglePicz Logo" width="200" style="border-radius: 20px; box-shadow: 0 4px 8px rgba(0,0,0,0.1);">

  # 🖼️ GooglePicz

  [![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
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
- [Example Config](docs/EXAMPLE_CONFIG.md) – sample `AppConfig` file.

## Sync CLI

Run the `sync_cli` binary for manual synchronization or to inspect the local cache.
The tool exposes subcommands for `sync`, `status`, `clear-cache` and `list-albums` and prints progress updates
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
