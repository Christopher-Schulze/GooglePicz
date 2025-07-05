# Creating Release Artifacts

This document describes how to build installers for all supported platforms.

## Prerequisites

- Rust toolchain installed (`rustup`)
- Required signing credentials if you want signed binaries
- Development libraries on Linux such as `glib2.0-dev`, `gstreamer1.0-dev` and `libssl-dev` (or distribution equivalents). For example:

  ```bash
  sudo apt install glib2.0-dev gstreamer1.0-dev libssl-dev
  ```

  On Fedora/RHEL run:

  ```bash
  sudo dnf install glib2-devel gstreamer1-devel openssl-devel
  ```

### Required Tools {#required-tools}

| Tool | Purpose | Installation |
| --- | --- | --- |
| `cargo-deb` | Build Debian packages | `cargo install cargo-deb` |
| `cargo-bundle` | Bundle macOS apps | `cargo install cargo-bundle` |
| `cargo-bundle-licenses` | Collect license metadata | `cargo install cargo-bundle-licenses` |
| `makensis` | Create Windows installers | Install the [NSIS](https://nsis.sourceforge.io/) package |

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

### GitHub Actions

The workflow in `.github/workflows/rust.yml` runs the packager on
Linux, macOS and Windows. Each run uploads the generated `.deb`, `.dmg`
and Windows installer via `upload-artifact`. You can download these
artifacts from the workflow run page without building them locally.
