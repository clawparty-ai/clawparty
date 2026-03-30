# Payment App Design (v1)

## Architecture Overview

```
ep-A (merchant)                              ep-B (buyer)
+-----------------+                          +-----------------+
| 1. Create payment|                          |                 |
|    request       |---- mesh FS ----------->| 2. See "Buy"    |
|                  |     + mesh.request()     |    button       |
|                  |                          |                 |
| 3. Start temp    |<--- mesh.request() ------| 4. Click "Buy"  |
|    HTTP server   |    /api/incoming/{id}    |                 |
|    + tunnel      |     action=open         |                 |
|                  |------- response ------->| 5. Get tunnel   |
|                  |  {port, tunnelName}      |    info         |
|                  |                          |                 |
| 6. Serve 402     |<=== HTTP CONNECT =======| 6. CONNECT via  |
|    + invoice     |    via mesh tunnel       |    tunnel       |
|                  |                          |                 |
| 7. Verify proof  |<=== PUT /pay ===========| 7. Send payment |
|    return 200    |                          |    proof        |
|                  |                          |                 |
| 8. Stop HTTP,    |                          |                 |
|    tear down     |                          |                 |
|    tunnel        |                          |                 |
+-----------------+                          +-----------------+
```

## Directory Structure

```
agent/apps/ztm/payment/
  main.js        Entry point, user/peer route tables (~180 lines)
  api.js         Core logic: payment CRUD, HTTP server lifecycle, tunnel management (~400 lines)
  cli.js         CLI commands for managing payments (~150 lines)
```

## Data Model

### Payment Request

Stored at `/shared/{sender-username}/payments/{id}.json`:

```json
{
  "id": "uuid-string",
  "sender": "alice",
  "senderEndpoint": "ep-uuid-A",
  "receiver": "bob",
  "product": {
    "name": "Premium Widget",
    "description": "A premium widget license",
    "url": "https://example.com/widget"
  },
  "amount": 100,
  "currency": "USD",
  "status": "pending|processing|completed|failed|expired",
  "proof": null,
  "createdAt": 1711111111111,
  "expiresAt": 1711114711111
}
```

### In-memory Active Payment (ep-A side)

```json
{
  "paymentId": "uuid",
  "port": 18234,
  "listenerAddr": "127.0.0.1:18234",
  "tunnelName": "payment/uuid",
  "buyerEndpoint": "ep-uuid-B",
  "timeout": "<Timeout object>"
}
```

## API Routes

### User Routes (local, `serveUser`)

| Path | Method | Description |
|------|--------|-------------|
| `/cli` | CONNECT | CLI session |
| `/api/appinfo` | GET | App metadata |
| `/api/payments` | GET | List all payments (sent + received) |
| `/api/payments` | POST | Create new payment request |
| `/api/payments/{id}` | GET | Get payment by ID |
| `/api/payments/{id}` | DELETE | Cancel payment |
| `*` | GET | Static GUI fallback |

**`POST /api/payments` request body:**

```json
{
  "receiver": "bob",
  "product": { "name": "Widget", "description": "...", "url": "..." },
  "amount": 100,
  "currency": "USD"
}
```

### Peer Routes (remote, `servePeer`)

| Path | Method | Description |
|------|--------|-------------|
| `/api/incoming/{id}` | GET | Get payment details (buyer checks invoice) |
| `/api/incoming/{id}` | POST | Actions: `{ "action": "open" }` or `{ "action": "pay", "proof": "..." }` |

## Detailed Flow

### 1. Create Payment (`POST /api/payments` on ep-A)

```
api.createPayment({ receiver, product, amount, currency }):
  1. Generate payment ID (algo.uuid())
  2. Build payment object with status="pending"
  3. mesh.write('/shared/{username}/payments/{id}.json', payment)
  4. mesh.acl(path, { users: { [receiver]: 'readonly' } })
  5. mesh.request(receiverEndpoint, POST /api/incoming/{id})
     -> Notify ep-B of new payment request
  6. Return payment object
```

### 2. Buyer Receives Notification (ep-B)

```
servePeer /api/incoming/{id} POST:
  1. Fetch payment from mesh FS: /shared/{sender}/payments/{id}.json
  2. Validate: receiver matches ep-B's username
  3. Store locally: /local/incoming/{id}.json
  4. Return 200
```

