export class SpeechService {
  constructor() {
    this.recognition = null
    this.synth = window.speechSynthesis || null
    this.isListening = false
    this.isSpeaking = false
    this.transcript = ''
    this.interimTranscript = ''
    this.transcriptCallback = null
    this.speakingEndCallback = null
  }

  static isSupported() {
    return 'SpeechRecognition' in window || 'webkitSpeechRecognition' in window
  }

  static isTTSSupported() {
    return 'speechSynthesis' in window
  }

  startListening(options = {}) {
    if (!SpeechService.isSupported()) {
      console.warn('[SpeechService] Speech recognition not supported')
      return false
    }

    if (this.isListening) {
      this.stopListening()
    }

    const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition
    this.recognition = new SpeechRecognition()
    this.recognition.lang = options.language || 'zh-CN'
    this.recognition.continuous = options.continuous !== false
    this.recognition.interimResults = options.interimResults !== false

    this.recognition.onstart = () => {
      this.isListening = true
      console.log('[SpeechService] Listening started')
    }

    this.recognition.onresult = (event) => {
      let finalTranscript = ''
      let interimTranscript = ''

      for (let i = event.resultIndex; i < event.results.length; i++) {
        const transcript = event.results[i][0].transcript
        if (event.results[i].isFinal) {
          finalTranscript += transcript
        } else {
          interimTranscript += transcript
        }
      }

      if (finalTranscript) {
        this.transcript = finalTranscript
        if (this.transcriptCallback) {
          this.transcriptCallback(finalTranscript, true)
        }
      }

      if (interimTranscript) {
        this.interimTranscript = interimTranscript
        if (this.transcriptCallback) {
          this.transcriptCallback(interimTranscript, false)
        }
      }
    }

    this.recognition.onerror = (event) => {
      console.error('[SpeechService] Recognition error:', event.error)
      if (event.error === 'not-allowed') {
        this.isListening = false
      }
    }

    this.recognition.onend = () => {
      this.isListening = false
      console.log('[SpeechService] Listening ended')
    }

    try {
      this.recognition.start()
      return true
    } catch (err) {
      console.error('[SpeechService] Failed to start recognition:', err)
      return false
    }
  }

  stopListening() {
    if (this.recognition) {
      try {
        this.recognition.stop()
      } catch (e) {}
      this.recognition = null
    }
    this.isListening = false
    this.interimTranscript = ''
  }

  onTranscript(callback) {
    this.transcriptCallback = callback
  }

  static getBestVoice(synth, lang = 'zh-CN') {
    if (!synth) return null
    const voices = synth.getVoices()
    if (!voices || voices.length === 0) {
      console.warn('[SpeechService] No voices available yet')
      return null
    }
    const prefix = lang.toLowerCase().split('-')[0]
    const exact = voices.find(v => v.lang.toLowerCase() === lang.toLowerCase() && v.localService)
      || voices.find(v => v.lang.toLowerCase() === lang.toLowerCase())
    const partial = voices.find(v => v.lang.toLowerCase().startsWith(prefix) && v.localService)
      || voices.find(v => v.lang.toLowerCase().startsWith(prefix))
    const fallback = voices[0]
    const chosen = exact || partial || fallback
    if (chosen) {
      console.log('[SpeechService] Selected voice:', chosen.name, chosen.lang, 'local=' + chosen.localService)
    }
    return chosen
  }

  speak(text, options = {}) {
    if (!SpeechService.isTTSSupported()) {
      console.warn('[SpeechService] TTS not supported')
      return false
    }

    if (!text || !text.trim()) {
      console.warn('[SpeechService] TTS: empty text')
      return false
    }

    console.log('[SpeechService] TTS speak:', text.substring(0, 80) + (text.length > 80 ? '...' : ''))

    this.stopSpeaking()

    // Chrome sometimes pauses the synthesis engine; resume it first
    if (this.synth) {
      if (this.synth.paused) {
        console.log('[SpeechService] Resuming paused synthesis engine')
        this.synth.resume()
      }
      if (this.synth.pending || this.synth.speaking) {
        this.synth.cancel()
      }
    }

    const utterance = new SpeechSynthesisUtterance(text)
    utterance.lang = options.language || 'zh-CN'
    utterance.rate = options.rate || 1.0
    utterance.pitch = options.pitch || 1.0

    const voice = SpeechService.getBestVoice(this.synth, utterance.lang)
    if (voice) {
      utterance.voice = voice
    } else {
      console.warn('[SpeechService] No voice found for', utterance.lang, '— TTS may fail silently')
    }

    utterance.onstart = () => {
      console.log('[SpeechService] TTS started speaking')
      this.isSpeaking = true
    }

    utterance.onend = () => {
      console.log('[SpeechService] TTS finished speaking')
      this.isSpeaking = false
      if (this.speakingEndCallback) {
        this.speakingEndCallback()
      }
    }

    utterance.onerror = (event) => {
      console.error('[SpeechService] TTS error:', event.error, event)
      this.isSpeaking = false
    }

    this.synth.speak(utterance)
    return true
  }

  stopSpeaking() {
    if (this.synth) {
      this.synth.cancel()
    }
    this.isSpeaking = false
  }

  onSpeakingEnd(callback) {
    this.speakingEndCallback = callback
  }
}
