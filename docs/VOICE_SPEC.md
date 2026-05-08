# ClawParty Voice Call Technical Specification

## 1. Overview

This specification defines the peer-to-peer voice calling feature for ClawParty using **WebRTC for media transport** and **ZeroClaw WebSocket for signaling**.

## 2. Architecture

```
┌────────────────────┐     WebSocket      ┌────────────────────┐
│   Caller Browser   │ ◄───(signaling)──► │  Callee Browser   │
│                    │      /ws/chat       │                    │
│ ┌──────────────┐   │                    │   ┌──────────────┐  │
│ │ Voice UI     │   │                    │   │ Voice UI     │  │
│ │ Components   │   │                    │   │ Components   │  │
│ └──────┬───────┘   │                    │   └──────┬───────┘  │
│        │           │                    │          │          │
│ ┌──────▼───────┐   │                    │   ┌──────▼───────┐  │
│ │ voiceService │   │                    │   │ voiceService │  │
│ │ (state mgmt) │   │                    │   │ (state mgmt) │  │
│ └──────┬───────┘   │                    │   └──────┬───────┘  │
│        │           │                    │          │          │
│ ┌──────▼───────┐   │                    │   ┌──────▼───────┐  │
│ │ webRTCService│   │   SRTP/UDP (P2P)   │   │ webRTCService│  │
│ │(RTCPeerConn) │◄──────────────────────►│   │(RTCPeerConn) │  │
│ └──────────────┘   │   (via STUN/ICE)   │   └──────────────┘  │
│        │           │                    │          │          │
│ ┌──────▼───────┐   │                    │   ┌──────▼───────┐  │
│ │speechService │   │                    │   │speechService │  │
│ │ (STT + TTS)  │   │                    │   │ (STT + TTS)  │  │
│ └──────────────┘   │                    │   └──────────────┘  │
└────────────────────┘                    └────────────────────┘
         │                                          │
         └────────── ZeroClaw WebSocket ────────────┘
              (via ZTM mesh, authenticated)
```

## 3. Message Schemas

All voice signaling messages travel over the existing ZeroClaw WebSocket connection (`/ws/chat` endpoint). They reuse the existing JSON envelope with a `type` field.

### 3.1 Base Message Structure

```typescript
interface VoiceMessage {
  type: string;        // Voice message type (see below)
  callId: string;      // UUID for the call session
  from: string;        // Sender agent/user name
  to: string;          // Recipient agent/user name
  timestamp: number;   // Unix timestamp (ms)
  // ... type-specific payload
}
```

### 3.2 Message Types

#### `voice-invite`
Sent by caller to initiate a call.

```json
{
  "type": "voice-invite",
  "callId": "550e8400-e29b-41d4-a716-446655440000",
  "from": "alice",
  "to": "bob",
  "timestamp": 1715097600000,
  "payload": {
    "mediaType": "audio",
    "sdpOffer": "v=0\r\no=- 12345...",
    "iceServers": [
      { "urls": "stun:stun.l.google.com:19302" }
    ]
  }
}
```

#### `voice-accept`
Sent by callee to accept an incoming call.

```json
{
  "type": "voice-accept",
  "callId": "550e8400-e29b-41d4-a716-446655440000",
  "from": "bob",
  "to": "alice",
  "timestamp": 1715097601000,
  "payload": {
    "sdpAnswer": "v=0\r\no=- 67890..."
  }
}
```

#### `voice-reject`
Sent by callee to decline an incoming call.

```json
{
  "type": "voice-reject",
  "callId": "550e8400-e29b-41d4-a716-446655440000",
  "from": "bob",
  "to": "alice",
  "timestamp": 1715097602000,
  "payload": {
    "reason": "busy"  // "busy" | "declined" | "timeout" | "error"
  }
}
```

#### `voice-end`
Sent by either party to terminate an active call.

```json
{
  "type": "voice-end",
  "callId": "550e8400-e29b-41d4-a716-446655440000",
  "from": "alice",
  "to": "bob",
  "timestamp": 1715097700000,
  "payload": {
    "reason": "hangup"  // "hangup" | "error" | "network"
  }
}
```

#### `voice-ice-candidate`
Sent by either party to exchange ICE candidates (trickle ICE).

