# ClawParty Voice Call - Implementation Plan

## Overview

Implement peer-to-peer voice calling in ClawParty using **WebRTC for media transport** with **ZTM Chat mesh filesystem signaling**.

## Chosen Architecture

| Layer | Technology | Rationale |
|---|---|---|
| **Signaling** | ZTM Chat mesh filesystem (`/api/peers/{peer}/messages`) | Reuses existing authenticated P2P route; no backend changes |
| **Media** | WebRTC (`RTCPeerConnection`) | Browser-native, handles NAT/STUN automatically |
| **STT/TTS** | Web Speech API (MVP) | Zero-cost, zero-backend, immediate availability |

## Branches

- Base: `main`
- Feature: `feature/voice-call`

---

## Phase 1: Signaling Protocol Extension ✅ COMPLETE

**Goal**: Define and implement voice call control message types.

**Deliverables**:
1. Voice message type constants defined
2. `sendVoiceMessage()` added to WebSocket services
3. `VoiceService` state machine created

**Tasks Completed**:
- Added `VOICE_MESSAGE_TYPES` constants (`voice-invite`, `voice-accept`, `voice-reject`, `voice-end`, `voice-ice-candidate`, `voice-busy`, `voice-hello`)
- Extended `ZeroClawWS.sendVoiceMessage()` to send JSON signaling messages via WebSocket
- Added `sendVoiceMessage()` to `wsService.js`
- Created `chat-gui/src/services/voiceService.js` with full call state machine (`idle → calling → receiving → connecting → connected → ended`)
- Created `chat-gui/src/services/webRTCService.js` with `RTCPeerConnection` wrapper (STUN, offer/answer, ICE candidate forwarding)
- Created `chat-gui/src/services/speechService.js` with Web Speech API wrapper (STT + TTS)

**Files Touched**:
- `chat-gui/src/services/chatService.js`
- `chat-gui/src/services/voiceService.js` (new)
- `chat-gui/src/services/webRTCService.js` (new)
- `chat-gui/src/services/speechService.js` (new)
- `chat-gui/src/services/wsService.js`

**Build Status**: ✅ Passes

---

## Phase 2: Frontend Voice Call UI ✅ COMPLETE

**Goal**: Build Vue 3 components for initiating, receiving, and managing voice calls.

**Deliverables**:
1. Voice call button in chat interface
2. Call overlay/modal with callee info and controls
3. Composable tying voice state + WebRTC + UI

**Tasks Completed**:
- Created `VoiceCallButton.vue` — phone icon button in chat header
- Created `VoiceCallOverlay.vue` — full-screen overlay for incoming/active calls with accept/reject, mute, speaker, end controls
- Created `useVoiceCall.js` composable — bridges `VoiceService`, `WebRTCService`, and `VoiceCallOverlay`
- Integrated voice components into `ChatMain.vue` (inject voiceCallStore, handle call events)
- Integrated voiceCallStore global singleton in `App.vue` (`provide('voiceCallStore')`)
- Updated `ChatHeader.vue` with `@initiateVoiceCall` and `@endVoiceCall` events

**Files Touched**:
- `chat-gui/src/components/VoiceCallButton.vue` (new)
- `chat-gui/src/components/VoiceCallOverlay.vue` (new)
- `chat-gui/src/composables/useVoiceCall.js` (new)
- `chat-gui/src/components/ChatHeader.vue`
- `chat-gui/src/components/ChatMain.vue`
- `chat-gui/src/App.vue`

**Build Status**: ✅ Passes

---

## Phase 3: Backend Signaling Relay ✅ COMPLETE

**Goal**: Route voice signaling messages between peers without modifying the backend agent.

**Decision**: Reuse existing ZTM Chat mesh filesystem (Option A) — no backend code changes needed.

