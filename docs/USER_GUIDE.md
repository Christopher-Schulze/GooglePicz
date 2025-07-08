# User Guide

## Overview
GooglePicz is an early stage native Google Photos client. The workspace contains several crates for the UI, synchronization and optional features like face recognition.

## Configuration Options
The application reads `AppConfig` from `~/.googlepicz/config`. Available keys are:

| Option | Type | Default | Description |
| ------ | ---- | ------- | ----------- |
| `log_level` | `String` | `"info"` | Verbosity of application logging. |
| `oauth_redirect_port` | `u16` | `8080` | Port used during the OAuth flow. |
| `thumbnails_preload` | `usize` | `20` | Number of thumbnails to preload. |
| `preload_threads` | `usize` | `4` | Number of worker threads for preloading thumbnails. |
| `sync_interval_minutes` | `u64` | `5` | Minutes between automatic sync runs. |
| `cache_path` | `String` | `"~/.googlepicz"` | Location for cache and logs. |
| `debug_console` | `bool` | `false` | Enable the Tokio console subscriber. |
| `trace_spans` | `bool` | `false` | Record tracing spans when built with the `trace-spans` features. |
| `detect_faces` | `bool` | `false` | Run face detection after downloads when built with `sync/face-recognition`. |

### Example Config
Create `~/.googlepicz/config` and adjust the values as needed:

```toml
log_level = "debug"
oauth_redirect_port = 9000
thumbnails_preload = 30
preload_threads = 4
sync_interval_minutes = 15
cache_path = "/tmp/googlepicz"
debug_console = false
trace_spans = false
detect_faces = false
```

### Environment Variables
The following variables influence GooglePicz and the packager:

- `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` – OAuth credentials.
- `MAC_SIGN_ID` – macOS signing identity (optional).
- `APPLE_ID` and `APPLE_PASSWORD` – credentials for notarization (optional).
- `WINDOWS_CERT` and `WINDOWS_CERT_PASSWORD` – Windows code signing certificate (optional).
- `LINUX_SIGN_KEY` – GPG key ID to sign Debian packages (optional).
- `MOCK_REFRESH_TOKEN` – used only for tests.
- `MOCK_COMMANDS` – skip external tools during packaging tests.
- `USE_FILE_STORE` – write tokens to `~/.googlepicz/tokens.json` when set to `1` and compiled with the `file-store` feature.
- `MOCK_API_CLIENT` and `MOCK_KEYRING` – together with `MOCK_ACCESS_TOKEN` allow running tests without network access.

## Optional Features

### Video Playback
Video support relies on GStreamer. Install `glib2.0-dev`, `gstreamer1.0-dev` and `libssl-dev` (or the equivalents for your distribution) before building. If these libraries are unavailable, build the `ui` crate without default features:

```bash
cargo build -p ui --no-default-features
```

Without GStreamer the application still runs but cannot play videos.

### Face Recognition
The `face_recognition` crate can detect faces in a `MediaItem`. Building with
the `cache` feature stores the results permanently using `insert_faces` from
`cache::CacheManager`. When the `ui` feature is also enabled the
`ui::FaceRecognizer` widget overlays the saved bounding boxes whenever a photo
is opened. The sync process automatically runs face detection and persists the
boxes, making them available across sessions. This module is experimental and
disabled by default.

#### Linux Dependencies
Compiling the `face_recognition` crate requires OpenCV with development headers
and the LLVM tooling. On Debian/Ubuntu install:

```bash
sudo apt install libopencv-dev clang libclang-dev llvm-dev
```

On Fedora/RHEL the packages are named:

```bash
sudo dnf install opencv-devel clang llvm-devel
```

If the build fails because `libclang` or `llvm-config` cannot be located, set
the environment variable `LIBCLANG_PATH` or `LLVM_CONFIG_PATH` to the
appropriate location.

### Building Without Extras
Compile the workspace without the video and face recognition crates:

```bash
cargo build --workspace --no-default-features --exclude face_recognition --exclude e2e
```

Individual crates can also be built without optional features:

```bash
cargo build -p ui --no-default-features
cargo build -p face_recognition --no-default-features
```

## Profiling
Install `tokio-console` once:

```bash
cargo install tokio-console
```

Run it in a separate terminal and launch GooglePicz with tracing enabled:

```bash
cargo run --package googlepicz --features sync/trace-spans,ui/trace-spans -- --debug-console --trace-spans
```

The console shows active tasks while span data is written to `~/.googlepicz/googlepicz.log`.

## Packaging
To create installers:

1. Install the tools listed in `docs/RELEASE_ARTIFACTS.md#required-tools`.
2. Export signing variables like `MAC_SIGN_ID` or `WINDOWS_CERT`.
3. Run:

   ```bash
   cargo run --package packaging --bin packager
   ```

Artifacts appear in `target/` (e.g. `GooglePicz-<version>-Setup.exe` or `.deb`).

## Command Line Interface
The workspace provides `sync_cli` for manual synchronization and cache
inspection. Run `--help` for available subcommands. It respects the same
configuration and environment variables as the GUI. The `search` command now
supports filtering with `--start` and `--end` date parameters as well as the
`--favorite` flag to only list starred items.

Further options include `--camera-model`, `--camera-make`, `--mime-type` and
`--faces` to only return items with detected faces. Use `set-favorite <ID> true`
or `false` to update the favorite state of a cached item. Face metadata can be
exported and imported with the `export-faces` and `import-faces` subcommands.


