// Default filter chain applied to every peer (comma-separated filter names)
var DEFAULT_FILTER_CHAIN = 'repeat-message,blocked-keywords'

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
    return db.getChatPeer(mesh.name, peer) || { peer, autoReply: false, autoReplyAgent: 'main', credit: BASE_CREDIT, filterChain: '', sendFilterChain: '' }
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
    var hint = `Auto-reply is currently disabled for this conversation. ` +
      `You can enable it with: clawparty chat auto-reply ${chat.peer} --enable [--agent <name>]`
    var time = Date.now()
    chat.messages.push({ time, message: { text: hint }, sender: app.username, isSystemHint: true })
    if (time > chat.updateTime) chat.updateTime = time
  }

  // Fetch local openclaw agent names as a Promise resolving to a string[]
  function getLocalAgentNames() {
    if (!spawnOpenclaw) return Promise.resolve([])
    return spawnOpenclaw(['openclaw', 'agents', 'list', '--json']).then(
      output => {
        try {
          var list = JSON.parse(output.split('\n').join(''))
          if (Array.isArray(list)) {
            return list.map(a => a.identityName || a.id || a.name).filter(Boolean)
          }
        } catch {}
        return []
      },
      () => []
    )
  }

  function triggerGroupAutoReply(chat, msg) {
    if (!spawnOpenclaw) return
    if (!chat.group) return
    var gcid = chat.gcid || ''
    var text = typeof msg.message === 'string' ? msg.message : (msg.message?.text || JSON.stringify(msg.message))
    var senderField = msg.sender || ''
    // Derive the plain username from a possibly-tagged sender (gcid/username)
    var senderUsername = senderField.indexOf('/') !== -1 ? senderField.split('/')[1] : senderField

    getLocalAgentNames().then(function (localAgents) {
      chat.members.forEach(function (member) {
        // Skip the sender themselves
        if (member === senderUsername) return
        // Skip ourselves (our own messages are not auto-replied to)
        if (member === app.username) return

        var isLocalAgent = localAgents.indexOf(member) !== -1

        if (isLocalAgent) {
          // ── Local openclaw agent: call it and post reply back to the group ──
          var sessionId = gcid + '~' + member
          var cmd = ['openclaw', 'agent', '--agent', member, '--message', text, '--session-id', sessionId, '--json']
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
              console.info('[group auto-reply] agent', member, 'reply to group', gcid || chat.group, ':', replyText)
              addGroupMessage(chat.creator, chat.group, { text: replyText, agentName: member })
            },
            function (err) {
              console.error('[group auto-reply] openclaw error for agent', member, ':', err?.toString?.() || err)
            }
          )
        } else {
          // ── ZTM EP member: check their auto-reply config via chat_peer (keyed by gcid) ──
          if (!gcid) return
          var peerConfig = getPeerConfig(gcid)
          if (!peerConfig.autoReply) return
          var agentName = peerConfig.autoReplyAgent || 'main'
          var credit = onReceive(gcid, senderUsername, text)
          var sessionId = gcid + '~' + app.username
          var cmd = ['openclaw', 'agent', '--agent', agentName, '--message', text, '--session-id', sessionId, '--json']
          console.info('[group auto-reply] calling agent', agentName, 'for EP member', member, 'in group', gcid)
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
              onSend(gcid, replyText, credit).then(function (shouldSend) {
                if (!shouldSend) return
                console.info('[group auto-reply] EP member', member, 'reply to group', gcid, ':', replyText)
                addGroupMessage(chat.creator, chat.group, { text: replyText, agentName: agentName })
              })
            },
            function (err) {
              console.error('[group auto-reply] openclaw error for EP member', member, ':', err?.toString?.() || err)
            }
          )
        }
      })
    })
  }

  function triggerAutoReply(chat, msg) {
    if (!spawnOpenclaw) return
    if (!chat.peer) return
    var peerConfig = getPeerConfig(chat.peer)
    if (!peerConfig.autoReply) return
    var agentName = peerConfig.autoReplyAgent || 'main'
    var text = typeof msg.message === 'string' ? msg.message : (msg.message?.text || JSON.stringify(msg.message))
    var sender = msg.sender

    // Run onReceive filters, adjust credit, get updated credit value
    var credit = onReceive(chat.peer, sender, text)
    console.info('[chat auto-reply] credit after onReceive:', credit)

    var sessionId = chat.peer + '~' + app.username
    var cmd = ['openclaw', 'agent', '--agent', agentName, '--message', text, '--session-id', sessionId, '--json']

    console.info('[chat auto-reply] calling openclaw:', cmd.join(' '))
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

        // Run onSend filter chain (handles delay + suppress-json + any future filters)
        onSend(chat.peer, replyText, credit).then(function (shouldSend) {
          if (!shouldSend) return
          console.info('[chat auto-reply] reply to', chat.peer, ':', replyText)
          addPeerMessage(chat.peer, { text: replyText, agentName: agentName })
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
        if (sender !== app.username) {
          var msgText = typeof msg.message === 'string' ? msg.message : (msg.message?.text || JSON.stringify(msg.message))
          console.info('[chat recv]', app.username, '<-', sender, ':', msgText)
          triggerAutoReply(chat, msg)
          try {
            if (chat.peer) {
              db.logChat(mesh.name, 'peer', chat.peer, null, null, sender, 'message',
                typeof msg.message === 'string' ? msg.message : JSON.stringify(msg.message),
                null)
            } else if (chat.group) {
              db.logChat(mesh.name, 'group', chat.group, chat.name, chat.creator, sender, 'message',
                typeof msg.message === 'string' ? msg.message : JSON.stringify(msg.message),
                null)
            }
          } catch {}
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
            if (hasIncoming && !initial) {
              var cfg = db.getChatPeer(mesh.name, chat.peer)
              if (cfg && !cfg.autoReply && !chat.messages.some(m => m.isSystemHint)) {
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
        if (data) {
          var pathSender = params.sender
          var creator = params.creator
          var group = params.group
          var chat = findGroupChat(creator, group)
          // If no local chat found, check info.json for existing gcid before creating new
          if (!chat) {
            // Try to read gcid from info.json first
            var infoPath = `/shared/${creator}/publish/groups/${creator}/${group}/info.json`
            var existingGcid = null
            try {
              var infoData = mesh.read(infoPath)
              if (infoData) {
                var info = JSON.decode(infoData)
                existingGcid = info.gcid || null
              }
            } catch {}
            chat = newGroupChat(creator, group, existingGcid)
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
          if (newMsgs.length > 0) {
            newMsgs.forEach(msg => {
              if (msg.sender !== app.username && !msg.sender.endsWith('/' + app.username)) {
                triggerGroupAutoReply(chat, msg)
              }
            })
          }
        }
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

  function addPeerMessage(peer, message) {
    if (!peer) return Promise.resolve(false)
    var msgText = typeof message === 'string' ? message : (message?.text || JSON.stringify(message))
    console.info('[chat send]', app.username, '->', peer, ':', msgText)
    var dirname = `/shared/${app.username}/publish/peers/${peer}/messages`
    return mesh.acl(dirname, { users: { [peer]: 'readonly' }}).then(
      () => publishMessage(dirname, message)
    ).then(() => {
      try {
        db.logChat(mesh.name, 'peer', peer, null, null, app.username, 'message',
          typeof message === 'string' ? message : JSON.stringify(message), null)
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
      db.setChatPeer(mesh.name, chat.gcid, { autoReply: false, autoReplyAgent: 'main' })
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

  function delGroup(creator, group) {
    if (creator !== app.username) return Promise.resolve(false)
    var chat = findGroupChat(creator, group)
    if (!chat) return Promise.resolve(false)
    var dirname = `/shared/${creator}/publish/groups/${creator}/${group}`
    // Remove from mesh filesystem
    return mesh.dir(dirname).then(files => {
      return Promise.all(files.map(f => mesh.unlink(os.path.join(dirname, f))))
    }).then(() => mesh.unlink(os.path.join(dirname, 'info.json'))).then(() => {
      // Remove from in-memory list
      var idx = chats.indexOf(chat)
      if (idx >= 0) chats.splice(idx, 1)
      try {
        db.logChat(mesh.name, 'group', group, chat.name, creator, app.username,
          'group_delete', null, chat.members)
      } catch {}
      return true
    }).catch(() => {
      // Even if unlink fails, remove from memory and log
      var idx = chats.indexOf(chat)
      if (idx >= 0) chats.splice(idx, 1)
      try {
        db.logChat(mesh.name, 'group', group, chat.name, creator, app.username,
          'group_delete', null, chat.members)
      } catch {}
      return true
    })
  }

  function leaveGroup(creator, group) {
    var chat = findGroupChat(creator, group)
    if (!chat) return Promise.resolve(false)
    // Non-creator: just remove from local in-memory list
    var idx = chats.indexOf(chat)
    if (idx >= 0) chats.splice(idx, 1)
    try {
      db.logChat(mesh.name, 'group', group, chat.name, creator, app.username,
        'group_leave', null, chat.members)
    } catch {}
    return Promise.resolve(true)
  }

  function addGroupMessage(creator, group, message) {
    var chat = findGroupChat(creator, group)
    if (!chat) return Promise.resolve(false)
    if (app.username !== creator && !chat.members.includes(app.username)) return Promise.resolve(false)
    // Embed sender as [gcid]/[username] inside the message payload
    var gcid = chat.gcid || ''
    var taggedSender = gcid ? (gcid + '/' + app.username) : app.username
    var taggedMessage = typeof message === 'string'
      ? { text: message, sender: taggedSender }
      : Object.assign({}, message, { sender: taggedSender })
    var msgText = typeof message === 'string' ? message : (message?.text || JSON.stringify(message))
    console.info('[chat send]', taggedSender, '-> group', group, ':', msgText)
    var dirname = `/shared/${app.username}/publish/groups/${creator}/${group}`
    return mesh.acl(dirname, { users: Object.fromEntries(chat.members.map(name => [name, 'readonly'])) }).then(
      () => publishMessage(os.path.join(dirname, 'messages'), taggedMessage)
    ).then(() => {
      try {
        db.logChat(mesh.name, 'group', group, chat.name, creator, taggedSender, 'message',
          msgText, null)
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
  }
}
