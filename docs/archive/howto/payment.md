# Payment Guide

The Payment app provides zero-trust payment functionality using the ZTM mesh network. It enables secure payment requests between users with HTTP 402 protocol support.

## CLI Commands

The Payment App exposes a CLI via the ZT-App tunnel. Commands are issued as:

```
ztm payment <command> [options]
```

### `send`

Send a payment request to another user.

```
ztm payment send <username> [options]
```

Arguments:

| Argument | Description |
|----------|-------------|
| `<username>` | Receiver's username (required) |

Options:

| Option | Description |
|--------|-------------|
| `--name <string>` | Product name (required) |
| `--amount <number>` | Payment amount (required, must be positive) |
| `--currency <string>` | Currency code (default: USD) |
| `--description <string>` | Product description |
| `--url <string>` | Product URL |

Example:

```sh
ztm payment send alice --name "Premium Service" --amount 99.99 --currency USD --description "Monthly subscription"
# Payment created:
#   ID:       abc12345-6789-def0-1234-567890abcdef
#   To:       alice
#   Product:  Premium Service
#   Amount:   99.99 USD
#   Status:   pending
```

---

### `list`

List all your payments.

```
ztm payment list
```

Example output:

```
ID        DIRECTION  PRODUCT          AMOUNT    STATUS
abc12345  -> alice   Premium Service  99.99 USD pending
def67890  <- bob     API Access       19.99 USD completed
```

---

### `get`

Get detailed information about a specific payment.

```
ztm payment get <payment-id>
```

Arguments:

| Argument | Description |
|----------|-------------|
| `<payment-id>` | Payment ID (required) |

Example:

```sh
ztm payment get abc12345-6789-def0-1234-567890abcdef
# Payment: abc12345-6789-def0-1234-567890abcdef
#   Sender:    root
#   Receiver:  alice
#   Product:   Premium Service
#   Desc:      Monthly subscription
#   Amount:    99.99 USD
#   Status:    pending
#   Created:   2026-03-26T10:30:00.000Z
#   Expires:   2026-03-26T10:40:00.000Z
```

---

### `buy`

Buy a payment by opening a payment server and connecting to it.

```
ztm payment buy <payment-id> [options]
```

Arguments:

| Argument | Description |
|----------|-------------|
| `<payment-id>` | Payment ID (required) |

Options:

| Option | Description |
|--------|-------------|
| `--ep <endpoint>` | Seller endpoint to connect to (required) |

Example:

```sh
ztm payment buy abc12345-6789-def0-1234-567890abcdef --ep seller-ep
# Opening payment server on seller-ep...
# Payment: Premium Service - 99.99 USD
# Status:  pending
#
# To complete payment, use the payment GUI or call:
#   ztm payment get abc12345-6789-def0-1234-567890abcdef
```

---

### `cancel`

Cancel a pending payment (only the sender can cancel).

```
ztm payment cancel <payment-id>
```

Arguments:

| Argument | Description |
|----------|-------------|
| `<payment-id>` | Payment ID (required) |

Example:

```sh
ztm payment cancel abc12345-6789-def0-1234-567890abcdef
# Payment abc12345-6789-def0-1234-567890abcdef cancelled.
```

---

## Workflow

### Typical Payment Flow

1. **Sender** creates a payment request:
   ```sh
   ztm payment send receiver --name "Product" --amount 100 --currency USD
   ```

2. **Receiver** can view pending payments:
   ```sh
   ztm payment list
   ```

3. **Receiver** gets payment details:
   ```sh
   ztm payment get <payment-id>
   ```

4. **Receiver** opens payment server and connects:
   ```sh
   ztm payment buy <payment-id> --ep sender-ep
   ```

5. **Receiver** pays via HTTP 402 protocol

6. **Payment** status changes to `completed`

### Cancel Payment

If sender wants to cancel:
```sh
ztm payment cancel <payment-id>
```

## Payment Status

| Status | Description |
|--------|-------------|
| `pending` | Payment created, waiting for action |
| `processing` | Payment server opened, buyer can pay |
| `completed` | Payment successful with proof |
| `expired` | Payment expired (10 minute timeout) |
| `failed` | Payment cancelled by sender |
