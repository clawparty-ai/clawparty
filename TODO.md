# TODO.md - ClawParty Agent-side PipyJS Compatibility Issues

> **NOTE**: This document tracks PJS compatibility issues in **PipyJS/agent-side code** (`agent/`, `cli/`, `hub/`, `ca/`).
> GUI code in `chat-gui/` runs in Node.js/browsers and has full JS support - do NOT report those here.

## PipyJS Limitations

- **No arrow functions**: `.map(x => x)`, `.filter(x => x)`, `.forEach(x => x)` are not supported in callbacks
- **No `.forEach()`**: PJS `.forEach()` callback throws `TypeError: not a function`. Use `for` loops instead.
- **No `.some()`**: PJS does not support `Array.prototype.some()`. Use `for` loops with early `break`.
- **No `RegExp`**: Use `split`, `indexOf`, `charAt`, `substring`, `startsWith`, `endsWith`, `includes`.
- **No `continue`/`break`**: Use `if` blocks to wrap logic instead of `if (!condition) continue`.
- **No `while` loops**: Use `for` loops instead.
- **No `Number.isNaN()`**: Use try-catch with `Number()` cast.
- **No `Promise.try()`**: Bluebird library not available. Use direct `.then()` chaining.

---

## Remaining `.forEach()` calls in agent/apps/ztm/chat/api.js

These use PJS-unsupported callback passing to `.forEach()`:

```text
Line 93:   filterNames.forEach(function (name) { loadFilter(name) })
Line 206:  ids.forEach(function (id) { openclawAgents.push('' + id) })
Line 220:  zeroclawSessions.forEach(function(session) { ... })
Line 405:  text.split(' ').forEach(function (token) { ... })
Line 450:  chat.members.forEach(function (member) { ... })
Line 460:  targetMembers.forEach(function (member) { ... })
Line 879:  messages.forEach(function (msg) { ... })
Line 950:  Object.keys(paths).forEach(function(path) { ... })
Line 1036: messages.forEach(function (msg) { ... })
Line 1164: messages.forEach(function (msg) { ... })
Line 1243: msgs.forEach(function(m) { ... })
Line 1587: info.members.forEach(function (member) { ... })
```

**Note**: These use `function` expressions (not arrow functions), but PJS still doesn't support the `.forEach()` method at all. All must be converted to `for` loops.

---

## Remaining `.some()` calls in agent/apps/ztm/chat/api.js

These use PJS-unsupported `Array.prototype.some()`:

```text
Line 168:  chat.messages.some(function (m) { return m.isPeerRequest })
Line 301:  chat.messages.some(function (m) { return m.isGroupEpRequest })
Line 1047: chat.messages.some(function (m) { })
Line 1062: chat.messages.some(function (m) { return m.isPeerRequest })
Line 1175: chat.messages.some(function (m) { })
```

All must be converted to `for` loops with early `break`.

---

## Remaining `Promise.try()` calls

```text
Line 1299: (already fixed in previous commit)
```

---

## Summary

**Total to fix**: 
- 12 `.forEach()` calls
- 5 `.some()` calls

**High-priority items**: The ones in `readMessages()` (lines 1036, 1047, 1164, 1175) are the most likely to cause runtime errors during sync/watch operations.
