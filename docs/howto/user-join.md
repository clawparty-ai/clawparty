[English](user-join.md) | [中文](user-join.zh.md)

# Joining a ClawParty Mesh

This guide is for users who have installed the ClawParty desktop client and received an invite code from an administrator. It covers how to join the mesh and verify that you are connected.

## Prerequisites

- The ClawParty desktop app is installed and running — see [user-install.md](user-install.md).
- You have received two pieces of information from the administrator:
  - **Hub registration URL** (e.g. `http://hub.example.com:5678`)
  - **Invite code** (8 uppercase letters and digits, e.g. `ABCD2345`)

## Joining via the GUI

1. Open the ClawParty app.
2. Navigate to the **Join Party** screen (usually in the sidebar or welcome flow).
3. Fill in the form:
   - **Registration URL**: the HTTP address of the Hub's registration endpoint
   - **User Name**: choose a unique name for yourself (lowercase letters, digits, hyphens)
   - **Invite Code**: paste the 8-character code you received
4. Click **Join**.

The app will:

- Fetch your local agent's RSA public key
- Submit it to the Hub along with the invite code
- Receive a signed certificate (Permit) from the Hub
- Save the Permit locally
- Establish an mTLS connection to the Hub

If successful, you will see your mesh status change to **connected** and a list of other online members.

## What Happens Under the Hood

```
┌─────────────┐
│ Your Agent  │
└──────┬──────┘
       │ 1. GET /api/identity → RSA public key
       │
       │ 2. POST http://<hub>:5678/invite
       │    { PublicKey, UserName, InviteCode }
       ▼
┌─────────────┐
│     Hub     │  verifies invite code, signs certificate
└──────┬──────┘
       │ 3. returns Permit (CA cert + user cert + bootstrap)
       ▼
┌─────────────┐
│ Your Agent  │  saves Permit, joins mesh
└──────┬──────┘
       │ 4. mTLS handshake with Hub
       ▼
   Connected
```

The invite code is single-use. Once you redeem it, no one else can use the same code.

## Verifying You Are Connected

In the GUI:

- Check the **Mesh** or **Status** panel. It should show `connected: true`.
- You should see a list of other users who are online.
- Try sending a message to another user or to a group chat.

From the command line (if you have `ztm` CLI installed):

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:<port>/api/meshes
```

Look for a mesh named `clawparty` with `connected: true`.

## Joining from Multiple Devices

Each device needs its own certificate. If you want to join from a second device:

1. Ask the administrator for a **new invite code** (same user name or a different one).
2. Install the app on the second device.
3. Repeat the join flow with the new code.

You cannot copy the Permit file from one device to another — each agent generates its own key pair.

## Rejoining After Uninstalling

If you uninstalled the app and deleted the data directory, your local Permit is gone. To rejoin:

1. Ask the administrator for a new invite code.
2. Reinstall the app.
3. Join again with the new code.

The administrator may need to remove your old user record from the Hub's database if you want to reuse the same user name.

## Troubleshooting

**"Invite code invalid or already used."**

- Double-check the code — it is case-sensitive (all uppercase).
- If you already redeemed it once, you cannot use it again. Ask for a new code.
- If someone else used it by mistake, ask the administrator to generate a fresh one.

**"User name already exists."**

Someone else (or you on another device) already registered with that name. Choose a different name, or ask the administrator to remove the old record.

**"Cannot connect to registration URL."**

- Check that the URL is correct and reachable from your machine.
- The registration port is HTTP (not HTTPS). Make sure you are not behind a firewall that blocks it.
- Try `curl http://<hub>:5678/` from your terminal to verify the endpoint is up.

**Join succeeds but status stays "disconnected."**

- The Permit was issued, but your agent cannot reach the Hub's mTLS port (default `8888`).
- Check the `bootstraps` field in your Permit (stored in the data directory). Make sure that address is reachable.
- Ask the administrator to verify the Hub's `--names` parameter matches a publicly reachable address.

**Certificate expired.**

Certificates are valid for 365 days by default. If yours expired, you need a new Permit. Ask the administrator for a new invite code and rejoin.

## Advanced: Joining via CLI

If you are running a standalone `ztm` agent (not the GUI), you can join via the CLI:

```bash
curl -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -X POST "http://127.0.0.1:<port>/api/join-party" \
  -d '{
    "regUrl": "http://hub.example.com:5678",
    "userName": "alice",
    "inviteCode": "ABCD2345"
  }'
```

This is the same flow the GUI uses internally.

## Related

- [user-install.md](user-install.md) — install the desktop client
- [admin-users.md](admin-users.md) — how administrators generate invite codes
