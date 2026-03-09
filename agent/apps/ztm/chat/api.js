export default function ({ app, mesh, db, spawnOpenclaw }) {
  var chats = []

  function getPeerConfig(peer) {
    return db.getChatPeer(mesh.name, peer) || { peer, autoReply: false, autoReplyAgent: 'main' }
  }

  function setPeerConfig(peer, config) {
    db.setChatPeer(mesh.name, peer, config)
  }

  function allPeerConfigs() {
    return db.allChatPeers(mesh.name)
  }

  function notifyAutoReplySetup(chat) {
    var hint = `Auto-reply is currently disabled for this conversation. ` +
      `You can enable it with: clawparty chat auto-reply ${chat.peer} --enable [--agent <name>]`
    var time = Date.now()
    chat.messages.push({ time, message: { text: hint }, sender: app.username, isSystemHint: true })
    if (time > chat.updateTime) chat.updateTime = time
  }

  function triggerAutoReply(chat, msg) {
    if (!spawnOpenclaw) return
    if (!chat.peer) return
    var peerConfig = getPeerConfig(chat.peer)
    if (!peerConfig.autoReply) return
    var agentName = peerConfig.autoReplyAgent || 'main'
    var text = typeof msg.message === 'string' ? msg.message : (msg.message?.text || JSON.stringify(msg.message))
    var cmd = ['openclaw', 'agent', '--agent', agentName, '--message', text, '--json']
    console.info('[chat auto-reply] calling openclaw:', cmd.join(' '))
    spawnOpenclaw(cmd).then(
      output => {
        console.info('[chat auto-reply] openclaw output:', output)
        var replyText
        try {
          var parsed = JSON.parse(output.split('\n').join(''))
          replyText = parsed?.payloads?.[0]?.text ||
                      parsed?.result?.payloads?.[0]?.text ||
                      parsed?.message || parsed?.content || parsed?.text
        } catch {}
        if (!replyText) replyText = output.split('\n').join('').trim()
        if (!replyText) return
        console.info('[chat auto-reply] reply to', chat.peer, ':', replyText)
        addPeerMessage(chat.peer, { text: replyText })
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

  function newGroupChat(creator, group) {
    var chat = {
      creator,
      group,
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
          } catch {}
        }
      })
    }
    var params = matchPublishGroupMsgs(path)
    if (params) {
      return mesh.read(path).then(data => {
        if (data) {
          var sender = params.sender
          var creator = params.creator
          var group = params.group
          var chat = findGroupChat(creator, group)
          if (!chat) chat = newGroupChat(creator, group)
          try {
            var messages = JSON.decode(data)
            messages.forEach(msg => msg.sender = sender)
            mergeMessages(chat, messages)
          } catch {}
          if (initial) chat.newCount = 0
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
        var latest = chat.messages.reduce((a, b) => a.time > b.time ? a : b)
        if (chat.peer) {
          return {
            peer: chat.peer,
            time: chat.updateTime,
            updated,
            latest,
          }
        } else {
          return {
            creator: chat.creator,
            group: chat.group,
            name: chat.name,
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
    var dirname = `/shared/${app.username}/publish/groups/${creator}/${group}`
    return mesh.acl(dirname, { users: Object.fromEntries(chat.members.map(name => [name, 'readonly'])) }).then(
      () => publishMessage(os.path.join(dirname, 'messages'), message)
    ).then(() => {
      try {
        db.logChat(mesh.name, 'group', group, chat.name, creator, app.username, 'message',
          typeof message === 'string' ? message : JSON.stringify(message), null)
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

  return {
    allEndpoints,
    allUsers,
    allChats,
    allPeerMessages,
    addPeerMessage,
    allGroupMessages,
    getGroup,
    setGroup,
    delGroup,
    leaveGroup,
    addGroupMessage,
    addFile,
    getFile,
    delFile,
    getPeerConfig,
    setPeerConfig,
    allPeerConfigs,
  }
}