```json
{
  "type": "voice-ice-candidate",
  "callId": "550e8400-e29b-41d4-a716-446655440000",
  "from": "alice",
  "to": "bob",
  "timestamp": 1715097603000,
  "payload": {
    "candidate": "candidate:842163049 1 udp 1686052607 192.168.1.5 54321 typ srflx raddr 0.0.0.0 rport 0 generation 0",
    "sdpMid": "0",
    "sdpMLineIndex": 0
  }
}
```

#### `voice-busy`
Sent when callee is already in another call.

```json
{
  "type": "voice-busy",
  "callId": "550e8400-e29b-41d4-a716-446655440000",
  "from": "bob",
  "to": "alice",
  "timestamp": 1715097604000
}
```

#### `voice-hello` (for group calls, Phase 4)
Sent to announce presence in a group call.

```json
{
  "type": "voice-hello",
  "callId": "550e8400-e29b-41d4-a716-446655440000",
  "from": "alice",
  "timestamp": 1715097605000,
  "payload": {
    "groupId": "group-123",
    "participants": ["alice", "bob"]
  }
}
```

### 3.3 Message Type Constants

```javascript
export const VOICE_MESSAGE_TYPES = {
  INVITE: 'voice-invite',
  ACCEPT: 'voice-accept',
  REJECT: 'voice-reject',
  END: 'voice-end',
  ICE_CANDIDATE: 'voice-ice-candidate',
  BUSY: 'voice-busy',
  HELLO: 'voice-hello',
}
```

## 4. State Machine

### 4.1 Call States

```
                    ┌─────────┐
                    │  IDLE   │◄──────────────────────┐
                    └────┬────┘                       │
                         │ on voice-invite (callee)   │
            ┌────────────┼────────────┐               │
            │            │            │               │
            ▼            ▼            ▼               │
     ┌──────────┐ ┌──────────┐  ┌──────────┐         │
     │CALLING   │ │ RINGING  │  │ RECEIVING│         │
     │(caller)  │ │(caller)  │  │(callee)  │         │
     └────┬─────┘ └────┬─────┘  └────┬─────┘         │
          │            │             │               │
          │   on voice-accept        │               │
          │◄─────────────────────────┘               │
          │                                          │
          ▼                                          │
     ┌──────────┐                                    │
     │CONNECTED │                                    │
     │          │                                    │
     └────┬─────┘                                    │
          │                                          │
          │ on voice-end / error / timeout           │
          ▼                                          │
     ┌──────────┐                                    │
     │  ENDED   │────────────────────────────────────┘
     └──────────┘
```

### 4.2 State Definitions

| State | Actor | Description |
|-------|-------|-------------|
| `idle` | Both | No active call. Ready to initiate or receive. |
| `calling` | Caller | User clicked call button. WebRTC offer created and sent. Waiting for response. |
| `ringing` | Caller | Offer delivered. Call is ringing on callee side. |
| `receiving` | Callee | Incoming call received. User sees accept/reject UI. |
| `connecting` | Both | Accept/Reject processed. ICE negotiation in progress. |
| `connected` | Both | WebRTC peer connection established. Audio flowing. |
| `ended` | Both | Call terminated. Cleanup performed. Returns to `idle`. |

### 4.3 State Transitions

| From | Event | To | Action |
|------|-------|-----|--------|
| `idle` | `initiateCall()` | `calling` | Createoffer, send `voice-invite` |
| `calling` | `voice-accept` | `connecting` | Set remote answer, finalize connection |
| `calling` | `voice-reject` | `ended` | Show rejection reason, cleanup |
| `calling` | `voice-busy` | `ended` | Show busy indicator, cleanup |
| `calling` | timeout (30s) | `ended` | Show "no answer", cleanup |
| `idle` | `voice-invite` (received) | `receiving` | Show incoming call UI, play ringtone |
| `receiving` | `user-accept` | `connecting` | Create answer, send `voice-accept` |
| `receiving` | `user-reject` | `ended` | Send `voice-reject`, cleanup |
| `receiving` | timeout (30s) | `ended` | Send `voice-reject` (timeout), cleanup |
| `connecting` | `ice-connected` | `connected` | Show "connected" UI, start timer |
| `connecting` | `ice-failed` | `ended` | Show connection error, cleanup |
| `connected` | `voice-end` | `ended` | Stop audio, cleanup, show summary |
| `connected` | `connection-error` | `ended` | Show error, cleanup |

## 5. Component APIs

### 5.1 voiceService.js

Central call state management service.

