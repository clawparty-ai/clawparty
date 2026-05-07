# Chat Filter Chain Design

## Overview

The chat auto-reply pipeline is split into two hook points, each backed by a configurable filter chain:

- **`onReceive`** — runs when an incoming message arrives, before the AI agent is called. Filters evaluate the message and adjust the peer's *credit* score.
- **`onSend`** — runs after the AI agent produces a reply, before the reply is delivered. Filters can delay, transform, or suppress the outgoing message.

Each hook executes a sequence of independent filter scripts loaded from `agent/filters/`. The active chain for each peer is stored in the database, allowing per-peer customisation without code changes.

---

## Overall Flow

```
Incoming message
      │
      ▼
  onReceive(peer, sender, text)
      │  runs filter chain: repeat-message, blocked-keywords, ...
      │  accumulates credit penalty
      │  writes updated credit to DB
      │  returns current credit value
      │
      ▼
  spawnOpenclaw(cmd)          ← calls openclaw agent CLI
      │
      ▼
  parse reply text
      │
      ▼
  onSend(peer, replyText, credit)
      │  runs filter chain: credit-delay, suppress-json, ...
      │  each filter may delay, abort, or pass through
      │  returns Promise<boolean>  (true = send, false = abort)
      │
      ▼
  addPeerMessage(peer, replyText)   ← delivered only if onSend resolves true
```

---

## Credit System

Every peer has a **credit** value stored in the `chat_peer` table. The default value is `100`.

- `onReceive` filters return a numeric score. A negative score reduces the peer's credit.
- Credit is adjusted atomically in the database via `db.adjustCredit(mesh, peer, delta)`.
- The updated credit is passed into `onSend` so send-side filters (e.g. `credit-delay`) can react to it.
- Credit accumulates across messages: a peer that repeatedly violates rules will have progressively lower credit and receive slower replies.

---

## `onReceive` Filter Chain

### Context object (`ctx`)

| Field    | Type   | Description                              |
|----------|--------|------------------------------------------|
| `mesh`   | string | Current mesh name                        |
| `peer`   | string | Peer username (sender of the message)    |
| `sender` | string | Same as `peer` for direct messages       |
| `text`   | string | Plain-text content of the received message |
| `db`     | object | Database handle (full access)            |

### Filter contract

An `onReceive` filter is a synchronous function:

```js
export default function (ctx, config) {
  // inspect ctx.text, ctx.peer, etc.
  // return a negative number to penalise, 0 or positive to pass
  return 0
}
```

| Return value   | Meaning                     |
|----------------|-----------------------------|
| negative number | Credit penalty (e.g. `-1`) |
| `0` or positive | No penalty, continue       |

All filters in the chain always run. The total penalty is summed and applied to the peer's credit once at the end.

### Default chain

```
repeat-message, blocked-keywords
```

Configured in `api.js`:
```js
var DEFAULT_FILTER_CHAIN = 'repeat-message,blocked-keywords'
```

### Per-peer override

The `filter_chain` column in `chat_peer` overrides the default for a specific peer. It is a comma-separated list of filter names:

```
repeat-message,blocked-keywords,my-custom-filter
```

An empty string falls back to `DEFAULT_FILTER_CHAIN`.

---

## `onSend` Filter Chain

### Context object (`ctx`)

| Field       | Type   | Description                                    |
|-------------|--------|------------------------------------------------|
| `mesh`      | string | Current mesh name                              |
| `peer`      | string | Peer to whom the reply will be sent            |
| `replyText` | string | The reply text produced by the AI agent        |
| `credit`    | number | Peer's current credit after `onReceive`        |
| `db`        | object | Database handle                                |

### Filter contract

An `onSend` filter may be synchronous or asynchronous:

```js
export default function (ctx, config) {
  // return false to abort, null/undefined to continue, or a Promise
}
```

| Return value         | Meaning                                           |
|----------------------|---------------------------------------------------|
| `false`              | Abort: do not send the reply                      |
| `null` / `undefined` | Continue to the next filter                       |
| `Promise`            | Await resolution; resolved value follows the same rules |

Filters run sequentially. If any filter returns `false`, the chain stops immediately and the reply is not sent.

### Execution model

```js
function runNext(i) {
  if (i >= filterNames.length) return Promise.resolve(true)  // all passed
  var result = fn(ctx)
  if (result === false) return Promise.resolve(false)         // abort
  if (result?.then) return result.then(val =>
    val === false ? Promise.resolve(false) : runNext(i + 1)
  )
  return runNext(i + 1)
}
```

### Default chain

```
credit-delay, suppress-json
```

Configured in `api.js`:
```js
var DEFAULT_SEND_FILTER_CHAIN = 'credit-delay,suppress-json'
```

### Per-peer override

The `send_filter_chain` column in `chat_peer` overrides the default. Same format as `filter_chain`.

---

## Built-in Filters

### `repeat-message` (onReceive)

**File:** `agent/filters/repeat-message.js`

Queries `chat_log` to count how many times the same peer has sent identical content within a time window. If the count meets or exceeds the threshold, a credit penalty is applied.

