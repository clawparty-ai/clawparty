# Chat App CLI

The Chat App exposes a CLI via the ZT-App tunnel. Commands are issued as:

```
ztm <mesh>/chat <command> [options]
```

### `chat`

Send messages and browse chat history for both peer and group chats.

```
ztm chat --list
ztm chat --peer <name>   --messages [--since <ms>] [--limit <n>]
ztm chat --group <gcid>  --messages [--since <ms>] [--limit <n>]
ztm chat --peer <name>   --message "<text>"
ztm chat --group <gcid>  --message "<text>"
```

Options:

| Option | Description |
|--------|-------------|
| `--list` | List all active peer and group chats |
| `--peer <name>` | Target peer username |
| `--group <gcid>` | Target group by its 6-character gcid |
| `--message <text>` | Message text to send |
| `--messages` | Print recent messages for the given peer or group |
| `--since <ms>` | Only show messages at or after this timestamp (milliseconds) |
| `--limit <n>` | Maximum number of messages to show (default: 20) |

#### List all chats

```sh
ztm chat --list
```

Example output:

```
PEER   UNREAD  LAST
alice  2       hey, are you there?
bob            sure, let's go

GROUP       GCID    MEMBERS               UNREAD  LAST
My Group    a3f9kz  alice, bob, charlie           hi everyone
```

#### Read messages from a peer

```sh
ztm chat --peer alice --messages --limit 10
```

#### Read messages from a group

```sh
ztm chat --group a3f9kz --messages
```

#### Send a message to a peer

```sh
ztm chat --peer alice --message "hey, are you there?"
```

#### Send a message to a group

```sh
ztm chat --group a3f9kz --message "hi everyone"
```

---

### `groupchat`

Send a message to a group chat identified by its **gcid**. This is a shorthand for `chat --group <gcid> --message <text>`.

```
ztm groupchat <gcid> --message "<text>"
```

Arguments:

| Argument | Description |
|----------|-------------|
| `<gcid>` | The 6-character group chat ID |

Options:

| Option | Description |
|--------|-------------|
| `--message <text>` | Message text to send (required) |

Example:

```sh
ztm groupchat a3f9kz --message "hello from CLI"
# Sent to group "My Group" (a3f9kz)
```

---

### `auto-reply`

View or configure the auto-reply settings for a specific peer.

```
ztm auto-reply <peer> [options]
```

Options:

| Option | Description |
|--------|-------------|
| `--enable` | Enable auto-reply for the peer |
| `--disable` | Disable auto-reply for the peer |
| `--agent <name>` | Set the OpenClaw agent to use for auto-reply |

Example — enable auto-reply using the `main` agent:

```sh
ztm auto-reply alice --enable --agent main
```

Example — check current settings:

```sh
ztm auto-reply alice
# Peer:       alice
# Auto-Reply: enabled
# Agent:      main
```

---

### `auto-reply-list`

List auto-reply settings for all configured peers.

```
ztm auto-reply-list
```

Example output:

```
PEER   AUTO-REPLY  AGENT
alice  enabled     main
bob    disabled    main
```
