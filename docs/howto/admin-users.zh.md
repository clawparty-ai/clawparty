[English](admin-users.md) | [中文](admin-users.zh.md)

# 邀请用户与用户生命周期管理

本文面向 ClawParty 管理员，介绍如何邀请新用户、观察其活动以及封禁违规用户。

## 背景：用户身份模型

ClawParty 用户的身份是 **TLS 客户端证书**，没有传统密码。新用户接入时，Hub 完成以下步骤：

1. 通过注册端点接收用户的 RSA 公钥。
2. 校验请求中的邀请码有效且未被使用。
3. 用 Hub 的 CA 私钥为公钥签发一张证书（CN = 用户名）。
4. 返回 **Permit** — 一份包含 CA 证书、用户证书、Hub bootstrap 地址的 JSON 包，由客户端本地持久化。

之后该用户的 `ztm` agent 通过 mTLS 连接 Hub。Hub 在 `users` 表中跟踪每一个曾签发过证书的用户及其当前状态。

## 用户状态

| 状态 | 含义 |
|------|------|
| `registering` | 请求已受理，证书签发中 |
| `register-failed` | 签发失败（公钥无效、CA 错误） |
| `permit-issued` | 证书已签发、Permit 已返回，等待首次连接 |
| `activated` | 用户已用 Permit 至少成功连接过一次 |
| `evicted` | 用户已被封禁，现有连接被踢、新连接被拒 |

状态流转：

```
 registering ──→ register-failed
      │
      └──→ permit-issued ──→ activated ⇄ evicted
```

## 邀请新用户

### 生成邀请码

邀请码由已加入 mesh 的 root 管理员 agent 生成。在管理员主机上执行：

```bash
ZTM_CONFIG=127.0.0.1:7781 ZTM_API_TOKEN=<token> \
ztm add-invite-code \
  --name  "alice" \
  --email "alice@example.com"
```

输出：

```
Invite code generated successfully:
  Code:  ABCD2345
  Name:  alice
  Email: alice@example.com
```

| 参数 | 用途 | 必填 |
|------|------|------|
| `--name` | 标注邀请码用途的名字 | 是 |
| `--email` | 标注邀请码用途的邮箱 | 是 |
| `--code` | 自定义 8 位邀请码（大写字母 + 数字），不指定则随机生成 | 否 |

`--name` / `--email` 仅作管理员侧的备注，不会限制谁能兑换邀请码。任何拿到这 8 位字符的人都可以使用。

环境变量：

- `ZTM_CONFIG` — 本地 agent 地址（例如 `127.0.0.1:7781`）
- `ZTM_API_TOKEN` — agent 的 API token

### 分发邀请码

通过私密渠道把邀请码发给目标用户。邀请码一次性使用：兑换后不能再创建新身份。

### 查看邀请码列表

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:7781/api/invite-codes
```

响应：

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

## 观察用户

### 用户列表

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

### 单用户审计日志

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:7781/api/user-log/alice?limit=50
```

记录的事件：

| action | 触发时机 |
|--------|----------|
| `cert_issued` | 为该用户签发了证书 |
| `connect` | endpoint 建立首个连接 |
| `disconnect` | endpoint 最后一个连接断开 |
| `evict` | 管理员封禁用户 |
| `evict_removed` | 管理员解除封禁 |

不带用户名查询（`/api/user-log`）返回所有用户事件，仅限 `root` 调用。

### 注册请求日志

每一次访问注册端口的请求都会写入 `api_log`。排查可疑请求时直接查 SQLite：

```bash
sqlite3 /var/lib/clawparty/hub/ztm-hub.db \
  "SELECT time, client_ip, status, username, detail FROM api_log ORDER BY time DESC LIMIT 20"
```

## 封禁用户

封禁立即踢掉该用户当前所有连接，并在 TLS 层拒绝后续连接。

### 封禁

```bash
NOW=$(date +%s)
EXPIRY=$((NOW + 86400 * 30))   # 30 天

curl -H "Authorization: Bearer <token>" \
  -X POST \
  "http://127.0.0.1:7781/api/evictions/alice?time=$NOW&expiration=$EXPIRY"
```

查询参数：

- `time` — 封禁生效时间（Unix 秒）
- `expiration` — 封禁到期时间（Unix 秒）

效果：

- `users.status` 变为 `evicted`
- `evictions` 表写入一行记录
- 证书签发时间早于 `time` 的所有 TLS 会话立即被踢
- 该用户后续 TLS 握手在证书校验阶段就被拒绝
- `users` 中该用户名仍保留。重名再注册返回 `403`，而不是 `409`

### 解封

```bash
curl -H "Authorization: Bearer <token>" \
  -X DELETE \
  "http://127.0.0.1:7781/api/evictions/alice"
```

效果：

- 删除 `evictions` 中的封禁记录
- 若 `users.status` 仍是 `evicted`，恢复为 `activated`
- 证书未过期则可直接重连

### 解封后重新签发证书

若解封后用户证书已过期或不可用，需要新 Permit。同名重新注册会返回 `409 user already exists`。管理员需先从 `users` 表删除该记录：

```bash
sqlite3 /var/lib/clawparty/hub/ztm-hub.db "DELETE FROM users WHERE username = 'alice'"
```

然后为该用户生成新邀请码即可。

## 常见问题

**邀请码被判为无效或已使用。** 查 `invite_codes` 表 — 邀请码区分大小写（统一大写），成功兑换后 `used` 翻为 `true`。用户输错的话，直接生成新码，不要试图回滚已兑换的码。

**用户卡在 `permit-issued`。** 证书已签发并返回，但客户端从未完成过 TLS 握手。让用户检查其 Permit 中的 `bootstraps` 是否从其本机可达。

**`add-invite-code` 报错。** 管理员 agent 必须已加入 mesh（`/api/meshes` 中 `connected: true`），且身份是 `root`。检查 `ZTM_CONFIG` / `ZTM_API_TOKEN` 是否指向正确的 agent。

**封禁后用户仍能发消息。** 封禁在 TLS 层生效，但其他对端通过 ZTFS 已收下的消息不会被回溯删除。封禁阻止后续流量，不抹除历史流量。

## 相关文档

- [admin-hub.zh.md](admin-hub.zh.md) — 部署与运行 Hub
- [user-join.zh.md](user-join.zh.md) — 用户兑换邀请码的实际体验
