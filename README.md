# devwatch

A cross-platform GitHub PR / VCS event monitor, written in Rust.

```
┌──────────────────────────────────────────────────────────────────┐
│  config.toml                                                     │
│  (repos + tokens)                                                │
└──────────────┬───────────────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────┐
│           daemon                 │
│  ┌──────────┐  ┌───────────────┐ │
│  │  poller  │→ │  DaemonState  │ │   notify-rust (system tray)
│  │(octocrab)│  │  (in-memory)  │ │ ──────────────────────────▶ 🔔
│  └──────────┘  └───────┬───────┘ │
│                        │ SQLite  │
│                  state.db (disk) │
│                                  │
│  ┌─────────────────────────────┐ │
│  │  JSON / TCP server          │ │ 127.0.0.1:7878
│  │  (newline-delimited JSON)   │◀├──────────────────── clients
│  └─────────────────────────────┘ │
└──────────────────────────────────┘
        ▲               ▲
        │               │
  crates/tui       crates/tauri-app
  (Phase 2)          (Phase 3)
```

## Prerequisites

- Rust stable (`rustup update stable`)
- A GitHub [Personal Access Token](https://github.com/settings/tokens) with the `repo` scope

## Setup

1. Copy and edit the config:
   ```sh
   cp examples/config.example.toml config.toml
   $EDITOR config.toml
   ```

2. Fill in your GitHub token and repos:
   ```toml
   [[repos]]
   provider = "github"
   name     = "myorg/myrepo"
   token    = "ghp_..."
   ```

   Alternatively, export the token and omit it from the config:
   ```sh
   export GITHUB_TOKEN=ghp_...
   ```

## Build

```sh
# Build just the daemon (fast):
cargo build -p daemon

# Build everything:
cargo build --workspace
```

## Run

### Daemon only

```sh
cargo run -p daemon
```

With verbose logging:
```sh
RUST_LOG=debug cargo run -p daemon
```

### TUI (terminal UI)

```sh
cargo run -p tui
```

The TUI will **auto-start the daemon** if it is not already running — no need
to launch two terminals manually. It locates the `daemon` binary as a sibling
of the `devwatch-tui` executable, or falls back to `daemon` on `PATH`.

Debug logging (written to a file so it doesn't corrupt the terminal):
```sh
DEVWATCH_TUI_LOG=/tmp/tui.log cargo run -p tui
```

## IPC — interacting with the daemon manually

The daemon listens on `127.0.0.1:7878` (configurable via `daemon_port`).
Messages are newline-delimited JSON.

```sh
# Open a connection:
nc 127.0.0.1 7878

# Ping/pong:
{"Ping":null}

# Get current PR snapshot:
{"GetState":null}

# Subscribe — receive StateSnapshot, then live events:
{"Subscribe":null}
```

### Message types

**Client → Daemon**

| Message       | Effect                                              |
|---------------|-----------------------------------------------------|
| `{"Ping":null}` | Responds with `{"Pong":null}`                     |
| `{"GetState":null}` | Responds with `{"StateSnapshot": {...}}`      |
| `{"Subscribe":null}` | StateSnapshot, then streams `{"Event": ...}` |

**Daemon → Client**

| Message | Description |
|---------|-------------|
| `{"Pong":null}` | Liveness response |
| `{"StateSnapshot":{"pull_requests":[...]}}` | Current PR list |
| `{"Event":{"NewPullRequest":{...}}}` | New open PR detected |
| `{"Event":{"PullRequestUpdated":{"old":{...},"new":{...}}}}` | PR changed |
| `{"Event":{"PullRequestClosed":{...}}}` | PR closed or merged |
| `{"Error":{"message":"..."}}` | Error message |

## State persistence

The daemon stores PR state in SQLite at:

- **macOS / Linux:** `~/.local/share/devwatch/state.db`
- **Windows:** `%LOCALAPPDATA%\devwatch\state.db`

This prevents duplicate notifications across daemon restarts.

## Project layout

```
crates/
├── core/                # Shared types, VcsProvider trait, IPC messages
├── daemon/              # Background polling service (Phase 1 — complete)
├── providers/
│   ├── github/          # octocrab-based VcsProvider
│   └── gitlab/          # stub (Phase N)
├── tui/                 # ratatui terminal UI (Phase 2 — stub)
└── tauri-app/           # Tauri GUI (Phase 3 — stub)
```

## Configuration reference

| Key | Default | Description |
|-----|---------|-------------|
| `daemon_port` | `7878` | TCP port for the IPC server |
| `poll_interval_secs` | `60` | How often to poll each repo |
| `repos[].provider` | — | `"github"` or `"gitlab"` |
| `repos[].name` | — | `"owner/repo"` |
| `repos[].token` | — | Per-repo PAT (falls back to `GITHUB_TOKEN`) |

Environment variables are also supported, prefixed with `DEVWATCH__`
(double-underscore separator), e.g. `DEVWATCH__DAEMON_PORT=9000`.
