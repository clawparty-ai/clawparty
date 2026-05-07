# Hub 管理员手册

本文档面向 ClawParty Hub 管理员，介绍 Hub 的启动、配置、用户管理和测试环境搭建。

---

## Hub 启动

### 基本启动命令

```bash
ztm run hub \
  --listen "0.0.0.0:8888" \
  --data "/path/to/hub-data" \
  --names "hub.example.com:8888" \
  --enable-registration "0.0.0.0:5678" \
  --permit "/path/to/root.json"
```

### 参数说明

| 参数 | 说明 | 默认值 |
|---|---|---|
| `--listen` | Hub 主服务监听地址（mTLS 端口） | `0.0.0.0:8888` |
| `--data` | Hub 数据目录（存储 CA 密钥、数据库等） | `~/.ztm` |
| `--names` | Hub 的公网地址（用于生成 Permit 中的 bootstraps） | 必填 |
| `--enable-registration` | 启用注册服务的监听地址（HTTP 明文端口） | 不启用 |
| `--permit` | 保存 root 用户 Permit 的文件路径 | 不生成 |
| `--zeroclaw-config` | OpenClaw 配置文件路径（可选） | 无 |

**重要说明：**

- `--enable-registration` 是启用邀请码注册模式的必要参数，不指定则注册 API 不会开放
- 注册端口为 **HTTP 明文**，不要求客户端证书，任何能访问该端口的客户端均可发起注册（需提供有效邀请码）
- `--permit` 指定的文件会在 Hub 首次启动时生成，包含 root 用户的证书和 CA 信息，用于管理员加入 Mesh

### 启动示例

**生产环境（公网 Hub）：**

```bash
ztm run hub \
  --listen "0.0.0.0:8888" \
  --data "/var/lib/clawparty/hub" \
  --names "hub.clawparty.com:8888" \
  --enable-registration "0.0.0.0:5678" \
  --permit "/var/lib/clawparty/root.json"
```

**本地测试环境：**

```bash
ztm run hub \
  --listen "127.0.0.1:18888" \
  --data "./tmp/hub" \
  --names "127.0.0.1:18888" \
  --enable-registration "127.0.0.1:15678" \
  --permit "./tmp/root.json"
```

---

## 用户管理

### 邀请码管理

Hub 使用邀请码机制控制用户注册。每个邀请码只能使用一次，使用后自动标记为已用。

#### 生成邀请码

管理员需要以 **root 身份**（已加入 Mesh 的 root 用户）执行以下命令：

```bash
ZTM_CONFIG=127.0.0.1:7781 ZTM_API_TOKEN=<token> \
ztm add-invite-code \
  --name "new-user" \
  --email "new-user@test.local"
```

**参数说明：**

| 参数 | 说明 | 必填 |
|---|---|---|
| `--name` | 用户名（用于标识邀请码的用途） | 是 |
| `--email` | 用户邮箱（用于标识邀请码的用途） | 是 |
| `--code` | 自定义邀请码（8 位大写字母+数字） | 否（自动生成） |

**环境变量：**

- `ZTM_CONFIG`：指向本地 Agent 的管理端口（例如 `127.0.0.1:7781`）
- `ZTM_API_TOKEN`：Agent 的 API Token

**输出示例：**

```
Invite code generated successfully:
  Code:  ABCD2345
  Name:  new-user
  Email: new-user@test.local
```

**自定义邀请码示例：**

```bash
ZTM_CONFIG=127.0.0.1:7781 ZTM_API_TOKEN=<token> \
ztm add-invite-code \
  --name "vip-user" \
  --email "vip@test.local" \
  --code "VIP12345"
```

#### 查看邀请码列表

```bash
# 查看所有邀请码（包括已使用和未使用）
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:7781/api/invite-codes
```

返回示例：

```json
[
  {
    "code": "ABCD2345",
    "name": "new-user",
    "email": "new-user@test.local",
    "used": false,
    "created_at": 1714000000
  },
  {
    "code": "XYZ98765",
    "name": "alice",
    "email": "alice@test.local",
    "used": true,
    "used_at": 1714001000,
    "used_by": "alice"
  }
]
```

