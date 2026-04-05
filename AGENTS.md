# AGENTS.md - ClawParty Agent Guide

Guidance for coding agents working in `/Users/caishu/github/clawparty`.

## Scope

ClawParty is a mixed codebase:
- Runtime + CLI: Pipy JavaScript in `cli/`, `agent/`, `hub/`, `ca/`
- Web UI: Vue 3 + Vite in `chat-gui/`
- Native build plumbing: shell scripts in `build/`

When making changes, preserve existing patterns by area (Pipy-side style differs from Vue-side style).

## Commands: Build, Lint, Test

### Prerequisites
- Node.js >= 16 for GUI build (`build/gui.sh` enforces this)
- CMake + clang toolchain for `build/pipy.sh`
- npm (used in `chat-gui/`)

### Install dependencies
```bash
# For GUI development (Vue 3 + Vite)
cd chat-gui && npm install

# For full build (includes Pipy runtime)
./build/deps.sh
```

### Build

GUI only (outputs to `agent/gui/`):
```bash
cd chat-gui && npm run build
```

Run GUI dev server:
```bash
cd chat-gui && npm run dev
```

Build Pipy binary (`bin/ztm`):
```bash
./build/pipy.sh
```

Convenience scripts at repo root:
```bash
./build-cli-only.sh  # CLI + Agent only
./build.sh           # Full build (GUI + Pipy)
```

Note: `build.sh` references a `gui/` directory in this checkout; for day-to-day local work, use `chat-gui` commands above.

Run app after build:
```bash
./bin/ztm
```

### Lint

There is currently no configured linter (no ESLint/Prettier scripts in `chat-gui/package.json`, no root lint config).

If you add linting in a PR, document exact commands in this file.

### Tests

ClawParty uses Shell scripts for API and CLI tests.

#### Test Environment Setup

```bash
cd tests
./prepare.sh
```

This will start test services:
- Hub: http://localhost:18888
- Agent1: http://localhost:7778
- Agent2: http://localhost:7779

#### Run Tests

```bash
# Run all tests
./run-tests.sh

# Run individually
./api/run-tests.sh      # API tests
./cli/run-tests.sh      # CLI tests
```

#### Clean Up

```bash
./cleanup.sh
```

#### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| HUB_URL | http://localhost:18888 | Hub service address |
| AGENT_URL | http://localhost:7778 | Agent service address |
| API_TOKEN | enjoy-party | API authentication token |
| ZTM_CONFIG | 127.0.0.1:7778 | CLI connected Agent address |

#### Test Directory Structure

```
tests/
├── prepare.sh           # Initialize test environment
├── cleanup.sh           # Clean up test environment
├── run-tests.sh         # Run all tests
├── api/                 # API tests
│   ├── hub/             # Hub API tests
│   └── agent/           # Agent API tests
└── cli/                 # CLI tests
```

### Verification

After making changes, verify the build succeeds:
```bash
cd chat-gui && npm run build && echo "Build successful"
```

## Repository Map

- CLI entrypoint: `cli/main.js`
- Agent entrypoint: `agent/main.js`
- GUI entrypoint: `chat-gui/src/App.vue`
- GUI services: `chat-gui/src/services/`
- Build scripts: `build/*.sh`
- Version metadata: `version.env`, `*/version.json`

## Code Style and Conventions

### Language and modules
- Use JavaScript ES modules (`import` / `export`), not CommonJS.
- Keep file-local style consistent with surrounding code.
- Do not introduce TypeScript unless explicitly requested.

### Imports
- Keep imports at top of file.
- Group external dependencies before local imports when practical.
- Use explicit relative paths with extensions where project already does so (e.g. `./db.js`).

### Naming
- Variables/functions: `camelCase`
- Constants: `UPPER_SNAKE_CASE`
- Vue SFC component filenames: `PascalCase.vue`
- Keep API/service method names descriptive (`getChats`, `sendGroupMessage`).

### Formatting
- Follow existing file formatting rather than reformatting unrelated lines.
- Semicolon usage is mixed; match the file you edit.
- Prefer short, readable functions and early returns.

### Types and data handling
- No static typing in repo; use runtime guards.
- Validate nullable/optional values before access.
- Parse external data defensively (`Array.isArray`, `typeof`, `try/catch` around JSON parsing).

### Error handling
- Wrap async I/O in `try/catch`.
- Use descriptive errors for CLI/runtime failures.
- For UI/service failures, use `console.error(...)` with useful context.
- Preserve user-facing language patterns already used in the module (many GUI errors are Chinese).

### Vue 3 conventions (`chat-gui/`)
- Use `<script setup>` composition API.
- Prefer `ref` / `computed`; keep state local unless shared via `provide/inject`.
- Keep API calls in service modules under `chat-gui/src/services/`.
- Avoid large template logic; move logic to script helpers.

### Pipy runtime conventions (`cli/`, `agent/`, `hub/`, `ca/`)
- Existing code often uses `var` and function declarations; preserve local style in touched files.
- Use Pipy APIs directly where appropriate (`pipy.argv`, `pipy.exit`, `pipy.load`).
- Use `JSON.decode` / `JSON.encode` in Pipy-side code.
- **No regex**: PipyJS does not support `RegExp` APIs (`exec`, `match`, `test`, `replace(regex)`, `new RegExp`). Use string operations instead (`split`, `indexOf`, `charAt`, `substring`, `startsWith`, `endsWith`, `includes`).
- **No locale methods**: PipyJS does not support `toLocaleTimeString()`, `toLocaleDateString()`, or similar locale-dependent Date methods. Format dates manually using `getHours()`, `getMinutes()`, etc. Example: `d.getHours().toString().padStart(2, '0') + ':' + d.getMinutes().toString().padStart(2, '0')`
- **No continue/break**: PipyJS does not support `continue` or `break` statements in loops. Use `if` blocks to wrap logic instead of `if (!condition) continue`.

### Comments
- Keep comments minimal.
- Add comments only for non-obvious behavior or protocol constraints.

## Agent Workflow Expectations

- Read nearby code before editing; mirror local patterns.
- Make focused changes; avoid unrelated refactors.
- Do not invent commands/scripts that do not exist.
- When no tests exist, state that clearly and run the most relevant build/check command instead.

## Cursor/Copilot Rules Check

Checked for additional agent rules in:
- `.cursor/rules/`
- `.cursorrules`
- `.github/copilot-instructions.md`

Result: No Cursor or Copilot rules files exist in this repository.

If these files are added later, merge their guidance into this document and treat them as higher-priority agent instructions.
