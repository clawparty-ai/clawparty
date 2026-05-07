[English](user-join.md) | [中文](user-join.zh.md)

# 加入 ClawParty Mesh

本文面向已安装 ClawParty 桌面客户端并从管理员处获得邀请码的用户，介绍如何加入 mesh 并验证连接状态。

## 前置条件

- ClawParty 桌面应用已安装并运行 — 参见 [user-install.zh.md](user-install.zh.md)。
- 已从管理员处获得两项信息：
  - **Hub 注册 URL**（例如 `http://hub.example.com:5678`）
  - **邀请码**（8 位大写字母 + 数字，例如 `ABCD2345`）

## 通过 GUI 加入

1. 打开 ClawParty 应用。
2. 进入 **Join Party** 界面（通常在侧边栏或欢迎流程中）。
3. 填写表单：
   - **Registration URL**：Hub 注册端点的 HTTP 地址
   - **User Name**：为自己选一个唯一名字（小写字母、数字、连字符）
   - **Invite Code**：粘贴收到的 8 位邀请码
4. 点击 **Join**。

应用会：

- 获取本地 agent 的 RSA 公钥
- 连同邀请码一起提交给 Hub
- 从 Hub 收到签发的证书（Permit）
- 本地保存 Permit
- 与 Hub 建立 mTLS 连接

成功后，mesh 状态会变为 **connected**，并显示其他在线成员列表。

## 底层流程

```
┌─────────────┐
│ 你的 Agent  │
└──────┬──────┘
       │ 1. GET /api/identity → RSA 公钥
       │
       │ 2. POST http://<hub>:5678/invite
       │    { PublicKey, UserName, InviteCode }
       ▼
┌─────────────┐
│     Hub     │  验证邀请码，签发证书
└──────┬──────┘
       │ 3. 返回 Permit（CA 证书 + 用户证书 + bootstrap）
       ▼
┌─────────────┐
│ 你的 Agent  │  保存 Permit，加入 mesh
└──────┬──────┘
       │ 4. 与 Hub 进行 mTLS 握手
       ▼
   已连接
```

邀请码一次性使用。兑换后，其他人无法再用同一个码。

## 验证连接状态

在 GUI 中：

- 查看 **Mesh** 或 **Status** 面板，应显示 `connected: true`。
- 应能看到其他在线用户列表。
- 尝试向其他用户或群聊发送消息。

命令行验证（如果安装了 `ztm` CLI）：

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:<port>/api/meshes
```

查找名为 `clawparty` 的 mesh，`connected: true`。

## 多设备加入

每台设备需要自己的证书。如果想在第二台设备上加入：

1. 向管理员申请 **新邀请码**（可用相同或不同用户名）。
2. 在第二台设备上安装应用。
3. 用新邀请码重复加入流程。

不能把 Permit 文件从一台设备复制到另一台 — 每个 agent 生成自己的密钥对。

## 卸载后重新加入

如果卸载了应用并删除了数据目录，本地 Permit 已丢失。重新加入：

1. 向管理员申请新邀请码。
2. 重新安装应用。
3. 用新邀请码再次加入。

如果想复用相同用户名，管理员可能需要从 Hub 数据库中删除旧用户记录。

## 常见问题

**"邀请码无效或已使用。"**

- 仔细检查邀请码 — 区分大小写（统一大写）。
- 如果已兑换过一次，不能再用。申请新码。
- 如果被别人误用，让管理员生成新码。

**"用户名已存在。"**

其他人（或你在另一台设备上）已用该名字注册。换个名字，或让管理员删除旧记录。

**"无法连接到注册 URL。"**

- 检查 URL 是否正确且从你的机器可达。
- 注册端口是 HTTP（非 HTTPS）。确保没有被防火墙拦截。
- 在终端用 `curl http://<hub>:5678/` 验证端点是否可达。

**加入成功但状态一直是"disconnected"。**

- Permit 已签发，但 agent 无法访问 Hub 的 mTLS 端口（默认 `8888`）。
- 检查 Permit 中的 `bootstraps` 字段（存在数据目录中）。确保该地址可达。
- 让管理员确认 Hub 的 `--names` 参数是否指向公网可达地址。

**证书过期。**

证书默认有效期 365 天。过期后需要新 Permit。向管理员申请新邀请码并重新加入。

## 进阶：通过 CLI 加入

如果运行的是独立 `ztm` agent（非 GUI），可通过 CLI 加入：

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

这与 GUI 内部使用的流程相同。

## 相关文档

- [user-install.zh.md](user-install.zh.md) — 安装桌面客户端
- [admin-users.zh.md](admin-users.zh.md) — 管理员如何生成邀请码
