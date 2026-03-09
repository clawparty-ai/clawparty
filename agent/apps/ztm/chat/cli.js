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

    try {
      return utils.parseArgv(argv, {
        help: text => Promise.resolve(output(text + '\n')),
        commands: [
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

              var enable = args['--enable']
              var disable = args['--disable']
              var agentName = args['--agent']

              if (enable && disable) throw 'options --enable and --disable are mutually exclusive'

              var changed = enable || disable || agentName

              if (changed) {
                var config = {}
                if (enable) config.autoReply = true
                if (disable) config.autoReply = false
                if (agentName) config.autoReplyAgent = agentName
                api.setPeerConfig(peer, config)
              }

              var cfg = api.getPeerConfig(peer)
              output(`Peer:    ${cfg.peer}\n`)
              output(`Auto-Reply: ${cfg.autoReply ? 'enabled' : 'disabled'}\n`)
              output(`Agent:   ${cfg.autoReplyAgent}\n`)
              return Promise.resolve()
            }
          },

          {
            title: 'List auto-reply settings for all peers',
            usage: 'auto-reply-list',
            options: '',
            action: () => {
              var configs = api.allPeerConfigs()
              if (configs.length === 0) {
                output('No auto-reply settings configured.\n')
              } else {
                output(printTable(configs, {
                  'PEER':        row => row.peer,
                  'AUTO-REPLY':  row => row.autoReply ? 'enabled' : 'disabled',
                  'AGENT':       row => row.autoReplyAgent,
                }))
              }
              return Promise.resolve()
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
