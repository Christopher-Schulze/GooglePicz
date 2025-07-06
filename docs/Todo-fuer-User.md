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

Baue und starte GooglePicz mit aktivierten `trace-spans` und `tokio-console` Features:

```bash
cargo run --package googlepicz --features googlepicz/tokio-console,sync/trace-spans,ui/trace-spans -- --debug-console --trace-spans
```

Die Konsole zeigt laufende Tasks an, detaillierte Span-Daten finden sich in `~/.googlepicz/googlepicz.log`.

## Manuell zu installierende Packaging-Werkzeuge

Der Packager von GooglePicz nutzt einige externe Tools, die nicht automatisch mit
`cargo` installiert werden können. Stelle sicher, dass sie über die jeweilige
Paketverwaltung verfügbar sind und im `PATH` liegen:

- **macOS**: `codesign`, `hdiutil` und `xcrun` sind Teil der Xcode Command Line
  Tools. Installiere sie mit:

  ```bash
  xcode-select --install
  ```

- **Windows**: Der Installer wird mit **NSIS** erstellt. Lade das Paket von
  <https://nsis.sourceforge.io/> herunter und füge `makensis` zum `PATH`
  hinzu. Für Codesigning wird `signtool` aus dem Windows SDK benötigt.

- **Linux**: Optionales Signieren von `.deb`-Paketen setzt `dpkg-sig` voraus.

Ohne diese Programme werden die entsprechenden Arbeitsschritte übersprungen bzw.
schlagen fehl.
