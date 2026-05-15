# Tool Call Persistence Design

## Status

Implemented (commit `2c1b946`).

## Context

In group chat and individual agent sessions, LLM agents invoke external tools (shell, browser, HTTP, etc.) via `<tool_call>` tags. Previously, these tool invocations were only visible as transient WebSocket messages (`type: "tool_call"` / `type: "tool_result"`) consumed by the frontend console log. They were never persisted, making it impossible to:

- Audit which tools an agent invoked in a session
- Correlate a tool invocation with its result after the WebSocket closed
- Associate tool usage with tasks created from the same conversation turn
- Analyze tool usage patterns across sessions

## Goals

1. **Persist** every `ToolCall` and `ToolResult` event emitted by ZeroClaw
2. **Associate** tool calls with the conversation turn that triggered them (`turn_id`)
3. **Expose** a query API so the frontend (or other consumers) can read tool call history per session
4. **Do not break** non-SQLite session backends (JSONL, memory-only)

## Non-Goals

- Cross-session analytics / aggregation (can be added later)
- Real-time streaming of tool call history (history is query-only)
- UI rendering of tool call history in the chat panel (API is ready; UI may adopt it incrementally)

## Architecture

```
┌─────────────────┐     TurnEvent::ToolCall      ┌──────────────────────┐
│  Agent turn     │ ───────────────────────────> │  ws.rs (gateway)     │
│  (orchestrator) │                                │                      │
└─────────────────┘                                │  ┌──────────────┐    │
     │                                             │  │ SessionBackend│    │
     │ TurnEvent::ToolResult                       │  │  (trait)     │    │
     └────────────────────────────────────────────> │  └──────┬───────┘    │
                                                    │         │          │
                                                    │         ▼          │
                                                    │  ┌──────────────┐  │
                                                    │  │SqliteSession │  │
                                                    │  │   Backend    │  │
                                                    │  └──────┬───────┘  │
                                                    └─────────┼──────────┘
                                                              │
                                                              ▼
                                                    ┌─────────────────────┐
                                                    │   sessions.db         │
                                                    │   └── tool_calls      │
                                                    │        table          │
                                                    └─────────────────────┘
                                                              │
                                                              ▼
                                                    ┌─────────────────────┐
                                                    │ GET /api/sessions/  │
                                                    │ {id}/tool-calls     │
                                                    └─────────────────────┘
```

## Data Model

### `tool_calls` table (SQLite, in `sessions.db`)

| Column        | Type    | Description                                           |
|---------------|---------|-------------------------------------------------------|
| `id`          | INTEGER | Primary key, auto-increment                           |
| `session_key` | TEXT    | ZeroClaw session key (e.g. `gw_abc123`)               |
| `turn_id`     | TEXT    | UUID identifying one user-message → agent-response turn |
| `message_id`  | INTEGER | FK to `sessions.id` (nullable, reserved for future)   |
| `tool_name`   | TEXT    | Name of the invoked tool                              |
| `tool_args`   | TEXT    | JSON string of arguments                              |
| `tool_output` | TEXT    | JSON string of result (populated on completion)         |
| `status`      | TEXT    | `called` → `completed` / `error`                      |
| `called_at`   | TEXT    | ISO-8601 timestamp of invocation                      |
| `completed_at`| TEXT    | ISO-8601 timestamp of result (nullable)                 |
| `duration_ms` | INTEGER | Execution duration (nullable)                         |
| `error_msg`   | TEXT    | Error description if status = `error`                 |

### Indexes

- `idx_tool_calls_session` on `(session_key, called_at)` — for session-level listing
- `idx_tool_calls_turn` on `(turn_id)` — for turn-level correlation
- `idx_tool_calls_msg` on `(message_id)` — reserved for future FK join

### Why `turn_id` is critical

A single WebSocket session can contain multiple independent conversation turns:

