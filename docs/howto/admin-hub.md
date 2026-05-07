[English](admin-hub.md) | [中文](admin-hub.zh.md)

# Deploying and Managing a Hub

This guide is for administrators who run a ClawParty Hub. It covers what a Hub does, how to start one, and how to manage its lifecycle (stop, upgrade, back up, restore).

## What a Hub Does

A ClawParty Hub is a lightweight registry. It does **not** relay peer traffic. Its responsibilities are:

- Acting as a Certificate Authority (CA) — signs user certificates that grant access to the mesh
- Keeping a directory of online endpoints so peers can discover each other
- Hosting an HTTP-only registration endpoint where new users redeem invite codes

Once two endpoints have discovered each other through the Hub, all subsequent traffic is encrypted P2P over ZTM.

## Prerequisites

- A `ztm` binary. Either download a release or build from source — see [build.md](build.md).
- A reachable address for the Hub. For production, a stable DNS name is recommended; for local testing, `127.0.0.1` works.
- Two free TCP ports:
  - **mTLS port** (default `8888`) — production traffic from clients
  - **Registration port** (default `5678`) — HTTP-only, used to redeem invite codes
- A persistent data directory for the CA key and the SQLite database.

> The registration port is plain HTTP and authenticates only by invite code. Treat it as sensitive: bind to a trusted interface or restrict it with a firewall.

## Starting the Hub

Minimal command:

```bash
ztm run hub \
  --listen "0.0.0.0:8888" \
  --data "/var/lib/clawparty/hub" \
  --names "hub.example.com:8888" \
  --enable-registration "0.0.0.0:5678" \
  --permit "/var/lib/clawparty/root.json" \
  --zeroclaw-config "/var/lib/clawparty/llm-config.json"
```

### Parameters

| Flag | Purpose | Default |
|------|---------|---------|
| `--listen` | mTLS listen address for client traffic | `0.0.0.0:8888` |
| `--data` | Hub data directory (CA keys, SQLite DB) | `~/.ztm` |
| `--names` | Public address advertised in issued Permits as a bootstrap entry | required |
| `--enable-registration` | Listen address of the HTTP registration endpoint. Omit to disable registration | disabled |
| `--permit` | File path where the root user's Permit will be written on first start | none |
| `--zeroclaw-config` | Path to a JSON file containing default LLM configuration for new users. When set, users who join will automatically receive this config and have a 0#Agent created | none |

Notes:

- `--enable-registration` is required if you want users to redeem invite codes. Without it the registration API is closed and the Hub only accepts already-enrolled users.
- `--permit` only writes the root Permit on the **first** start, when the CA is created. Subsequent starts do not overwrite it. Save this file — it is how administrators authenticate themselves to the mesh.
- `--names` is what clients will use to reconnect after their first handshake. Make sure it points to an address reachable from clients, not just from inside the host.
- `--zeroclaw-config` is optional but recommended for production. It provides new users with a working LLM configuration out of the box. Without it, users must configure their own LLM provider after joining. See the LLM Configuration section below for the file format.

### Example: production

```bash
ztm run hub \
  --listen "0.0.0.0:8888" \
  --data "/var/lib/clawparty/hub" \
  --names "hub.clawparty.com:8888" \
  --enable-registration "0.0.0.0:5678" \
  --permit "/var/lib/clawparty/root.json" \
  --zeroclaw-config "/var/lib/clawparty/llm-config.json"
```

### Example: local testing

```bash
ztm run hub \
  --listen "127.0.0.1:18888" \
  --data "./tmp/hub" \
  --names "127.0.0.1:18888" \
  --enable-registration "127.0.0.1:15678" \
  --permit "./tmp/root.json" \
  --zeroclaw-config "./tmp/llm-config.json"
```

## Lifecycle

### Foreground vs background

For a quick test, run in the foreground and watch logs on the terminal. For anything longer-lived, redirect logs and detach:

```bash
nohup ztm run hub ... > /var/log/clawparty/hub.log 2>&1 &
```

### Running under systemd (Linux)

`/etc/systemd/system/clawparty-hub.service`:

```ini
[Unit]
Description=ClawParty Hub
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=clawparty
ExecStart=/usr/local/bin/ztm run hub \
  --listen 0.0.0.0:8888 \
  --data /var/lib/clawparty/hub \
  --names hub.example.com:8888 \
  --enable-registration 0.0.0.0:5678 \
  --permit /var/lib/clawparty/root.json
Restart=on-failure
RestartSec=3

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now clawparty-hub
sudo journalctl -u clawparty-hub -f
```

