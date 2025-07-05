# Linux Build Requirements

Zum Kompilieren von GooglePicz werden unter Linux einige Entwicklungsbibliotheken benötigt. Installiere auf Debian/Ubuntu die folgenden Pakete:

```bash
sudo apt install glib2.0-dev gstreamer1.0-dev libssl-dev
```

Auf Fedora/RHEL heißen die Pakete:

```bash
sudo dnf install glib2-devel gstreamer1-devel openssl-devel
```

Wenn GStreamer nicht verfügbar ist oder die Video-Unterstützung nicht benötigt wird, kann das `ui`-Paket ohne die Standardfeatures gebaut werden:

```bash
cargo build -p ui --no-default-features
```

## Profiling mit tokio-console

Installiere die Konsole einmalig:

```bash
cargo install tokio-console
```

Starte sie in einem separaten Terminal:

```bash
tokio-console
```

Baue und starte GooglePicz mit aktivierten `trace-spans` Features:

```bash
cargo run --package googlepicz --features sync/trace-spans,ui/trace-spans -- --debug-console --trace-spans
```

Die Konsole zeigt laufende Tasks an, detaillierte Span-Daten finden sich in `~/.googlepicz/googlepicz.log`.