| Config option  | Default | Description                                   |
|----------------|---------|-----------------------------------------------|
| `withinSeconds`| `60`    | Time window to look back in seconds           |
| `maxCount`     | `3`     | Maximum allowed repetitions before penalising |
| `penalty`      | `1`     | Credit points to deduct                       |

**Example:** if `white-buffalo` sends `"hello"` 3 or more times within 60 seconds, credit is reduced by 1.

---

### `blocked-keywords` (onReceive)

**File:** `agent/filters/blocked-keywords.js`

Loads the keyword list from the `blocked_keywords` table and performs a case-insensitive substring search. Each matched keyword deducts one penalty unit.

| Config option | Default | Description                              |
|---------------|---------|------------------------------------------|
| `penalty`     | `1`     | Credit points deducted per matched keyword |

Keywords are managed via:
```js
db.addBlockedKeyword(mesh, 'openclaw.json')
db.delBlockedKeyword(mesh, 'openclaw.json')
db.getBlockedKeywords(mesh)
```

---

### `credit-delay` (onSend)

**File:** `agent/filters/credit-delay.js`

Inserts a delay before the reply is sent. The delay grows as credit decreases:

```
delay = max(0, baseDelayMs + (baseCredit - credit) × msPerPoint)
```

| Config option  | Default  | Description                                         |
|----------------|----------|-----------------------------------------------------|
| `baseCredit`   | `100`    | Credit level that corresponds to `baseDelayMs`      |
| `baseDelayMs`  | `3000`   | Base delay in milliseconds (applied at full credit) |
| `msPerPoint`   | `3000`   | Extra milliseconds added per missing credit point   |

**Examples:**

| Credit | Delay    |
|--------|----------|
| 100    | 3 s      |
| 99     | 6 s      |
| 98     | 9 s      |
| 90     | 33 s     |
| 70     | 93 s     |

---

### `suppress-json` (onSend)

**File:** `agent/filters/suppress-json.js`

Attempts to parse `ctx.replyText` as JSON. If it succeeds (i.e. the entire reply is a raw JSON value), the filter returns `false` and the reply is suppressed. This prevents agent error payloads or structured data objects from being forwarded as chat messages.

---

## Database Schema

### `chat_peer` table (relevant columns)

| Column            | Type    | Default | Description                                    |
|-------------------|---------|---------|------------------------------------------------|
| `credit`          | INTEGER | `100`   | Current credit for this peer                   |
| `filter_chain`    | TEXT    | `''`    | Comma-separated `onReceive` filter names; empty = use default |
| `send_filter_chain` | TEXT  | `''`    | Comma-separated `onSend` filter names; empty = use default    |

### `blocked_keywords` table

| Column    | Type    | Description                         |
|-----------|---------|-------------------------------------|
| `id`      | INTEGER | Auto-increment primary key          |
| `mesh`    | TEXT    | Mesh name                           |
| `keyword` | TEXT    | Keyword string (case-insensitive match) |

---

## Writing a Custom Filter

Place a `.js` file in `agent/filters/` and export a default function.

### `onReceive` example

```js
// agent/filters/my-receive-filter.js
export default function (ctx, config) {
  // ctx: { mesh, peer, sender, text, db }
  if (ctx.text.length > 500) {
    console.info('[filter my-receive-filter] message too long, penalty -1')
    return -1
  }
  return 0
}
```

### `onSend` example

```js
// agent/filters/my-send-filter.js
export default function (ctx, config) {
  // ctx: { mesh, peer, replyText, credit, db }
  if (ctx.replyText.includes('ERROR')) {
    console.info('[send-filter my-send-filter] suppressing error reply')
    return false   // abort
  }
  return null      // continue
}
```

Then add the filter name to the peer's chain in the database, or update `DEFAULT_FILTER_CHAIN` / `DEFAULT_SEND_FILTER_CHAIN` in `agent/apps/ztm/chat/api.js` to apply it globally.

---

## Logging

Every filter call produces a `console.info` log line with the filter name, peer, and result score or action. This makes the chain execution traceable in the agent log.

`triggerAutoReply` additionally logs:
- `[chat auto-reply] calling openclaw: <cmd>` — before the CLI call
- `[chat auto-reply] openclaw output: <first 120 chars> ...` — abbreviated in the console
- Full openclaw output is written to the `api_log` table via `db.logApi` for forensic inspection

```
[chat filter] repeat-message score: 0 for peer: white-buffalo
[chat filter] blocked-keywords score: -1 for peer: white-buffalo
[chat filter] adjusting credit for white-buffalo by -1
[chat auto-reply] credit after onReceive: 99
[chat auto-reply] calling openclaw: openclaw agent --agent photo-agent ...
[chat auto-reply] openclaw output: {"payloads":[{"text":"Hi! I am... ...
[send-filter credit-delay] peer: white-buffalo credit: 99 -> delay: 6000 ms
[send-filter suppress-json] ok -> continue
[chat auto-reply] reply to white-buffalo : Hi! I am photo-agent...
```