```
Turn A (turn_id = "uuid-a")
  user: "What's the weather in Beijing?"
  agent: chunk... → tool_call(weather) → tool_result → chunk... → done

Turn B (turn_id = "uuid-b")
  user: "Plan a trip for me"
  agent: chunk... → tool_call(flights) → tool_result
         → tool_call(hotel) → tool_result → chunk... → done
         → <task>trip-plan-001</task>  (task detected)
```

If we only matched by `(session_key, tool_name, time)`, the `weather` tool from Turn A could be incorrectly associated with a task created in Turn B. `turn_id` scopes tool calls to the exact turn they belong to.

## Trait Extension

`SessionBackend` (in `zeroclaw-infra`) receives three new **default no-op** methods so that JSONL and other backends are not forced to implement them:

```rust
pub trait SessionBackend: Send + Sync {
    // ... existing methods ...

    fn record_tool_call(
        &self,
        session_key: &str,
        turn_id: Option<&str>,
        message_id: Option<i64>,
        tool_name: &str,
        tool_args: &str,
    ) -> std::io::Result<i64> { Ok(0) }

    fn record_tool_result(
        &self,
        session_key: &str,
        turn_id: Option<&str>,
        tool_name: &str,
        tool_output: &str,
        duration_ms: Option<i64>,
    ) -> std::io::Result<()> { Ok(()) }

    fn get_tool_calls(
        &self,
        session_key: &str,
        limit: usize,
    ) -> std::io::Result<Vec<ToolCallRecord>> { Ok(Vec::new()) }
}
```

Only `SqliteSessionBackend` provides real implementations.

## Recording Flow

### 1. Turn start (`ws.rs`)

When the gateway receives a user message over WebSocket, it starts a new **turn**:

```rust
let turn_id = uuid::Uuid::new_v4().to_string();
backend.set_session_state(session_key, "running", Some(&turn_id));
```

### 2. Tool call (`ws.rs`)

Inside `forward_fut` (the event-forwarding async block):

```rust
TurnEvent::ToolCall { name, args } => {
    if let Some(ref backend) = state.session_backend {
        let _ = backend.record_tool_call(
            session_key,
            Some(turn_id_clone.as_str()),  // scoped to this turn
            None,                            // message_id reserved
            name,
            &args.to_string(),
        );
    }
    // ... forward to WebSocket client as before ...
}
```

### 3. Tool result (`ws.rs`)

```rust
TurnEvent::ToolResult { name, output } => {
    if let Some(ref backend) = state.session_backend {
        let _ = backend.record_tool_result(
            session_key,
            Some(turn_id_clone.as_str()),
            name,
            output,
            None,
        );
    }
    // ... forward to WebSocket client as before ...
}
```

### 4. Matching `tool_call` → `tool_result`

`record_tool_result` uses a scoped UPDATE:

```sql
UPDATE tool_calls
SET tool_output = ?, status = 'completed', completed_at = ?, duration_ms = ?
WHERE session_key = ? AND tool_name = ? AND status = 'called'
  AND (turn_id = ? OR (? IS NULL AND turn_id IS NULL))
ORDER BY called_at DESC LIMIT 1
```

This guarantees that even if the same tool is invoked multiple times in one turn, each `called` record is paired with the nearest subsequent `result` within the same `turn_id`.

## API

### `GET /api/sessions/{id}/tool-calls`

Requires bearer token auth (same as other gateway API endpoints).

**Request**: none

**Response** (200):

```json
{
  "session_id": "abc123",
  "session_persistence": true,
  "tool_calls": [
    {
      "id": 1,
      "tool_name": "shell",
      "status": "completed",
      "called_at": "2026-05-15T09:12:34Z",
      "turn_id": "uuid-b",
      "tool_args": "{\"command\":\"curl wttr.in/Beijing\"}",
      "tool_output": "{\"stdout\":\"Sunny 28°C\"}",
      "completed_at": "2026-05-15T09:12:35Z",
      "duration_ms": 890
    }
  ]
}
```

If session persistence is disabled:

```json
{
  "session_id": "abc123",
  "session_persistence": false,
  "tool_calls": []
}
```

## Frontend Integration

### Service (`chatService.js`)

```javascript
export const toolCallService = {
  getSessionToolCalls(sessionId) {
    return api.get(`/sessions/${encodeURIComponent(sessionId)}/tool-calls`)
  }
}
```

### Usage pattern

When a task is detected from a conversation (e.g. via task-refresh), the frontend can load the tool calls for that session and display them alongside task details:

```javascript
const tcRes = await toolCallService.getSessionToolCalls(sessionId)
const toolCalls = tcRes.data?.tool_calls || []
```

## Frontend Renaming (Collateral)

Alongside this feature, console log prefixes were unified for consistency:

| Before          | After        | File                     |
|-----------------|--------------|--------------------------|
| `[ZeroClaw]`    | `[zAgent]`   | `chat-gui/src/App.vue`   |
| `[ZeroClawWS]`  | `[zAgentWS]` | `chat-gui/src/services/chatService.js` |

## Key Design Decisions

### 1. Store inside ZeroClaw, not in ZTM Agent

ZeroClaw already owns the `sessions.db` SQLite database. Persisting tool calls there avoids cross-process synchronization and keeps the data close to the source of truth. ZTM Agent remains a proxy/pass-through for WebSocket events.

### 2. `turn_id` instead of `message_id` for primary association

`message_id` would require knowing the exact `sessions.id` row ID at the moment the tool call happens, which is awkward because:
- The user's message is already persisted before the turn starts
- But the assistant's response (which contains the `<tool_call>`) is only persisted **after** the turn completes
- Therefore `message_id` is reserved for future backfill if needed

`turn_id` is generated at turn start and flows through the entire `forward_fut` closure, making it naturally available for every event.

### 3. Default no-op trait methods

`SessionBackend` is a public trait with multiple implementations (SQLite, JSONL, in-memory test backends). Adding required methods would be a breaking change for all downstream crates. Default no-ops allow incremental adoption.

### 4. Fire-and-forget writes

Both `record_tool_call` and `record_tool_result` are called with `let _ = ...`. Errors are silently swallowed. This is intentional: tool persistence is an observability feature; a DB write failure should not abort the agent turn or crash the WebSocket.

## Future Extensions

1. **message_id backfill**: After the assistant response is appended to `sessions`, update `tool_calls.message_id` to point to that row for precise "which message triggered this tool" queries.

2. **Error status**: Currently only `called` → `completed`. If a tool throws, the `ToolResult` event could carry an error flag and we would update `status = 'error'` with `error_msg`.

3. **Task association**: When the frontend (or task-refresh logic) detects `<task>task-id</task>` in a completed turn, it can call a future `POST /api/tool-calls/backfill` endpoint to set `task_id` on all `tool_calls` rows matching that `turn_id`.

4. **Cross-session analytics**: Add aggregation queries (e.g. "most frequently used tools per agent") for admin dashboards.

## Migration

ZeroClaw's `SqliteSessionBackend::new()` already runs idempotent `CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS` statements. Existing databases are automatically migrated on the next ZeroClaw restart; no manual migration script is required.

## Files Changed

| File | Role |
|------|------|
| `zeroclaw-infra/src/session_backend.rs` | `ToolCallRecord` struct + trait default methods |
| `zeroclaw-infra/src/session_sqlite.rs` | `tool_calls` schema + real implementations |
| `zeroclaw-gateway/src/ws.rs` | Insert `record_tool_call/result` into event forwarder |
| `zeroclaw-gateway/src/api.rs` | `GET /api/sessions/{id}/tool-calls` handler |
| `zeroclaw-gateway/src/lib.rs` | Route registration |
| `chat-gui/src/services/chatService.js` | `toolCallService.getSessionToolCalls()` |
| `chat-gui/src/App.vue` | Log prefix rename `[ZeroClaw]` → `[zAgent]` |
