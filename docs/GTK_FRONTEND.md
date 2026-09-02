# GTK4 + libadwaita frontend

NexPlay's official Linux frontend is the native GTK4/libadwaita application.
It runs in the root Rust binary and shares the existing `AppContext`, SQLite
database, configuration, metadata providers, scanner, Nyaa integration, and
qBittorrent integration in the same process.

## Requirements

- Linux with GTK 4, libadwaita, GStreamer, and the GTK4 GStreamer video sink.
- `pkg-config`.
- A Rust toolchain with edition 2024 support.

On Arch Linux, the relevant packages are:

```bash
sudo pacman -S gtk4 libadwaita gst-plugin-gtk4 gst-plugins-base gst-plugins-good pkgconf
```

## Run and build

Run the native frontend from the repository:

```bash
cargo run -- gtk
```

The equivalent npm convenience commands are:

```bash
npm run dev:gtk
npm run build:gtk
```

The release binary is `target/release/nexplay`; the desktop entry in
`data/dev.nexplay.NexPlay.desktop` starts it with `nexplay gtk`. Install that
desktop file into `$XDG_DATA_HOME/applications/` (or
`~/.local/share/applications/`) after installing the binary in `PATH`.

## Configuration and data paths

`NEXPLAY_CONFIG` has the highest priority. Without it, the GTK frontend uses:

- Configuration: `$XDG_CONFIG_HOME/nexplay/config.toml`, or
  `~/.config/nexplay/config.toml`.
- Compatibility fallback: an existing repository `config.toml` is selected
  when the XDG configuration is absent or has no media sources.
- New default database: `$XDG_DATA_HOME/nexplay/nexplay.sqlite3`, or
  `~/.local/share/nexplay/nexplay.sqlite3`.

The GTK command also accepts an explicit path:

```bash
cargo run -- gtk --config /path/to/config.toml
```

Relative database and media paths are resolved relative to an explicitly
located configuration file.

An existing configuration is read as-is. The GTK frontend does not migrate or
rewrite an existing database, media directory, or watch-state schema.

For isolated first-run, offline, or error tests, set `NEXPLAY_CONFIG` to a
temporary file and point its `database.path` at a temporary SQLite file.

## Scope

GTK uses native Adwaita navigation, preferences, status pages, action rows,
dialogs, toast notifications, responsive GTK layouts, adaptive wrapping, and
shared skeleton loading states. Settings apply and persist automatically.
Backend work is
performed by a bounded worker pool; the GTK main thread only updates widgets.

Local episodes open in an in-app `GtkVideo` surface backed by GTK's native
`GtkMediaFile`/GStreamer integration. The player restores the saved position,
persists watch progress, records local insight sessions, and updates Bangumi
progress through the shared backend. Subtitle-track selection and danmaku are
not migrated yet. Existing Electron, React, daemon, mpv, and native bridge
files remain in the repository as a temporary reference/fallback. They are not
part of the GTK Linux run or release path and will be removed in a separate
task.

The untracked `experiments/` tree, including `experiments/gtk-adwaita`, is not
part of this frontend.
