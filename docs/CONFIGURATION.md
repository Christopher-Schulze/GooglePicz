# Configuration Guide

This document lists the settings available in `AppConfig` (see [`app/src/config.rs`](../app/src/config.rs)).
Values can be placed in `~/.googlepicz/config` and are loaded via the [config](https://docs.rs/config) crate.

| Option | Type | Default | Description |
| ------ | ---- | ------- | ----------- |
| `log_level` | `String` | `"info"` | Verbosity of application logging. Follows `env_logger` levels. |
| `oauth_redirect_port` | `u16` | `8080` | Port used for OAuth redirect during authentication. |
| `thumbnails_preload` | `usize` | `20` | Number of thumbnails to preload when displaying an album. |
| `sync_interval_minutes` | `u64` | `5` | Minutes between automatic synchronization runs. |

Create or edit `~/.googlepicz/config` and provide any of these keys to customize the application.

All settings can also be overridden at runtime using command line options. Run `googlepicz --help` or `sync_cli --help` to see the available flags (e.g. `--log-level debug`).
