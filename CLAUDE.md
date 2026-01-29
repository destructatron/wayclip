# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build all crates
cargo build --release

# Build specific crate
cargo build -p wayclip-daemon
cargo build -p wayclip-client
cargo build -p wayclip-common

# Run tests
cargo test

# Run tests for specific crate
cargo test -p wayclip-common

# Run daemon (requires Wayland compositor with wlr-data-control)
cargo run -p wayclip-daemon

# Run client (requires daemon running)
cargo run -p wayclip-client
```

## Architecture

Wayclip is a clipboard history manager for Wayland with a daemon + client architecture.

### Crates

- **wayclip-daemon** (`wayclip-daemon` binary): Background service that monitors clipboard via wlr-data-control Wayland protocol, stores history in SQLite, and serves IPC requests
- **wayclip-client** (`wayclip` binary): GTK 4 GUI that connects to daemon, displays searchable history, and copies selected items back to clipboard
- **wayclip-common**: Shared types including IPC protocol (Request/Response enums), ContentType, HistoryEntry, and XDG path helpers

### Communication Flow

1. Daemon monitors Wayland clipboard using `wayland-client` and `wayland-protocols-wlr`
2. Clipboard changes are stored in SQLite with SHA-256 deduplication
3. Client connects via Unix socket at `$XDG_RUNTIME_DIR/wayclip/wayclip.sock`
4. IPC uses newline-delimited JSON (see `protocol.rs` for Request/Response types)
5. When user selects an item, daemon uses `wl-copy` subprocess to set clipboard

### Key Implementation Details

- Daemon runs Wayland event loop in dedicated thread (blocking), tokio for IPC
- Client uses synchronous IPC (not async) because GTK uses GLib main loop
- Content data is base64-encoded in IPC responses
- GTK widgets use GObject subclassing with `#[derive(Properties)]` macro

## Runtime Dependencies

- `wl-clipboard` (wl-copy) for copying items back to clipboard
- Wayland compositor with wlr-data-control protocol (Sway, Hyprland, Niri, River, etc.)
- GTK 4.12+
