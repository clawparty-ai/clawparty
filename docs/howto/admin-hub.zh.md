[English](admin-hub.md) | [中文](admin-hub.zh.md)

# 部署与管理 Hub

本文面向 ClawParty Hub 管理员，介绍 Hub 的职责、启动方式以及生命周期管理（停止、升级、备份、恢复）。

## Hub 的职责

ClawParty Hub 是一个轻量级注册中心，**不**转发对等流量。它的职责是：

- 充当证书颁发机构（CA），签发授权进入 mesh 的用户证书
- 维护在线 endpoint 目录，让对等节点彼此发现
- 提供一个 HTTP 注册端点，供新用户兑换邀请码

两个 endpoint 通过 Hub 完成发现后，所有后续流量都通过 ZTM 在 P2P 加密通道上传输。

## 前置条件

- 一个 `ztm` 可执行文件。下载 release 或源码构建，参见 [build.md](build.zh.md)。
- 一个可达的 Hub 地址。生产环境建议稳定 DNS 名；本地测试 `127.0.0.1` 即可。
- 两个空闲 TCP 端口：
  - **mTLS 端口**（默认 `8888`）— 生产流量
  - **注册端口**（默认 `5678`）— HTTP 明文，用于兑换邀请码
- 一个持久化数据目录，用于存放 CA 密钥和 SQLite 数据库。

> 注册端口是 HTTP 明文，仅靠邀请码鉴权。请视为敏感入口：绑定到可信网卡或用防火墙限制。

## 启动 Hub

最简命令：

```bash
ztm run hub \
  --listen "0.0.0.0:8888" \
  --data "/var/lib/clawparty/hub" \
  --names "hub.example.com:8888" \
  --enable-registration "0.0.0.0:5678" \
  --permit "/var/lib/clawparty/root.json"
```

### 参数说明

| 参数 | 用途 | 默认值 |
|------|------|--------|
| `--listen` | mTLS 监听地址，承接客户端流量 | `0.0.0.0:8888` |
| `--data` | Hub 数据目录（CA 密钥、SQLite 数据库） | `~/.ztm` |
| `--names` | 写入 Permit 中的 bootstrap 地址，客户端用它回连 | 必填 |
| `--enable-registration` | HTTP 注册端点监听地址。不指定则注册接口关闭 | 不启用 |
| `--permit` | root 用户 Permit 的输出路径，仅首次启动时生成 | 无 |
| `--zeroclaw-config` | 包含新用户默认 LLM 配置的 JSON 文件路径。指定后，新用户加入时会自动收到此配置并创建 0#Agent | 无 |

要点：

- 启用邀请码注册必须指定 `--enable-registration`，否则注册接口完全关闭，Hub 只接受已注册的用户。
- `--permit` 只在 **首次启动**（CA 同时创建）时写入文件，后续启动不覆盖。务必妥善保存：管理员靠它进入 mesh。
- `--names` 是客户端首次握手后用于回连的地址。要确保从客户端侧可达，而不是只在 Hub 主机内部可达。
- `--zeroclaw-config` 可选但生产环境推荐。它为新用户提供开箱即用的 LLM 配置。不指定的话，用户加入后需自行配置 LLM 提供商。文件格式见下文 LLM 配置章节。

### 示例：生产环境

```bash
ztm run hub \
  --listen "0.0.0.0:8888" \
  --data "/var/lib/clawparty/hub" \
  --names "hub.clawparty.com:8888" \
  --enable-registration "0.0.0.0:5678" \
  --permit "/var/lib/clawparty/root.json" \
  --zeroclaw-config "/var/lib/clawparty/llm-config.json"
```

### 示例：本地测试

```bash
ztm run hub \
  --listen "127.0.0.1:18888" \
  --data "./tmp/hub" \
  --names "127.0.0.1:18888" \
  --enable-registration "127.0.0.1:15678" \
  --permit "./tmp/root.json" \
  --zeroclaw-config "./tmp/llm-config.json"
```

## 生命周期

### 前台与后台

临时调试可前台运行直接看日志。长期部署应重定向日志并放到后台：

```bash
nohup ztm run hub ... > /var/log/clawparty/hub.log 2>&1 &
```

### 用 systemd 托管（Linux）

`/etc/systemd/system/clawparty-hub.service`：

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

### 停止

发送 `SIGTERM`（前台 Ctrl-C、`systemctl stop clawparty-hub` 或 `kill <pid>`）。Hub 会处理完正在进行的请求后干净退出。客户端之间已经建立的 P2P 数据流不受影响 — Hub 本就不在数据路径上。

