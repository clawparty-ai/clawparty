# ClawParty E2E Tests

Playwright-based E2E test suite for the ClawParty Day1 user journey.

## Prerequisites

1. Node.js >= 18
2. `ztm` binary available at `$PROJECT_ROOT/bin/ztm` (or set `ZTM_BIN` environment variable)
3. Test registration server running at `http://127.0.0.1:15678`

## Setup

```bash
cd tests/e2e
npm install
npx playwright install chromium
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `ZTM_BIN` | `../../bin/ztm` | Path to ztm binary |
| `TEST_INVITE_CODE` | `ABCD2345` | Valid invite code for TC-P-001 |

## Running Tests

### Run all tests

```bash
npm test
```

### Run specific test

```bash
npx playwright test specs/TC-P-001.spec.js
npx playwright test specs/TC-E-001.spec.js
```

### Run with browser visible (headed mode)

```bash
npm run test:headed
```

### Debug mode (step through test)

```bash
npm run test:debug
```

### View HTML report after run

```bash
npm run test:report
```

## Test Environment Setup

### Start test registration server

Use the existing setup scripts to start the registration server and hub:

```bash
cd tests/acl-local
./setup.sh
```

Then run the E2E tests:

```bash
cd tests/e2e
TEST_INVITE_CODE=ABCD2345 npm test
```

### How TC-P-001 Works

The test does the following:
1. Starts a ZTM agent on port 7784 (via `child_process`)
2. Opens Chromium and navigates to `http://127.0.0.1:7784/`
3. Submits the API token dialog
4. Clicks the 🌐 "Join Party" button in the sidebar
5. Fills in: Registration URL, Username, Invite Code
6. Submits and waits for success message
7. Navigates to the zAgents panel
8. Creates a new AI agent named `my-assistant`
9. Clicks on the agent to open chat
10. Sends "你好，请介绍一下你自己"
11. Waits for AI reply (up to 30 seconds)
12. Verifies at least 2 messages are in the chat

### How TC-E-001 Works

1. Same setup as TC-P-001
2. Fills invite code with `ZZZZ9999` (non-existent)
3. Submits and waits for error message
4. Verifies error text contains keywords like "invalid", "邀请码", "无效", etc.
5. Verifies the modal stays open (no navigation on error)

## Screenshots

Screenshots are saved to `tests/e2e/screenshots/{TEST_NAME}/`.

Naming format: `{counter}-{timestamp}-{step-name}.png`

Example:
```
screenshots/TC-P-001/
  01-2026-04-24T...-gui-loaded.png
  02-2026-04-24T...-token-submitted.png
  03-2026-04-24T...-join-party-modal-opened.png
  04-2026-04-24T...-join-party-form-filled.png
  05-2026-04-24T...-join-party-success.png
  06-2026-04-24T...-zagents-panel-opened.png
  07-2026-04-24T...-agent-created.png
  08-2026-04-24T...-chat-opened.png
  09-2026-04-24T...-message-sent.png
  10-2026-04-24T...-reply-received.png
```

## Project Structure

```
tests/e2e/
├── package.json              # Playwright dependency
├── playwright.config.js      # Browser, timeout, reporter config
├── fixtures/
│   ├── test-data.js          # Test data: invite codes, usernames, messages
│   └── test-env.js           # ZTM agent lifecycle management
├── pages/
│   ├── join-party.page.js    # Page Object: Join Party modal
│   ├── agent.page.js         # Page Object: zAgent list + create dialog
│   └── chat.page.js          # Page Object: Chat message area
├── specs/
│   ├── TC-P-001.spec.js      # Happy path: full Day1 flow
│   └── TC-E-001.spec.js      # Error: invalid invite code
├── utils/
│   ├── screenshot.js         # Screenshot helper with auto-naming
│   └── api-helper.js         # Network request/response monitoring
└── screenshots/              # Auto-created on first test run
```

## Selector Reference

Key selectors used in Page Objects (derived from actual Vue components):

| Element | Selector |
|---|---|
| Join Party button | `.new-group-rail-btn:first-child` |
| Join Party modal | `.modal-backdrop .modal-dialog` |
| Reg URL input | `.join-party-body input:nth-child(0)` |
| Username input | `.join-party-body input:nth-child(1)` |
| Invite code input | `.join-party-body input:nth-child(2)` |
| Submit button | `.modal-create-btn` |
| Error text | `.join-party-error` |
| Success text | `.join-party-success` |
| zAgents org icon | `.org-icon[title="zAgents"]` |
| Create agent button | `.add-agent-btn` |
| Create agent dialog | `.zagent-create-dialog` |
| Agent name input | `.zagent-create-dialog .search-input` |
| zAgent item | `.panel-item.zagent-item` |
| Chat message area | `.chat-main .messages` |
| Message input | `MessageInput` component textarea |
| Typing indicator | `.typing-indicator` |

## Troubleshooting

**Test fails at token dialog**: The GUI requires an API token. The test submits `test-token`. If your ZTM agent requires a specific token, set it via environment or adjust `test-env.js`.

**ZTM agent not starting**: Check `ZTM_BIN` is pointing to a valid executable. Verify port 7784 is not in use.

**Join Party timeout**: Ensure the test registration server at `http://127.0.0.1:15678` is running and the invite code has not been used.

**Selector mismatch**: If the Vue component templates change, Page Object selectors may need updating. Run with `--headed` to visually inspect the UI and use `npx playwright codegen http://127.0.0.1:7784` to generate new selectors.
