# Payment App Test Plan

## Prerequisites

- Two ZTM endpoints running on the same mesh: **ep-A** (seller/merchant) and **ep-B** (buyer)
- Both endpoints have the `payment` app installed
- Both agents are online and connected to the same hub
- Usernames: `alice` on ep-A, `bob` on ep-B

## Test Cases

### TC-01: Create Payment (Happy Path)

**Goal:** Verify that a seller can create a payment request.

**Steps (ep-A / alice):**
```
ztm payment send bob --name "Premium Widget" --amount 100 --currency USD --description "A fine widget" --url "https://example.com/widget"
```

**Expected:**
- CLI output shows payment created with ID, receiver, product name, amount, status=`pending`
- Payment file exists at `/shared/alice/payments/{id}.json` in mesh FS
- ACL on the file grants `bob` readonly access
- ep-B receives notification via `POST /api/incoming/{id}` (check ep-B agent logs for "Received payment request")

**Verify:**
```
ztm payment list          # shows the new payment with DIRECTION -> bob, STATUS pending
ztm payment get <id>      # shows full details including expiresAt
```

---

### TC-02: List Payments

**Goal:** Verify payment listing works for both empty and non-empty states.

**Steps (ep-A / alice):**
```
ztm payment list
```

**Before TC-01:** Expected output: `No payments found.`

**After TC-01:** Expected output: table with columns ID, DIRECTION, PRODUCT, AMOUNT, STATUS. The DIRECTION column should show `-> bob`.

---

### TC-03: Get Payment Details

**Goal:** Verify detailed payment retrieval.

**Steps (ep-A / alice):**
```
ztm payment get <payment-id>
```

**Expected output:**
```
Payment: <full-uuid>
  Sender:    alice
  Receiver:  bob
  Product:   Premium Widget
  Desc:      A fine widget
  URL:       https://example.com/widget
  Amount:    100 USD
  Status:    pending
  Created:   2026-03-25T...
  Expires:   2026-03-25T...
```

---

### TC-04: Cancel Payment

**Goal:** Verify that the sender can cancel a pending payment.

**Steps:**
1. Create a new payment: `ztm payment send bob --name "Test" --amount 10`
2. Note the payment ID
3. Cancel: `ztm payment cancel <id>`

**Expected:**
- CLI: `Payment <id> cancelled.`
- `ztm payment get <id>` shows status=`failed`
- If a payment server was running for this payment, it is stopped

---

### TC-05: Cancel Denied for Non-Sender

**Goal:** Verify that only the sender can cancel.

**Steps (ep-B / bob):**
```
ztm payment cancel <alice-payment-id>
```

**Expected:** Error message: `Only sender can cancel`

---

### TC-06: Buyer Receives Notification

**Goal:** Verify ep-B is notified when ep-A creates a payment.

**Steps:**
1. On ep-A: `ztm payment send bob --name "Widget" --amount 50`
2. Check ep-B agent logs

**Expected (ep-B logs):**
```
Received payment request: <payment-id> from alice
```

**Verify on ep-B:**
- The payment file is readable at `/shared/alice/payments/{id}.json` (bob has ACL readonly)

---

### TC-07: Seller Opens Payment Server (HTTP 402 Flow)

**Goal:** Verify the core HTTP 402 payment flow via the peer API.

**Steps:**
1. ep-A creates payment: `ztm payment send bob --name "Widget" --amount 100`
2. ep-B calls seller to open payment server (simulated via mesh request):

```
# On ep-B agent, send mesh request to ep-A:
POST /api/incoming/<payment-id>
Body: { "action": "open" }
```

**Expected response (200):**
```json
{ "port": 18001, "tunnelName": "tcp/payment/<payment-id>" }
```

**Verify on ep-A:**
- Agent log: `Started payment server for <id> on 127.0.0.1:<port>`
- Local HTTP server is listening on `127.0.0.1:<port>`
- Tunnel outbound config published at `/shared/alice/tcp/payment/<id>/<ep-A-id>/outbound.json`
- Payment status changed to `processing`

---

### TC-08: HTTP 402 Invoice Retrieval

**Goal:** Verify the temporary HTTP server returns 402 with invoice details.

**Prerequisite:** TC-07 completed (server is running on ep-A)

**Steps:**
```
# On ep-B, fetch invoice via peer API:
GET /api/incoming/<payment-id>
```

**Expected (200):** Full payment object with status=`processing`

**Alternatively, via the local HTTP server (on ep-A):**
```
curl http://127.0.0.1:<port>/
```

**Expected (402):**
```json
HTTP/1.1 402 PaymentRequired
Content-Type: application/json

{
  "id": "<payment-id>",
  "amount": 100,
  "currency": "USD",
  "product": { "name": "Widget", ... },
  "status": "processing",
  "expiresAt": ...
}
```

---

### TC-09: Complete Payment via Peer API

**Goal:** Verify payment completion through the mesh peer API.

**Prerequisite:** TC-07 completed

**Steps:**
```
# On ep-B:
POST /api/incoming/<payment-id>
Body: { "action": "pay", "proof": "proof-token-abc123" }
```

**Expected (200):**
```json
{ "status": "completed" }
```

**Verify:**
- Payment status in mesh FS is `completed`
- `proof` field is `proof-token-abc123`
- `completedAt` timestamp is set
- Payment server on ep-A is stopped (port released)
- Tunnel outbound config removed from `/shared/alice/tcp/payment/<id>/...`

**Verify via CLI:**
```
ztm payment get <id>
# Shows: Status: completed, Proof: proof-token-abc123, Completed: 2026-...
```

