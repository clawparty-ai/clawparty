const DEFAULT_ICE_SERVERS = [
  { urls: 'stun:stun.l.google.com:19302' },
  { urls: 'stun:stun1.l.google.com:19302' },
]

export class WebRTCService {
  constructor(options) {
    this.iceServers = options.iceServers || DEFAULT_ICE_SERVERS
    this.onLocalStream = options.onLocalStream || (() => {})
    this.onRemoteStream = options.onRemoteStream || (() => {})
    this.onIceCandidate = options.onIceCandidate || (() => {})
    this.onConnectionStateChange = options.onConnectionStateChange || (() => {})
    this.onTrack = options.onTrack || (() => {})
    this.onError = options.onError || (() => {})
    this.onNegotiationNeeded = options.onNegotiationNeeded || (() => {})

    this.peerConnection = null
    this.localStream = null
    this.remoteStream = null
    this.isAudioEnabled = true
  }

  async initialize() {
    try {
      this.localStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
          sampleRate: 48000,
        },
        video: false,
      })

      this.onLocalStream(this.localStream)
      return this.localStream
    } catch (err) {
      console.error('[WebRTCService] getUserMedia error:', err)
      this.onError('media_denied', err)
      throw err
    }
  }

  createPeerConnection() {
    if (this.peerConnection) {
      this.close()
    }

    this.peerConnection = new RTCPeerConnection({
      iceServers: this.iceServers,
      iceCandidatePoolSize: 10,
    })

    this.peerConnection.onicecandidate = (event) => {
      if (event.candidate) {
        this.onIceCandidate(event.candidate)
      }
    }

    this.peerConnection.ontrack = (event) => {
      this.remoteStream = event.streams[0]
      this.onRemoteStream(this.remoteStream)
      this.onTrack(event)
    }

    this.peerConnection.onconnectionstatechange = () => {
      const state = this.peerConnection.connectionState
      console.log('[WebRTCService] Connection state:', state)
      this.onConnectionStateChange(state)
    }

    this.peerConnection.onnegotiationneeded = () => {
      console.log('[WebRTCService] Negotiation needed')
      this.onNegotiationNeeded()
    }

    this.peerConnection.oniceconnectionstatechange = () => {
      console.log('[WebRTCService] ICE state:', this.peerConnection.iceConnectionState)
    }

    if (this.localStream) {
      this.localStream.getTracks().forEach(track => {
        this.peerConnection.addTrack(track, this.localStream)
      })
    }

    return this.peerConnection
  }

  async createOffer() {
    if (!this.peerConnection) {
      this.createPeerConnection()
    }

    const offer = await this.peerConnection.createOffer({
      offerToReceiveAudio: true,
      offerToReceiveVideo: false,
      voiceActivityDetection: true,
    })

    await this.peerConnection.setLocalDescription(offer)
    return offer
  }

  async createAnswer(offer) {
    if (!this.peerConnection) {
      this.createPeerConnection()
    }

    await this.peerConnection.setRemoteDescription(new RTCSessionDescription(offer))
    const answer = await this.peerConnection.createAnswer()
    await this.peerConnection.setLocalDescription(answer)
    return answer
  }

  async setRemoteDescription(desc) {
    if (!this.peerConnection) {
      throw new Error('PeerConnection not initialized')
    }
    await this.peerConnection.setRemoteDescription(new RTCSessionDescription(desc))
  }

  async addIceCandidate(candidate) {
    if (!this.peerConnection) {
      return
    }
    if (this.peerConnection.remoteDescription) {
      await this.peerConnection.addIceCandidate(new RTCIceCandidate(candidate))
    }
  }

  enableAudio() {
    if (this.localStream) {
      this.localStream.getAudioTracks().forEach(track => {
        track.enabled = true
      })
      this.isAudioEnabled = true
    }
  }

  disableAudio() {
    if (this.localStream) {
      this.localStream.getAudioTracks().forEach(track => {
        track.enabled = false
      })
      this.isAudioEnabled = false
    }
  }

  toggleAudio() {
    if (this.isAudioEnabled) {
      this.disableAudio()
    } else {
      this.enableAudio()
    }
    return this.isAudioEnabled
  }

  close() {
    if (this.localStream) {
      this.localStream.getTracks().forEach(track => track.stop())
      this.localStream = null
    }

    if (this.remoteStream) {
      this.remoteStream = null
    }

    if (this.peerConnection) {
      this.peerConnection.close()
      this.peerConnection = null
    }

    this.isAudioEnabled = true
  }
}
