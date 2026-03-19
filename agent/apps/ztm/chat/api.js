// Default filter chain applied to every peer (comma-separated filter names)
var DEFAULT_FILTER_CHAIN = 'repeat-message,blocked-keywords'

function firstLine(text) {
  if (typeof text !== 'string') return text
  var idx = text.indexOf('\n')
  return idx === -1 ? text : text.slice(0, idx) + '...'
}

function makeSessionId(peerA, peerB) {
  return peerA < peerB ? peerA + '~' + peerB : peerB + '~' + peerA
}

// Default onSend filter chain (comma-separated filter names)
var DEFAULT_SEND_FILTER_CHAIN = 'credit-delay,suppress-json'

// Base credit for a new peer
var BASE_CREDIT = 100

// Cache of loaded filter functions keyed by name
var filterCache = {}

function loadFilter(name) {
  if (filterCache[name]) return filterCache[name]
  try {
    var mod = pipy.import(`../../../filters/${name}.js`)
    if (typeof mod.default !== 'function') {
      console.error('[chat filter] filter', name, 'does not export a default function')
      return null
    }
    filterCache[name] = mod.default
    return mod.default
  } catch (e) {
    console.error('[chat filter] failed to load filter', name, ':', e?.toString?.() || e)
    return null
  }
}