---

### TC-10: Complete Payment via Local HTTP Server

**Goal:** Verify payment completion through the temporary HTTP server directly.

**Prerequisite:** TC-07 completed, server running on ep-A

**Steps:**
```
# On ep-A (direct to local server):
curl -X PUT http://127.0.0.1:<port>/pay \
  -H "Content-Type: application/json" \
  -d '{"proof": "my-proof-token"}'
```

**Expected (200):**
```json
{ "status": "completed" }
```

**Verify:**
- Payment status = `completed` in mesh FS
- Server is stopped (subsequent curl to the port fails to connect)

---

### TC-11: Payment Expiry (Auto-Cleanup)

**Goal:** Verify that a payment server auto-cleans after timeout.

**Note:** For testing, temporarily set `EXPIRY_MS` to a short value (e.g., 30 seconds) in `api.js`.

**Steps:**
1. Create payment on ep-A
2. Open payment server via TC-07
3. Wait for expiry (default 10 min, or reduced value)
4. Check state

**Expected after expiry:**
- Payment server stopped (port freed)
- `activeServers` entry removed
- Payment status changed to `expired` in mesh FS
- Tunnel outbound config removed

---

### TC-12: Pay on Non-Pending Payment

**Goal:** Verify that paying a completed/expired payment returns an error.

**Steps:**
1. Complete a payment via TC-09
2. Try to pay again: `POST /api/incoming/<id> { "action": "pay", "proof": "..." }`

**Expected (409):**
```json
{ "error": "Payment not pending", "status": "completed" }
```

---

### TC-13: Open Server on Expired Payment

**Goal:** Verify that opening a server on an already-expired payment returns 410.

**Steps:**
1. Create a payment
2. Manually modify the payment file in mesh FS: set `expiresAt` to a past timestamp
3. Call `POST /api/incoming/<id> { "action": "open" }`

**Expected (410):**
```json
{ "status": 410, "message": "Payment expired" }
```

---

### TC-14: Access Control - Wrong User

**Goal:** Verify that an unauthorized endpoint cannot interact with a payment.

**Steps:**
1. ep-A creates payment to bob
2. A third endpoint (ep-C / charlie) tries:
   - `GET /api/incoming/<id>` → should get 403 Forbidden
   - `POST /api/incoming/<id> { "action": "open" }` → should get 403 (server opens but charlie can't connect through tunnel due to entrance restriction)

---

### TC-15: CLI - Send Without Required Options

**Goal:** Verify CLI validation.

**Steps:**
```
ztm payment send bob
# Missing --name

ztm payment send bob --name "X"
# Missing --amount

ztm payment send bob --name "X" --amount -5
# Invalid amount

ztm payment send bob --name "X" --amount abc
# Invalid amount
```

**Expected:** Each returns a descriptive error message.

---

### TC-16: CLI - Get Non-Existent Payment

**Goal:** Verify graceful handling of missing payment.

**Steps:**
```
ztm payment get 00000000-0000-0000-0000-000000000000
```

**Expected:** `Payment not found.`

---

### TC-17: App Exit Cleanup

**Goal:** Verify that active payment servers are cleaned up when the app exits.

**Steps:**
1. Create payment, open server (TC-07)
2. Stop the ep-A agent (or unload the payment app)

**Expected:**
- Agent log: `Closed payment server for <id>`
- All `pipy.listen` ports are released
- No orphaned listeners remain

---

### TC-18: Full End-to-End Flow

**Goal:** Complete happy path from creation to completion.

**Steps:**
1. **ep-A:** `ztm payment send bob --name "Premium Access" --amount 250 --currency EUR --description "1-year subscription"`
2. **Verify ep-B:** receives notification in logs
3. **ep-B:** `GET /api/incoming/<id>` → sees payment details, status=pending
4. **ep-B:** `POST /api/incoming/<id> { "action": "open" }` → gets `{ port, tunnelName }`
5. **ep-B:** `GET /` via tunnel → gets 402 with invoice
6. **ep-B:** `PUT /pay { "proof": "lightning-preimage-xyz" }` via tunnel → gets 200
7. **Verify ep-A:** `ztm payment get <id>` → status=completed, proof=lightning-preimage-xyz
8. **Verify:** server stopped, tunnel config cleaned up

---

## Test Matrix Summary

| ID | Category | Description | Priority |
|----|----------|-------------|----------|
| TC-01 | CRUD | Create payment | P0 |
| TC-02 | CRUD | List payments | P0 |
| TC-03 | CRUD | Get payment details | P0 |
| TC-04 | CRUD | Cancel payment | P1 |
| TC-05 | Access | Cancel denied for non-sender | P1 |
| TC-06 | Notification | Buyer receives notification | P1 |
| TC-07 | Server | Seller opens payment server | P0 |
| TC-08 | HTTP 402 | Invoice retrieval (402 response) | P0 |
| TC-09 | Payment | Complete via peer API | P0 |
| TC-10 | Payment | Complete via local HTTP | P1 |
| TC-11 | Cleanup | Auto-expiry and cleanup | P1 |
| TC-12 | Error | Pay on non-pending payment | P1 |
| TC-13 | Error | Open server on expired payment | P2 |
| TC-14 | Access | Unauthorized endpoint access | P1 |
| TC-15 | CLI | Validation errors | P2 |
| TC-16 | CLI | Non-existent payment | P2 |
| TC-17 | Lifecycle | App exit cleanup | P1 |
| TC-18 | E2E | Full happy path | P0 |
