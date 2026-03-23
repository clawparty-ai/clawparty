export default function ({ app, mesh, api, utils }) {
  return pipeline($=>$
    .onStart(ctx => main(ctx))
  )

  function main({ argv, cwd, endpoint }) {
    var buffer = new Data

    function output(str) {
      buffer.push(str)
    }

    function error(err) {
      output('ztm: ')
      output(err.message || err.toString())
      output('\n')
    }

    function flush() {
      return Promise.resolve([buffer, new StreamEnd])
    }

    function resolveGroupGcid(nameOrGcid) {
      return api.allChats().then(chats => {
        var groups = chats.filter(c => c.group)
        var found = groups.find(g => g.gcid === nameOrGcid)
                 || groups.find(g => g.name === nameOrGcid)
        if (!found) throw 'group not found: ' + nameOrGcid
        return found
      })
    }

    try {
      return utils.parseArgv(argv, {
        help: text => Promise.resolve(output(text + '\n')),
        commands: [

          // ── chat ──────────────────────────────────────────────────────────
          {
            title: 'Send or list peer/group chat messages',
            usage: 'chat',
            options: `
              --peer    <name>      Peer username to chat with
              --group   <gcid>      Group chat ID (gcid) to send to
              --message <text>      Message text to send
              --list                List all chats
              --messages            List recent messages for the given peer or group
              --since   <ts>        Only show messages after this timestamp (ms)
              --limit   <n>         Max number of messages to show (default 20)
            `,
            action: (args) => {
              var peer    = args['--peer']
              var gcid    = args['--group']
              var msg     = args['--message']
              var doList  = args['--list']
              var doMsgs  = args['--messages']
              var since   = args['--since'] ? Number(args['--since']) : 0
              var limit   = args['--limit'] ? Number(args['--limit']) : 20

              if (doList) {
                return api.allChats().then(chats => {
                  if (chats.length === 0) {
                    output('No chats.\n')
                    return
                  }
                  var peers  = chats.filter(c => c.peer)
                  var groups = chats.filter(c => c.group)
                  if (peers.length > 0) {
                    output(printTable(peers, {
                      'PEER':    r => r.peer,
                      'UNREAD':  r => r.updated > 0 ? String(r.updated) : '',
                      'LAST':    r => r.latest?.message?.text?.substring(0, 40) || '',
                    }))
                  }
                  if (groups.length > 0) {
                    if (peers.length > 0) output('\n')
                    output(printTable(groups, {
                      'GROUP':   r => r.name,
                      'GCID':    r => r.gcid,
                      'MEMBERS': r => (r.members || []).join(', '),
                      'UNREAD':  r => r.updated > 0 ? String(r.updated) : '',
                      'LAST':    r => r.latest?.message?.text?.substring(0, 40) || '',
                    }))
                  }
                })
              }

              if (doMsgs) {
                if (peer) {
                  return api.allPeerMessages(peer, since, null).then(msgs => {
                    var slice = msgs.slice(-limit)
                    if (slice.length === 0) { output('No messages.\n'); return }
                    slice.forEach(m => {
                      var text = typeof m.message === 'string' ? m.message : (m.message?.text || '')
                      output('[' + new Date(m.time).toISOString() + '] ' + m.sender + ': ' + text + '\n')
                    })
                  })
                } else if (gcid) {
                  return api.getGroupByGcid(gcid).then(g => {
                    if (!g) throw 'group not found: ' + gcid
                    return api.allGroupMessages(g.creator, g.group, since, null).then(msgs => {
                      var slice = msgs.slice(-limit)
                      if (slice.length === 0) { output('No messages.\n'); return }
                      slice.forEach(m => {
                        var senderDisplay = m.sender.indexOf('/') !== -1 ? m.sender.split('/')[1] : m.sender
                        var text = typeof m.message === 'string' ? m.message : (m.message?.text || '')
                        output('[' + new Date(m.time).toISOString() + '] ' + senderDisplay + ': ' + text + '\n')
                      })
                    })
                  })
                } else {
                  throw 'specify --peer <name> or --group <gcid>'
                }
              }

              if (msg) {
                if (peer) {
                  return api.addPeerMessage(peer, { text: msg }).then(() => {
                    output('Sent to ' + peer + '\n')
                  })
                } else if (gcid) {
                  return api.addGroupMessageByGcid(gcid, { text: msg }).then(ret => {
                    if (!ret) throw 'group not found: ' + gcid
                    output('Sent to group ' + ret.name + ' (' + gcid + ')\n')
                  })
                } else {
                  throw 'specify --peer <name> or --group <gcid>'
                }
              }

              if (!doList && !doMsgs && !msg) {
                throw 'specify --list, --messages, or --message <text>'
              }

              return Promise.resolve()
            }
          },

          // ── groupchat ─────────────────────────────────────────────────────
          {
            title: 'Send a message to a group chat by its gcid',
            usage: 'groupchat <gcid>',
            options: `
              --message <text>      Message text to send (required)
            `,
            action: (args) => {
              var gcid = args['<gcid>']
              var msg  = args['--message']
              if (!gcid)  throw 'missing argument: <gcid>'
              if (!msg)   throw 'missing option: --message <text>'

              return api.addGroupMessageByGcid(gcid, { text: msg }).then(ret => {
                if (!ret) throw 'group not found: ' + gcid
                output('Sent to group "' + ret.name + '" (' + gcid + ')\n')
              })
            }
          },

          // ── auto-reply ────────────────────────────────────────────────────
          {
            title: 'View or configure auto-reply settings for a peer',
            usage: 'auto-reply <peer>',
            options: `
              --enable            Enable auto-reply for the peer
              --disable           Disable auto-reply for the peer
              --agent  <name>     Set the openclaw agent to use for auto-reply
            `,
            action: (args) => {
              var peer = args['<peer>']
              if (!peer) throw 'missing argument: <peer>'

              var enable    = args['--enable']
              var disable   = args['--disable']
              var agentName = args['--agent']

              if (enable && disable) throw 'options --enable and --disable are mutually exclusive'

              var changed = enable || disable || agentName
              if (changed) {
                var config = {}
                if (enable)     config.autoReply = true
                if (disable)    config.autoReply = false
                if (agentName)  config.autoReplyAgent = agentName
                api.setPeerConfig(peer, config)
              }

              var cfg = api.getPeerConfig(peer)
              output('Peer:       ' + cfg.peer + '\n')
              output('Auto-Reply: ' + (cfg.autoReply ? 'enabled' : 'disabled') + '\n')
              output('Agent:      ' + cfg.autoReplyAgent + '\n')
              return Promise.resolve()
            }
          },

          // ── auto-reply-list ───────────────────────────────────────────────
          {
            title: 'List auto-reply settings for all peers and groups',
            usage: 'auto-reply-list',
            options: '',
            action: () => {
              return api.allChats().then(chats => {
                var groups = chats.filter(c => c.group)
                var configs = api.allPeerConfigs()

                // Peer DM configs: peer field is a plain username (no '~', not a UUID)
                function isUUID(s) {
                  if (s.length !== 36) return false
                  return s[8] === '-' && s[13] === '-' && s[18] === '-' && s[23] === '-'
                }

                var peerConfigs = configs.filter(c => !isUUID(c.peer) && c.peer.indexOf('~') === -1)
                var groupEpConfigs = configs.filter(c => isUUID(c.peer))
                var groupAgentConfigs = configs.filter(c => c.peer.indexOf('~') !== -1)

                var any = false

                if (peerConfigs.length > 0) {
                  any = true
                  output('Peer auto-reply:\n')
                  output(printTable(peerConfigs, {
                    'PEER':        row => row.peer,
                    'AUTO-REPLY':  row => row.autoReply ? 'enabled' : 'disabled',
                    'AGENT':       row => row.autoReplyAgent,
                  }))
                }

                if (groupEpConfigs.length > 0) {
                  if (any) output('\n')
                  any = true
                  output('Group auto-reply (EP-level):\n')
                  output(printTable(groupEpConfigs, {
                    'GROUP':       row => { var g = groups.find(g => g.gcid === row.peer); return g ? g.name : row.peer },
                    'GCID':        row => row.peer,
                    'AUTO-REPLY':  row => row.autoReply ? 'enabled' : 'disabled',
                    'AGENT':       row => row.autoReplyAgent,
                  }))
                }

                if (groupAgentConfigs.length > 0) {
                  if (any) output('\n')
                  any = true
                  output('Group auto-reply (per-agent):\n')
                  output(printTable(groupAgentConfigs, {
                    'GROUP':       row => { var gcid = row.peer.split('~')[0]; var g = groups.find(g => g.gcid === gcid); return g ? g.name : gcid },
                    'GCID':        row => row.peer.split('~')[0],
                    'AGENT':       row => row.peer.split('~')[1],
                    'AUTO-REPLY':  row => row.autoReply ? 'enabled' : 'disabled',
                  }))
                }

                if (!any) output('No auto-reply settings configured.\n')
              })
            }
          },

          // ── group-auto-reply ──────────────────────────────────────────────
          {
            title: 'View or configure EP-level auto-reply for a group chat',
            usage: 'group-auto-reply <group>',
            options: `
              --enable            Enable auto-reply for this group
              --disable           Disable auto-reply for this group
              --agent  <name>     Set the openclaw agent to use for auto-reply (default: main)
            `,
            action: (args) => {
              var nameOrGcid = args['<group>']
              if (!nameOrGcid) throw 'missing argument: <group> (group name or gcid)'

              var enable    = args['--enable']
              var disable   = args['--disable']
              var agentName = args['--agent']

              if (enable && disable) throw 'options --enable and --disable are mutually exclusive'

              return resolveGroupGcid(nameOrGcid).then(group => {
                var changed = enable || disable || agentName
                if (changed) {
                  var config = {}
                  if (enable)    config.autoReply = true
                  if (disable)   config.autoReply = false
                  config.autoReplyAgent = agentName || 'main'
                  api.setPeerConfig(group.gcid, config)
                }

                var cfg = api.getPeerConfig(group.gcid)
                output('Group:      ' + group.name + '\n')
                output('GCID:       ' + group.gcid + '\n')
                output('Auto-Reply: ' + (cfg.autoReply ? 'enabled' : 'disabled') + '\n')
                output('Agent:      ' + cfg.autoReplyAgent + '\n')
                return Promise.resolve()
              })
            }
          },

          // ── group-agent-auto-reply ────────────────────────────────────────
          {
            title: 'View or configure per-agent auto-reply for a group chat',
            usage: 'group-agent-auto-reply <group> <agent>',
            options: `
              --enable            Enable auto-reply for this agent in the group
              --disable           Disable auto-reply for this agent in the group
            `,
            action: (args) => {
              var nameOrGcid = args['<group>']
              var agentName  = args['<agent>']
              if (!nameOrGcid) throw 'missing argument: <group> (group name or gcid)'
              if (!agentName)  throw 'missing argument: <agent>'

              var enable  = args['--enable']
              var disable = args['--disable']

              if (enable && disable) throw 'options --enable and --disable are mutually exclusive'

              return resolveGroupGcid(nameOrGcid).then(group => {
                var key = group.gcid + '~' + agentName
                var changed = enable || disable
                if (changed) {
                  var config = { autoReplyAgent: agentName }
                  if (enable)  config.autoReply = true
                  if (disable) config.autoReply = false
                  api.setPeerConfig(key, config)
                }

                var cfg = api.getPeerConfig(key)
                output('Group:      ' + group.name + '\n')
                output('GCID:       ' + group.gcid + '\n')
                output('Agent:      ' + agentName + '\n')
                output('Auto-Reply: ' + (cfg.autoReply ? 'enabled' : 'disabled') + '\n')
                return Promise.resolve()
              })
            }
          },
        ]

      }).then(flush).catch(err => {
        error(err)
        return flush()
      })

    } catch (err) {
      error(err)
      return flush()
    }
  }
}

function printTable(data, columns) {
  var output = new Data
  var cols = Object.entries(columns)
  var colHeaders = cols.map(i => i[0])
  var colFormats = cols.map(i => i[1])
  var colSizes = colHeaders.map(name => name.length)
  var rows = data.map(row => colFormats.map(
    (format, i) => {
      var v = (format(row) || '').toString()
      colSizes[i] = Math.max(colSizes[i], v.length)
      return v
    }
  ))
  colHeaders.forEach((name, i) => {
    output.push(name.padEnd(colSizes[i]))
    output.push('  ')
  })
  output.push('\n')
  rows.forEach(row => {
    row.forEach((v, i) => {
      output.push(v.padEnd(colSizes[i]))
      output.push('  ')
    })
    output.push('\n')
  })
  return output
}
