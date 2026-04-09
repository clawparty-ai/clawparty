import initAPI from './api.js'
import initCLI from './cli.js'
import db from '../../../db.js'

// Module-level static pipeline for running openclaw commands — must be defined at top level in Pipy
var $openclawCmd
var $openclawOutput
var $openclawStartTime
var $openclawExitCode = 0
var $openclawErrorMessage = ''
var openclawExec = pipeline($=>$
  .onStart(cmd => { $openclawCmd = cmd; $openclawStartTime = Date.now(); return new Data })
  .exec(() => $openclawCmd, {
    stderr: true,
    onExit: (code, err) => {
      $openclawExitCode = code
      if (err) {
        var errStr = err.toString()
        // Skip JSON output (normal --json result), only print real errors
        var hasJsonBraces = errStr.indexOf('{') !== -1 && errStr.indexOf('}') !== -1
        if (!hasJsonBraces) {
          $openclawErrorMessage = errStr
          errStr.split('\n').filter(Boolean).forEach(
            line => console.error('[openclaw]', line)
          )
        }
      }
      return new StreamEnd
    }
  })
  .replaceStreamStart(evt => [new MessageStart, evt])
  .replaceStreamEnd(() => new MessageEnd)
  .replaceMessage(msg => {
    $openclawOutput = msg?.body?.toString?.() || ''
    return new StreamEnd
  })
  .onEnd(() => {
    var durationMs = Date.now() - $openclawStartTime
    var success = $openclawExitCode === 0 && !$openclawErrorMessage
    db.logCliCall($openclawCmd[0], $openclawCmd.slice(1), $openclawOutput, $openclawExitCode, success, durationMs, $openclawErrorMessage)
    $openclawErrorMessage = ''
    return $openclawOutput
  })
)

