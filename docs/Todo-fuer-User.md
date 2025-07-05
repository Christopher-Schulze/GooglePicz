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

## Pakete für den Packager

Zur Erstellung von Installern benötigt der Packager einige externe Tools. Stelle sicher, dass folgende Programme verfügbar sind:

- `cargo-deb`, `cargo-bundle`, `cargo-bundle-licenses`, `cargo-rpm`
  ```bash
  cargo install cargo-deb cargo-bundle cargo-bundle-licenses cargo-rpm
  ```
- `appimagetool` und `dpkg-sig` (optional für Linux-Signing)
  ```bash
  sudo apt install appimagetool dpkg-sig   # Debian/Ubuntu
  sudo dnf install appimagetool dpkg-sig   # Fedora/RHEL
  ```
- `makensis` (Windows) und `signtool` aus dem Windows SDK
- `codesign`, `hdiutil` und `xcrun` auf macOS (Teil der Xcode Command Line Tools)

Diese Programme müssen im `PATH` liegen, damit `cargo run --package packaging --bin packager` erfolgreich ist.