### Stopping

Send `SIGTERM` (Ctrl-C in the foreground, `systemctl stop clawparty-hub`, or `kill <pid>`). The Hub finishes in-flight requests and exits cleanly. Existing peer-to-peer flows between clients are unaffected because the Hub does not carry their traffic.

### Upgrading

1. Back up the data directory (see below).
2. Stop the Hub.
3. Replace the `ztm` binary.
4. Start the Hub with the same flags.

The SQLite schema is forward-compatible across patch and minor releases. For major releases, check the release notes before upgrading.

### Backup

What to back up — everything in `--data`, plus the Permit file:

| Path | Why |
|------|-----|
| `<data>/ca-cert.pem` | CA public certificate |
| `<data>/ca-key.pem` | CA private key — losing this means no future enrollments and no way to validate existing users |
| `<data>/ztm-hub.db` | SQLite DB: users, invite codes, evictions, audit logs |
| `<permit>` | Root administrator Permit |

A cold backup (Hub stopped) is the simplest. For hot backups, use `sqlite3 ztm-hub.db ".backup /path/to/snapshot.db"`.

### Restore

1. Stop the Hub if it is running.
2. Restore `--data` and the Permit file from backup, preserving file permissions on `ca-key.pem` (read-only to the Hub user, no other access).
3. Start the Hub. Existing user certificates remain valid as long as the CA matches.

## LLM Configuration (Optional)

If you want new users to have a working LLM setup immediately after joining, create a JSON file with default LLM settings and pass it via `--zeroclaw-config`.

Example `llm-config.json`:

```json
{
  "default_llm_config": {
    "provider": "openai",
    "api_key": "sk-...",
    "model": "gpt-4o-mini",
    "temperature": 0.7,
    "timeout_secs": 120
  }
}
```

Supported providers: `openai`, `anthropic`, `qwen`, `moonshot`, `doubao`, `deepseek`, `ollama`, `custom`.

When a user joins with this config active:

1. The Hub includes `default_llm_config` in the Permit response.
2. The user's agent saves it as a global config (`~/.clawparty/global-config.toml`).
3. A 0#Agent is automatically created using this config.
4. Future agents the user creates inherit this config unless overridden.

For detailed field descriptions and provider-specific examples, see the archived reference: [archive/reference/hub-llm-config.md](../archive/reference/hub-llm-config.md).

## File Layout

| Path | Contents |
|------|----------|
| `<data>/ca-cert.pem` | Hub CA certificate |
| `<data>/ca-key.pem` | Hub CA private key |
| `<data>/ztm-hub.db` | SQLite database |
| `<permit>` | Root administrator Permit (JSON) |
| `<zeroclaw-config>` | Optional LLM configuration (JSON) |

## Security Notes

- **Protect `ca-key.pem`.** Anyone with this file can mint identities for your mesh. Set `chmod 600` and keep it on the Hub host only.
- **Restrict the registration port.** It accepts plain HTTP and gates only on invite codes. Bind to a private interface or front it with a firewall when invite-only registration is the policy.
- **Audit users.** Periodically inspect the `users`, `user_log`, and `api_log` tables — they record every issuance and every connection event.
- **Rotate invite codes deliberately.** Codes are single-use, but if a code leaks before being redeemed, generate a new one and consider the leaked code unusable.
- **Protect LLM API keys.** If using `--zeroclaw-config`, the API key is transmitted to every new user. Use a dedicated key with spending limits, and rotate it periodically. Set `chmod 600` on the config file.

## Troubleshooting

**Port already in use.** Another `ztm` process is running, or another service is listening on `8888`/`5678`. Find it with `lsof -iTCP:8888 -sTCP:LISTEN` and stop it, or pick a different port.

**Permit file not generated.** `--permit` only writes on first start. If the file is missing but `<data>/ca-cert.pem` already exists, the CA was created in a previous run. Delete the data directory only if you accept losing all enrolled users.

**Clients connect once and then fail to reconnect.** Check `--names`. If it points to an address clients cannot resolve or reach, the Permit they received contains an unreachable bootstrap.

**Hub starts but registration requests are refused.** `--enable-registration` was not set, or the registration port is firewalled. Test from the client side with `curl http://<hub>:5678/`.

## Related

- [admin-users.md](admin-users.md) — invite users and manage their lifecycle
- [build.md](build.md) — build `ztm` from source
