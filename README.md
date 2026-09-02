# NexPlay

NexPlay is a desktop anime media library. The official Linux frontend is a
native GTK4 + libadwaita application backed by Rust, SQLite, Bangumi metadata,
Nyaa resource discovery, and qBittorrent integration.

The GTK application runs `AppContext` and the frontend in one process. It uses
GNOME's standard adaptive navigation, preferences, status pages, dialogs, and
notifications instead of the former renderer's visual system.

## Features

- Recursive local media indexing with SQLite persistence and per-episode watch
  state.
- Home, discovery, local/cloud library, global search, subject details,
  resource search, downloads, insights, and settings pages.
- Bangumi public-calendar discovery with six-hour caching, metadata hydration,
  collection sync, OAuth loopback login, and episode status updates.
- Nyaa search with resolution/batch filters and qBittorrent task creation,
  file selection, progress polling, pause/resume/cancel, and cleanup.
- Native background image loading with local files, HTTP(S), process cache,
  and failure placeholders.
- A bounded backend worker pool so scanning, network calls, database work, and
  download operations do not block the GTK main loop.

The GTK frontend intentionally does not migrate video playback yet. Its detail
page keeps a disabled `播放器尚未迁移` entry and never calls mpv, subtitles,
danmaku rendering, playback controls, or new playback-session recording.

Electron/React, the backend daemon, mpv path, and native bridge remain in the
repository as temporary reference and fallback code. They are not used by the
GTK Linux entrypoint and will be removed separately.

## Current status

| Area | GTK status | Notes |
| --- | --- | --- |
| Library and scan | Available | Local folders, scan progress, logs, grid/list, cloud/local switch, and sorting. |
| Bangumi | Available | Public discovery, detail fallback, search, OAuth, sync, and episode status. |
| Resources/downloads | Available | Nyaa filters, torrent file selection, qBittorrent controls, and confirmation dialogs. |
| Insights/settings | Available | Existing history/settings are preserved; settings apply and persist automatically. |
| Video playback | Not migrated | Electron/mpv remains a temporary fallback. |

## Requirements

For the official Linux frontend:

- Linux with GTK 4 and libadwaita development packages.
- `pkg-config`.
- Rust with edition 2024 support.

On Arch Linux:

```bash
sudo pacman -S gtk4 libadwaita pkgconf
```

Node.js/npm and the Electron dependencies are only needed for the temporary
Electron fallback and its existing Windows/macOS packaging path.

Optional service configuration includes Bangumi OAuth or access-token
credentials, dandanplay credentials retained for the existing playback path,
and qBittorrent Web UI access.

## Quick start

Run the native Linux frontend:

```bash
cargo run -- gtk
```

Or use the npm convenience command:

```bash
npm run dev:gtk
```

Open Settings, add one or more media-library directories, and start a scan
from the Library page. Settings apply and persist automatically.

The release binary and desktop entry are built/located with:

```bash
cargo build --release
# target/release/nexplay
# data/dev.nexplay.NexPlay.desktop
```

See [docs/GTK_FRONTEND.md](docs/GTK_FRONTEND.md) for installation, isolated
configuration, XDG paths, and the GTK playback boundary.

## Configuration

The repository includes [config.example.toml](config.example.toml). For GTK,
`NEXPLAY_CONFIG` has the highest priority. Without it, configuration is stored
under `$XDG_CONFIG_HOME/nexplay/config.toml` or `~/.config/nexplay/config.toml`.
When that file is absent or still has no media sources, an existing repository
`config.toml` is used as a compatibility fallback. An explicit path can also
be supplied with `cargo run -- gtk --config /path/to/config.toml`.
When creating a new GTK configuration, the default database is under
`$XDG_DATA_HOME/nexplay/nexplay.sqlite3` or `~/.local/share/nexplay/nexplay.sqlite3`.

Relative database and media paths in an explicitly located configuration are
resolved relative to that configuration file.

Existing configurations, databases, media paths, and watch states are read
without schema migration or path rewriting.

Important configuration sections are:

- `media_libraries`: directories to scan.
- `database.path`: SQLite database location.
- `bangumi`: API, OAuth, token, image-cache, and matching options.
- `dandanplay`: retained danmaku/playback credentials.
- `nyaa`: resource-search provider settings.
- `qbittorrent`: Web UI connection and download defaults.
- `experience` and `logging`: appearance, privacy, and backend log settings.

## Development commands

```bash
cargo fmt --check             # formatting gate
cargo check                   # compile the GTK and backend paths
cargo test                    # Rust tests
cargo build --release         # official Linux binary
cargo run -- gtk              # native GTK frontend
npm run dev:gtk               # native GTK convenience command
npm run generate:types        # regenerate legacy Electron API contracts
npm run test:backend-daemon   # legacy daemon protocol smoke test
```

The Electron fallback remains available through explicitly named commands when
needed for Windows/macOS or playback comparison:

```bash
npm run dev
npm run build
npm run package:electron
npm run dist:electron
```

`npm run package` and `npm run dist` now resolve to the native GTK release
build. Electron Builder is not part of the official Linux release path.

## Architecture

```text
GTK4/libadwaita application (`nexplay gtk`)
  -> bounded Rust worker pool
  -> shared AppContext
  -> SQLite, filesystem scan, Bangumi, Nyaa, qBittorrent

Electron/React fallback (temporary)
  -> preload IPC / backend daemon
  -> existing Rust services and playback/native bridge
```

Main directories:

- `src/gtk_frontend/`: GTK shell, native pages, worker runtime, image loader,
  and OAuth callback handling.
  - `bootstrap.rs` and `shell.rs`: application startup and navigation shell.
  - `pages/`: one module per route, with detail episodes, resources, and
    settings sections split into focused modules.
  - `components/`, `state.rs`, and `events.rs`: shared widgets, UI state, and
    backend-event refresh handling.
  - `player/` and `skeleton/`: playback/session helpers and loading-state UI.
- `src/`: shared Rust backend, domain model, repository, services, metadata
  providers, and JSON-RPC daemon.
- `frontend/` and `electron/`: retained temporary React/Electron reference and
  fallback.
- `native/mpv-render-bridge/`: retained playback bridge.
- `data/dev.nexplay.NexPlay.desktop`: Linux desktop launcher.
- `docs/GTK_FRONTEND.md`: native frontend build and runtime notes.
- `experiments/`: experimental work, intentionally outside this migration.

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.
For security-sensitive reports, use [SECURITY.md](SECURITY.md).

## License

NexPlay is licensed under the MIT License. See [LICENSE](LICENSE).