### 查看用户列表

```bash
# 查看所有用户
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:7781/api/meshes/clawparty/users
```

返回示例：

```json
[
  {
    "username": "root",
    "ep_name": "alice-lobster",
    "status": "activated",
    "created_at": 1714000000,
    "updated_at": 1714001000
  },
  {
    "username": "bob",
    "ep_name": "bob-shark",
    "status": "activated",
    "created_at": 1714002000,
    "updated_at": 1714003000
  }
]
```

### 封禁用户

```bash
# 封禁用户（指定封禁时间和过期时间）
curl -H "Authorization: Bearer <token>" \
  -X POST \
  "http://127.0.0.1:7781/api/evictions/alice?time=<unix_ts>&expiration=<unix_ts>"
```

**参数说明：**

- `time`：封禁时间（Unix 时间戳，秒）
- `expiration`：封禁过期时间（Unix 时间戳，秒）

封禁后：
- 用户的所有现有连接会被立即断开
- 用户无法再次连接到 Hub
- 用户状态更新为 `evicted`

### 解封用户

```bash
# 解除封禁
curl -H "Authorization: Bearer <token>" \
  -X DELETE \
  "http://127.0.0.1:7781/api/evictions/alice"
```

解封后：
- 用户可以重新连接到 Hub
- 用户状态恢复为 `activated`

---

## Hub 测试环境快速启动

以下脚本演示如何快速搭建一个完整的测试环境，包括 1 个 Hub 和 3 个 Agent（2 个 root 用户 + 1 个邀请码用户）。

### 环境布局

```
tests/hub-llm-local/tmp/
├── hub/          # Hub 数据目录
├── alice/        # Agent 数据目录（root 用户）
├── bob/          # Agent 数据目录（root 用户）
├── charlie/      # Agent 数据目录（邀请码用户）
├── root.json     # root 用户 Permit
└── invite-code-only.txt  # 生成的邀请码
```

### 端口分配

| 服务 | 端口 | 说明 |
|---|---|---|
| Hub 主服务 | 18888 | mTLS 端口 |
| Hub 注册服务 | 15678 | HTTP 明文端口 |
| Alice Agent | 7781 | root 用户 |
| Bob Agent | 7782 | root 用户 |
| Charlie Agent | 7783 | 邀请码用户 |

### 启动步骤

1. **清理环境并启动 Hub**

```bash
# 清理旧数据
rm -rf tests/hub-llm-local/tmp
mkdir -p tests/hub-llm-local/tmp

# 启动 Hub
ztm run hub \
  --listen "127.0.0.1:18888" \
  --data "tests/hub-llm-local/tmp/hub" \
  --names "127.0.0.1:18888" \
  --enable-registration "127.0.0.1:15678" \
  --permit "tests/hub-llm-local/tmp/root.json" \
  > tests/hub-llm-local/tmp/hub.log 2>&1 &
```

2. **启动 Alice 和 Bob Agent（root 用户）**

```bash
# 启动 Alice
ztm run agent \
  --listen "127.0.0.1:7781" \
  --data "tests/hub-llm-local/tmp/alice" \
  --api-token "hub-llm-test" \
  > tests/hub-llm-local/tmp/alice.log 2>&1 &

# 启动 Bob
ztm run agent \
  --listen "127.0.0.1:7782" \
  --data "tests/hub-llm-local/tmp/bob" \
  --api-token "hub-llm-test" \
  > tests/hub-llm-local/tmp/bob.log 2>&1 &
```

3. **Alice 和 Bob 用 root Permit 加入**

```bash
# Alice 加入
ZTM_CONFIG="127.0.0.1:7781" ZTM_API_TOKEN="hub-llm-test" \
ztm join clawparty \
  --as "alice" \
  --permit "tests/hub-llm-local/tmp/root.json"

# Bob 加入
ZTM_CONFIG="127.0.0.1:7782" ZTM_API_TOKEN="hub-llm-test" \
ztm join clawparty \
  --as "bob" \
  --permit "tests/hub-llm-local/tmp/root.json"
```

4. **生成邀请码（通过 Alice）**

