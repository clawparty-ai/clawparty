import { ref, watch } from 'vue'
import { VoiceService } from '../services/voiceService'
import { WebRTCService } from '../services/webRTCService'
import { SpeechService } from '../services/speechService'

export function useVoiceCall(sendFn, currentUserName, sendChatFn) {
  const voiceService = new VoiceService(sendFn, currentUserName)
  const webRTCService = ref(null)
  const remoteStream = ref(null)
  const error = ref(null)
  const isAgentMode = ref(false)
  const speechService = ref(null)
  const agentTranscripts = ref([])
  const isSpeaking = ref(false)

  watch(() => voiceService.error.value, (err) => {
    if (err) error.value = err
  })

  // ── Agent voice mode: STT → chat → TTS ──────────────────────────────────

  function startAgentVoiceSession(targetName) {
    if (!voiceService.canInitiateCall.value) {
      console.warn('[useVoiceCall] Cannot initiate agent call')
      return false
    }

    isAgentMode.value = true
    agentTranscripts.value = []

    voiceService.initiateCall(targetName, null)
    voiceService.setConnected()

    speechService.value = new SpeechService()

    speechService.value.onTranscript((text, isFinal) => {
      if (isFinal && text.trim()) {
        console.log('[AgentVoice] STT result:', text)
        agentTranscripts.value.push(text)

        // Send to agent via normal chat
        if (sendChatFn) {
          sendChatFn(text)
        }
      }
    })

    speechService.value.onSpeakingEnd(() => {
      isSpeaking.value = false
      // TTS finished, resume listening
      if (voiceService.state.value === 'connected' && speechService.value) {
        setTimeout(() => {
          speechService.value.startListening()
        }, 300)
      }
    })

    // Start listening
    const started = speechService.value.startListening()
    if (!started) {
      console.error('[AgentVoice] Failed to start speech recognition')
      endCall()
      return false
    }

    console.log('[AgentVoice] Session started with', targetName)
    return true
  }

  function handleAgentResponse(text) {
    console.log('[useVoiceCall] handleAgentResponse called, isAgentMode:', isAgentMode.value, 'text length:', text?.length)
    if (!isAgentMode.value || !speechService.value) {
      console.warn('[useVoiceCall] handleAgentResponse skipped — isAgentMode:', isAgentMode.value, 'speechService:', !!speechService.value)
      return
    }
    if (!text || !text.trim()) {
      console.warn('[useVoiceCall] handleAgentResponse skipped — empty text')
      return
    }

    // Stop listening while speaking
    speechService.value.stopListening()
    isSpeaking.value = true

    // Speak the agent's response
    const ok = speechService.value.speak(text.trim())
    console.log('[useVoiceCall] TTS speak returned:', ok)
  }

  // ── P2P WebRTC mode ──────────────────────────────────────────────────────

  async function initiateP2PCall(targetName) {
    if (!voiceService.canInitiateCall.value) {
      console.warn('[useVoiceCall] Cannot initiate P2P call')
      return false
    }

    isAgentMode.value = false

    webRTCService.value = new WebRTCService({
      onRemoteStream: (stream) => {
        remoteStream.value = stream
      },
      onIceCandidate: (candidate) => {
        voiceService.sendIceCandidate(candidate)
      },
      onConnectionStateChange: (state) => {
        console.log('[useVoiceCall] Connection state:', state)
        if (state === 'connected') {
          voiceService.setConnected()
        } else if (state === 'failed' || state === 'disconnected' || state === 'closed') {
          voiceService.setConnectionFailed()
        }
      },
      onError: (code, err) => {
        console.error('[useVoiceCall] WebRTC error:', code, err)
        error.value = code
        voiceService.endCall('error')
      },
    })

    try {
      await webRTCService.value.initialize()
      await webRTCService.value.createOffer()
      const offer = webRTCService.value.peerConnection.localDescription

      voiceService.initiateCall(targetName, offer.sdp)
      return true
    } catch (err) {
      console.error('[useVoiceCall] Initiate call error:', err)
      error.value = 'init_failed'
      cleanup()
      return false
    }
  }

  async function acceptCall(sdpOffer) {
    if (!voiceService.currentCall.value) {
      console.warn('[useVoiceCall] No current call to accept')
      return false
    }

    if (isAgentMode.value) return false

    webRTCService.value = new WebRTCService({
      onRemoteStream: (stream) => {
        remoteStream.value = stream
      },
      onIceCandidate: (candidate) => {
        voiceService.sendIceCandidate(candidate)
      },
      onConnectionStateChange: (state) => {
        if (state === 'connected') {
          voiceService.setConnected()
        } else if (state === 'failed' || state === 'disconnected' || state === 'closed') {
          voiceService.setConnectionFailed()
        }
      },
      onError: (code, err) => {
        console.error('[useVoiceCall] WebRTC error:', code, err)
        error.value = code
        voiceService.endCall('error')
      },
    })

    try {
      await webRTCService.value.initialize()
      const offer = { type: 'offer', sdp: sdpOffer }
      const answer = await webRTCService.value.createAnswer(offer)

      voiceService.acceptCall(
        voiceService.currentCall.value.callId,
        answer.sdp
      )
      return true
    } catch (err) {
      console.error('[useVoiceCall] Accept call error:', err)
      error.value = 'accept_failed'
      voiceService.rejectCall(voiceService.currentCall.value.callId, 'error')
      cleanup()
      return false
    }
  }

  function rejectCall() {
    if (voiceService.currentCall.value) {
      voiceService.rejectCall(voiceService.currentCall.value.callId, 'declined')
    }
  }

  function endCall() {
    voiceService.endCall('hangup')
    cleanup()
  }

  function handleIncomingMessage(msg) {
    if (!msg.type || !msg.type.startsWith('voice-')) return

    if (isAgentMode.value) return

    if (msg.type === 'voice-ice-candidate') {
      if (webRTCService.value && msg.payload) {
        webRTCService.value.addIceCandidate({
          candidate: msg.payload.candidate,
          sdpMid: msg.payload.sdpMid,
          sdpMLineIndex: msg.payload.sdpMLineIndex,
        })
      }
      return
    }

    if (msg.type === 'voice-accept' && msg.payload?.sdpAnswer) {
      if (webRTCService.value) {
        webRTCService.value.setRemoteDescription({
          type: 'answer',
          sdp: msg.payload.sdpAnswer,
        })
      }
    }

    voiceService.handleIncomingMessage(msg)
  }

  function toggleMute() {
    const muted = voiceService.toggleMute()
    if (isAgentMode.value && speechService.value) {
      if (muted) {
        speechService.value.stopListening()
      } else {
        speechService.value.startListening()
      }
    }
    if (webRTCService.value) {
      if (muted) {
        webRTCService.value.disableAudio()
      } else {
        webRTCService.value.enableAudio()
      }
    }
  }

  function toggleSpeaker() {
    voiceService.toggleSpeaker()
  }

  function cleanup() {
    if (speechService.value) {
      speechService.value.stopListening()
      speechService.value.stopSpeaking()
      speechService.value = null
    }
    if (webRTCService.value) {
      webRTCService.value.close()
      webRTCService.value = null
    }
    remoteStream.value = null
    error.value = null
    isAgentMode.value = false
    isSpeaking.value = false
    agentTranscripts.value = []
  }

  return {
    state: voiceService.state,
    currentCall: voiceService.currentCall,
    isMuted: voiceService.isMuted,
    isSpeakerOn: voiceService.isSpeakerOn,
    callDuration: voiceService.callDuration,
    isInCall: voiceService.isInCall,
    canInitiateCall: voiceService.canInitiateCall,
    isAgentMode,
    isSpeaking,
    agentTranscripts,
    error,
    remoteStream,
    // Methods
    initiateP2PCall,
    startAgentVoiceSession,
    handleAgentResponse,
    acceptCall,
    rejectCall,
    endCall,
    toggleMute,
    toggleSpeaker,
    handleIncomingMessage,
    cleanup,
  }
}
