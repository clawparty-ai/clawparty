[English](admin-users.md) | [中文](admin-users.zh.md)

# Inviting Users and Managing Their Lifecycle

This guide is for ClawParty administrators. It covers how to invite new users, observe what they do, and evict misbehaving ones.

## Background: How User Identity Works

ClawParty users authenticate by **TLS client certificate**, not by password. To onboard a user, the Hub:

1. Receives the user's RSA public key over the registration endpoint.
2. Verifies the request carries a valid, unused invite code.
3. Signs a certificate (CN = user name) with the Hub's CA.
4. Returns a **Permit** — a JSON bundle containing the CA cert, the user cert, and the Hub's bootstrap address — which the client persists locally.

From that point on the user's `ztm` agent connects to the Hub over mTLS. The Hub keeps a `users` table tracking everyone it has ever issued a certificate for, plus their current state.

## User States

| State | Meaning |
|-------|---------|
| `registering` | Request accepted, certificate issuance in progress |
| `register-failed` | Issuance failed (invalid public key, CA error) |
| `permit-issued` | Certificate signed, Permit returned, awaiting first connection |
| `activated` | User has connected to the Hub at least once with the Permit |
| `evicted` | User is banned; existing connections are dropped and new ones rejected |

State transitions:

```
 registering ──→ register-failed
      │
      └──→ permit-issued ──→ activated ⇄ evicted
```

## Inviting a New User

### Generate an invite code

Invite codes are issued from an authenticated administrator agent. Run from a host where you have a `ztm` agent already joined to the mesh as `root`:

```bash
ZTM_CONFIG=127.0.0.1:7781 ZTM_API_TOKEN=<token> \
ztm add-invite-code \
  --name  "alice" \
  --email "alice@example.com"
```

Output:

```
Invite code generated successfully:
  Code:  ABCD2345
  Name:  alice
  Email: alice@example.com
```

| Flag | Purpose | Required |
|------|---------|----------|
| `--name` | Label identifying who the code is for | yes |
| `--email` | Label identifying who the code is for | yes |
| `--code` | Custom 8-character code (uppercase letters and digits). If omitted one is generated | no |

`--name` and `--email` are descriptive labels for your records — they do not constrain who can redeem the code. Anyone holding the 8-character string can use it.

Environment:

- `ZTM_CONFIG` — the local agent address (e.g. `127.0.0.1:7781`)
- `ZTM_API_TOKEN` — the agent's API token

### Distribute the code

Send the code to the recipient through a private channel. Codes are single-use: once redeemed, the same code cannot create another identity.

### List invite codes

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:7781/api/invite-codes
```

Response:

```json
[
  {
    "code": "ABCD2345",
    "name": "alice",
    "email": "alice@example.com",
    "used": false,
    "created_at": 1714000000
  },
  {
    "code": "XYZ98765",
    "name": "bob",
    "email": "bob@example.com",
    "used": true,
    "used_at": 1714001000,
    "used_by": "bob"
  }
]
```

## Observing Users

### List users

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:7781/api/meshes/clawparty/users
```

```json
[
  { "username": "root",  "ep_name": "alice-lobster", "status": "activated" },
  { "username": "alice", "ep_name": "alice-mac",     "status": "activated" },
  { "username": "bob",   "ep_name": "bob-laptop",    "status": "evicted"   }
]
```

### Per-user audit log

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:7781/api/user-log/alice?limit=50
```

Events recorded:

| action | When |
|--------|------|
| `cert_issued` | Certificate signed for this user |
| `connect` | Endpoint established its first connection |
| `disconnect` | Endpoint's last connection dropped |
| `evict` | Administrator banned the user |
| `evict_removed` | Administrator unbanned the user |

Querying without a username (`/api/user-log`) returns events for all users; this is restricted to `root`.

### Registration request log

Every hit to the registration port is recorded in `api_log`. Inspect the SQLite DB directly when investigating a suspicious request:

```bash
sqlite3 /var/lib/clawparty/hub/ztm-hub.db \
  "SELECT time, client_ip, status, username, detail FROM api_log ORDER BY time DESC LIMIT 20"
```

## Evicting a User

Eviction immediately drops the user's connections and refuses future ones at the TLS layer.

### Ban

```bash
NOW=$(date +%s)
EXPIRY=$((NOW + 86400 * 30))   # 30 days

curl -H "Authorization: Bearer <token>" \
  -X POST \
  "http://127.0.0.1:7781/api/evictions/alice?time=$NOW&expiration=$EXPIRY"
```

Query parameters:

- `time` — ban effective time (Unix seconds)
- `expiration` — ban end time (Unix seconds)

Effects:

- `users.status` becomes `evicted`
- A row is written to the `evictions` table
- All TLS sessions whose certificate was issued before `time` are dropped immediately
- Subsequent TLS handshakes from this user are rejected during certificate validation
- The username remains in `users`. Re-registering with the same name returns `403`, not `409`

### Unban

```bash
curl -H "Authorization: Bearer <token>" \
  -X DELETE \
  "http://127.0.0.1:7781/api/evictions/alice"
```

Effects:

- The eviction row is deleted
- If `users.status` is still `evicted`, it is restored to `activated`
- The user can reconnect provided their certificate has not expired

### Re-issuing a certificate after eviction

If a user's certificate is expired or otherwise unusable after unban, they need a fresh Permit. Re-running the registration flow with the same username returns `409 user already exists`. To allow re-registration, an administrator must remove the row from `users` first:

```bash
sqlite3 /var/lib/clawparty/hub/ztm-hub.db "DELETE FROM users WHERE username = 'alice'"
```

Then issue a new invite code for that user.

## Troubleshooting

**Invite code rejected as invalid or used.** Check the `invite_codes` table — codes are case-sensitive uppercase, and a successful redemption flips `used` to `true`. If the user mistyped, generate a new code rather than trying to undo the redemption.

**User stuck in `permit-issued`.** The certificate was signed and returned, but the client never managed a TLS handshake. Have them check that the Hub address in their Permit (`bootstraps`) is reachable from their machine.

**`add-invite-code` returns an error.** The administrator agent must be joined to the mesh (`connected: true` in `/api/meshes`) and must be authenticated as `root`. Verify `ZTM_CONFIG` and `ZTM_API_TOKEN` point at the right agent.

**Evicted user can still send messages.** Eviction takes effect at the TLS layer, but ZTFS-cached messages already received by peers are not retroactively deleted. Eviction stops new traffic; it does not redact past traffic.

## Related

- [admin-hub.md](admin-hub.md) — deploy and run the Hub
- [user-join.md](user-join.md) — what users see when they redeem an invite code