```typescript
class VoiceService {
  // Properties
  state: Ref<CallState>        // Current call state
  currentCall: Ref<CallInfo | null>
  isMuted: Ref<boolean>
  isSpeakerOn: Ref<boolean>
  callDuration: Ref<number>    // Seconds
  error: Ref<string | null>

  // Methods
  initiateCall(to: string): Promise<void>
  acceptCall(callId: string): Promise<void>
  rejectCall(callId: string, reason?: string): void
  endCall(): void
  toggleMute(): void
  toggleSpeaker(): void
  sendIceCandidate(candidate: RTCIceCandidate): void
  handleIncomingMessage(msg: VoiceMessage): void
  cleanup(): void
}

interface CallInfo {
  callId: string
  from: string
  to: string
  startTime: number | null
  endTime: number | null
  participants: string[]
}

type CallState = 
  | 'idle' 
  | 'calling' 
  | 'ringing' 
  | 'receiving' 
  | 'connecting' 
  | 'connected' 
  | 'ended'
```

### 5.2 webRTCService.js

WebRTC peer connection wrapper.

```typescript
class WebRTCService {
  // Properties
  peerConnection: RTCPeerConnection | null
  localStream: MediaStream | null
  remoteStream: MediaStream | null
  isAudioEnabled: boolean

  // Constructor options
  constructor(options: {
    iceServers: RTCIceServer[]
    onLocalStream: (stream: MediaStream) => void
    onRemoteStream: (stream: MediaStream) => void
    onIceCandidate: (candidate: RTCIceCandidate) => void
    onConnectionStateChange: (state: RTCPeerConnectionState) => void
    onTrack: (event: RTCTrackEvent) => void
  })

  // Methods
  async initialize(): Promise<void>           // Get local media
  async createOffer(): Promise<RTCSessionDescriptionInit>
  async createAnswer(offer: RTCSessionDescriptionInit): Promise<RTCSessionDescriptionInit>
  async setRemoteDescription(desc: RTCSessionDescriptionInit): Promise<void>
  async addIceCandidate(candidate: RTCIceCandidateInit): Promise<void>
  enableAudio(): void
  disableAudio(): void
  toggleAudio(): boolean
  close(): void
}

// Default ICE servers (STUN only for MVP)
const DEFAULT_ICE_SERVERS = [
  { urls: 'stun:stun.l.google.com:19302' },
  { urls: 'stun:stun1.l.google.com:19302' },
]
```

### 5.3 speechService.js

Web Speech API wrapper for STT + TTS.

```typescript
class SpeechService {
  // Properties
  isListening: Ref<boolean>
  isSpeaking: Ref<boolean>
  transcript: Ref<string>
  interimTranscript: Ref<string>
  isSupported: boolean

  // STT Methods
  startListening(options?: {
    language?: string        // default: 'zh-CN'
    continuous?: boolean     // default: true
    interimResults?: boolean // default: true
  }): void
  stopListening(): void
  onTranscript(callback: (text: string, isFinal: boolean) => void): void

  // TTS Methods
  speak(text: string, options?: {
    language?: string        // default: 'zh-CN'
    rate?: number           // default: 1.0
    pitch?: number          // default: 1.0
  }): void
  stopSpeaking(): void
  onSpeakingEnd(callback: () => void): void
}
```

### 5.4 VoiceCallButton.vue

Floating action button displayed in chat header for eligible chats.

```vue
<template>
  <button 
    class="voice-call-btn"
    :class="{ 'is-calling': isCalling }"
    @click="handleClick"
    :title="buttonTitle"
  >
    <svg v-if="!isCalling" ... ><!-- phone icon --></svg>
    <svg v-else ... ><!-- phone-off icon --></svg>
  </button>
</template>

<script setup>
const props = defineProps({
  chat: Object,           // Current chat context
  isInCall: Boolean,      // Whether user is currently in a call
  canCall: Boolean        // Whether this chat supports voice calls
})

const emit = defineEmits(['initiateCall', 'endCall'])
</script>
```

**Rules:**
- Show only for 1-on-1 chats (`!chat.isGroupChat`)
- Show disabled state if user is already in a call with someone else
- Pulse animation when `isCalling` is true
- Position: in `ChatHeader.vue` header-right section

### 5.5 VoiceCallOverlay.vue

Full-screen overlay shown during active calls.