The chat GUI can render a "Buy" button for messages of type `payment`, or the CLI shows a notification.

### 3. Buyer Clicks "Buy" (ep-B calls ep-A)

```
ep-B calls: mesh.request(ep-A, POST /api/incoming/{id}, { action: "open" })

ep-A servePeer handler:
  1. Load payment from /shared/{username}/payments/{id}.json
  2. Validate: status=pending, not expired, sender matches
  3. Find a free port (e.g., 18000-19000 range)
  4. Start HTTP server on 127.0.0.1:{port} with payment handler pipeline
  5. Create tunnel inbound config:
     - protocol: tcp
     - name: "payment/{id}"
     - listens: [{ ip: "127.0.0.1", port: {port} }]
     - exits: [buyerEndpointId]
  6. Store active payment in memory
  7. Set timeout (e.g., 10 minutes) for auto-cleanup
  8. Update payment status to "processing"
  9. Return { port: {port}, tunnelName: "tcp/payment/{id}" }
```

### 4. Buyer Connects Through Tunnel (ep-B)

```
ep-B:
  1. Build tunnel: inbound tcp/payment/{id} -> exits [ep-A]
  2. Create local tunnel inbound pointing to ep-A's outbound
  3. Connect via tunnel:
     connectHTTPTunnel(CONNECT /api/outbound/tcp/payment/{id})
       -> muxHTTP -> mesh.connect(ep-A)
  4. Send GET / through tunnel
```

The tunnel mechanism is identical to the existing tunnel app -- ep-A publishes a `/shared/{username}/tcp/payment/{id}/{ep}/outbound.json` that ep-B's inbound can discover.

### 5. Payment HTTP Service (ep-A, temporary server)

HTTP handler pipeline:

```
GET /:
  1. Load payment
  2. If status=pending or processing:
     -> Return 402 { id, amount, currency, product, status, expiresAt }
  3. If status=completed:
     -> Return 200 { status: "completed", product }
  4. Else:
     -> Return 410 Gone

PUT /pay:
  1. Parse body: { id, proof }
  2. Load payment
  3. Verify proof (placeholder: accept any non-empty string)
  4. Update payment status to "completed", store proof
  5. mesh.write updated payment
  6. Return 200 { status: "completed" }
  7. Trigger cleanup (stop server, tear down tunnel)
```

### 6. Cleanup

```
api.closePayment(paymentId):
  1. Look up active payment in memory
  2. pipy.listen(addr, 'tcp', null) -> stop HTTP server
  3. Delete tunnel inbound config via tunnel API pattern:
     deleteInbound(ep, 'tcp', 'payment/{id}')
     -> removes /shared/{username}/tcp/payment/{id}/{ep}/inbound.json
  4. Clear timeout
  5. Remove from active payments map
  6. If payment still processing -> set status to "expired"
```

## HTTP 402 Response Contract

The temporary HTTP service returns `402 PaymentRequired` with JSON body:

```json
HTTP/1.1 402 PaymentRequired
Content-Type: application/json

{
  "id": "payment-uuid",
  "amount": 100,
  "currency": "USD",
  "product": {
    "name": "Premium Widget",
    "description": "A premium widget license",
    "url": "https://example.com/widget"
  },
  "status": "pending",
  "expiresAt": 1711114711111
}
```

## HTTP Server Pipeline (temporary, per-payment)

