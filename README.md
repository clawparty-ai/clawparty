# ClawParty

Multi-agent orchestration platform with CLI, Web, and Desktop interfaces.

## Architecture

```
clawparty/
├── src/
│   ├── cli/       # Rust CLI core (ratatui + crossterm + tokio)
│   ├── tui/       # Node.js TUI (blessed) — symlinked to cli/ by build.sh
│   ├── web/       # Vue 3 + Vite + Tauri 2 (web / desktop / mobile)
│   └── desktop/   # Swift macOS menu bar app (SwiftUI + AppKit)
├── ztm/           # ZTM mesh networking (submodule)
├── zeroclaw/      # ZeroClaw agent runtime (submodule)
└── opencode/      # OpenCode AI dev tool (submodule)
```

## Components

### CLI (`src/cli/`)

Terminal-based interface for managing agents, real-time logs, and task orchestration.

| Layer | Tech |
|-------|------|
| Core | Rust (ratatui, crossterm, tokio) |
| Alt | Node.js TUI (blessed, blessed-contrib) — served via `src/tui/` symlink |
| DB | SQLite (rusqlite) |

```bash
cd src/cli
cargo build --release
cargo run
```

### Web (`src/web/`)

Browser and Tauri-based desktop/mobile app with chat, task panels, and agent configuration.

| Layer | Tech |
|-------|------|
| Frontend | Vue 3 + Vite |
| Desktop | Tauri 2 (Rust backend) |
| Mobile | Android / iOS |

```bash
cd src/web
yarn install
yarn dev            # Dev server on port 1420
yarn build          # Production build → src/cli/gui/ (embedded by CLI)
yarn tauri          # Tauri desktop app
```

### Desktop (`src/desktop/`)

Native macOS menu bar app for process management, real-time log viewing, and per-agent LLM configuration.

| Layer | Tech |
|-------|------|
| Language | Swift 5.9+ |
| UI | SwiftUI + AppKit |
| Build | Swift Package Manager |
| Platform | macOS 13.0+ |

```bash
cd src/desktop
./build.sh
open build/ClawPartyDesktop.app
```

## Prerequisites

- **Rust** 1.70+ (CLI, Tauri backend)
- **Node.js** 18+ (Web frontend)
- **Swift** 5.9+ (Desktop, macOS only)
- **Xcode** 15+ (Desktop, macOS only)

## Quick Start

```bash
# Build Web + CLI (default, order: Web → CLI embeds web output)
./build.sh

# Build with sub-modules
./build.sh --ztm --zeroclaw --opencode

# Build everything
./build.sh --desktop --ztm --zeroclaw --opencode
```

## License

MIT — see [LICENSE](LICENSE).

## Submodules

| Submodule | Repository | Description |
|-----------|-----------|-------------|
| `ztm/` | [flomesh-io/ztm](https://github.com/flomesh-io/ztm) | Mesh networking (C++/pipy) |
| `zeroclaw/` | [zeroclaw-labs/zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) | Agent runtime (Rust) |
| `opencode/` | [anomalyco/opencode](https://github.com/anomalyco/opencode) | AI dev tool (Bun/TypeScript) |
