# TUI HTTPS 反向代理设计文档

## 1. 总体架构

```
┌─────────────────────────────────────────────┐
│   Web Browser (HTTPS)                       │
│   https://clawparty.local/                  │
└──────────────┬──────────────────────────────┘
               │ HTTPS 443  (终止 TLS)
               ▼
┌─────────────────────────────────────────────┐
│   TUI Proxy (Rust + hyper + tokio-rustls)   │
│   监听 0.0.0.0:443 + 80                     │
│   ┌─────────────────────────────┐           │
│   │ 自签名证书 (RSA 2048)        │           │
│   │ ~/.clawparty/certs/          │           │
│   └─────────────────────────────┘           │
│   路由 → 反向代理到本地服务                   │
└──────────────┬──────────────────────────────┘
               │
    ┌──────────┼──────────┬──────────────────┐
    ▼          ▼          ▼                  ▼
  http://    ws://      ws://             ws://
127.0.0.1:6789  127.0.0.1:6789  127.0.0.1:{port}  ...
 (Web UI/    (ZTM WS)   (zAgent WS)
  API)
```

## 2. 背景与动机

当前 ClawParty Web UI 在浏览器中运行时存在以下限制：
- 所有 WebSocket 直接连接 `ws://localhost:{port}`，在 HTTPS 页面下触发浏览器 **Mixed Content** 安全策略被拦截
- 前端硬编码多个 `localhost` 地址，无法通过单一域名访问
- 多 Agent 动态端口无法统一管理

本方案在 TUI service 模式下增加一个 HTTPS 反向代理，统一暴露 443/80 端口，代理所有本地服务。

## 3. 路由表（TUI Proxy）

| 路径 | 代理目标 | 说明 |
|------|---------|------|
| `/*` | `http://127.0.0.1:6789` | Web UI 静态文件 (index.html, assets/...) |
| `/api/*` | `http://127.0.0.1:6789` | ZTM Agent REST API |
| `/ws/chat` | `ws://127.0.0.1:6789` | ZTM Agent WebSocket（ZTM Agent 内部会根据 `?agent=` 参数代理到具体 zAgent） |
| `/api/zeroclaw/*` | `http://127.0.0.1:42617` | ZeroClaw Gateway API（解决前端直连 42617 的 Mixed Content） |

> **说明**：所有 zAgent 的 WebSocket 连接统一走 `wss://host/ws/chat?agent={name}&session_id={id}`，由 ZTM Agent 在本地根据 `agent` 参数路由到对应端口（`agent/main.js` 第 2400-2421 行已有此逻辑）。TUI Proxy 不需要单独为每个 zAgent 建路由。

## 4. 证书方案

- **自动生成**：TUI 启动时检查 `~/.clawparty/certs/cert.pem` + `key.pem`
- 不存在则用 `rcgen` 生成 RSA 2048 自签名证书，CN=`clawparty.local`
- 持久化到磁盘，后续复用（避免每次启动浏览器告警）
- **依赖**：`rcgen`, `tokio-rustls`, `rustls-pemfile`

## 5. HTTP 80 重定向

- 单独启动一个 `hyper` 服务监听 `0.0.0.0:80`
- 所有请求返回 `301 Moved Permanently` -> `https://{host}{path}`
- 绑定失败（权限不足）仅打印警告，不阻断 service 模式

## 6. 后端实现

### 6.1 依赖

在 `tui/Cargo.toml` 中新增：

```toml
# HTTP 代理
hyper = { version = "1", features = ["full"] }
hyper-util = { version = "0.1", features = ["full"] }
http-body-util = "0.1"

# TLS
tokio-rustls = "0.26"
rustls-pemfile = "2"

# 证书生成
rcgen = "0.13"

# WebSocket 代理 (upgrade 透传)
tokio-tungstenite = "0.24"
```

### 6.2 CLI 参数

在 `tui/src/args.rs` 中新增：

```rust
#[arg(long, default_value = "443")]
pub proxy_https_port: u16,

#[arg(long, default_value = "80")]
pub proxy_http_port: u16,

#[arg(long, default_value = "~/.clawparty/certs")]
pub proxy_cert_dir: String,
```

### 6.3 核心模块 `proxy.rs`

新建 `tui/src/proxy.rs`，职责：

- `ensure_cert()`：检查/生成自签名证书
- `run_http_redirect()`：启动 80 端口 HTTP 重定向服务
- `run_https_proxy()`：启动 443 端口 HTTPS 反向代理服务
- `handle_request()`：路由分发逻辑
- `proxy_websocket()`：WS upgrade 透传

