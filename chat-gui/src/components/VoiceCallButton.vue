<template>
  <button
    v-if="canCall"
    class="voice-call-btn"
    :class="{ 'is-calling': isInCall }"
    :disabled="isInCall && !isCurrentCall"
    @click="handleClick"
    :title="buttonTitle"
  >
    <svg v-if="!isInCall" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z"/>
    </svg>
    <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M10.68 13.31a16 16 0 0 0 3.41 2.6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91"/>
      <line x1="23" y1="1" x2="1" y2="23"/>
    </svg>
    <span v-if="callDuration > 0" class="call-duration">{{ formatDuration(callDuration) }}</span>
  </button>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  chat: {
    type: Object,
    required: true,
  },
  isInCall: {
    type: Boolean,
    default: false,
  },
  isCurrentCall: {
    type: Boolean,
    default: false,
  },
  callDuration: {
    type: Number,
    default: 0,
  },
})

const emit = defineEmits(['initiateCall', 'endCall'])

const canCall = computed(() => {
  return !props.chat.isGroupChat && (props.chat.isZeroClaw || props.chat.peer)
})

const buttonTitle = computed(() => {
  if (props.isInCall && !props.isCurrentCall) return 'In another call'
  if (props.isInCall && props.isCurrentCall) return 'End call'
  return 'Voice call'
})

function handleClick() {
  if (props.isInCall && props.isCurrentCall) {
    emit('endCall')
  } else {
    emit('initiateCall')
  }
}

function formatDuration(seconds) {
  const m = Math.floor(seconds / 60).toString().padStart(2, '0')
  const s = Math.floor(seconds % 60).toString().padStart(2, '0')
  return `${m}:${s}`
}
</script>

<style scoped>
.voice-call-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s;
  position: relative;
}

.voice-call-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.voice-call-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.voice-call-btn.is-calling {
  color: #ef4444;
  animation: pulse 1.5s ease-in-out infinite;
}

.voice-call-btn.is-calling:hover {
  background: #fee2e2;
}

.call-duration {
  position: absolute;
  bottom: -14px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 10px;
  color: #ef4444;
  font-weight: 600;
  white-space: nowrap;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
