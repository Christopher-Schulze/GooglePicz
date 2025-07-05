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

Example values:

```bash
export MAC_SIGN_ID="Developer ID Application: Example Corp (ABCD1234)"
export APPLE_ID="user@example.com"
export APPLE_PASSWORD="app-password"
export WINDOWS_CERT="C:/certs/googlepicz.pfx"
export WINDOWS_CERT_PASSWORD="secret"
export LINUX_SIGN_KEY="0xDEADBEEF"
```

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

## Release Process {#release-process}

Follow these steps to create and sign final release artifacts:

1. Bump the workspace version in `Cargo.toml` and update `docs/Changelog.md`.
2. Run the full test suite with `cargo test`.
3. Export the signing credentials as shown above.
4. Run `cargo run --package packaging --bin packager` on each target platform.
   - **macOS** – The `.app` bundle and resulting `.dmg` are signed with
     `codesign`. If `APPLE_ID` and `APPLE_PASSWORD` are present the DMG is
     notarized with `notarytool` and stapled.
   - **Windows** – Both the installer and `googlepicz.exe` are signed using
     `signtool`.
   - **Linux** – The generated `.deb` is signed via `dpkg-sig`.
5. The packager verifies each signature (`codesign --verify`, `signtool verify`,
   `dpkg-sig --verify`). Check the console output for any errors.
6. Upload the versioned artifacts from the `target` directory when creating the
   GitHub release.
