<template>
  <Teleport to="body">
    <div v-if="isVisible" class="voice-call-overlay" :class="state">
      <div class="overlay-backdrop" @click="handleBackdropClick"></div>
      <div class="overlay-content">
        <div class="call-participant">
          <div class="participant-avatar" :style="{ background: getAvatarColor(name) }">
            {{ avatarLetter }}
          </div>
          <div class="participant-name">{{ name }}</div>
          <div class="call-status">{{ statusText }}</div>
          <div v-if="state === 'connected' && duration > 0" class="call-timer">
            {{ formatDuration(duration) }}
          </div>
        <div v-if="state === 'connecting' || (isAgentMode && state === 'connected')" class="connection-spinner"></div>

        <!-- Agent Mode: Transcripts -->
        <div v-if="isAgentMode && agentTranscripts && agentTranscripts.length" class="agent-transcripts">
          <div v-for="(t, i) in agentTranscripts.slice(-3)" :key="i" class="agent-transcript-bubble">
            {{ t }}
          </div>
        </div>
      </div>

      <div v-if="state === 'receiving'" class="incoming-call-actions">
          <button class="btn-accept" @click="handleAccept">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z"/>
            </svg>
            Accept
          </button>
          <button class="btn-reject" @click="handleReject">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="23" y1="1" x2="1" y2="23"/>
              <path d="M10.68 13.31a16 16 0 0 0 3.41 2.6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91"/>
            </svg>
            Reject
          </button>
        </div>

        <div v-if="state === 'calling' || state === 'ringing'" class="calling-actions">
          <button class="btn-cancel" @click="handleEnd">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="23" y1="1" x2="1" y2="23"/>
              <path d="M10.68 13.31a16 16 0 0 0 3.41 2.6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91"/>
            </svg>
            Cancel
          </button>
        </div>

        <div v-if="state === 'connected' || state === 'connecting'" class="call-controls">
          <button
            class="control-btn"
            :class="{ active: isMuted }"
            @click="handleToggleMute"
          >
            <svg v-if="!isMuted" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
              <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
              <line x1="12" y1="19" x2="12" y2="23"/>
              <line x1="8" y1="23" x2="16" y2="23"/>
            </svg>
            <svg v-else width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="1" y1="1" x2="23" y2="23"/>
              <path d="M9 9v6a3 3 0 0 0 5.12 2.12M15 9.34V4a3 3 0 0 0-5.94-.6"/>
              <path d="M17 16.95A7 7 0 0 1 5 12v-2m14 0v2a7 7 0 0 1-.11 1.23"/>
              <line x1="12" y1="19" x2="12" y2="23"/>
              <line x1="8" y1="23" x2="16" y2="23"/>
            </svg>
            {{ isMuted ? 'Unmute' : 'Mute' }}
          </button>
          <button class="control-btn btn-end" @click="handleEnd">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10.68 13.31a16 16 0 0 0 3.41 2.6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91"/>
              <line x1="23" y1="1" x2="1" y2="23"/>
            </svg>
            End
          </button>
          <button
            class="control-btn"
            :class="{ active: isSpeakerOn }"
            @click="handleToggleSpeaker"
          >
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
              <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
              <path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>
            </svg>
            Speaker
          </button>
        </div>

        <audio
          ref="remoteAudio"
          :srcObject="remoteStream"
          autoplay
          playsinline
        ></audio>
      </div>
    </div>
  </Teleport>
</template>

<script setup>
import { computed, ref, watch } from 'vue'

const props = defineProps({
  state: {
    type: String,
    required: true,
  },
  name: {
    type: String,
    default: 'Unknown',
  },
  isMuted: {
    type: Boolean,
    default: false,
  },
  isSpeakerOn: {
    type: Boolean,
    default: false,
  },
  duration: {
    type: Number,
    default: 0,
  },
  remoteStream: {
    type: Object,
    default: null,
  },
  isAgentMode: {
    type: Boolean,
    default: false,
  },
  agentTranscripts: {
    type: Array,
    default: () => [],
  },
  isSpeaking: {
    type: Boolean,
    default: false,
  },
})

const emit = defineEmits(['accept', 'reject', 'end', 'toggleMute', 'toggleSpeaker'])

const remoteAudio = ref(null)

const isVisible = computed(() => {
  return props.state !== 'idle' && props.state !== 'ended'
})

const statusText = computed(() => {
  const map = {
    calling: 'Calling...',
    ringing: 'Ringing...',
    receiving: 'Incoming call',
    connecting: 'Connecting...',
    connected: '',
    ended: 'Call ended',
  }
  if (props.state === 'connected' && props.isAgentMode) {
    return props.isSpeaking ? 'Speaking...' : 'Listening...'
  }
  return map[props.state] || props.state
})

const avatarLetter = computed(() => {
  return props.name ? props.name[0].toUpperCase() : '?'
})

function getAvatarColor(name) {
  const colors = [
    '#e17055', '#00b894', '#0984e3', '#6c5ce7',
    '#fd79a8', '#e84393', '#a29bfe', '#fdcb6e',
  ]
  let hash = 0
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash)
  }
  return colors[Math.abs(hash) % colors.length]
}

function formatDuration(seconds) {
  const m = Math.floor(seconds / 60).toString().padStart(2, '0')
  const s = Math.floor(seconds % 60).toString().padStart(2, '0')
  return `${m}:${s}`
}

function handleAccept() {
  emit('accept')
}

function handleReject() {
  emit('reject')
}

function handleEnd() {
  emit('end')
}

function handleToggleMute() {
  emit('toggleMute')
}

function handleToggleSpeaker() {
  emit('toggleSpeaker')
}

function handleBackdropClick() {
  if (props.state === 'calling' || props.state === 'ringing') {
    handleEnd()
  }
}

watch(() => props.remoteStream, (stream) => {
  if (remoteAudio.value && stream) {
    remoteAudio.value.srcObject = stream
  }
})
</script>

<style scoped>
.voice-call-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.overlay-backdrop {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(4px);
}

.overlay-content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 40px;
  padding: 40px;
}

.call-participant {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.participant-avatar {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 40px;
  font-weight: 700;
  color: white;
}

.participant-name {
  color: white;
  font-size: 24px;
  font-weight: 600;
}

.call-status {
  color: rgba(255, 255, 255, 0.7);
  font-size: 16px;
}

.call-timer {
  color: white;
  font-size: 20px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.connection-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(255, 255, 255, 0.2);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.incoming-call-actions,
.calling-actions,
.call-controls {
  display: flex;
  gap: 24px;
  align-items: center;
}

.btn-accept,
.btn-reject,
.btn-cancel,
.control-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 24px;
  border: none;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  min-width: 80px;
}

.btn-accept {
  background: #22c55e;
  color: white;
}

.btn-accept:hover {
  background: #16a34a;
}

.btn-reject,
.btn-cancel,
.control-btn.btn-end {
  background: #ef4444;
  color: white;
}

.btn-reject:hover,
.btn-cancel:hover,
.control-btn.btn-end:hover {
  background: #dc2626;
}

.control-btn {
  background: rgba(255, 255, 255, 0.15);
  color: white;
}

.control-btn:hover {
  background: rgba(255, 255, 255, 0.25);
}

.control-btn.active {
  background: white;
  color: #1f2937;
}

.agent-transcripts {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 320px;
  max-height: 120px;
  overflow-y: auto;
}

.agent-transcript-bubble {
  background: rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  padding: 10px 14px;
  color: rgba(255, 255, 255, 0.9);
  font-size: 14px;
  line-height: 1.4;
  word-break: break-word;
}

audio {
  display: none;
}
</style>
