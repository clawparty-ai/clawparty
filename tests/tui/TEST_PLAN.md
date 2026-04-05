# ClawParty TUI Test Plan

## Overview

This document describes the test plan for the ClawParty Terminal User Interface (TUI).

## Test Environment

### Prerequisites
- Rust toolchain (for building `clawparty` binary)
- Node.js >= 16 (for running the agent)
- A running ClawParty agent on `http://localhost:6789`
- API token (default: `enjoy-party`)

### Test Setup
```bash
# Start the agent
./bin/ztm run agent --listen :6789 --data ~/.clawparty --api-token enjoy-party

# Run TUI tests
./tests/tui/run-tests.sh
```

## Test Categories

### 1. Startup Tests (`tests/tui/test_startup.sh`)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| `test_tui_starts_with_agent` | Start TUI when agent is already running | TUI launches, shows sidebar with local agents |
| `test_tui_starts_agent` | Start TUI when agent is not running | TUI starts agent automatically, then shows UI |
| `test_tui_fails_without_tty` | Run TUI with piped input | Error: "TUI requires a terminal (TTY)" |
| `test_tui_custom_api_host` | Start TUI with custom `--api-host` | TUI connects to specified host |
| `test_tui_custom_token` | Start TUI with custom `--token` | TUI authenticates with specified token |

### 2. Sidebar Tests (`tests/tui/test_sidebar.sh`)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| `test_local_agents_displayed` | Check sidebar shows local agents | Local agents listed with display names and emojis |
| `test_group_chats_displayed` | Check sidebar shows group chats | Group chats listed with `#` prefix |
| `test_remote_agents_without_mesh` | Check sidebar without mesh | Only local agents and group chats shown |
| `test_remote_agents_with_mesh` | Check sidebar with mesh joined | Remote agents (endpoints) shown below local agents |
| `test_first_item_auto_selected` | Start TUI and check selection | First sidebar item is highlighted |

### 3. Message Tests (`tests/tui/test_messages.sh`)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| `test_send_peer_message` | Send message to a peer chat | Message sent, "Message sent" logged |
| `test_send_group_message` | Send message to a group chat | Message sent, "Message sent" logged |
| `test_send_agent_message` | Send message to an OpenClaw agent | Message sent to agent |
| `test_send_fails_no_selection` | Send message without selecting chat | Error: "No chat or agent selected" |
| `test_exit_command` | Enter `#exit` in input | TUI exits gracefully |
| `test_message_display` | Check messages are displayed | Messages shown with timestamp and sender |
| `test_message_polling` | Wait for polling to fetch messages | New messages appear automatically |

### 4. Navigation Tests (`tests/tui/test_navigation.sh`)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| `test_up_down_navigation` | Press Up/Down arrows | Selection moves in sidebar |
| `test_tab_switches_panel` | Press Tab key | Focus cycles: Sidebar → Input → Sidebar |
| `test_enter_selects_chat` | Press Enter on sidebar item | Chat selected, messages loaded |
| `test_left_right_arrows` | Press Left/Right arrows | Panel switches correctly |
| `test_backspace_in_input` | Type then Backspace in input | Last character deleted |
| `test_quit_with_q` | Press `q` | TUI exits gracefully |
| `test_quit_with_ctrl_c` | Press Ctrl+C | TUI exits gracefully |

### 5. Log Tests (`tests/tui/test_logs.sh`)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| `test_log_file_created` | Start TUI and check log file | `console-log-<timestamp>.log` created |
| `test_log_content` | Check log file content | Contains INFO/ERROR/AGENT entries |
| `test_log_panel_displayed` | Check TUI log panel | Logs displayed in bottom panel |
| `test_agent_output_captured` | Check agent output in logs | Agent stdout/stderr captured in log panel |

### 6. API Tests (`tests/tui/test_api.sh`)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| `test_health_check` | Check `/ok` endpoint | Returns 200 OK |
| `test_get_meshes` | Check `/api/meshes` endpoint | Returns mesh list |
| `test_get_openclaw_agents` | Check `/api/openclaw/agents` endpoint | Returns agent list with identityName |
| `test_get_chats` | Check chat API endpoint | Returns chat list |
| `test_get_endpoints` | Check endpoints API | Returns endpoint list |

### 7. Build Tests (`tests/tui/test_build.sh`)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| `test_cargo_build` | Run `cargo build --release` | Build succeeds with no errors |
| `test_cargo_warnings` | Check for compiler warnings | Zero warnings |
| `test_binary_exists` | Check `bin/clawparty` exists | Binary file exists and is executable |
| `test_binary_signed` | Check macOS code signature | Binary is properly signed |
| `test_incremental_build` | Run build again without changes | Build uses cache, skips recompilation |
| `test_clean_build` | Run `./build.sh --clean` | Full clean rebuild succeeds |

## Running Tests

### Run All Tests
```bash
cd tests/tui
./run-tests.sh
```

### Run Individual Test File
```bash
cd tests/tui
./test_messages.sh
```

### Run Single Test
```bash
cd tests/tui
./test_messages.sh test_send_peer_message
```

## Test Utilities

### Helper Functions

```bash
# Start agent in background
start_agent() {
    ./bin/ztm run agent --listen :6789 --data ~/.clawparty --api-token enjoy-party &
    AGENT_PID=$!
    sleep 3
}

# Stop agent
stop_agent() {
    kill $AGENT_PID 2>/dev/null
    wait $AGENT_PID 2>/dev/null
}

# Check if TUI is running
check_tui_running() {
    ps -p $TUI_PID > /dev/null 2>&1
}

# Get latest log file
get_latest_log() {
    ls -t console-log-*.log 2>/dev/null | head -1
}
```

## Known Issues

- TUI requires a real TTY; cannot be tested in CI without a pseudo-terminal
- Some tests require a pre-configured mesh with endpoints