核心路由逻辑：

```rust
async fn handle_request(req: Request) -> Response {
    let path = req.uri().path();
    
    let target = if path.starts_with("/ws/chat") {
        "ws://127.0.0.1:6789/ws/chat"
    } else if path.starts_with("/api/zeroclaw/") {
        "http://127.0.0.1:42617"
    } else {
        "http://127.0.0.1:6789"
    };
    
    forward(req, target).await
}
```

### 6.4 启动点

在 `tui/src/main.rs` 的 `run_service_mode()` 末尾启动 proxy：

```rust
let proxy_handle = tokio::spawn(async move {
    proxy::start(args.proxy_https_port, args.proxy_http_port, &args.proxy_cert_dir).await
});
```

## 7. 前端修改

### 7.1 `chat-gui/src/services/chatService.js`

**改动 1：`ZeroClawWS.connect()`**

当前逻辑：
```javascript
const url = this.wsPort
  ? `${protocol}//localhost:${this.wsPort}/ws/chat?agent=...`
  : `${protocol}//${host}/ws/chat?agent=...`
```

改为：
```javascript
const isTauri = !!window.__TAURI_INTERNALS__
const shouldUseDirectPort = isTauri && this.wsPort
const url = shouldUseDirectPort
  ? `${protocol}//localhost:${this.wsPort}/ws/chat?agent=...`
  : `${protocol}//${host}/ws/chat?agent=...`
```

**原因**：HTTPS 页面中 `ws://localhost` 会被浏览器 Mixed Content 拦截。Web 浏览器模式下统一走当前 host，由后端代理到对应 zAgent。

**改动 2：`zeroclawService.checkHealth()`**

当前：
```javascript
return api.get('http://localhost:42617/api/health')
```

改为：
```javascript
return api.get('/api/zeroclaw/health')
```

**原因**：避免 HTTPS 下直接访问 `http://localhost:42617` 触发 Mixed Content。ZTM Agent（`agent/main.js`）已有 `/api/zeroclaw/*` 代理到 `localhost:42617` 的路由。

### 7.2 `chat-gui/src/services/wsService.js`

当前：
```javascript
const WS_URL = 'ws://127.0.0.1:18789/'
```

改为：
```javascript
const WS_URL = window.location.protocol === 'https:'
  ? `wss://${window.location.host}/ws/`
  : `ws://${window.location.host}/ws/`
```

> 该文件当前未被使用（被注释掉的旧代码），保险起见一并修改。

### 7.3 `chat-gui/src/App.vue`

**无需修改**。所有 `ZeroClawWS` 调用点共用 `ZeroClawWS` 类，改动 `connect()` 后全局生效。

## 8. 实施步骤

1. `git checkout -b web-remote`
2. 修改 `tui/Cargo.toml` 添加依赖
3. 修改 `tui/src/args.rs` 添加参数
4. 新建 `tui/src/proxy.rs`（核心代理逻辑）
5. 修改 `tui/src/main.rs` 在 `run_service_mode` 中启动 proxy
6. 修改 `chat-gui/src/services/chatService.js`（WS 协议 + checkHealth）
7. 修改 `chat-gui/src/services/wsService.js`（保险）
8. `cd tui && cargo build` 验证编译
9. `cd chat-gui && npm run build` 验证前端编译

## 9. 潜在风险与应对

| 风险 | 应对方案 |
|------|---------|
| 80/443 端口权限 | 需要 root 或 `setcap`，建议文档说明；绑定失败优雅降级（仅打印警告） |
| 自签名证书浏览器告警 | 首次访问需要用户手动信任证书（本地/内网场景可接受） |
| 前端 `getPort()` 逻辑 | HTTPS 模式下所有请求走同域 443，`getPort()` 不影响 |
| Tauri 桌面模式受影响 | `window.__TAURI_INTERNALS__` 判断确保桌面版仍直连 localhost |

## 10. 验证方式

```bash
# 1. 编译 TUI
cd tui && cargo build --release

# 2. 编译前端
cd chat-gui && npm run build

# 3. 启动 service 模式（需 root 绑定 80/443）
sudo ./target/release/clawparty --service

# 4. 浏览器访问
# https://localhost/  -> 应显示 ClawParty Web UI
# https://localhost/api/meshes -> 应返回 meshes JSON
# https://localhost/ws/chat?agent=0%23Agent&session_id=me -> 应建立 WebSocket
```
