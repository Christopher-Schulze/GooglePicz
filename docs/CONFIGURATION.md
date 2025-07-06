# Configuration Guide

This document lists the settings available in `AppConfig` (see [`app/src/config.rs`](../app/src/config.rs)).
Values can be placed in `~/.googlepicz/config` and are loaded via the [config](https://docs.rs/config) crate.

| Option | Type | Default | Description |
| ------ | ---- | ------- | ----------- |
| `log_level` | `String` | `"info"` | Verbosity of application logging. Follows `env_logger` levels. |
| `oauth_redirect_port` | `u16` | `8080` | Port used for OAuth redirect during authentication. |
| `thumbnails_preload` | `usize` | `20` | Number of thumbnails to preload when displaying an album. |
| `sync_interval_minutes` | `u64` | `5` | Minutes between automatic synchronization runs. |
| `cache_path` | `String` | `"~/.googlepicz"` | Directory where cache and logs are stored. |
| `debug_console` | `bool` | `false` | Enable the tokio console subscriber for debugging asynchronous tasks. |
| `trace_spans` | `bool` | `false` | Record detailed tracing spans when compiled with the `trace-spans` features. |
| `detect_faces` | `bool` | `false` | Run face detection after downloading images when built with `sync/face-recognition`. |

Create or edit `~/.googlepicz/config` and provide any of these keys to customize the application. Setting `debug_console = true` turns on Tokio's debugging console.

Setting `trace_spans = true` enables tracing instrumentation across all crates. The application must be built with the corresponding `trace-spans` features, e.g. `cargo run --features sync/trace-spans,ui/trace-spans`.

All settings can also be overridden at runtime using command line options. Run `googlepicz --help` or `sync_cli --help` to see the available flags. The `debug_console` option can be enabled with the `--debug-console` flag.
Use `--trace-spans` to enable `trace_spans` from the command line.
The `search` subcommand of `sync_cli` supports additional filters: use `--start`
and `--end` to specify a date range and `--favorite` to only display starred
items.

If the application is built with the optional `file-store` feature, authentication
tokens may be written to `~/.googlepicz/tokens.json` instead of the system
keyring. Enable this behaviour by passing `--use-file-store` on the command line
or by setting the environment variable `USE_FILE_STORE=1` before running the
tools.

## Environment Variables

Several environment variables influence how GooglePicz and the packaging scripts run:

- `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` – OAuth credentials required for authentication.
- `MAC_SIGN_ID` – Signing identity used on macOS (optional).
- `APPLE_ID` and `APPLE_PASSWORD` – Credentials for notarizing macOS builds (optional).
- `WINDOWS_CERT` and `WINDOWS_CERT_PASSWORD` – Windows code signing certificate (optional).
- `LINUX_SIGN_KEY` – GPG key ID used to sign the generated `.deb` package (optional).
- `MOCK_REFRESH_TOKEN` – Used only for automated tests to bypass live authentication.
- `MOCK_COMMANDS` – Skips running external tools during packaging tests.
- `USE_FILE_STORE` – Write tokens to `~/.googlepicz/tokens.json` when set to `1` and the optional `file-store` feature is enabled.
- `MOCK_API_CLIENT` and `MOCK_KEYRING` – together with `MOCK_ACCESS_TOKEN` and `MOCK_REFRESH_TOKEN` allow running the test suite without network access.

### Background Sync Messages

Synchronization tasks send `SyncTaskError` events over an error channel. Relevant variants are:

- `PeriodicSyncFailed` – a sync run failed and will retry.
- `TokenRefreshFailed` – refreshing the OAuth token failed.
- `RestartAttempt(u32)` – retry counter during exponential backoff.
- `Aborted(String)` – task stopped after repeated errors.
- `Status { last_synced, message }` – informational updates.


### Video Playback Dependencies

Video playback relies on the GStreamer multimedia framework. On most Linux
systems you need to install the development packages `glib2.0-dev`,
`gstreamer1.0-dev` and `libssl-dev` (or their distribution equivalents) before
building. If GStreamer is not available you can disable video support by
building the `ui` crate without default features:

```bash
cargo build -p ui --no-default-features
```

Without these libraries the application will still run, but videos cannot be
played back.

### Building Workspace Crates Without Optional Features

Some crates enable additional capabilities through default features.
The `ui` crate pulls in the GStreamer backend and the `face_recognition` crate
contains experimental code. To compile the workspace without these extras, use
`--no-default-features` and exclude the unused crate:

```bash
cargo build --workspace --no-default-features --exclude face_recognition --exclude e2e
```

You can also build individual crates without their optional features:

```bash
cargo build -p ui --no-default-features
cargo build -p face_recognition --no-default-features
```

When built with the `face_recognition` crate and its `cache` feature, the sync
process will detect faces during downloads and store the results in the local
database. Enable these features to keep face data available for the UI.