**How It Works**:
1. **Sending**: `voiceCallSendFn()` checks if a ZeroClaw WebSocket is active (agent chat). If yes, sends via WebSocket. If no (P2P mesh chat), calls `chatService.sendVoiceSignaling()` which POSTs to `/api/meshes/{mesh}/apps/ztm/chat/api/peers/{peer}/messages` with the voice JSON payload embedded as the `text` field.
2. **Receiving**: `ChatMain.vue` polls messages every second via `getMessagesSince()`. Before adding messages to the UI, it checks if `msg.text` starts with `{"type":"voice-`. If so, it parses it and routes to `voiceCallStore.handleIncomingMessage()` instead of displaying it as chat text.
3. **ZTM Backend**: The existing `addPeerMessage()` in `agent/apps/ztm/chat/api.js` writes the message to `/shared/{sender}/publish/peers/{receiver}/messages/{timestamp}.json`. The existing `syncPeerMessages()` and `allPeerMessages()` retrieve it. No modifications needed.

**Tasks Completed**:
- Added `chatService.sendVoiceSignaling()` wrapper in `chatService.js`
- Updated `App.vue` `voiceCallSendFn` to use **dual path**: ZeroClaw WS for agent chats, ZTM mesh REST for P2P chats
- Updated `ChatMain.vue` `pollMessages()` and `fetchMessages()` to intercept voice signaling JSON from message polling
- Architecture decision documented: voice signaling overwrites `text` field with JSON; mesh READ polls once per second; voice messages are never shown as chat text

**Files Touched**:
- `chat-gui/src/services/chatService.js`
- `chat-gui/src/App.vue`
- `chat-gui/src/components/ChatMain.vue`

**Build Status**: ✅ Passes

---

## Phase 4: Testing & Verification ⏳ PENDING

**Goal**: Verify end-to-end voice calling between two browsers.

**Tasks**:
- [ ] **Localhost Test**: Open two browsers (or two tabs), log in as two different users, initiate voice call, verify SDP offer/answer exchange
- [ ] **ICE Connectivity**: Verify STUN candidates are exchanged and connection reaches `connected` state
- [ ] **Audio Flow**: Confirm microphone audio from caller is heard by callee
- [ ] **Call Controls**: Test mute/unmute, speaker toggle, end call from both sides
- [ ] **Timeout Handling**: Verify 30-second call timeout if no answer
- [ ] **Busy Signal**: Verify busy response when callee is already in a call
- [ ] **STT/TTS**: Test browser speech recognition and synthesis

**Test Environment**:
- Requires two endpoints on the same ZTM mesh
- Start ZTM agent (`./bin/ztm` or `./build-cli-only.sh`)
- Open `http://localhost:18789` in two browsers
- Log in with different usernames
- Navigate to each other’s chat and click voice call button

---

## Known Limitations

| Limitation | Cause | Workaround |
|---|---|---|
| ~1s signaling latency | Mesh filesystem polling interval (1s) | Acceptable for MVP; could switch to WebSocket proxy in future |
| No TURN server | Not configured in WebRTCService | Add TURN config for symmetric NAT |
| Voice messages not filtered from chat history | Voice JSON appears as text in old messages | Frontend now intercepts on receive; older voice messages may appear as raw JSON in history |

---

## Risk Register

| Risk | Impact | Status |
|---|---|---|
| ZTM mesh filesystem has too much latency for WebRTC signaling | Medium | Mitigated: 1s polling is acceptable for offer/answer, ICE trickle handles rest |
| Browser WebRTC blocked by firewall | Medium | Can add TURN relay as Phase 4 enhancement |
| Web Speech API not supported | Low | Feature-detected in `speechService.js` |

---

## Implementation Summary

| Phase | Status | Key Files |
|---|---|---|
| 1. Signaling Protocol | ✅ | `voiceService.js`, `webRTCService.js`, `speechService.js`, `chatService.js` |
| 2. Frontend UI | ✅ | `VoiceCallButton.vue`, `VoiceCallOverlay.vue`, `useVoiceCall.js`, `ChatMain.vue`, `App.vue` |
| 3. Backend Relay | ✅ | `chatService.sendVoiceSignaling()`, `App.vue` dual path, `ChatMain.vue` interceptor |
| 4. Testing | ⏳ | Needs manual browser-to-browser verification |

**Next Step**: Run manual end-to-end test with two browser sessions.
