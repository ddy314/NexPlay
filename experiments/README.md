# NexPlay native frontend experiments

This branch contains two deliberately small, non-functional frontends for a
technology choice review. They use the same fictional media snapshot so the
comparison is about interaction language and implementation ergonomics, not
feature completeness.

## Qt Quick 6 + RinUI

The Qt prototype is a QML application built with Qt 6 and the upstream RinUI
QML module. CMake fetches RinUI at commit `36ad74c888a39c86568b1b8b566103f216c3aff5`
when `RINUI_SOURCE_ROOT` is not supplied. The prototype uses RinUI's
`FluentWindow`, `NavigationView`, `FluentPage`, `Frame`, `Button`, `PillButton`,
`SettingCard`, `InfoBadge`, and `ProgressBar`; it does not define replacement
controls or a parallel theme.

```bash
cmake -S experiments/qt-quick-rinui -B /tmp/nexplay-qt-build -DCMAKE_BUILD_TYPE=Debug
cmake --build /tmp/nexplay-qt-build
/tmp/nexplay-qt-build/nexplay-qt
```

For an offline build, point CMake at an existing RinUI checkout:

```bash
cmake -S experiments/qt-quick-rinui -B /tmp/nexplay-qt-build \
  -DRINUI_SOURCE_ROOT=/path/to/Rin-UI
```

## GTK4 + libadwaita (Rust)

The GTK prototype is a separate Rust package using `gtk4` and `libadwaita`
from gtk-rs. It intentionally uses Adwaita's application window, navigation
split view, view switcher sidebar, toolbar view, banner, carousel, status page,
preferences groups, and action rows. No custom CSS, widget subclass, or
hand-built navigation control is used.

```bash
cargo run --manifest-path experiments/gtk-adwaita/Cargo.toml
```

The two prototypes are static visual probes. Buttons only exercise native
feedback, transitions, banners, carousel motion, and state presentation; they
do not call the existing Electron/Rust backend.