```js
function startPaymentServer(paymentId, payment, port) {
  var handler = pipeline($=>$
    .demuxHTTP().to($=>$
      .replaceMessage(msg => {
        var method = msg.head.method
        var path = msg.head.path

        if (method === 'GET' && path === '/') {
          return mesh.read('/shared/' + app.username + '/payments/' + paymentId + '.json')
            .then(data => {
              var p = JSON.decode(data)
              if (p.status === 'completed') {
                return new Message({ status: 200 }, JSON.encode({ status: 'completed', product: p.product }))
              }
              if (p.status === 'expired' || p.status === 'failed') {
                return new Message({ status: 410 }, JSON.encode({ status: p.status }))
              }
              return new Message(
                { status: 402, headers: { 'Content-Type': 'application/json' } },
                JSON.encode({
                  id: p.id,
                  amount: p.amount,
                  currency: p.currency,
                  product: p.product,
                  status: p.status,
                  expiresAt: p.expiresAt,
                })
              )
            })
        }

        if (method === 'PUT' && path === '/pay') {
          return msg.body.toArray().then(chunks => {
            var body = JSON.decode(chunks[0])
            return mesh.read('/shared/' + app.username + '/payments/' + paymentId + '.json')
              .then(data => {
                var p = JSON.decode(data)
                if (p.status !== 'pending' && p.status !== 'processing') {
                  return new Message({ status: 409 }, JSON.encode({ error: 'Payment not pending' }))
                }
                if (!body.proof) {
                  return new Message({ status: 400 }, JSON.encode({ error: 'Proof required' }))
                }
                p.status = 'completed'
                p.proof = body.proof
                p.completedAt = Date.now()
                return mesh.write('/shared/' + app.username + '/payments/' + paymentId + '.json', JSON.encode(p))
                  .then(() => {
                    closePayment(paymentId)
                    return new Message({ status: 200 }, JSON.encode({ status: 'completed' }))
                  })
              })
          })
        }

        return new Message({ status: 404 })
      })
    )
  )

  pipy.listen('127.0.0.1:' + port, 'tcp', handler)
}
```

## Tunnel Setup Pattern (reuses tunnel app patterns)

When ep-A opens a payment server, it creates tunnel configs identical to `tunnel/api.js`:

```js
// Publish outbound config so ep-B can connect inbound
var outboundData = JSON.encode({
  targets: [{ host: '127.0.0.1', port: port }],
  entrances: [buyerEndpointId],
  users: [buyerUsername],
})
mesh.write(
  '/shared/' + app.username + '/tcp/payment/' + paymentId + '/' + app.endpoint.id + '/outbound.json',
  outboundData
)
```

ep-B then creates an inbound that exits to ep-A:

```js
// On ep-B side: create inbound that connects to ep-A's outbound
var inboundData = JSON.encode({
  listens: [{ ip: '127.0.0.1', port: localPort }],
  exits: [sellerEndpointId],
})
mesh.write(
  '/shared/' + app.username + '/tcp/payment/' + paymentId + '/' + app.endpoint.id + '/inbound.json',
  inboundData
)
```

## CLI Commands

```
ztm payment send <username> --name <name> --amount <amount> [--currency <currency>] [--description <desc>] [--url <url>]
ztm payment list [--sent|--received|--all]
ztm payment get <payment-id>
ztm payment buy <payment-id>         # Click "Buy" on a received payment
ztm payment cancel <payment-id>
```

## Mesh Filesystem Layout

```
/shared/{sender-username}/payments/{payment-id}.json             # Payment request (readonly for receiver)
/shared/{sender-username}/tcp/payment/{payment-id}/{ep}/outbound.json  # Tunnel outbound config
/shared/{receiver-username}/tcp/payment/{payment-id}/{ep}/inbound.json # Tunnel inbound config
/local/incoming/{payment-id}.json                                # Local cache of received payments
```

## Key Design Decisions

1. **Reuse tunnel app patterns**: The temporary HTTP->tunnel setup mirrors `tunnel/api.js`'s `applyLocalConfig` -- open a listener, publish inbound/outbound configs to `/shared/`, and let the tunnel mesh handle routing.

2. **HTTP 402 as contract**: The temporary server always returns `402` with invoice JSON until payment is confirmed, then returns `200`. This is the core HTTP 402 usage.

3. **Proof is a placeholder**: For now, any non-empty string is accepted as proof. This can later be extended to cryptographic signatures, Lightning preimage hashes, etc.

4. **Auto-cleanup via timeout**: Active payments have a configurable TTL (default 10 min). On expiry, the server is stopped and tunnel torn down automatically.

5. **Dual notification**: Payment created -> mesh.request to receiver's peer API. Payment completed -> mesh.request to both sides.

6. **Pipy compatibility**: No regex (`RegExp`) APIs are used. All path matching uses `http.Match` and string operations (`split`, `indexOf`, `startsWith`, etc.), consistent with PipyJS constraints.
