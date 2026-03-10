import initAPI from './api.js'
import initCLI from './cli.js'
import db from '../../../db.js'

export default function ({ app, mesh, utils }) {
  function makeOpenclawPipeline(cmd) {
    var $output
    return pipeline($=>$
      .onStart(new Data)
      .exec(() => cmd, {
        onExit: (code, err) => {
          if (err) err.toString().split('\n').filter(Boolean).forEach(
            line => console.error('[openclaw]', line)
          )
          return new StreamEnd
        }
      })
      .replaceStreamStart(evt => [new MessageStart, evt])
      .replaceStreamEnd(() => new MessageEnd)
      .replaceMessage(msg => {
        $output = msg?.body?.toString?.() || ''
        return new StreamEnd
      })
      .onEnd(() => $output)
    )
  }

  var spawnOpenclaw = (cmd) => makeOpenclawPipeline(cmd).spawn()

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
        return api.addPeerMessage(peer, JSON.decode(req.body)).then(
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
        return api.addGroupMessage(creator, group, JSON.decode(req.body)).then(
          ret => response(ret ? 201 : 404)
        )
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
        return Promise.resolve(response(200, api.getPeerConfig(peer)))
      }),
    },

    '/api/auto-reply': {
      'GET': responder(() => Promise.resolve(response(200, api.allPeerConfigs()))),
    },

    '/api/files': {
      'POST': responder((_, req) => {
        return api.addFile(req.body).then(
          hash => hash ? response(200, hash) : response(404)
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
