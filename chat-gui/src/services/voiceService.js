import { ref, computed } from 'vue'
import { VOICE_MESSAGE_TYPES } from './chatService'

export class VoiceService {
  constructor(sendFn, currentUserName) {
    this.sendFn = sendFn
    this.currentUserName = currentUserName
    this.state = ref('idle')
    this.currentCall = ref(null)
    this.isMuted = ref(false)
    this.isSpeakerOn = ref(false)
    this.callDuration = ref(0)
    this.error = ref(null)
    this.durationTimer = null
    this.callTimeout = null
  }

  get isInCall() {
    return computed(() => {
      return this.state.value === 'calling' ||
        this.state.value === 'ringing' ||
        this.state.value === 'receiving' ||
        this.state.value === 'connecting' ||
        this.state.value === 'connected'
    })
  }

  get canInitiateCall() {
    return computed(() => this.state.value === 'idle')
  }

  generateCallId() {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
      const r = Math.random() * 16 | 0
      const v = c === 'x' ? r : (r & 0x3 | 0x8)
      return v.toString(16)
    })
  }

  initiateCall(to, sdpOffer) {
    if (this.state.value !== 'idle') {
      console.warn('[VoiceService] Cannot initiate call - not idle:', this.state.value)
      return false
    }

    const callId = this.generateCallId()
    this.currentCall.value = {
      callId,
      from: this.currentUserName,
      to,
      startTime: null,
      endTime: null,
    }

    this.state.value = 'calling'
    this.error.value = null

    this.sendFn(VOICE_MESSAGE_TYPES.INVITE, {
      callId,
      from: this.currentUserName,
      to,
      payload: {
        mediaType: 'audio',
        sdpOffer,
        iceServers: [
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' },
        ],
      },
    })

    this.callTimeout = setTimeout(() => {
      if (this.state.value === 'calling') {
        this.endCall('timeout')
      }
    }, 30000)

    return true
  }

  acceptCall(callId, sdpAnswer) {
    if (this.state.value !== 'receiving') {
      console.warn('[VoiceService] Cannot accept - not receiving:', this.state.value)
      return false
    }

    if (!this.currentCall.value || this.currentCall.value.callId !== callId) {
      console.warn('[VoiceService] Call ID mismatch')
      return false
    }

    this.state.value = 'connecting'
    this.clearCallTimeout()

    this.sendFn(VOICE_MESSAGE_TYPES.ACCEPT, {
      callId,
      from: this.currentUserName,
      to: this.currentCall.value.from,
      payload: { sdpAnswer },
    })

    return true
  }

  rejectCall(callId, reason = 'declined') {
    if (this.state.value !== 'receiving') {
      return false
    }

    if (!this.currentCall.value || this.currentCall.value.callId !== callId) {
      return false
    }

    this.sendFn(VOICE_MESSAGE_TYPES.REJECT, {
      callId,
      from: this.currentUserName,
      to: this.currentCall.value.from,
      payload: { reason },
    })

    this.cleanup()
    return true
  }

  endCall(reason = 'hangup') {
    if (this.currentCall.value) {
      this.sendFn(VOICE_MESSAGE_TYPES.END, {
        callId: this.currentCall.value.callId,
        from: this.currentUserName,
        to: this.currentCall.value.to || this.currentCall.value.from,
        payload: { reason },
      })
    }

    this.cleanup()
  }

  sendIceCandidate(candidate) {
    if (!this.currentCall.value) return

    this.sendFn(VOICE_MESSAGE_TYPES.ICE_CANDIDATE, {
      callId: this.currentCall.value.callId,
      from: this.currentUserName,
      to: this.currentCall.value.to || this.currentCall.value.from,
      payload: {
        candidate: candidate.candidate,
        sdpMid: candidate.sdpMid,
        sdpMLineIndex: candidate.sdpMLineIndex,
      },
    })
  }

  handleIncomingMessage(msg) {
    if (!msg.type || !msg.type.startsWith('voice-')) return

    const { type, callId, from, to, payload } = msg

    switch (type) {
      case VOICE_MESSAGE_TYPES.INVITE:
        if (this.state.value !== 'idle') {
          this.sendFn(VOICE_MESSAGE_TYPES.BUSY, {
            callId,
            from: this.currentUserName,
            to: from,
          })
          return
        }

        this.currentCall.value = {
          callId,
          from,
          to: this.currentUserName,
          startTime: null,
          endTime: null,
          payload,
        }
        this.state.value = 'receiving'
        this.error.value = null

        this.callTimeout = setTimeout(() => {
          if (this.state.value === 'receiving') {
            this.rejectCall(callId, 'timeout')
          }
        }, 30000)
        break

      case VOICE_MESSAGE_TYPES.ACCEPT:
        if (this.state.value !== 'calling') return
        if (!this.currentCall.value || this.currentCall.value.callId !== callId) return

        this.state.value = 'connecting'
        this.clearCallTimeout()
        break

      case VOICE_MESSAGE_TYPES.REJECT:
        if (this.state.value !== 'calling') return
        if (!this.currentCall.value || this.currentCall.value.callId !== callId) return

        this.error.value = payload?.reason || 'rejected'
        this.clearCallTimeout()
        this.cleanup()
        break

      case VOICE_MESSAGE_TYPES.BUSY:
        if (this.state.value !== 'calling') return
        if (!this.currentCall.value || this.currentCall.value.callId !== callId) return

        this.error.value = 'busy'
        this.clearCallTimeout()
        this.cleanup()
        break

      case VOICE_MESSAGE_TYPES.END:
        if (!this.currentCall.value || this.currentCall.value.callId !== callId) return

        this.error.value = payload?.reason || 'ended'
        this.cleanup()
        break

      case VOICE_MESSAGE_TYPES.ICE_CANDIDATE:
        if (!this.currentCall.value || this.currentCall.value.callId !== callId) return
        break
    }
  }

  setConnected() {
    if (this.state.value !== 'connecting') return

    this.state.value = 'connected'
    this.currentCall.value.startTime = Date.now()
    this.callDuration.value = 0

    this.durationTimer = setInterval(() => {
      if (this.currentCall.value && this.currentCall.value.startTime) {
        this.callDuration.value = Math.floor(
          (Date.now() - this.currentCall.value.startTime) / 1000
        )
      }
    }, 1000)
  }

  setConnectionFailed() {
    if (this.state.value !== 'connecting') return
    this.error.value = 'connection_failed'
    this.cleanup()
  }

  toggleMute() {
    this.isMuted.value = !this.isMuted.value
    return this.isMuted.value
  }

  toggleSpeaker() {
    this.isSpeakerOn.value = !this.isSpeakerOn.value
    return this.isSpeakerOn.value
  }

  clearCallTimeout() {
    if (this.callTimeout) {
      clearTimeout(this.callTimeout)
      this.callTimeout = null
    }
  }

  cleanup() {
    this.clearCallTimeout()

    if (this.durationTimer) {
      clearInterval(this.durationTimer)
      this.durationTimer = null
    }

    if (this.currentCall.value) {
      this.currentCall.value.endTime = Date.now()
    }

    this.state.value = 'idle'
    this.currentCall.value = null
    this.isMuted.value = false
    this.isSpeakerOn.value = false
    this.callDuration.value = 0

    setTimeout(() => {
      this.error.value = null
    }, 3000)
  }
}
