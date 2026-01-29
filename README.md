# Wayclip

A clipboard history manager for Wayland (WLRoots compositors and Niri) with a daemon + GTK 4 client architecture.

## Features

- Stores clipboard history (text and images)
- Searchable history via GTK 4 client
- Full accessibility support for screen readers
- SQLite-based storage with automatic cleanup
- Keyboard-driven interface

## Requirements

### Runtime Dependencies

- **wl-clipboard** - Required for copying items back to clipboard
- A Wayland compositor supporting `wlr-data-control` protocol:
  - Sway
  - Hyprland
  - Niri
  - River
  - wayfire
  - and others

### Build Dependencies

- Rust 1.83+
- GTK 4.12+
- SQLite 3

## Installation

### Arch Linux

```bash
# Install runtime dependencies
sudo pacman -S wl-clipboard gtk4 rust

# Build from source
git clone https://github.com/destructatron/wayclip
cd wayclip
cargo build --release
```

### Manual Installation

```bash
# Copy binaries to your PATH
sudo cp target/release/wayclip-daemon /usr/local/bin/
sudo cp target/release/wayclip /usr/local/bin/
```

## Usage

### Starting the Daemon

The daemon monitors clipboard changes and stores them in a local database.

```bash
wayclip-daemon
```

Add to your compositor's autostart configuration to run at login:

**Sway** (`~/.config/sway/config`):
```
exec wayclip-daemon
```

**Hyprland** (`~/.config/hypr/hyprland.conf`):
```
exec-once = wayclip-daemon
```

**Niri** (`~/.config/niri/config.kdl`):
```
spawn-at-startup "wayclip-daemon"
```

### Opening the Clipboard History

```bash
wayclip
```

Bind this to a keyboard shortcut in your compositor:

**Sway**:
```
bindsym $mod+v exec wayclip
```

**Hyprland**:
```
bind = $mainMod, V, exec, wayclip
```

**Niri**:
```
Mod+V { spawn "wayclip"; }
```

### Keyboard Shortcuts (Client)

| Key | Action |
|-----|--------|
| Up/Down | Navigate list |
| Enter | Copy selected item to clipboard and close |
| Escape | Clear search / close window |
| Ctrl+F | Focus search |
| Tab | Move between search and list |

## File Locations

| File | Path |
|------|------|
| Socket | `$XDG_RUNTIME_DIR/wayclip/wayclip.sock` |
| Database | `$XDG_DATA_HOME/wayclip/history.db` |
| Config | `$XDG_CONFIG_HOME/wayclip/config.toml` |

## Configuration

Create `~/.config/wayclip/config.toml` (optional):

```toml
[daemon]
# Maximum number of entries to keep
max_entries = 1000

# Maximum size of a single entry in bytes (default: 10MB)
max_entry_size = 10485760

# Minimum size of an entry in bytes
min_entry_size = 1

# Auto-delete entries older than this many days (0 = disabled)
max_age_days = 30

[clipboard]
# MIME type patterns to ignore (not yet implemented)
ignore_mime_patterns = []

# Application patterns to ignore (not yet implemented)
ignore_app_patterns = []
```

## Accessibility

Wayclip is designed to be fully accessible to screen reader users:

- ListView uses `GTK_ACCESSIBLE_ROLE_LIST`
- List items use `GTK_ACCESSIBLE_ROLE_LIST_ITEM`
- Search entry uses `GTK_ACCESSIBLE_ROLE_SEARCH_BOX`
- All items have descriptive accessible labels
- Full keyboard navigation support

## Troubleshooting

### "Failed to spawn wl-copy: No such file or directory"

Install wl-clipboard:
```bash
# Arch Linux
sudo pacman -S wl-clipboard

# Fedora
sudo dnf install wl-clipboard

# Ubuntu/Debian
sudo apt install wl-clipboard
```

### "Compositor does not support wlr-data-control protocol"

Your compositor doesn't support the required protocol. Make sure you're using a compatible Wayland compositor (Sway, Hyprland, Niri, etc.). GNOME and KDE use different clipboard protocols.

### Client shows "Error: Failed to connect to daemon"

Make sure the daemon is running:
```bash
wayclip-daemon
```

## License

MIT