### 升级

1. 备份数据目录（见下文）
2. 停止 Hub
3. 替换 `ztm` 二进制
4. 用相同参数重新启动

SQLite schema 在 patch / minor 版本之间向前兼容。major 升级前请查阅 release notes。

### 备份

备份范围 — `--data` 全部内容加上 Permit 文件：

| 路径 | 说明 |
|------|------|
| `<data>/ca-cert.pem` | CA 证书（公钥） |
| `<data>/ca-key.pem` | CA 私钥 — 一旦丢失，再也无法签发新证书或验证现有用户 |
| `<data>/ztm-hub.db` | SQLite 数据库：users、invite_codes、evictions、审计日志 |
| `<permit>` | root 管理员 Permit |

冷备份（Hub 停止状态下复制）最简单。需要热备时用 `sqlite3 ztm-hub.db ".backup /path/to/snapshot.db"`。

### 恢复

1. 如果 Hub 在运行，先停止
2. 把 `--data` 和 Permit 从备份恢复回原位，注意保持 `ca-key.pem` 的文件权限（仅 Hub 运行用户可读，其他人无权限）
3. 启动 Hub。只要 CA 没变，已签发的用户证书继续有效

## LLM 配置（可选）

如果希望新用户加入后立即拥有可用的 LLM 设置，可创建一个包含默认 LLM 配置的 JSON 文件，通过 `--zeroclaw-config` 传入。

示例 `llm-config.json`：

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

支持的 provider：`openai`、`anthropic`、`qwen`、`moonshot`、`doubao`、`deepseek`、`ollama`、`custom`。

启用此配置后，用户加入时：

1. Hub 在 Permit 响应中包含 `default_llm_config`。
2. 用户的 agent 将其保存为全局配置（`~/.clawparty/global-config.toml`）。
3. 自动创建一个使用此配置的 0#Agent。
4. 用户后续创建的 agent 会继承此配置，除非手动覆盖。

详细字段说明和各 provider 示例，参见归档参考文档：[archive/reference/hub-llm-config.md](../archive/reference/hub-llm-config.md)。

## 文件布局

| 路径 | 内容 |
|------|------|
| `<data>/ca-cert.pem` | Hub CA 证书 |
| `<data>/ca-key.pem` | Hub CA 私钥 |
| `<data>/ztm-hub.db` | SQLite 数据库 |
| `<permit>` | root 管理员 Permit（JSON） |
| `<zeroclaw-config>` | 可选 LLM 配置（JSON） |

## 安全建议

- **保护 `ca-key.pem`。** 拿到这个文件的人就能为你的 mesh 任意签发身份。设置 `chmod 600`，只放在 Hub 主机上。
- **限制注册端口暴露面。** HTTP 明文，仅靠邀请码鉴权。如果策略是仅限邀请注册，请绑定到内网网卡或加防火墙。
- **定期审计。** 定期检查 `users`、`user_log`、`api_log` 三张表 — 所有签发与连接事件都在其中。
- **谨慎管理邀请码。** 邀请码一次性使用。如果在兑换前泄露，生成一个新码，原码视为不可用。
- **保护 LLM API 密钥。** 如果使用 `--zeroclaw-config`，API 密钥会传递给每个新用户。使用专用密钥并设置消费限额，定期轮换。配置文件设置 `chmod 600`。

## 常见问题

**端口已被占用。** 另一个 `ztm` 进程在跑，或别的服务占用了 `8888` / `5678`。`lsof -iTCP:8888 -sTCP:LISTEN` 排查，停掉冲突进程或换端口。

**Permit 文件没生成。** `--permit` 只在首次启动写入。如果文件没有但 `<data>/ca-cert.pem` 已存在，说明 CA 早已创建。除非你能接受丢失所有用户，否则不要清空数据目录。

**客户端首次连上后无法再连。** 检查 `--names`。如果它指向客户端无法解析或访问的地址，Permit 里携带的 bootstrap 就是死的。

**Hub 启动了但注册请求被拒。** 没指定 `--enable-registration`，或注册端口被防火墙挡了。在客户端侧用 `curl http://<hub>:5678/` 验证。

## 相关文档

- [admin-users.zh.md](admin-users.zh.md) — 邀请用户与用户生命周期管理
- [build.zh.md](build.zh.md) — 源码构建 `ztm`
