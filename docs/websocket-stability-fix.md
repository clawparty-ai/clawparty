# ClawParty WebSocket 稳定性修复包

## 文件说明

| 文件 | 用途 |
|---|---|
| `src/web/src/services/zagent-ws.js` | 修复版 WebSocket 客户端，替换 `src/web/src/services/chatService.js` 中的 `zAgentWS` 类 |
| `src/web/src/components/WsStatusBanner.vue` | 断线状态条组件，放在聊天页顶部 |
| `deploy/nginx-ws.conf` | nginx WebSocket 代理关键配置（合并到服务器的 server 块） |

## 集成步骤

### 1. 替换客户端类

现有代码中创建连接的地方（bundle 中大致是 `new ia(T, "me", Ue, Tn, Ft, nn, Ge)` 这一段，
源码里应该是 `new ZAgentWS(...)`）替换为：

```js
import { ZAgentWS } from './zagent-ws.js';

const ws = new ZAgentWS(agentName, 'me', {
  onMessage: (data) => handleMessage(data),
  onOpen: () => console.log('connected'),
  onStateChange: (state, info) => { /* 交给 WsStatusBanner，或自行处理 */ },
  onReconnected: () => refetchMissedMessages(),   // 关键：补拉断线期间的消息
});
ws.connect();

// 切换频道/登出时：
ws.destroy();
```

**删掉旧的"最大重连 3/5 次"调度逻辑**（源码中 `maxReconnectAttempts`、`Ye=5`、
`Ze`/`Le` 那一组重试函数），重连完全由新类内部接管。

### 2. 断线期间拦截发送

`ZAgentWS.send()` 现在返回布尔值。发送按钮逻辑改为：

```js
function onSend() {
  const ok = ws.send({ type: 'chat', content: inputValue });
  if (!ok) {
    showToast('当前未连接，消息将在恢复后发送，请稍候');
    return;   // 不要把消息 push 进消息列表
  }
  messages.push(localEcho(inputValue));
}
```

### 3. 重连后增量补拉消息

给消息列表记录最后一条消息的 `id` 或 `timestamp`：

```js
async function refetchMissedMessages() {
  const last = messages.value[messages.value.length - 1];
  const res = await fetch(`/api/chat/history?agent=${agentName}&after=${last?.id ?? 0}`, {
    headers: { Authorization: `Bearer ${localStorage.getItem('clawparty_login_token')}` },
  });
  const missed = await res.json();
  messages.value.push(...missed);
}
```

> 需要服务端 `/api/chat/history` 支持 `after` 参数。如果暂不支持，退化为
> 重新拉取最近 N 条并按 id 去重即可。

### 4. 服务端配合（Go/Node 通用要点）

- **响应 close 帧**：收到客户端 close 必须回 close 帧再关 TCP
  （当前服务端不回，浏览器卡 CLOSING 20s+）
- **识别 ping 消息**：收到 `{"type":"ping"}` 回 `{"type":"pong"}`；
  也可以同时开启协议层 ping（如 Go gorilla/websocket 的 `SetPingHandler`，每 30s）
- **广播前检查连接活性**：向死连接写消息应及时移除订阅，避免消息丢失且无报错

### 5. nginx 配置

把 `nginx-ws.conf` 的关键行合并进现有 server 块，重点是：

- `proxy_read_timeout 300s`（默认 60s 会杀空闲连接，配合心跳后只是兜底）
- 完整的 Upgrade/Connection 头（你目前的能用，保留即可）

改完 `nginx -t && nginx -s reload`。**注意：reload 会断开所有 WS，
这正是为什么要客户端无限重连——两者配合后用户完全无感。**

## 验证清单

1. 登录后 DevTools → Network → WS，确认 `wss://.../ws/chat` 建立
2. 杀掉后端进程 → 页面 3s 内出现黄色"正在重连"横幅 → 拉起后端 → 自动恢复 + 绿色"已恢复"提示
3. `nginx -s reload` → 同上，自动恢复
4. 断网 2 分钟再恢复（模拟半开连接）→ 心跳超时（约 50~75s）后自动重连
5. 断线期间点发送 → 有明确提示，消息不静默进列表
