# Configuration Guide

`AppConfig` (defined in [`app/src/config.rs`](../app/src/config.rs)) controls runtime behavior of GooglePicz. Values can be loaded from `~/.googlepicz/config` using the [config](https://docs.rs/config) crate.

| Option | Type | Default | Description |
| ------ | ---- | ------- | ----------- |
| `log_level` | `String` | `"info"` | Verbosity of application logging. Valid values follow the `env_logger` log levels. |
| `oauth_redirect_port` | `u16` | `8080` | Port used during OAuth authentication for the local redirect server. |
| `thumbnails_preload` | `usize` | `20` | Number of thumbnails to prefetch when loading an album. |
| `sync_interval_minutes` | `u64` | `5` | Time between automatic background sync operations. |

Edit or create `~/.googlepicz/config` and provide any of these keys to customize the application.