```vue
<template>
  <div class="voice-call-overlay" :class="state">
    <!-- Caller/Callee info -->
    <div class="call-participant">
      <div class="participant-avatar">{{ name[0] }}</div>
      <div class="participant-name">{{ name }}</div>
      <div class="call-status">{{ statusText }}</div>
      <div class="call-timer" v-if="state === 'connected'">{{ formattedDuration }}</div>
    </div>

    <!-- Incoming call UI (callee only) -->
    <div v-if="state === 'receiving'" class="incoming-call-actions">
      <button class="btn-accept" @click="acceptCall">Accept</button>
      <button class="btn-reject" @click="rejectCall">Reject</button>
    </div>

    <!-- Active call controls -->
    <div v-if="state === 'connected' || state === 'connecting'" class="call-controls">
      <button :class="{ active: isMuted }" @click="toggleMute">
        {{ isMuted ? 'Unmute' : 'Mute' }}
      </button>
      <button class="btn-end" @click="endCall">End Call</button>
      <button :class="{ active: isSpeakerOn }" @click="toggleSpeaker">
        Speaker
      </button>
    </div>
  </div>
</template>

<script setup>
const props = defineProps({
  state: String,          // CallState
  name: String,           // Display name of other party
  isMuted: Boolean,
  isSpeakerOn: Boolean,
  duration: Number,       // Seconds
})

const emit = defineEmits(['accept', 'reject', 'end', 'toggleMute', 'toggleSpeaker'])
</script>
```

**States render:**
- `calling`: Show "Calling..." with cancel button
- `ringing`: Show "Ringing..." with cancel button
- `receiving`: Show incoming with accept/reject
- `connecting`: Show "Connecting..."
- `connected`: Show timer, mute, speaker, end buttons
- `ended`: Brief "Call ended" then auto-dismiss

## 6. Data Flow

### 6.1 Outgoing Call Flow

```
User clicks call button
    │
    ▼
VoiceCallButton emits 'initiateCall'
    │
    ▼
voiceService.initiateCall(to)
    │
    ├──► webRTCService.initialize() ──► getUserMedia({audio:true})
    │                                    │
    ├──► webRTCService.createOffer()    │
    │      │                            │
    │      └──► setLocalDescription()   │
    │                                    │
    ├──► ZeroClawWS.sendVoiceMessage() ◄┘
    │      └──► type: 'voice-invite'
    │              payload: { sdpOffer }
    │
    ▼
State = 'calling'
Show VoiceCallOverlay (calling state)
    │
    ├──► Wait for voice-accept
    │      ├──► webRTCService.setRemoteDescription(answer)
    │      ├──► State = 'connecting'
    │      └──► Wait for ICE to connect
    │             ├──► State = 'connected'
    │             └──► Start timer
    │
    ├──► Wait for voice-reject
    │      └──► State = 'ended', show reason, cleanup
    │
    └──► Timeout (30s)
           └──► State = 'ended', show "No answer", cleanup
```

### 6.2 Incoming Call Flow

```
ZeroClawWS receives voice-invite
    │
    ▼
voiceService.handleIncomingMessage(msg)
    │
    ├──► Validate call (not busy, etc.)
    │
    ├──► State = 'receiving'
    │
    ├──► Show VoiceCallOverlay (receiving state)
    │
    └──► Play ringtone
    │
    ├──► User clicks Accept
    │      │
    │      ├──► webRTCService.initialize()
    │      ├──► webRTCService.createAnswer(offer)
    │      ├──► ZeroClawWS.sendVoiceMessage()
    │      │      └──► type: 'voice-accept'
    │      │              payload: { sdpAnswer }
    │      ├──► State = 'connecting'
    │      └──► Wait for ICE
    │             ├──► State = 'connected'
    │             └──► Stop ringtone
    │
    ├──► User clicks Reject
    │      ├──► ZeroClawWS.sendVoiceMessage()
    │      │      └──► type: 'voice-reject'
    │      ├──► State = 'ended'
    │      └──► Cleanup
    │
    └──► Timeout (30s)
           ├──► Send voice-reject (timeout)
           ├──► State = 'ended'
           └──► Cleanup
```

### 6.3 ICE Exchange (Trickle ICE)

