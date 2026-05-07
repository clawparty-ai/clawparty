# ZeroClaw Integration Progress

## ✅ Completed

### Phase 1: Code Copy
- [x] Copied ZeroClaw to `clawparty/zeroclaw/`
- [x] Verified all crates and dependencies

### Phase 2: Build System
- [x] Modified `build.sh` to compile ZeroClaw
- [x] Added ZeroClaw binary output to `bin/zeroclaw`
- [x] Creates config directory `~/.clawparty/.zeroclaw/`

### Phase 3: ZTM Channel
- [x] Created `zeroclaw/crates/zeroclaw-channels/src/ztm.rs`
- [x] Implemented Channel trait
- [x] Added module declaration in `lib.rs`
- [x] Supports sender-based session model

### Phase 4: Gateway API
- [x] Added `handle_api_ztm_sessions_list()` - List ZTM sessions
- [x] Added `handle_api_ztm_session_chat()` - Send message to session
- [x] Registered routes in gateway lib.rs:
  - `GET /api/ztm/sessions`
  - `POST /api/sessions/{id}/chat`

### Phase 5: Configuration
- [x] Created `zeroclaw/config.example.toml`
- [x] Template includes provider, channel, memory, security settings
- [x] Supports environment variables for API keys

## 🔄 In Progress

### Phase 6: TUI Modifications

#### 6.1 Create ZeroClawDaemon Manager
- [ ] Create `tui/src/zeroclaw.rs`
- [ ] Implement process management
- [ ] Add log capture
- [ ] Add health check

#### 6.2 Modify Main Entry Point
- [ ] Update `tui/src/main.rs`
- [ ] Start ZeroClaw before ZTM agent
- [ ] Add 20-second timeout for readiness
- [ ] Exit on ZeroClaw startup failure
- [ ] Fetch initial sessions list

#### 6.3 Update AppState
- [ ] Add `zeroclaw_sessions: Vec<ZeroClawSession>`
- [ ] Add `current_zeroclaw_session: Option<ZeroClawSession>`
- [ ] Add `zeroclaw_running: bool`
- [ ] Add `zeroclaw_mgr: Option<ZeroClawDaemon>`

#### 6.4 Update API Client
- [ ] Add `check_zeroclaw_health()` method
- [ ] Add `get_zeroclaw_sessions()` method  
- [ ] Add `send_zeroclaw_message()` method
- [ ] Add `get_zeroclaw_messages()` method

#### 6.5 Update UI Rendering
- [ ] Modify `tui/src/ui.rs` sidebar
- [ ] Add "🦀 ZeroClaw" section
- [ ] Display sessions as user list
- [ ] Handle session selection logic

#### 6.6 Add Models
- [ ] Define `ZeroClawSession` struct in `models.rs`

### Phase 7: ClawParty Agent API (Optional)
- [ ] Add `/api/zeroclaw/messages` GET endpoint
- [ ] Add `/api/zeroclaw/messages` POST endpoint

## 📋 Remaining Tasks

1. **Complete TUI modifications** (Phase 6)
2. **Test compilation** - Run `./build.sh`
3. **Test runtime** - Verify ZeroClaw starts and communicates
4. **Debug and fix issues**
5. **Update documentation**

## 🏗️ Architecture Summary

```
┌─────────────────┐
│  ClawParty TUI  │
│  (Rust binary)  │
└────────┬────────┘
         │
         ├─ Starts: bin/zeroclaw daemon (port 42617)
         │   ├─ Gateway API: http://localhost:42617
         │   ├─ ZTM Channel: polls ClawParty API
         │   └─ Sessions: ztm_{user_id}
         │
         └─ Starts: bin/ztm run agent (port 6789)
             └─ HTTP API: http://localhost:6789
```

## 🚀 Next Steps

1. Create `ZeroClawDaemon` struct for process management
2. Modify TUI main.rs to start ZeroClaw first
3. Add API methods for ZeroClaw communication
4. Update UI to display ZeroClaw sessions
5. Test the complete integration

## 📝 Configuration

Example `~/.clawparty/.zeroclaw/config.toml`:

```toml
[gateway]
port = 42617
host = "127.0.0.1"

[provider]
name = "aliyun"
base_url = "http://your-endpoint/v1"
api_key = "${ALIYUN_API_KEY}"
model = "Qwen3.5-397B-A17B"

[channels.ztm]
enabled = true
api_url = "http://127.0.0.1:6789"
api_token = "enjoy-party"
mesh_name = "clawparty"
poll_interval_secs = 1

[memory]
backend = "sqlite"
path = "~/.clawparty/.zeroclaw/memory.db"

[security]
require_pairing = false
```