export default function ({ app, mesh, db, spawnOpenclaw }) {
  var chats = []

  function getPeerConfig(peer) {
    return db.getChatPeer(mesh.name, peer) || { peer, autoReply: false, autoReplyAgent: 'main', credit: BASE_CREDIT, filterChain: '', sendFilterChain: '', isBlocked: false, run: 1, muted: false, thinkingTime: 3 }
  }

  function setPeerConfig(peer, config) {
    db.setChatPeer(mesh.name, peer, config)
  }

  function allPeerConfigs() {
    return db.allChatPeers(mesh.name)
  }

  // Run filter chain for an incoming message.
  // Adjusts the peer's credit in the DB based on filter scores.
  // Returns the updated credit value.
  function onReceive(peer, sender, text) {
    var peerConfig = getPeerConfig(peer)
    var chainStr = peerConfig.filterChain || DEFAULT_FILTER_CHAIN
    var filterNames = chainStr.split(',').map(s => s.trim()).filter(Boolean)

    var ctx = {
      mesh: mesh.name,
      peer,
      sender,
      text,
      db,
    }

    var totalDelta = 0
    filterNames.forEach(function (name) {
      var fn = loadFilter(name)
      if (!fn) return
      try {
        var score = fn(ctx)
        console.info('[chat filter]', name, 'score:', score, 'for peer:', peer)
        if (typeof score === 'number' && score < 0) {
          totalDelta += score
        }
      } catch (e) {
        console.error('[chat filter]', name, 'threw error:', e?.toString?.() || e)
      }
    })

    if (totalDelta < 0) {
      console.info('[chat filter] adjusting credit for', peer, 'by', totalDelta)
      db.adjustCredit(mesh.name, peer, totalDelta)
    }

    // Re-read updated credit
    var updated = db.getChatPeer(mesh.name, peer)
    return updated ? updated.credit : BASE_CREDIT
  }

  // Run onSend filter chain sequentially.
  // Each filter receives ctx and may return:
  //   false         → abort sending
  //   null/undefined → continue to next filter
  //   Promise        → wait for resolution before continuing (resolved value used as return)
  // Returns a Promise that resolves to true (send) or false (abort).
  function onSend(peer, replyText, credit) {
    var peerConfig = getPeerConfig(peer)
    var chainStr = peerConfig.sendFilterChain || DEFAULT_SEND_FILTER_CHAIN
    var filterNames = chainStr.split(',').map(s => s.trim()).filter(Boolean)

    var ctx = {
      mesh: mesh.name,
      peer,
      replyText,
      credit,
      db,
    }

    function runNext(i) {
      if (i >= filterNames.length) return Promise.resolve(true)
      var name = filterNames[i]
      var fn = loadFilter(name)
      if (!fn) return runNext(i + 1)
      var result
      try {
        result = fn(ctx)
      } catch (e) {
        console.error('[send-filter]', name, 'threw error:', e?.toString?.() || e)
        return runNext(i + 1)
      }
      var proceed = function (val) {
        if (val === false) {
          console.info('[send-filter]', name, 'aborted send for peer:', peer)
          return Promise.resolve(false)
        }
        console.info('[send-filter]', name, 'passed for peer:', peer)
        return runNext(i + 1)
      }
      if (result && typeof result.then === 'function') {
        return result.then(proceed)
      }
      return proceed(result)
    }

    return runNext(0)
  }

  function notifyAutoReplySetup(chat) {
    if (!chat.peer) return
    // Only insert once
    if (chat.messages.some(function (m) { return m.isPeerRequest })) return
    var placeholder = { isPeerRequest: true, isSystemHint: true, _placeholder: true }
    chat.messages.push(placeholder)
    getLocalAgentNames().then(function (localAgents) {
      var idx = chat.messages.indexOf(placeholder)
      var time = Date.now()
      var hint = {
        time: time,
        sender: app.username,
        message: { text: 'New chat request from ' + chat.peer + '. Select an agent to auto-reply on your behalf.' },
        isSystemHint: true,
        isPeerRequest: true,
        peer: chat.peer,
        availableAgents: localAgents.map(function (id) { return '' + id }),
      }
      if (idx !== -1) chat.messages.splice(idx, 1, hint)
      else chat.messages.push(hint)
      if (time > chat.updateTime) chat.updateTime = time
    })
  }

  function clearPeerRequestHint(peer) {
    var chat = findPeerChat(peer)
    if (!chat) return
    chat.messages = chat.messages.filter(function (m) { return !m.isPeerRequest })
  }

  // Fetch local openclaw agent names as a Promise resolving to a string[]
  function getLocalAgentNames() {
    try {
      var ids = db.getCache('local_agent_ids')
      if (ids && typeof ids.forEach === 'function') {
        // Re-map into a fresh local array to avoid cross-module serialization issues
        var fresh = []
        ids.forEach(function (id) { fresh.push('' + id) })
        return Promise.resolve(fresh)
      }
    } catch {}
    // cache miss: local_agent_ids is populated by main.js startup and /api/openclaw/agents route
    console.info('[openclaw] local_agent_ids cache miss, waiting for main.js to populate')
    return Promise.resolve([])
  }

  // Key used in chat_peer for a local agent's auto-reply config within a specific group
  function groupAgentKey(gcid, agentName) {
    return gcid + '~' + agentName
  }

  // Remove any group EP request hints from the group chat message list (called after approve)
  function clearGroupEpRequestHint(gcid) {
    var chat = chats.find(function (c) { return c.gcid === gcid })
    if (!chat) return
    chat.messages = chat.messages.filter(function (m) { return !m.isGroupEpRequest })
  }

  // Insert an approve-auto-reply hint into the group chat message list (local only, not published)
  function notifyGroupEpRequest(chat) {
    if (!chat.gcid) return
    // Do the duplicate check synchronously before entering async, to avoid race conditions
    if (chat.messages.some(function (m) { return m.isGroupEpRequest })) return
    // Mark immediately so concurrent calls don't race past the check above
    var placeholder = { isGroupEpRequest: true, _placeholder: true }
    chat.messages.push(placeholder)
    getLocalAgentNames().then(function (localAgents) {
      // Replace placeholder with the real hint
      var idx = chat.messages.indexOf(placeholder)
      var time = Date.now()
      var hint = {
        time: time,
        sender: app.username,
        message: { text: 'You have been added to a group chat. Select an agent to auto-reply on your behalf.' },
        isSystemHint: true,
        isGroupRequest: true,
        isGroupEpRequest: true,
        gcid: chat.gcid,
        groupName: chat.name || '',
        availableAgents: localAgents.map(function (id) { return '' + id }),
      }
      if (idx !== -1) chat.messages.splice(idx, 1, hint)
      else chat.messages.push(hint)
      if (time > chat.updateTime) chat.updateTime = time
    })
  }

  // Insert a group-chat-request hint into the local peer chat for a given agent
  function notifyGroupChatRequest(chat, agentName) {
    if (!chat.gcid) {
      console.info('[group request] skipping hint for agent', agentName, '- group has no gcid yet')
      return
    }
    // Only insert once per agent per group
    var agentChat = findPeerChat(agentName)
    if (!agentChat) agentChat = newPeerChat(agentName)
    var alreadyHinted = false
    for (var i = 0; i < agentChat.messages.length; i++) {
      var m = agentChat.messages[i]
      if (m.isGroupRequest && m.gcid === chat.gcid) { alreadyHinted = true; break }
    }
    if (alreadyHinted) {
      console.info('[group request] hint already present for agent', agentName, 'in group', chat.gcid)
      return
    }
    var time = Date.now()
    var hint = {
      time: time,
      sender: app.username,
      message: {
        text: 'Group chat request: agent "' + agentName + '" has been added to group "' + (chat.name || chat.gcid) + '". Enable auto-reply so it can participate in the group chat.',
      },
      isSystemHint: true,
      isGroupRequest: true,
      gcid: chat.gcid,
      agentName: agentName,
      groupName: chat.name || '',
    }
    agentChat.messages.push(hint)
    if (time > agentChat.updateTime) agentChat.updateTime = time
    agentChat.newCount++
    console.info('[group request] inserted group chat request hint for agent', agentName, 'in group', chat.gcid)
  }

  function triggerGroupAutoReply(chat, msg) {
    if (!spawnOpenclaw) { console.info('[group auto-reply] skip: no spawnOpenclaw'); return }
    if (!chat.group) { console.info('[group auto-reply] skip: no chat.group'); return }
    var gcid = chat.gcid || ''
    var text = typeof msg.message === 'string' ? msg.message : (msg.message?.text || '')
    // Skip auto-reply for image-only messages with no text
    if (!text) { console.info('[group auto-reply] skip: empty text (possibly image-only)'); return }
    var senderField = msg.sender || ''
    // Derive the plain username from a possibly-tagged sender (gcid/username)
    var senderUsername = senderField.indexOf('/') !== -1 ? senderField.split('/')[1] : senderField

    // ── Parse @ mentions ──
    var mentionedMembers = []
    text.split(' ').forEach(function (token) {
      if (token.length > 1 && token.charAt(0) === '@') {
        mentionedMembers.push(token.substring(1))
      }
    })
    var hasMentions = mentionedMembers.length > 0

    // Clean message text: remove all @name tokens and trim
    var cleanedText = hasMentions
      ? text.split(' ').filter(function (token) { return !(token.length > 1 && token.charAt(0) === '@') }).join(' ').trim()
      : text
    var groupName = chat.name || chat.group

    // Build rewritten message for the openclaw agent CLI
    function buildAgentMessage(isMentioned) {
      if (isMentioned) {
        return '在group chat ' + groupName + '里，' + senderUsername + ' 给你发送了信息，给他回复一下，发送的内容是"' + cleanedText + '"'
      } else {
        return '在group chat ' + groupName + '里，' + senderUsername + ' 说话了看看如何回复，说的内容是"' + text + '"'
      }
    }

    console.info('[group auto-reply] triggered | gcid:', gcid, 'members:', JSON.stringify(chat.members), 'sender:', senderUsername, 'self:', app.username, 'mentions:', JSON.stringify(mentionedMembers))
    getLocalAgentNames().then(function (localAgents) {
      // ── 1. Trigger local openclaw agents that are members of this group ──
      chat.members.forEach(function (member) {
        if (member === senderUsername) return   // skip sender
        if (member === app.username) return     // skip self (handled below)
        if (localAgents.indexOf(member) === -1) return  // skip remote EP members

        // @ filter: if mentions exist, only trigger mentioned members
        if (hasMentions && mentionedMembers.indexOf(member) === -1) return

        // Build the rewritten message for this agent
        var agentMsg = buildAgentMessage(hasMentions && mentionedMembers.indexOf(member) !== -1)

        // Local openclaw agent: auto-reply
        var sessionId = gcid
        var cmd = ['openclaw', 'agent', '--agent', member, '--message', agentMsg, '--session-id', sessionId, '--json', '--no-color']
        console.info('[group auto-reply] calling local agent', member, 'for group', gcid || chat.group)
        spawnOpenclaw(cmd).then(
          function (output) {
            var replyText
            try {
              var parsed = JSON.parse(output.split('\n').join(''))
              replyText = parsed?.payloads?.[0]?.text ||
                          parsed?.result?.payloads?.[0]?.text ||
                          parsed?.message || parsed?.content || parsed?.text
            } catch {}
            if (!replyText) replyText = output.split('\n').join('').trim()
            if (!replyText) return
            // muted: log the reply but do not send to ztm chat
            var gcidCfg = gcid ? db.getChatPeer(mesh.name, gcid) : null
            if (gcidCfg && gcidCfg.muted) {
              console.info('[group auto-reply] muted, logging reply without sending for agent', member, 'in group', gcid || chat.group)
              try {
                db.logChat(mesh.name, 'group', chat.group, chat.name, chat.creator, member, 'message',
                  JSON.stringify({ text: replyText, agentName: member }), null, sessionId, true)
              } catch {}
              return
            }
            var thinkingTime1 = getPeerConfig(sessionId).thinkingTime || 3
            new Timeout(thinkingTime1).wait().then(function () {
              console.info('[group auto-reply] agent', member, 'reply to group', gcid || chat.group, ':', firstLine(replyText))
              addGroupMessage(chat.creator, chat.group, { text: replyText, agentName: member }, true, sessionId)
            })
          },
          function (err) {
            console.error('[group auto-reply] openclaw error for agent', member, ':', err?.toString?.() || err)
          }
        )
      })

      // ── 2. Handle self (app.username) as a group member: EP-level auto-reply ──
      // Only triggered when the message was sent by a remote EP (not self, not a local agent)
      if (senderUsername === app.username) return
      if (localAgents.indexOf(senderUsername) !== -1) return
      if (!gcid) return
      // @ filter: if mentions exist and self is not mentioned, skip EP-level auto-reply
      if (hasMentions && mentionedMembers.indexOf(app.username) === -1) return
      var peerConfig = getPeerConfig(gcid)
      if (!peerConfig.autoReply) {
        // Show approve hint in this group's message flow so the user can enable auto-reply
        notifyGroupEpRequest(chat)
        return
      }
      var agentName = peerConfig.autoReplyAgent || 'main'
      var agentMsg2 = buildAgentMessage(hasMentions && mentionedMembers.indexOf(app.username) !== -1)
      var credit = onReceive(gcid, senderUsername, agentMsg2)
      var sessionId = gcid
      var thinkingTime2 = peerConfig.thinkingTime !== undefined ? peerConfig.thinkingTime : 3
      var cmd = ['openclaw', 'agent', '--agent', agentName, '--message', agentMsg2, '--session-id', sessionId, '--json', '--no-color']
      console.info('[group auto-reply] calling agent', agentName, 'for self in group', gcid)
      spawnOpenclaw(cmd).then(
        function (output) {
          var replyText
          try {
            var parsed = JSON.parse(output.split('\n').join(''))
            replyText = parsed?.payloads?.[0]?.text ||
                        parsed?.result?.payloads?.[0]?.text ||
                        parsed?.message || parsed?.content || parsed?.text
          } catch {}
          if (!replyText) replyText = output.split('\n').join('').trim()
          if (!replyText) return
          // muted: log the reply but do not send to ztm chat
          if (peerConfig.muted) {
            console.info('[group auto-reply] muted, logging reply without sending for self in group', gcid)
            try {
              db.logChat(mesh.name, 'group', chat.group, chat.name, chat.creator, app.username, 'message',
                JSON.stringify({ text: replyText, agentName: agentName }), null, sessionId, true)
            } catch {}
            return
          }
          onSend(gcid, replyText, credit).then(function (shouldSend) {
            if (!shouldSend) return
            new Timeout(thinkingTime2).wait().then(function () {
              console.info('[group auto-reply] self reply to group', gcid, 'via agent', agentName, ':', firstLine(replyText))
              // Use app.username as sender so the EP's own name appears in the group chat
              addGroupMessage(chat.creator, chat.group, { text: replyText, agentName: app.username }, true, sessionId)
            })
          })
        },
        function (err) {
          console.error('[group auto-reply] openclaw error for self in group', gcid, ':', err?.toString?.() || err)
        }
      )
    })
  }

  function triggerAutoReply(chat, msg) {
    if (!spawnOpenclaw) return
    if (!chat.peer) return
    var peerConfig = getPeerConfig(chat.peer)
    if (!peerConfig.autoReply) return
    var agentName = peerConfig.autoReplyAgent || 'main'
    var text = typeof msg.message === 'string' ? msg.message : (msg.message?.text || '')
    // Skip auto-reply for image-only messages with no text
    if (!text) { console.info('[auto-reply] skip: empty text (possibly image-only)'); return }
    var sender = msg.sender

    // Run onReceive filters, adjust credit, get updated credit value
    var credit = onReceive(chat.peer, sender, text)
    console.info('[chat auto-reply] credit after onReceive:', credit)

    var sessionId = makeSessionId(app.username, chat.peer)
    var cmd = ['openclaw', 'agent', '--agent', agentName, '--message', text, '--session-id', sessionId, '--json', '--no-color']
    var thinkingTime = peerConfig.thinkingTime !== undefined ? peerConfig.thinkingTime : 3

    console.info('[openclaw cli]', cmd.join(' ').slice(0, 40))
    spawnOpenclaw(cmd).then(
      output => {
        // Log full output to api_log, show abbreviated version in console
        try {
          var parsed = JSON.parse(output.split('\n').join(''))
          var brief = JSON.stringify(parsed).substring(0, 120) + ' ...'
          console.info('[chat auto-reply] openclaw output:', brief)
          db.logApi('auto-reply', cmd.join(' '), {}, text, {}, output)
        } catch {
          console.info('[chat auto-reply] openclaw output:', output.substring(0, 120) + ' ...')
          db.logApi('auto-reply', cmd.join(' '), {}, text, {}, output)
        }
        var replyText
        try {
          var parsed = JSON.parse(output.split('\n').join(''))
          replyText = parsed?.payloads?.[0]?.text ||
                      parsed?.result?.payloads?.[0]?.text ||
                      parsed?.message || parsed?.content || parsed?.text
        } catch {}
        if (!replyText) replyText = output.split('\n').join('').trim()
        if (!replyText) return

        // muted: log the reply to chat_log but do not send to ztm chat
        if (peerConfig.muted) {
          console.info('[chat auto-reply] muted, logging reply without sending to', chat.peer, ':', firstLine(replyText))
          try {
            db.logChat(mesh.name, 'peer', chat.peer, null, null, app.username, 'message',
              JSON.stringify({ text: replyText, agentName: agentName }), null, sessionId, true)
          } catch {}
          return
        }

        // Run onSend filter chain (handles delay + suppress-json + any future filters)
        onSend(chat.peer, replyText, credit).then(function (shouldSend) {
          if (!shouldSend) return
          new Timeout(thinkingTime).wait().then(function () {
            console.info('[chat auto-reply] reply to', chat.peer, ':', firstLine(replyText))
            addPeerMessage(chat.peer, { text: replyText, agentName: agentName }, sessionId)
          })
        })
      },
      err => {
        console.error('[chat auto-reply] openclaw error:', err?.toString?.() || err)
      }
    )
  }

  function findPeerChat(peer) {
    return chats.find(c => c.peer === peer)
  }

  function findGroupChat(creator, group) {
    return chats.find(c => c.creator === creator && c.group === group)
  }

  function findGroupChatByGcid(gcid) {
    return chats.find(c => c.gcid === gcid)
  }

  function newPeerChat(peer) {
    var chat = {
      peer,
      messages: [],
      newCount: 0,
      updateTime: 0,
      checkTime: 0,
    }
    chats.push(chat)
    return chat
  }

  function generateGcid() {
    var chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
    var result = ''
    for (var i = 0; i < 6; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length))
    }
    return result
  }

  function newGroupChat(creator, group, existingGcid) {
    var chat = {
      creator,
      group,
      gcid: existingGcid || generateGcid(),
      name: '',
      members: [],
      messages: [],
      newCount: 0,
      updateTime: 0,
      checkTime: 0,
    }
    chats.push(chat)
    return chat
  }

  function mergeMessages(chat, messages) {
    messages.forEach(msg => {
      var sender = msg.sender
      var time = msg.time
      if (!chat.messages.find(m => m.time === time && m.sender === sender)) {
        chat.messages.push(msg)
        if (sender !== app.username) chat.newCount++
        if (time > chat.updateTime) chat.updateTime = time
        // Write to chat_log — only for messages from others (own messages are logged in add*Message)
        var msgText = typeof msg.message === 'string' ? msg.message : (msg.message?.text || JSON.stringify(msg.message))
          if (sender !== app.username) {
            console.info('[chat recv]', app.username, '<-', sender, ':', firstLine(msgText))
            triggerAutoReply(chat, msg)
            try {
              if (chat.peer) {
                var recvSessionId = makeSessionId(sender, app.username)
                db.logChat(mesh.name, 'peer', chat.peer, null, null, sender, 'message',
                  typeof msg.message === 'string' ? msg.message : JSON.stringify(msg.message),
                  null, recvSessionId)
              } else if (chat.group) {
                var senderUsername = sender.indexOf('/') !== -1 ? sender.split('/')[1] : sender
                var recvSessionId = chat.gcid ? makeSessionId(senderUsername, chat.gcid) : null
                db.logChat(mesh.name, 'group', chat.group, chat.name, chat.creator, sender, 'message',
                  typeof msg.message === 'string' ? msg.message : JSON.stringify(msg.message),
                  null, recvSessionId)
              }
            } catch {}
          }
        // For group chats, trigger local agents regardless of who sent the message.
        // Local agents are virtual members of this EP — we proxy their participation.
        // Skip if this message was itself posted by a local agent (avoid infinite loop).
        var isAgentReply = typeof msg.message === 'object' && !!msg.message?.agentName
        if (chat.group && !isAgentReply) {
          triggerGroupAutoReply(chat, msg)
        }
      }
    })
  }

  //
  // /apps/ztm/chat
  //   /users
  //     /<username>
  //   /shared
  //     /<username>
  //       /publish
  //         /peers
  //           /<username>
  //             /messages
  //               /<timestamp>.json
  //         /groups
  //           /<creator>
  //             /<uuid>
  //               /info.json
  //               /messages
  //                 /<timestamp>.json
  //         /files
  //           /<hash>
  //

  var matchPublishPeerMsgs = new http.Match('/shared/{sender}/publish/peers/{receiver}/messages/*')
  var matchPublishGroupInfo = new http.Match('/shared/{sender}/publish/groups/{creator}/{group}/info.json')
  var matchPublishGroupMsgs = new http.Match('/shared/{sender}/publish/groups/{creator}/{group}/messages/*')

  mesh.acl(`/shared/${app.username}/publish`, { all: 'block' })
  mesh.acl(`/shared/${app.username}/publish/files`, { all: 'readonly' })

  mesh.list('/shared').then(paths => Promise.all(
    Object.keys(paths).map(path => readMessages(path, true))
  )).then(
    () => watchMessages()
  )

  function watchMessages() {
    mesh.watch('/shared').then(paths => Promise.all(
      paths.map(path => readMessages(path, false))
    ).then(() => watchMessages()))
  }

  function readMessages(path, initial) {
    var params = matchPublishPeerMsgs(path)
    if (params) {
      return mesh.read(path).then(data => {
        if (data) {
          var me = app.username
          var sender = params.sender
          var receiver = params.receiver
          if (receiver === me) {
            var peer = sender
          } else if (sender === me) {
            var peer = receiver
          } else {
            return
          }
          var chat = findPeerChat(peer)
          if (!chat) chat = newPeerChat(peer)
          try {
            var messages = JSON.decode(data)
            messages.forEach(msg => msg.sender = sender)
            var hasIncoming = messages.some(m => m.sender !== app.username)
            if (hasIncoming && !initial) {
              // run=0: discard messages entirely, do not store
              var peerCfg = db.getChatPeer(mesh.name, chat.peer)
              if (peerCfg && !peerCfg.run) {
                console.info('[chat] peer stopped, discarding messages from', peer)
                return
              }
              // is_blocked: log, reply "You are blocked", do not store or process further
              if (peerCfg && peerCfg.isBlocked) {
                console.info('[chat] peer blocked, replying "You are blocked" to', peer)
                messages.forEach(msg => {
                  if (msg.sender !== app.username) {
                    var msgText = typeof msg.message === 'string' ? msg.message : (msg.message?.text || JSON.stringify(msg.message))
                    console.info('[chat recv blocked]', app.username, '<-', msg.sender, ':', msgText)
                    var recvSessionId = makeSessionId(msg.sender, app.username)
                    try {
                      db.logChat(mesh.name, 'peer', chat.peer, null, null, msg.sender, 'message', msgText, null, recvSessionId)
                    } catch {}
                  }
                })
                addPeerMessage(chat.peer, { text: 'You are blocked' })
                return
              }
              var existingConfig = db.getChatPeer(mesh.name, chat.peer)
              if (!existingConfig) {
                db.setChatPeer(mesh.name, chat.peer, { autoReply: false, autoReplyAgent: 'main' })
              }
              // If any incoming message carries an agentName, record it as the peer's agent
              messages.forEach(msg => {
                if (msg.sender !== app.username) {
                  var msgAgentName = typeof msg.message === 'object' ? msg.message?.agentName : null
                  if (msgAgentName) {
                    db.setChatPeer(mesh.name, chat.peer, { peerAgentName: msgAgentName })
                  }
                }
              })
            }
            mergeMessages(chat, messages)
            if (hasIncoming) {
              var cfg = db.getChatPeer(mesh.name, chat.peer)
              if (cfg && !cfg.autoReply && !chat.messages.some(m => m.isPeerRequest)) {
                notifyAutoReplySetup(chat)
              }
            }
          } catch {}
          if (initial) chat.newCount = 0
        }
      })
    }
    var params = matchPublishGroupInfo(path)
    if (params) {
      return mesh.read(path).then(data => {
        if (data) {
          var sender = params.sender
          var creator = params.creator
          var group = params.group
          if (sender !== creator) return
          if (isGroupDismissed(creator, group)) return
          var chat = findGroupChat(creator, group)
          if (!chat) chat = newGroupChat(creator, group)
          try {
            var info = JSON.decode(data)
            if (info.name) chat.name = info.name
            if (info.members instanceof Array) chat.members = info.members
            // Restore gcid from info.json; only overwrite if info has one (creator's node sets it)
            if (info.gcid) chat.gcid = info.gcid
          } catch {}
        }
      })
    }
    var params = matchPublishGroupMsgs(path)
    if (params) {
      return mesh.read(path).then(data => {
        if (!data) return
        var pathSender = params.sender
        var creator = params.creator
        var group = params.group
        if (isGroupDismissed(creator, group)) return
        var chat = findGroupChat(creator, group)
        // If no local chat found, read info.json asynchronously to get gcid and members
        var infoPromise
        if (!chat) {
          var infoPath = `/shared/${creator}/publish/groups/${creator}/${group}/info.json`
          infoPromise = mesh.read(infoPath).then(function (infoData) {
            var existingGcid = null
            var existingMembers = null
            var existingName = null
            try {
              if (infoData) {
                var info = JSON.decode(infoData)
                existingGcid = info.gcid || null
                existingMembers = info.members instanceof Array ? info.members : null
                existingName = info.name || null
              }
            } catch {}
            chat = newGroupChat(creator, group, existingGcid)
            if (existingMembers) chat.members = existingMembers
            if (existingName) chat.name = existingName
          })
        } else {
          infoPromise = Promise.resolve()
        }
        return infoPromise.then(function () {
          var gcid = chat.gcid
          var groupCfg = gcid ? db.getChatPeer(mesh.name, gcid) : null
          // run=0: discard messages entirely, do not store
          if (groupCfg && !groupCfg.run) {
            console.info('[chat] group stopped, discarding messages for group', group)
            return
          }
          // is_blocked: silently discard, do not store or process
          if (groupCfg && groupCfg.isBlocked) {
            console.info('[chat] group blocked, silently discarding messages for group', group)
            return
          }
          var newMsgs = []
          try {
            var messages = JSON.decode(data)
            messages.forEach(msg => {
              // Prefer the tagged sender embedded in the message payload (gcid/username format);
              // fall back to the path-level sender for backwards compatibility.
              var embeddedSender = typeof msg.message === 'object' ? msg.message?.sender : null
              msg.sender = embeddedSender || pathSender
            })
            var beforeCount = chat.messages.length
            mergeMessages(chat, messages)
            // Collect only the newly merged messages for auto-reply
            if (!initial) {
              newMsgs = chat.messages.slice(beforeCount)
            }
          } catch {}
          if (initial) chat.newCount = 0
        })
      })
    }
    return Promise.resolve()
  }

  var publishQueue = []
  var isPublishing = false

  function publishMessage(dirname, message) {
    publishQueue.push([dirname, message])
    if (!isPublishing) {
      isPublishing = true
      publishNext()
    }
  }

  function publishNext() {
    if (publishQueue.length === 0) return (isPublishing = false)
    var item = publishQueue.shift();
    var dirname = item[0]
    var message = item[1]
    mesh.dir(dirname).then(filenames => {
      var timestamps = filenames.filter(
        name => name.endsWith('.json')
      ).map(
        name => Number.parseFloat(name.substring(0, name.length - 5))
      ).filter(
        n => !Number.isNaN(n)
      )

      var time = Date.now()
      var t = time
      var i = timestamps.findLastIndex(ts => t >= ts)
      if (i < 0) {
        t = t - 60 * 1000
      } else {
        t = timestamps[i]
      }

      var filename = os.path.join(dirname, `${t}.json`)
      return mesh.read(filename).then(
        data => {
          if (data && data.size >= 1024 && time > timestamps[timestamps.length - 1]) {
            filename = os.path.join(dirname, `${time}.json`)
            data = null
          }
          if (!data) {
            return mesh.write(filename, JSON.encode([{ time, message }]))
          }
          try {
            var list = JSON.decode(data)
          } catch {}
          if (!(list instanceof Array)) list = []
          list.push({ time, message })
          return mesh.write(filename, JSON.encode(list))
        }
      )
    }).then(() => publishNext())
  }

  function allEndpoints() {
    return mesh.discover()
  }

  function allUsers() {
    return mesh.discover().then(
      endpoints => {
        var users = []
        var set = new Set
        endpoints.forEach(ep => {
          var user = ep.username
          if (!set.has(user)) {
            users.push(user)
            set.add(user)
          }
        })
        return users.sort()
      }
    )
  }

  function allChats() {
    return Promise.resolve(chats.filter(
      chat => chat.peer || (chat.group && chat.name)
    ).map(
      chat => {
        var updated = chat.newCount
        var latest = chat.messages.length > 0
          ? chat.messages.reduce((a, b) => a.time > b.time ? a : b)
          : null
        if (chat.peer) {
          var peerCfg = db.getChatPeer(mesh.name, chat.peer)
          return {
            peer: chat.peer,
            peerAgentName: (peerCfg && peerCfg.peerAgentName) || '',
            time: chat.updateTime,
            updated,
            latest,
          }
        } else {
          return {
            creator: chat.creator,
            group: chat.group,
            gcid: chat.gcid || '',
            name: chat.name,
            members: chat.members || [],
            time: chat.updateTime,
            updated,
            latest,
          }
        }
      }
    ))
  }

  function allPeerMessages(peer, since, before) {
    var chat = findPeerChat(peer)
    if (chat) {
      chat.checkTime = chat.updateTime
      chat.newCount = 0
      // Filter out peer request hints if auto-reply is already approved
      var peerConfig = getPeerConfig(peer)
      if (peerConfig && peerConfig.autoReply) {
        chat.messages = chat.messages.filter(function (m) { return !m.isPeerRequest })
      }
      return Promise.resolve(getMessagesBetween(chat.messages, since, before))
    } else {
      return Promise.resolve(null)
    }
  }

  function allGroupMessages(creator, group, since, before) {
    var chat = findGroupChat(creator, group)
    if (chat) {
      chat.checkTime = chat.updateTime
      chat.newCount = 0
      // Filter out EP request hints if auto-reply is already approved (e.g. after page refresh)
      var gcid = chat.gcid
      if (gcid) {
        var peerConfig = getPeerConfig(gcid)
        if (peerConfig && peerConfig.autoReply) {
          chat.messages = chat.messages.filter(function (m) { return !m.isGroupEpRequest })
        }
      }
      return Promise.resolve(getMessagesBetween(chat.messages, since, before))
    } else {
      return Promise.resolve(null)
    }
  }

  function getMessagesBetween(messages, since, before) {
    since = since || 0
    before = before || Number.POSITIVE_INFINITY
    return messages.filter(
      msg => since <= msg.time && msg.time <= before
    )
  }

  function addPeerMessage(peer, message, sessionId) {
    if (!peer) return Promise.resolve(false)
    var msgText = typeof message === 'string' ? message : (message?.text || JSON.stringify(message))
    console.info('[chat send]', app.username, '->', peer, ':', firstLine(msgText))
    var dirname = `/shared/${app.username}/publish/peers/${peer}/messages`
    return mesh.acl(dirname, { users: { [peer]: 'readonly' }}).then(
      () => publishMessage(dirname, message)
    ).then(() => {
      try {
        var sid = sessionId || makeSessionId(app.username, peer)
        db.logChat(mesh.name, 'peer', peer, null, null, app.username, 'message',
          typeof message === 'string' ? message : JSON.stringify(message), null, sid)
      } catch {}
      return true
    })
  }

  function getGroup(creator, group) {
    var chat = findGroupChat(creator, group)
    if (chat) {
      return Promise.resolve({
        creator,
        group,
        name: chat.name,
        members: chat.members,
      })
    } else {
      return Promise.resolve(null)
    }
  }

  function setGroup(creator, group, info) {
    if (creator !== app.username) return Promise.resolve(false)
    var chat = findGroupChat(creator, group)
    var isNew = !chat
    if (!chat) chat = newGroupChat(creator, group)
    if (info.name) chat.name = info.name
    if (info.members instanceof Array) chat.members = info.members
    // Ensure gcid is registered in chat_peer so auto-reply config can be stored per group
    if (isNew) {
      db.setChatPeer(mesh.name, chat.gcid, { autoReply: false, autoReplyAgent: 'main', peerName: info.name || '' })
    } else if (info.name) {
      db.setChatPeer(mesh.name, chat.gcid, { peerName: info.name })
    }
    // Create chat_peer records for local agent members (peer = gcid~agentName)
    if (info.members instanceof Array) {
      getLocalAgentNames().then(function (localAgents) {
        info.members.forEach(function (member) {
          if (member === app.username) return
          if (localAgents.indexOf(member) === -1) return
          var key = groupAgentKey(chat.gcid, member)
          if (!db.getChatPeer(mesh.name, key)) {
            db.setChatPeer(mesh.name, key, { autoReply: true, autoReplyAgent: member, peerName: (info.name || '') + ' / ' + member })
          }
        })
      })
    }
    var dirname = `/shared/${creator}/publish/groups/${creator}/${group}`
    return mesh.acl(dirname, { users: Object.fromEntries(chat.members.map(name => [name, 'readonly'])) }).then(
      () => mesh.write(os.path.join(dirname, 'info.json'), JSON.encode(chat))
    ).then(() => {
      try {
        db.logChat(mesh.name, 'group', group, chat.name, creator, app.username,
          isNew ? 'group_create' : 'group_update', null, chat.members)
      } catch {}
      return true
    })
  }

  var dismissedGroupsCache = {}

  function groupDismissedKey(creator, group) {
    return 'group_dismissed:' + creator + ':' + group
  }

  function isGroupDismissed(creator, group) {
    var key = groupDismissedKey(creator, group)
    if (dismissedGroupsCache[key]) return true
    try {
      var val = !!db.getCache(key)
      if (val) dismissedGroupsCache[key] = true
      return val
    } catch { return false }
  }

  function markGroupDismissed(creator, group) {
    var key = groupDismissedKey(creator, group)
    dismissedGroupsCache[key] = true
    try {
      db.setCache(key, true)
      console.info('[group dismiss] marked dismissed:', creator, group)
    } catch (e) {
      console.error('[group dismiss] failed to persist dismissed flag:', creator, group, e?.toString?.() || e)
    }
  }

  function delGroup(creator, group) {
    if (creator !== app.username) return Promise.resolve(false)
    var chat = findGroupChat(creator, group)
    if (!chat) return Promise.resolve(false)

    // Mark dismissed and remove from memory FIRST (synchronously), before async mesh ops
    var idx = chats.indexOf(chat)
    if (idx >= 0) chats.splice(idx, 1)
    markGroupDismissed(creator, group)
    try {
      db.logChat(mesh.name, 'group', group, chat.name, creator, app.username,
        'group_delete', null, chat.members)
    } catch {}

    // Then try to clean up mesh filesystem (best-effort)
    var dirname = `/shared/${creator}/publish/groups/${creator}/${group}`
    var msgDir = os.path.join(dirname, 'messages')
    // Delete message files inside messages/ subdirectory first
    return mesh.dir(msgDir).then(function (msgFiles) {
      return Promise.all((msgFiles || []).map(function (f) { return mesh.unlink(os.path.join(msgDir, f)) }))
    }).catch(function () {}).then(function () {
      // Then delete top-level files (info.json, messages/ dir itself, etc.)
      return mesh.dir(dirname).then(function (files) {
        return Promise.all((files || []).map(function (f) { return mesh.unlink(os.path.join(dirname, f)) }))
      })
    }).catch(function () {}).then(function () {
      return true
    })
  }

  function leaveGroup(creator, group) {
    var chat = findGroupChat(creator, group)
    if (!chat) return Promise.resolve(false)
    // Remove from in-memory list and mark dismissed in db
    var idx = chats.indexOf(chat)
    if (idx >= 0) chats.splice(idx, 1)
    markGroupDismissed(creator, group)
    try {
      db.logChat(mesh.name, 'group', group, chat.name, creator, app.username,
        'group_leave', null, chat.members)
    } catch {}
    return Promise.resolve(true)
  }

  // fromAgent: true means this message is an auto-reply from a local agent — do not re-trigger
  function addGroupMessage(creator, group, message, fromAgent, sessionId) {
    var chat = findGroupChat(creator, group)
    if (!chat) return Promise.resolve(false)
    if (app.username !== creator && !chat.members.includes(app.username)) return Promise.resolve(false)
    // Embed sender as [gcid]/[username] inside the message payload.
    // For agent replies use the agent's own name as the username part so the UI shows the agent.
    var gcid = chat.gcid || ''
    var senderName = (fromAgent && message && message.agentName) ? message.agentName : app.username
    var taggedSender = gcid ? (gcid + '/' + senderName) : senderName
    var taggedMessage = typeof message === 'string'
      ? { text: message, sender: taggedSender }
      : Object.assign({}, message, { sender: taggedSender })
    var msgText = typeof message === 'string' ? message : (message?.text || JSON.stringify(message))
    console.info('[chat send]', taggedSender, '-> group', group, ':', firstLine(msgText))
    var dirname = `/shared/${app.username}/publish/groups/${creator}/${group}`
    return mesh.acl(dirname, { users: Object.fromEntries(chat.members.map(name => [name, 'readonly'])) }).then(
      () => publishMessage(os.path.join(dirname, 'messages'), taggedMessage)
    ).then(() => {
      try {
        var sid = sessionId || (chat.gcid ? makeSessionId(app.username, chat.gcid) : null)
        db.logChat(mesh.name, 'group', group, chat.name, creator, taggedSender, 'message',
          msgText, null, sid)
      } catch {}
      return true
    })
  }

  function addFile(data) {
    var h = new crypto.Hash('sha256')
    h.update(data)
    h.update(data.size.toString())
    var hash = h.digest().toString('hex')
    return mesh.write(`/shared/${app.username}/publish/files/${hash}`, data).then(
      () => hash
    )
  }

  function computeHash(data) {
    var h = new crypto.Hash('sha256')
    h.update(data)
    h.update(data.size.toString())
    return h.digest().toString('hex')
  }

  function addFileToSession(data, sessionId, fileName) {
    var hash = computeHash(data)
    var dir = os.path.join(os.home(), '.openclaw', 'workspace', 'clawparty', 'files', sessionId)
    try { os.mkdir(dir, { recursive: true }) } catch {}
    var filepath = os.path.join(dir, hash)
    try {
      os.write(filepath, data)
      return Promise.resolve({ hash, path: filepath, name: fileName })
    } catch (e) {
      return Promise.reject(new Error(e?.toString?.() || 'write failed'))
    }
  }

  function getFileFromSession(sessionId, hash) {
    var filepath = os.path.join(os.home(), '.openclaw', 'workspace', 'clawparty', 'files', sessionId, hash)
    try {
      var data = os.read(filepath)
      if (data) return Promise.resolve(data)
      return Promise.resolve(null)
    } catch {
      return Promise.resolve(null)
    }
  }

  function getFile(owner, hash) {
    return mesh.read(`/shared/${owner}/publish/files/${hash}`)
  }

  function delFile(owner, hash) {
    mesh.unlink(`/shared/${owner}/publish/files/${hash}`)
    return Promise.resolve(true)
  }

  function addGroupMessageByGcid(gcid, message) {
    var chat = findGroupChatByGcid(gcid)
    if (!chat) return Promise.resolve(null)
    return addGroupMessage(chat.creator, chat.group, message).then(ok => ok ? chat : null)
  }

  function getGroupByGcid(gcid) {
    var chat = findGroupChatByGcid(gcid)
    if (!chat) return Promise.resolve(null)
    return Promise.resolve({ gcid: chat.gcid, creator: chat.creator, group: chat.group, name: chat.name, members: chat.members })
  }

  return {
    allEndpoints,
    allUsers,
    allChats,
    allPeerMessages,
    addPeerMessage,
    allGroupMessages,
    getGroup,
    getGroupByGcid,
    setGroup,
    delGroup,
    leaveGroup,
    addGroupMessage,
    addGroupMessageByGcid,
    addFile,
    getFile,
    delFile,
    getPeerConfig,
    setPeerConfig,
    allPeerConfigs,
    clearGroupEpRequestHint,
    clearPeerRequestHint,
  }
}