```
WebRTC: onicecandidate event
    │
    ▼
webRTCService.onIceCandidate callback
    │
    ▼
voiceService.sendIceCandidate(candidate)
    │
    ▼
ZeroClawWS.sendVoiceMessage()
    └──► type: 'voice-ice-candidate'
         payload: { candidate, sdpMid, sdpMLineIndex }
    │
    ▼
Remote peer receives
    │
    ▼
voiceService.handleIncomingMessage()
    │
    ▼
webRTCService.addIceCandidate()
```

## 7. Backend Routing

### 7.1 ZeroClaw Agent Behavior

ZeroClaw's `/ws/chat` WebSocket endpoint is used for chat message relay between agents. Voice signaling messages must be treated identically — the agent does not need to understand or process voice messages, only forward them.

**Requirement:**
- Voice message types (`voice-*`) MUST be forwarded to the target peer's WebSocket connection without modification
- No persistence of voice messages in chat history is required
- No agent-side WebRTC processing is needed (media is P2P between browsers)

### 7.2 Message Routing Logic

Current behavior (chat):
```
Client A ──voice-invite──► Agent ──► Client B (target peer)
```

Required behavior (voice):
```
Client A ──voice-invite──► Agent ──► Client B (target peer, by 'to' field)
```

**Implementation note:** The agent should use the `to` field in the voice message to determine the recipient peer, or if the WebSocket connection is already scoped to a specific chat session, simply forward to the peer on the other end of that session.

### 7.3 Agent File Location

Voice message passthrough requires modifying the ZeroClaw agent's WebSocket message handler. The exact location needs exploration, but is likely in:
- `agent/` directory: ZeroClaw agent source
- WebSocket upgrade handler for `/ws/chat` route

**Expected change:**
```javascript
// In agent WebSocket message handler
if (msg.type && msg.type.startsWith('voice-')) {
  // Forward to target peer without processing
  forwardToPeer(msg.to, messageData)
  return
}
```

## 8. WebRTC Configuration

### 8.1 Peer Connection Options

```javascript
const pcConfig = {
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    { urls: 'stun:stun1.l.google.com:19302' }
  ],
  iceCandidatePoolSize: 10,
  // TURN servers (Phase 4)
  // iceServers: [
  //   { urls: 'turn:turn.example.com:3478', username: 'user', credential: 'pass' }
  // ]
}
```

### 8.2 Media Constraints

```javascript
const audioConstraints = {
  audio: {
    echoCancellation: true,
    noiseSuppression: true,
    autoGainControl: true,
    sampleRate: 48000
  },
  video: false  // Audio only for MVP
}
```

### 8.3 SDP Offer/Answer Options

```javascript
const offerOptions = {
  offerToReceiveAudio: true,
  offerToReceiveVideo: false,
  voiceActivityDetection: true
}
```

## 9. STT/TTS Integration

### 9.1 STT (Speech-to-Text)

**API:** `webkitSpeechRecognition` (Web Speech API)

```javascript
const recognition = new (window.SpeechRecognition || window.webkitSpeechRecognition)()
recognition.lang = 'zh-CN'
recognition.continuous = true
recognition.interimResults = true

recognition.onresult = (event) => {
  const transcript = event.results[event.results.length - 1][0].transcript
  const isFinal = event.results[event.results.length - 1].isFinal
  // Send transcript as chat message if final, or show interim
}
```

**Phase 4 feature:** Send STT output as text messages in the chat, enabling "voice-to-text" mode.

### 9.2 TTS (Text-to-Speech)

**API:** `speechSynthesis` (Web Speech API)

```javascript
const utterance = new SpeechSynthesisUtterance(text)
utterance.lang = 'zh-CN'
utterance.rate = 1.0
utterance.pitch = 1.0
speechSynthesis.speak(utterance)
```

**Phase 4 feature:** When an agent sends a text message, optionally read it aloud using TTS (user preference).

### 9.3 Browser Support Detection

```javascript
function isSpeechSupported() {
  return 'SpeechRecognition' in window || 'webkitSpeechRecognition' in window
}

function isTTSSupported() {
  return 'speechSynthesis' in window
}
```

## 10. UI/UX Design

### 10.1 Voice Call Button (ChatHeader)

- **Position:** Right side of header, after settings button
- **Icon:** Phone handset (📞) when idle, phone-off when in call
- **States:**
  - Normal: Gray, clickable
  - Disabled: Grayed out (already in call with another peer)
  - Active calling: Pulsing red animation

### 10.2 Voice Call Overlay

