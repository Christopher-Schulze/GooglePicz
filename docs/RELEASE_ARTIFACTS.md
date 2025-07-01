# Creating Release Artifacts

This document describes how to build installers for all supported platforms.

## Prerequisites

- Rust toolchain installed (`rustup`)
- `cargo-deb` for creating Debian packages (`cargo install cargo-deb`)
- Required signing credentials if you want signed binaries

Environment variables:

- `MAC_SIGN_ID` – Signing identity for macOS
- `APPLE_ID` and `APPLE_PASSWORD` – Apple account used for notarization
- `WINDOWS_CERT` and `WINDOWS_CERT_PASSWORD` – Code signing certificate for Windows
- `LINUX_SIGN_KEY` – GPG key ID for signing `.deb` files

## Steps

1. Run the packager from the workspace root:

   ```bash
   cargo run --package packaging --bin packager
   ```

   The command bundles license information, builds a release binary and
   creates an installer appropriate for the current platform.

2. The produced files are written to the `target` directory:

   - Windows: `target/windows/GooglePicz-<version>-Setup.exe`
   - macOS: `target/release/GooglePicz.dmg`
   - Linux: `target/GooglePicz-<version>.deb`

These paths include the workspace version from `Cargo.toml` to guarantee
reproducible artifact names across Linux, macOS and Windows.
