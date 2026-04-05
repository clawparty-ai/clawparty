// Picoclaw ZTM App
// Treats Picoclaw as a SINGLE chat participant in ClawParty

export default function ({ app, mesh, db }) {
  var picoclawPath = 'picoclaw'
  var agentName = 'picoclaw'
  var isOnline = false

  // Check if Picoclaw is available
  function checkPicoclaw() {
    return new Promise((resolve) => {
      var output = ''
      pipeline($=>$
        .onStart([picoclawPath, 'status'])
        .exec(() => [picoclawPath, 'status'], {
          stderr: true,
          onExit: (code) => {
            isOnline = code === 0
            resolve(isOnline)
          }
        })
        .replaceMessage(msg => {
          output += msg?.body?.toString?.() || ''
          return new StreamEnd
        })
      ).spawn()
    })
  }

  // Chat with Picoclaw (the single agent)
  function chat(message, sessionId) {
    return new Promise((resolve, reject) => {
      var output = ''
      var cmd = [picoclawPath, 'agent', '-m', message, '-s', sessionId]

      console.info('[picoclaw cli]', cmd.join(' ').slice(0, 200))

      pipeline($=>$
        .onStart(cmd)
        .exec(() => cmd, {
          stderr: true,
          onExit: (code, err) => {
            if (code === 0) {
              resolve(output.trim())
            } else {
              reject(err || 'Picoclaw command failed')
            }
          }
        })
        .replaceMessage(msg => {
          output += msg?.body?.toString?.() || ''
          return new StreamEnd
        })
      ).spawn()
    })
  }

  // App initialization
  app.onStart = function () {
    console.info('[picoclaw] Initializing Picoclaw ZTM app')
    return checkPicoclaw().then(online => {
      console.info('[picoclaw] Status:', online ? 'available' : 'unavailable')
      if (online) {
        db.setCache('picoclaw_status', 'online')
      }
    })
  }

  // API endpoints exposed by this ZTM app
  return {
    // Identity of this Picoclaw endpoint
    get identity() {
      return {
        name: agentName,
        type: 'picoclaw',
        status: isOnline ? 'online' : 'offline'
      }
    },

    // Health check
    'GET /api/picoclaw/health': function () {
      return checkPicoclaw().then(
        online => response(online ? 200 : 503, JSON.stringify({
          status: online ? 'online' : 'offline',
          agent: agentName
        }))
      )
    },

    // Chat with Picoclaw
    'POST /api/picoclaw/chat': function (_, req) {
      var body
      try {
        body = JSON.decode(req.body)
      } catch {
        return response(400, JSON.stringify({ error: 'invalid JSON' }))
      }

      var message = body?.message || ''
      var sessionId = body?.session_id || 'clawparty:' + Date.now()

      if (!message) {
        return response(400, JSON.stringify({ error: 'message is required' }))
      }

      if (!isOnline) {
        return response(503, JSON.stringify({ error: 'Picoclaw is offline' }))
      }

      return chat(message, sessionId).then(
        reply => response(200, JSON.stringify({
          agent: agentName,
          response: reply,
          session_id: sessionId
        })),
        err => response(500, JSON.stringify({ error: err?.toString?.() || err }))
      )
    }
  }
}