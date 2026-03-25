import initAPI from './api.js'
import initCLI from './cli.js'

export default function ({ app, mesh, utils }) {
  var api = initAPI({ app, mesh })
  var cli = initCLI({ app, mesh, utils, api })

  var $ctx

  var gui = new http.Directory(os.path.join(app.root, 'gui'))
  var response = utils.createResponse
  var responder = utils.createResponder
  var responderOwnerOnly = (f) => responder((params, req) => (
    $ctx.peer.username === app.username ? f(params, req) : Promise.resolve(response(403))
  ))

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

    '/api/payments': {
      'GET': responder(() => api.listPayments().then(
        ret => response(200, ret)
      )),

      'POST': responder((_, req) => {
        var obj = JSON.decode(req.body)
        if (!obj.receiver) throw 'receiver is required'
        if (!obj.product || !obj.product.name) throw 'product.name is required'
        if (typeof obj.amount !== 'number' || obj.amount <= 0) throw 'amount must be a positive number'
        return api.createPayment(obj.receiver, obj.product, obj.amount, obj.currency).then(
          payment => response(201, payment)
        )
      }),
    },

    '/api/payments/{id}': {
      'GET': responder(({ id }) => api.getPayment(id).then(
        ret => ret ? response(200, ret) : response(404)
      )),

      'DELETE': responder(({ id }) => api.cancelPayment(id).then(
        () => response(204)
      )),
    },

    '*': {
      'GET': responder((_, req) => {
        return Promise.resolve(gui.serve(req) || response(404))
      })
    },
  })

  var servePeer = utils.createServer({
    '/api/incoming/{id}': {
      'GET': responder(({ id }) => api.getPayment(id).then(payment => {
        if (!payment) return response(404, { error: 'Payment not found' })
        if ($ctx.peer.username !== payment.sender && $ctx.peer.username !== payment.receiver) {
          return response(403, { error: 'Forbidden' })
        }
        return response(200, payment)
      })),

      'POST': responder(({ id }, req) => {
        var body = JSON.decode(req.body)
        var peer = $ctx.peer

        if (body.action === 'open') {
          return api.openPaymentServer(id, peer.id, peer.username)
            .then(result => response(200, result))
            .catch(err => {
              if (typeof err === 'object' && err.status) return response(err.status, err)
              return response(500, { error: err.toString() })
            })
        }

        if (body.action === 'pay') {
          if (!body.proof) return response(400, { error: 'Proof required' })
          return api.getPayment(id).then(payment => {
            if (!payment) return response(404, { error: 'Not found' })
            if (peer.username !== payment.receiver) return response(403, { error: 'Forbidden' })
            if (payment.status !== 'pending' && payment.status !== 'processing') {
              return response(409, { error: 'Payment not pending', status: payment.status })
            }
            payment.status = 'completed'
            payment.proof = body.proof
            payment.completedAt = Date.now()
            return mesh.write(
              '/shared/' + app.username + '/payments/' + id + '.json',
              JSON.encode(payment)
            ).then(() => {
              api.closePayment(id)
              return response(200, { status: 'completed' })
            })
          })
        }

        // Notification from sender about new payment
        if (body.paymentId) {
          app.log('Received payment request: ' + body.paymentId + ' from ' + body.sender)
          return Promise.resolve(response(200, { received: true }))
        }

        return response(400, { error: 'Unknown action' })
      }),
    },
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