- **Position:** Fixed, full viewport, z-index above all content
- **Background:** Dark semi-transparent (`rgba(0,0,0,0.85)`)
- **Layout:**
  - Top: Participant avatar + name + status
  - Center: Call timer (when connected)
  - Bottom: Action buttons
- **Incoming call:** Two large buttons (green accept, red reject)
- **Active call:** Three buttons (mute, end-call [red large], speaker)

### 10.3 Incoming Call Modal

When user is on a different chat and receives a voice call:
- Show a toast/modal overlay at top of screen
- Play ringtone (browser audio)
- Allow answering without switching context, OR auto-switch to caller's chat

## 11. Error Handling

| Error | Cause | UX Action | Recovery |
|-------|-------|-----------|----------|
| `getUserMedia denied` | User rejected mic permission | Show "Microphone access denied" settings hint | User must grant permission in browser |
| `WebRTC not supported` | Old browser | Show "Browser not supported" | Upgrade browser |
| `ICE connection failed` | NAT/firewall blocks P2P | Show "Connection failed. Try again?" | Retry with new ICE gathering |
| `Peer disconnected` | Network drop | Show "Call ended: network error" | Auto-cleanup |
| `Signaling timeout` | 30s no response | Show "No answer" | Return to idle |
| `Peer busy` | Callee in another call | Show "User is busy" | Return to idle |

## 12. Security Considerations

1. **Authentication:** Voice messages ride on existing authenticated WebSocket. No additional auth needed.
2. **Encryption:** WebRTC media is SRTP-encrypted end-to-end. Signaling is over TLS (wss).
3. **Caller ID:** The `from` field in voice messages should be verified by the agent (cannot be forged by client).
4. **Rate limiting:** Prevent call spam by limiting voice-invite frequency per peer.

## 13. Performance Considerations

1. **Bundle size:** WebRTC + Web Speech APIs are browser-native. No library needed.
2. **Memory:** Clean up RTCPeerConnection and MediaStream tracks on call end to prevent leaks.
3. **Battery:** Use `continuous: false` in STT when not actively listening.
4. **Network:** Voice messages are small JSON (~KB). No impact on bandwidth.

## 14. Testing Checklist

### 14.1 Unit Tests (where applicable)
- [ ] Voice state machine transitions correctly
- [ ] Message validation accepts valid voice messages
- [ ] Message validation rejects malformed messages

### 14.2 Integration Tests
- [ ] Mock WebRTC peers: signaling flow works end-to-end
- [ ] Mock messages: all voice types are forwarded correctly

### 14.3 Manual Tests
- [ ] Same browser (two tabs): call-success
- [ ] Same machine (two browsers): call-success
- [ ] Same LAN (two devices): call-success with STUN
- [ ] Cross-network: call-success with STUN
- [ ] Callee rejects: caller sees rejection UI
- [ ] Callee busy: caller sees busy UI
- [ ] Caller hangs up before answer: no crash
- [ ] Network drop during call: both see disconnect
- [ ] STT: speech recognized and shown
- [ ] TTS: agent message read aloud

## 15. File List (MVP)

### New Files
- `chat-gui/src/services/voiceService.js`
- `chat-gui/src/services/webRTCService.js`
- `chat-gui/src/services/speechService.js`
- `chat-gui/src/components/VoiceCallButton.vue`
- `chat-gui/src/components/VoiceCallOverlay.vue`
- `chat-gui/src/composables/useVoiceCall.js` (optional)

### Modified Files
- `chat-gui/src/services/chatService.js` - Add `ZeroClawWS.sendVoiceMessage()`
- `chat-gui/src/services/chatService.js` - Add voice message type constants
- `chat-gui/src/components/ChatHeader.vue` - Add VoiceCallButton
- `chat-gui/src/components/ChatMain.vue` - Add VoiceCallOverlay
- `agent/` - Add voice message passthrough (location TBD)

## 16. Phase 4 Features (Post-MVP)

| Feature | Description | Complexity |
|---------|-------------|------------|
| TURN Server | Relay for symmetric NAT | Medium - requires TURN infrastructure |
| Group Voice | Multiple participants | High - SFU or mesh WebRTC |
| Agent STT/TTS | Browser speech sent to agent | Medium - wire speechService to chat |
| Call History | Persist call logs | Low - add to REST API |
| Quality Indicators | Show network quality in UI | Medium - use RTC stats |
| Screen Share | Add video track | Medium - extend webRTCService |
