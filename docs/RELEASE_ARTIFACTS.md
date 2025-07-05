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
| `cargo-rpm` | Build RPM packages | `cargo install cargo-rpm` |
| `appimagetool` | Create AppImage bundles | Install from your distro or [AppImage releases](https://github.com/AppImage/AppImageKit/releases) |
| `makensis` | Create Windows installers | Install the [NSIS](https://nsis.sourceforge.io/) package |

Environment variables:

- `MAC_SIGN_ID` – Signing identity for macOS
- `APPLE_ID` and `APPLE_PASSWORD` – Apple account used for notarization
- `WINDOWS_CERT` and `WINDOWS_CERT_PASSWORD` – Code signing certificate for Windows
- `LINUX_SIGN_KEY` – GPG key ID for signing `.deb` files
- `LINUX_PACKAGE_FORMAT` – Package type on Linux (`deb`, `rpm` or `appimage`)

Example values:

```bash
export MAC_SIGN_ID="Developer ID Application: Example Corp (ABCD1234)"
export APPLE_ID="user@example.com"
export APPLE_PASSWORD="app-password"
export WINDOWS_CERT="C:/certs/googlepicz.pfx"
export WINDOWS_CERT_PASSWORD="secret"
export LINUX_SIGN_KEY="0xDEADBEEF"
```

### Signing and notarization

The variables above enable code signing on all platforms and macOS notarization.

- `MAC_SIGN_ID` is the identity passed to `codesign`.
- `APPLE_ID` and `APPLE_PASSWORD` are used by `notarytool` when submitting the DMG.
- `WINDOWS_CERT` points to the `.pfx`/`.p12` certificate for `signtool` and `WINDOWS_CERT_PASSWORD` is its password.
- `LINUX_SIGN_KEY` is the GPG key ID used by `dpkg-sig`.

If any of them are unset the packager skips the respective signing or notarization steps.

### Step-by-Step Setup

1. **Install Rust** using `rustup` if it's not already installed:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install the required Cargo tools**:

   ```bash
   cargo install cargo-deb cargo-bundle cargo-bundle-licenses cargo-rpm
   ```

3. **Install platform utilities**:
   - **Linux** – install build dependencies and optional signing tools:

     ```bash
     sudo apt install glib2.0-dev gstreamer1.0-dev libssl-dev dpkg-sig appimagetool
     ```

     On Fedora/RHEL:

     ```bash
     sudo dnf install glib2-devel gstreamer1-devel openssl-devel rpm-build appimagetool
     ```

   - **macOS** – install Xcode command line tools with `xcode-select --install`.
   - **Windows** – install the NSIS suite so `makensis` is available.

4. With the tools available in your `PATH`, run the packager as shown below.
   Use `LINUX_PACKAGE_FORMAT=rpm` or `LINUX_PACKAGE_FORMAT=appimage` to switch
   the output format on Linux.

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
   - Linux: `target/GooglePicz-<version>.<ext>` where `<ext>` is `deb`, `rpm` or `AppImage`

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
   - **Linux** – The generated package (`.deb`, `.rpm` or `AppImage`) is optionally signed.
5. The packager verifies each signature (`codesign --verify`, `signtool verify`,
   `dpkg-sig --verify`). Check the console output for any errors.
6. Upload the versioned artifacts from the `target` directory when creating the
   GitHub release.

### Uploading the artifacts

The generated installers can be attached to a GitHub release either via the web
interface or using the `gh` CLI:

```bash
gh release upload <tag> target/release/GooglePicz-*.dmg GooglePicz-*.{deb,rpm,AppImage} target/windows/GooglePicz-*-Setup.exe
```

Replace `<tag>` with the version tag you are publishing. Drag‑and‑drop also
works on the release page.