export default function ({ app, mesh, utils }) {

  // ── Openclaw call queue ──────────────────────────────────────────────────
  // All openclaw CLI calls are serialised through this queue to prevent
  // gateway timeouts caused by concurrent requests. Failed jobs are retried
  // up to maxRetries times by appending them back to the tail of the queue.
  var jobQueue = []
  var isQueueRunning = false

  // Extract agentId from command for queue management
  // Handles both: openclaw agent --agent <name> and openclaw gateway call agent --params <json>
  function getAgentIdFromCmd(cmd) {
    var agentIdx = cmd.indexOf('--agent')
    if (agentIdx >= 0 && cmd[agentIdx + 1]) {
      return cmd[agentIdx + 1]
    }
    var paramsIdx = cmd.indexOf('--params')
    if (paramsIdx >= 0 && cmd[paramsIdx + 1]) {
      try {
        var params = JSON.parse(cmd[paramsIdx + 1])
        return params.agentId || null
      } catch { return null }
    }
    return null
  }

  // Extract sessionId from command for logging
  function getSessionIdFromCmd(cmd) {
    var sessionIdx = cmd.indexOf('--session-id')
    if (sessionIdx >= 0 && cmd[sessionIdx + 1]) {
      return cmd[sessionIdx + 1]
    }
    var paramsIdx = cmd.indexOf('--params')
    if (paramsIdx >= 0 && cmd[paramsIdx + 1]) {
      try {
        var params = JSON.parse(cmd[paramsIdx + 1])
        return params.sessionId || null
      } catch { return null }
    }
    return null
  }

  // Queue stats: return string describing current queue state
  function queueStats() {
    if (jobQueue.length === 0) return 'empty'
    return 'length:' + jobQueue.length + ' jobs: ' +
      jobQueue.map(function (j) {
        return (getAgentIdFromCmd(j.cmd) || '?') + '(retry:' + j.retries + ')'
      }).join(' ')
  }

  function runNextJob() {
    if (jobQueue.length === 0) { isQueueRunning = false; return }
    isQueueRunning = true
    var job = jobQueue.shift()
    var agentId = getAgentIdFromCmd(job.cmd)
    var sessionId = getSessionIdFromCmd(job.cmd)
    console.info('[queue][pop] agentId=' + (agentId || 'unknown') + ' session=' + (sessionId || 'N/A') + ' queue-remaining:' + jobQueue.length)
    console.info('[queue][stats]', queueStats())

    // Out-of-queue merging: collect all pending jobs for the same agentId
    var mergedJobs = []
    if (agentId) {
      var remaining = []
      jobQueue.forEach(function (j) {
        if (getAgentIdFromCmd(j.cmd) === agentId) {
          mergedJobs.push(j)
        } else {
          remaining.push(j)
        }
      })
      jobQueue = remaining
    }

    // Merge messages if there are pending jobs for the same agent
    var originalMessage = null
    if (mergedJobs.length > 0) {
      console.info('[queue][merge] agentId=' + agentId + ' absorbing ' + mergedJobs.length + ' jobs queue-remaining:' + jobQueue.length)
      var paramsIdx = job.cmd.indexOf('--params')
      if (paramsIdx >= 0 && job.cmd[paramsIdx + 1]) {
        try {
          var params = JSON.parse(job.cmd[paramsIdx + 1])
          originalMessage = params.message
          var messages = [params.message]
          mergedJobs.forEach(function (j) {
            var pi = j.cmd.indexOf('--params')
            if (pi >= 0 && j.cmd[pi + 1]) {
              try {
                messages.push(JSON.parse(j.cmd[pi + 1]).message)
              } catch {}
            }
          })
          params.message = '以下是连续收到的消息，请一并处理：\n' +
            messages.map(function (m, i) { return (i + 1) + '. ' + m }).join('\n')
          job.cmd[paramsIdx + 1] = JSON.stringify(params)
          console.info('[queue][merge] merged message chars:' + params.message.length)
        } catch {}
      }
    }

    var attempt = job.retries + 1
    console.info('[queue][run] agentId=' + (agentId || 'unknown') + ' session=' + (sessionId || 'N/A') + ' attempt:' + attempt + '/3')
    var startTs = Date.now()

    openclawExec.spawn(job.cmd).then(
      function (output) {
        console.info('[queue][done] agentId=' + (agentId || 'unknown') + ' durationMs:' + (Date.now() - startTs) + ' queue-remaining:' + jobQueue.length)
        console.info('[queue][stats]', queueStats())
        job.resolve(output)
        // Resolve all merged jobs with the same output
        mergedJobs.forEach(function (j) { j.resolve(output) })
        runNextJob()
      },
      function (err) {
        if (job.retries < job.maxRetries) {
          job.retries++
          console.warn('[queue][retry] agentId=' + (agentId || 'unknown') + ' attempt:' + job.retries + '/' + job.maxRetries + ' queue-length:' + jobQueue.length)
          console.info('[queue][stats]', queueStats())
          jobQueue.push(job)
          // Discard merged jobs on retry (already stale)
          mergedJobs.forEach(function (j) { j.resolve('') })
        } else {
          console.error('[queue][drop] agentId=' + (agentId || 'unknown') + ' exhausted ' + job.maxRetries + ' retries')
          console.info('[queue][stats]', queueStats())
          job.reject(err)
          mergedJobs.forEach(function (j) { j.reject(err) })
        }
        runNextJob()
      }
    )
  }

  var spawnOpenclaw = function (cmd) {
    var agentId = getAgentIdFromCmd(cmd)
    var sessionId = getSessionIdFromCmd(cmd)
    return new Promise(function (resolve, reject) {
      jobQueue.push({ cmd: cmd, resolve: resolve, reject: reject, retries: 0, maxRetries: 2 })
      console.info('[queue][enqueue] agentId=' + (agentId || 'unknown') + ' session=' + (sessionId || 'N/A') + ' queue-length:' + jobQueue.length)
      if (!isQueueRunning) runNextJob()
    })
  }
  // ────────────────────────────────────────────────────────────────────────

  var api = initAPI({ app, mesh, db, spawnOpenclaw })
  var cli = initCLI({ app, mesh, utils, api })

  var $ctx

  var gui = new http.Directory(os.path.join(app.root, 'gui'))
  var response = utils.createResponse
  var responder = utils.createResponder

  var serveUser = utils.createServer({
    '/cli': {
      'CONNECT': utils.createCLIResponder(cli),
    },

    '/api/appinfo': {
      'GET': responder(() => Promise.resolve(response(200, {
        name: app.name,
        provider: app.provider,
        username: app.username,
        endpoint: app.endpoint,
      })))
    },

    '/api/endpoints': {
      'GET': responder(() => api.allEndpoints().then(
        ret => ret ? response(200, ret) : response(404)
      ))
    },

    '/api/users': {
      'GET': responder(() => api.allUsers().then(
        ret => ret ? response(200, ret) : response(404)
      ))
    },

    '/api/chats': {
      'GET': responder(() => api.allChats().then(
        ret => ret ? response(200, ret) : response(404)
      ))
    },

    '/api/peers/{peer}/messages': {
      'GET': responder((params, req) => {
        var peer = URL.decodeComponent(params.peer)
        var url = new URL(req.head.path)
        var since = url.searchParams.get('since')
        var before = url.searchParams.get('before')
        if (since) since = Number.parseFloat(since)
        if (before) before = Number.parseFloat(before)
        return api.allPeerMessages(peer, since, before).then(
          ret => ret ? response(200, ret) : response(404)
        )
      }),

      'POST': responder((params, req) => {
        var peer = URL.decodeComponent(params.peer)
        var body = JSON.decode(req.body)
        var sessionId = body.sessionId || null
        return api.addPeerMessage(peer, body, sessionId).then(
          ret => response(ret ? 201 : 404)
        )
      }),
    },

    '/api/groups/{creator}/{group}': {
      'GET': responder((params) => {
        var creator = URL.decodeComponent(params.creator)
        var group = URL.decodeComponent(params.group)
        return api.getGroup(creator, group).then(
          ret => ret ? response(200, ret) : response(404)
        )
      }),

      'POST': responder((params, req) => {
        var creator = URL.decodeComponent(params.creator)
        var group = URL.decodeComponent(params.group)
        return api.setGroup(creator, group, JSON.decode(req.body)).then(
          ret => response(ret ? 201 : 403)
        )
      }),

      // Creator: DELETE destroys the group for everyone
      // Member:  DELETE ?leave=1 exits the group locally
      'DELETE': responder((params, req) => {
        var creator = URL.decodeComponent(params.creator)
        var group = URL.decodeComponent(params.group)
        var url = new URL(req.head.path)
        var leave = url.searchParams.get('leave') === '1'
        if (leave) {
          return api.leaveGroup(creator, group).then(
            ret => response(ret ? 204 : 404)
          )
        } else {
          return api.delGroup(creator, group).then(
            ret => response(ret ? 204 : 403)
          )
        }
      }),
    },

    '/api/groups/{creator}/{group}/messages': {
      'GET': responder((params, req) => {
        var creator = URL.decodeComponent(params.creator)
        var group = URL.decodeComponent(params.group)
        var url = new URL(req.head.path)
        var since = url.searchParams.get('since')
        var before = url.searchParams.get('before')
        if (since) since = Number.parseFloat(since)
        if (before) before = Number.parseFloat(before)
        return api.allGroupMessages(creator, group, since, before).then(
          ret => ret ? response(200, ret) : response(404)
        )
      }),

      'POST': responder((params, req) => {
        var creator = URL.decodeComponent(params.creator)
        var group = URL.decodeComponent(params.group)
        var body = JSON.decode(req.body)
        var sessionId = body.sessionId || null
        return api.addGroupMessage(creator, group, body, false, sessionId).then(
          ret => response(ret ? 201 : 404)
        )
      }),
    },

    '/api/groupchat/{gcid}': {
      'GET': responder((params) => {
        var gcid = URL.decodeComponent(params.gcid)
        return api.getGroupByGcid(gcid).then(
          ret => ret ? response(200, ret) : response(404, { message: 'group not found: ' + gcid })
        )
      }),

      'POST': responder((params, req) => {
        var gcid = URL.decodeComponent(params.gcid)
        var body
        try { body = JSON.decode(req.body) } catch {
          return Promise.resolve(response(400, { message: 'invalid request body' }))
        }
        var message = body.message || body.text || body
        if (!message) return Promise.resolve(response(400, { message: 'missing message field' }))
        return api.addGroupMessageByGcid(gcid, { text: message }).then(
          ret => ret ? response(201, { gcid: ret.gcid, group: ret.group, name: ret.name }) : response(404, { message: 'group not found: ' + gcid })
        )
      }),
    },

    '/api/groupchat/{gcid}/agents/{agentName}/auto-reply': {
      'POST': responder((params) => {
        var gcid = URL.decodeComponent(params.gcid)
        var agentName = URL.decodeComponent(params.agentName)
        var key = gcid + '~' + agentName
        api.setPeerConfig(key, { autoReply: true, autoReplyAgent: agentName })
        console.info('[group auto-reply] approved for agent', agentName, 'in group', gcid)
        return Promise.resolve(response(200, { gcid, agentName, autoReply: true }))
      }),

      'DELETE': responder((params) => {
        var gcid = URL.decodeComponent(params.gcid)
        var agentName = URL.decodeComponent(params.agentName)
        var key = gcid + '~' + agentName
        api.setPeerConfig(key, { autoReply: false })
        return Promise.resolve(response(200, { gcid, agentName, autoReply: false }))
      }),
    },

    '/api/groupchat/{gcid}/auto-reply': {
      'GET': responder((params) => {
        var gcid = URL.decodeComponent(params.gcid)
        return Promise.resolve(response(200, api.getPeerConfig(gcid)))
      }),

      'POST': responder((params, req) => {
        var gcid = URL.decodeComponent(params.gcid)
        var body
        try { body = JSON.decode(req.body) } catch { body = {} }
        var agentName = body.agent || 'main'
        api.setPeerConfig(gcid, { autoReply: true, autoReplyAgent: agentName })
        api.clearGroupEpRequestHint(gcid)
        console.info('[group auto-reply] EP auto-reply enabled for group', gcid, 'agent:', agentName)
        return Promise.resolve(response(200, api.getPeerConfig(gcid)))
      }),

      'DELETE': responder((params) => {
        var gcid = URL.decodeComponent(params.gcid)
        api.setPeerConfig(gcid, { autoReply: false })
        console.info('[group auto-reply] EP auto-reply disabled for group', gcid)
        return Promise.resolve(response(200, api.getPeerConfig(gcid)))
      }),
    },

    '/api/peers/{peer}/auto-reply': {
      'GET': responder((params) => {
        var peer = URL.decodeComponent(params.peer)
        return Promise.resolve(response(200, api.getPeerConfig(peer)))
      }),

      'POST': responder((params, req) => {
        var peer = URL.decodeComponent(params.peer)
        var body
        try { body = JSON.decode(req.body) } catch { body = {} }
        api.setPeerConfig(peer, body)
        if (body.autoReply) api.clearPeerRequestHint(peer)
        return Promise.resolve(response(200, api.getPeerConfig(peer)))
      }),
    },

    '/api/peers/{peer}/half-rewrite': {
      'POST': responder((params, req) => {
        var peer = URL.decodeComponent(params.peer)
        var body
        try { body = JSON.decode(req.body) } catch { body = {} }
        var draftText = body.draftText || ''
        var humanHint = body.humanHint || ''
        var sessionId = body.sessionId || ''
        if (!draftText) return Promise.resolve(response(400, JSON.stringify({ error: 'draftText required' })))
        return api.halfAutomationRewrite(peer, draftText, humanHint, sessionId).then(
          replyText => response(200, JSON.stringify({ text: replyText }))
        ).catch(
          e => response(500, JSON.stringify({ error: e?.toString?.() || 'rewrite failed' }))
        )
      }),
    },

    '/api/auto-reply': {
      'GET': responder(() => Promise.resolve(response(200, api.allPeerConfigs()))),
    },

    '/api/default-auto-reply': {
      'GET': responder(() => Promise.resolve(response(200, { agent: api.getDefaultAutoReplyAgent() }))),

      'POST': responder((_, req) => {
        var body
        try { body = JSON.decode(req.body) } catch { body = {} }
        var agentName = body.agent || 'main'
        api.setDefaultAutoReplyAgent(agentName)
        return Promise.resolve(response(200, { agent: agentName }))
      }),
    },

    '/api/files': {
      'POST': responder((_, req) => {
        return api.addFile(req.body).then(
          hash => hash ? response(200, hash) : response(404)
        )
      }),
    },

    '/api/files/upload': {
      'POST': responder((_, req) => {
        var url = new URL(req.head.path, 'http://localhost')
        var sessionId = url.searchParams.get('sessionId') || ''
        var fileName = url.searchParams.get('name') || 'file'
        if (!sessionId) return Promise.resolve(response(400, JSON.stringify({ error: 'sessionId required' })))
        return api.addFileToSession(req.body, sessionId, fileName).then(
          ret => ret ? response(200, JSON.stringify(ret)) : response(404)
        ).catch(
          e => response(500, JSON.stringify({ error: e?.toString?.() || 'upload failed' }))
        )
      }),
    },

    '/api/files/upload/{sessionId}/{hash}': {
      'GET': responder((params) => {
        var sessionId = params.sessionId
        var hash = params.hash
        return api.getFileFromSession(sessionId, hash).then(
          ret => ret ? new Message({ status: 200 }, ret) : response(404)
        )
      }),
    },

    '/api/files/{owner}/{hash}': {
      'GET': responder((params) => {
        var owner = params.owner
				// Fix safari <audio> need file extension
        var hash = params.hash.split(".")[0]
        return api.getFile(owner, hash).then(
          ret => ret ? response(200, ret) : response(404)
        )
      }),

      'DELETE': responder((params) => {
        var owner = params.owner
        var hash = params.hash
        return api.delFile(owner, hash).then(
          ret => response(ret ? 204 : 404)
        )
      }),
    },

    '*': {
      'GET': responder((_, req) => {
        return Promise.resolve(gui.serve(req) || response(404))
      })
    },
  })

  var servePeer = utils.createServer({
  })

  return pipeline($=>$
    .onStart(c => void ($ctx = c))
    .pipe(() => {
      switch ($ctx.source) {
        case 'user': return serveUser
        case 'peer': return servePeer
      }
    })
  )
}
