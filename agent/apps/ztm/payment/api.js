export default function ({ app, mesh }) {
  var activeServers = {}
  var PAYMENT_PORT_START = 18000
  var PAYMENT_PORT_END = 19000
  var EXPIRY_MS = 10 * 60 * 1000

  app.onExit(() => {
    Object.keys(activeServers).forEach(id => {
      var server = activeServers[id]
      pipy.listen(server.addr, 'tcp', null)
    })
  })

  // -- Payment CRUD --

  function createPayment(receiver, product, amount, currency) {
    var id = algo.uuid()
    var now = Date.now()
    var payment = {
      id,
      sender: app.username,
      senderEndpoint: app.endpoint.id,
      receiver,
      product,
      amount,
      currency: currency || 'USD',
      status: 'pending',
      proof: null,
      createdAt: now,
      expiresAt: now + EXPIRY_MS,
    }
    var path = getPaymentPath(id)
    return mesh.write(path, JSON.encode(payment)).then(
      () => mesh.acl(path, { users: { [receiver]: 'readonly' } })
    ).then(
      () => notifyReceiver(receiver, id, payment)
    ).then(
      () => payment
    )
  }

  function getPayment(id) {
    return mesh.read(getPaymentPath(id)).then(
      data => data ? JSON.decode(data) : null
    )
  }

  function listPayments() {
    return mesh.list('/shared').then(files => {
      var pattern = new http.Match('/shared/{username}/payments/{id}.json')
      var payments = []
      Object.keys(files).forEach(path => {
        var params = pattern(path)
        if (params && params.username === app.username) {
          payments.push(mesh.read(path).then(data => {
            try { return JSON.decode(data) } catch { return null }
          }))
        }
      })
      return Promise.all(payments).then(list => list.filter(p => p))
    })
  }

  function cancelPayment(id) {
    return getPayment(id).then(payment => {
      if (!payment) throw 'Payment not found'
      if (payment.sender !== app.username) throw 'Only sender can cancel'
      if (payment.status !== 'pending') throw 'Payment is not pending'
      payment.status = 'failed'
      return mesh.write(getPaymentPath(id), JSON.encode(payment)).then(() => {
        if (activeServers[id]) closePayment(id)
        return payment
      })
    })
  }

  // -- Receiver notification --

  function notifyReceiver(receiver, paymentId, payment) {
    return mesh.discover(null, receiver).then(eps => {
      var ep = (eps || []).find(e => e && e.online)
      if (!ep) return
      return mesh.request(ep.id, new Message(
        {
          method: 'POST',
          path: '/api/incoming/' + paymentId,
        },
        JSON.encode({
          sender: app.username,
          senderEndpoint: app.endpoint.id,
          paymentId,
          product: payment.product,
          amount: payment.amount,
          currency: payment.currency,
        })
      )).catch(() => {})
    })
  }

  // -- Payment HTTP Server (temporary) --

  function openPaymentServer(paymentId, buyerEndpoint, buyerUsername) {
    return getPayment(paymentId).then(payment => {
      if (!payment) throw { status: 404, message: 'Payment not found' }
      if (payment.status !== 'pending') throw { status: 409, message: 'Payment is not pending: ' + payment.status }
      if (payment.sender !== app.username) throw { status: 403, message: 'Only sender can open server' }
      if (Date.now() > payment.expiresAt) {
        payment.status = 'expired'
        return mesh.write(getPaymentPath(paymentId), JSON.encode(payment)).then(
          () => { throw { status: 410, message: 'Payment expired' } }
        )
      }

      var port = findFreePort()
      if (!port) throw { status: 503, message: 'No available port' }

      var addr = '127.0.0.1:' + port
      startLocalServer(paymentId, addr)

      var tunnelName = 'payment/' + paymentId
      var ep = app.endpoint.id

      // Publish outbound config for tunnel discovery
      return mesh.write(
        getOutboundPath(app.username, tunnelName, ep),
        JSON.encode({
          name: app.endpoint.name,
          entrances: [buyerEndpoint],
          users: [buyerUsername],
        })
      ).then(() => {
        payment.status = 'processing'
        return mesh.write(getPaymentPath(paymentId), JSON.encode(payment))
      }).then(() => {
        var timeout = new Timeout(EXPIRY_MS / 1000)
        timeout.wait().then(() => {
          if (activeServers[paymentId]) {
            closePayment(paymentId)
            getPayment(paymentId).then(p => {
              if (p && p.status === 'processing') {
                p.status = 'expired'
                mesh.write(getPaymentPath(paymentId), JSON.encode(p))
              }
            })
          }
        })

        activeServers[paymentId] = {
          addr,
          port,
          tunnelName,
          buyerEndpoint,
          timeout,
        }

        return { port, tunnelName: 'tcp/' + tunnelName }
      })
    })
  }

  function closePayment(paymentId) {
    var server = activeServers[paymentId]
    if (!server) return Promise.resolve()
    pipy.listen(server.addr, 'tcp', null)
    server.timeout.cancel && server.timeout.cancel()
    delete activeServers[paymentId]
    app.log('Closed payment server for ' + paymentId)
    return mesh.erase(
      getOutboundPath(app.username, server.tunnelName, app.endpoint.id)
    ).catch(() => {})
  }

  // -- Local HTTP Server --

  function startLocalServer(paymentId, addr) {
    var handler = pipeline($=>$
      .demuxHTTP().to($=>$
        .replaceMessage(req => handlePaymentRequest(paymentId, req))
      )
    )
    pipy.listen(addr, 'tcp', handler)
    app.log('Started payment server for ' + paymentId + ' on ' + addr)
  }

  function handlePaymentRequest(paymentId, req) {
    var method = req.head.method
    var path = req.head.path

    if (method === 'GET' && path === '/') {
      return getPayment(paymentId).then(payment => {
        if (!payment) return makeResponse(404, { error: 'Not found' })
        if (payment.status === 'completed') {
          return makeResponse(200, { status: 'completed', product: payment.product })
        }
        if (payment.status === 'expired' || payment.status === 'failed') {
          return makeResponse(410, { status: payment.status })
        }
        return make402Response({
          id: payment.id,
          amount: payment.amount,
          currency: payment.currency,
          product: payment.product,
          status: payment.status,
          expiresAt: payment.expiresAt,
        })
      })
    }

    if (method === 'PUT' && path === '/pay') {
      var body
      try { body = JSON.decode(req.body) } catch { return Promise.resolve(makeResponse(400, { error: 'Invalid JSON' })) }
      if (!body.proof) return Promise.resolve(makeResponse(400, { error: 'Proof required' }))

      return getPayment(paymentId).then(payment => {
        if (!payment) return makeResponse(404, { error: 'Not found' })
        if (payment.status !== 'pending' && payment.status !== 'processing') {
          return makeResponse(409, { error: 'Payment not pending', status: payment.status })
        }
        payment.status = 'completed'
        payment.proof = body.proof
        payment.completedAt = Date.now()
        return mesh.write(getPaymentPath(paymentId), JSON.encode(payment)).then(() => {
          closePayment(paymentId)
          return makeResponse(200, { status: 'completed' })
        })
      })
    }

    return Promise.resolve(makeResponse(404, { error: 'Not found' }))
  }

  // -- Helpers --

  function findFreePort() {
    var usedPorts = {}
    Object.values(activeServers).forEach(s => { usedPorts[s.port] = true })
    for (var port = PAYMENT_PORT_START; port <= PAYMENT_PORT_END; port++) {
      if (!usedPorts[port]) return port
    }
    return null
  }

  function getPaymentPath(id) {
    return '/shared/' + app.username + '/payments/' + id + '.json'
  }

  function getOutboundPath(username, tunnelName, ep) {
    return '/shared/' + username + '/tcp/' + tunnelName + '/' + ep + '/outbound.json'
  }

  return {
    createPayment,
    getPayment,
    listPayments,
    cancelPayment,
    openPaymentServer,
    closePayment,
    activeServers,
  }
}

function makeResponse(status, body) {
  if (!body) return new Message({ status })
  return new Message(
    { status, headers: { 'content-type': 'application/json' } },
    JSON.encode(body)
  )
}

function make402Response(body) {
  return new Message(
    { status: 402, headers: { 'content-type': 'application/json' } },
    JSON.encode(body)
  )
}

