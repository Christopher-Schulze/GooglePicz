# Example Config

Create `~/.googlepicz/config` with all fields of the `AppConfig` struct. Video
playback and face recognition are compile-time features and are **not**
configured here:

```toml
log_level = "debug"
oauth_redirect_port = 9000
thumbnails_preload = 30
sync_interval_minutes = 15
cache_path = "/tmp/googlepicz"
debug_console = false
trace_spans = false
detect_faces = false
```

Adjust the values as needed.