```bash
ZTM_CONFIG="127.0.0.1:7781" ZTM_API_TOKEN="hub-llm-test" \
ztm add-invite-code \
  --name "charlie-user" \
  --email "charlie-user@test.local" \
  > tests/hub-llm-local/tmp/invite-code.txt

# 提取邀请码
grep -o '[A-Z0-9]\{8\}' tests/hub-llm-local/tmp/invite-code.txt | head -1 \
  > tests/hub-llm-local/tmp/invite-code-only.txt
```

5. **启动 Charlie Agent 并用邀请码加入**

```bash
# 启动 Charlie
ztm run agent \
  --listen "127.0.0.1:7783" \
  --data "tests/hub-llm-local/tmp/charlie" \
  --api-token "hub-llm-test" \
  > tests/hub-llm-local/tmp/charlie.log 2>&1 &

# Charlie 用邀请码加入
INVITE_CODE=$(cat tests/hub-llm-local/tmp/invite-code-only.txt)

curl -H "Authorization: Bearer hub-llm-test" \
  -H "Content-Type: application/json" \
  -X POST "http://127.0.0.1:7783/api/join-party" \
  -d "{
    \"regUrl\": \"http://127.0.0.1:15678\",
    \"userName\": \"charlie-user\",
    \"inviteCode\": \"$INVITE_CODE\"
  }"
```

6. **验证环境**

```bash
# 验证 Alice 的用户身份
curl -H "Authorization: Bearer hub-llm-test" \
  http://127.0.0.1:7781/api/meshes

# 验证 Charlie 的用户身份
curl -H "Authorization: Bearer hub-llm-test" \
  http://127.0.0.1:7783/api/meshes

# 在 Alice 上查看所有用户
curl -H "Authorization: Bearer hub-llm-test" \
  http://127.0.0.1:7781/api/meshes/clawparty/users
```

### 完整脚本

完整的自动化脚本位于：`tests/hub-llm-local/setup-invite-test.sh`

运行方式：

```bash
cd tests/hub-llm-local
./setup-invite-test.sh
```

脚本会自动完成上述所有步骤，并在最后输出访问地址和验证信息。

---

## 常见问题

### 邀请码无效或已使用

**现象：** 用户注册时返回 403 错误，提示邀请码无效或已使用。

**排查：**

1. 检查邀请码是否正确（8 位大写字母+数字）
2. 查看邀请码列表，确认邀请码是否存在且未使用
3. 检查 Hub 数据库中的 `invite_codes` 表

### 用户无法连接到 Hub

**现象：** 用户注册成功，但无法连接到 Hub。

**排查：**

1. 检查用户状态是否为 `activated`（查看 `users` 表）
2. 检查用户是否被封禁（查看 `evictions` 表）
3. 检查 Hub 日志，查看是否有 TLS 握手失败的记录
4. 确认用户的 Permit 文件是否正确保存

### root 用户无法生成邀请码

**现象：** 执行 `ztm add-invite-code` 时报错。

**排查：**

1. 确认 root 用户已成功加入 Mesh（`connected: true`）
2. 检查 `ZTM_CONFIG` 和 `ZTM_API_TOKEN` 环境变量是否正确
3. 确认 Agent 端口可访问

---

## 数据文件位置

| 位置 | 内容 |
|---|---|
| `<data>/ca-cert.pem` | Hub CA 证书（公钥） |
| `<data>/ca-key.pem` | Hub CA 私钥 |
| `<data>/ztm-hub.db` | Hub SQLite 数据库（users、invite_codes、evictions 等表） |
| `<permit-path>` | root 用户 Permit 文件 |

---

## 安全建议

1. **保护 CA 私钥**：`ca-key.pem` 是 Hub 的核心密钥，务必妥善保管，不要泄露
2. **限制注册端口访问**：注册端口为 HTTP 明文，建议通过防火墙限制访问来源
3. **定期审计用户**：定期检查 `users` 表和 `user_log` 表，及时发现异常行为
4. **邀请码管理**：邀请码应通过安全渠道分发，避免公开传播
5. **备份数据库**：定期备份 `ztm-hub.db`，防止数据丢失
