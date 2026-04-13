# ZeroClaw Web UI Integration

## 概述

本文档描述了如何将 ZeroClaw 集成到 ClawParty 的 Web UI (chat-gui)中，使用户可以通过 Web 界面与 ZeroClaw sessions 进行交互。

## 修改的文件

### 1. `chat-gui/src/services/chatService.js`

添加了 `zeroclawService` 对象，提供以下 API 方法：

```javascript
export const zeroclawService = {
  checkHealth() {
    return api.get('http://localhost:42617/api/health')
  },
  
  getSessions() {
    return api.get('http://localhost:42617/api/ztm/sessions')
  },
  
  sendMessage(sessionId, message) {
    return api.post(`http://localhost:42617/api/sessions/${sessionId}/chat`, { 
      message 
    })
  },
  
  getMessages(sessionId) {
    return api.get(`http://localhost:42617/api/sessions/${sessionId}/messages`)
  }
}
```

### 2. `chat-gui/src/App.vue`

**添加的状态变量**:
- `zeroclawSessions`: 存储 ZeroClaw sessions 列表
- `activeZeroClawSession`: 当前选中的 ZeroClaw session

**添加的函数**:
- `loadZeroClawSessions()`: 从 ZeroClaw Gateway 加载 sessions
- `selectZeroClawSession(session)`: 选择一个 session 进行聊天
- `loadZeroClawChatHistory(session)`: 加载 session 的聊天历史
- 修改 `sendMessage()`: 添加 ZeroClaw 消息发送逻辑

**提供的上下文**:
```javascript
provide('zeroclawSessions', zeroclawSessions)
provide('activeZeroClawSession', activeZeroClawSession)
provide('selectZeroClawSession', selectZeroClawSession)
```

### 3. `chat-gui/src/components/ChatSidebar.vue`

**添加的注入**:
```javascript
const zeroclawSessions = inject('zeroclawSessions')
const activeZeroClawSession = inject('activeZeroClawSession')
const selectZeroClawSession = inject('selectZeroClawSession')
```

**添加的模板部分**:
在 "My Agents" 部分之前添加 ZeroClaw Sessions 显示：

```vue
<!-- ZeroClaw Sessions -->
<template v-if="zeroclawSessions && zeroclawSessions.length > 0">
  <div
    v-for="session in zeroclawSessions"
    :key="session.session_id"
    class="panel-item agent-item"
    :class="{ active: activeZeroClawSession?.session_id === session.session_id }"
    @click="selectZeroClawSession(session)"
  >
    <div class="item-avatar zeroclaw-avatar">🦀</div>
    <span class="item-name">{{ session.name || session.user_id }}</span>
  </div>
</template>
```

**添加的 CSS 样式**:
```css
.zeroclaw-avatar {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.zeroclaw-section-header {
  font-size: 12px;
  font-weight: 600;
  color: #667eea;
  padding: 8px 12px;
  margin-top: 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
```

## 数据流

### 1. 启动流程

```
App 启动
  ↓
loadZeroClawSessions()
  ↓
调用 zeroclawService.getSessions()
  ↓
GET http://localhost:42617/api/ztm/sessions
  ↓
更新 zeroclawSessions 状态
  ↓
ChatSidebar 显示 sessions 列表
```

### 2. 发送消息流程

```
用户输入消息
  ↓
点击发送
  ↓
sendMessage() 检测 activeZeroClawSession
  ↓
调用 zeroclawService.sendMessage(session_id, text)
  ↓
POST http://localhost:42617/api/sessions/{id}/chat
  ↓
ZeroClaw Gateway 处理消息
  ↓
返回响应
  ↓
更新 session.messages
  ↓
ChatMain 显示消息
```

### 3. 接收消息流程

```
ZeroClaw Gateway
  ↓
ZTM Channel 轮询 ClawParty API
  ↓
获取新消息
  ↓
创建 ChannelMessage
  ↓
ZeroClaw Agent 处理
  ↓
保存到 session 历史
  ↓
Web UI 定期轮询 (可选)
  ↓
更新消息显示
```

## UI 组件关系

```
App.vue (主应用)
  ├── provides: zeroclawSessions, activeZeroClawSession, selectZeroClawSession
  │
  ├── ChatSidebar.vue
  │     ├── injects: zeroclawSessions, activeZeroClawSession, selectZeroClawSession
  │     └── 显示 ZeroClaw sessions 列表
  │
  └── ChatMain.vue
        ├── receives: chat (可以是 ZeroClaw session)
        └── 显示聊天消息
```

## ZeroClaw Gateway API 端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/health` | GET | 健康检查 |
| `/api/ztm/sessions` | GET | 获取所有 ZTM sessions |
| `/api/sessions/{id}/chat` | POST | 发送消息到 session |
| `/api/sessions/{id}/messages` | GET | 获取 session 消息历史 |

## 配置要求

### ZeroClaw 配置

确保 `~/.clawparty/.zeroclaw/config.toml` 包含：

```toml
[gateway]
port = 42617
host = "127.0.0.1"

[channels.ztm]
enabled = true
api_url = "http://127.0.0.1:6789"
api_token = "enjoy-party"
mesh_name = "clawparty"
poll_interval_secs = 1
```

### Web UI 配置

Web UI 需要能够访问 ZeroClaw Gateway (默认端口 42617)。

如果是 Tauri 应用，需要在 `tauri.conf.json` 中添加 CORS 配置：

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self' http://localhost:42617"
    }
  }
}
```

## 测试步骤

### 1. 编译 ZeroClaw

```bash
cd ~/github/clawparty/zeroclaw
cargo build --release --features gateway
```

### 2. 启动 ZeroClaw Daemon

```bash
./target/release/zeroclaw daemon --port 42617
```

### 3. 编译 Web UI

```bash
cd ~/github/clawparty/chat-gui
npm install
npm run build
```

### 4. 启动 ClawParty

```bash
cd ~/github/clawparty
./bin/clawparty
```

### 5. 验证功能

1. 检查左侧边栏是否显示 "🦀" 图标的 ZeroClaw sessions
2. 点击 session 打开聊天窗口
3. 发送消息并等待回复
4. 检查消息历史是否正确加载

## 故障排除

### 问题：ZeroClaw sessions 不显示

**可能原因**:
- ZeroClaw daemon 未启动
- Gateway API 不可访问
- 没有活跃的 sessions

**解决方案**:
```bash
# 检查 ZeroClaw 进程
ps aux | grep zeroclaw

# 检查 Gateway 健康状态
curl http://localhost:42617/api/health

# 检查 sessions API
curl http://localhost:42617/api/ztm/sessions
```

### 问题：消息发送失败

**可能原因**:
- ZTM Channel 未正确配置
- ClawParty Agent API 不可访问
- 认证失败

**解决方案**:
1. 检查 ZeroClaw 日志
2. 验证 `config.toml` 中的 API URL 和 token
3. 检查 ClawParty agent 是否运行

### 问题：CORS 错误

**可能原因**:
- Web UI 无法访问 ZeroClaw Gateway

**解决方案**:
1. 在 Tauri 配置中添加 CORS 允许
2. 或者在 ZeroClaw Gateway 中添加 CORS 头

## 未来改进

1. **实时消息推送**: 使用 WebSocket 替代轮询
2. **Session 管理**: 添加创建/删除 session 功能
3. **用户配置**: 允许用户配置 ZeroClaw provider 和模型
4. **消息状态**: 显示消息发送状态（发送中/已发送/失败）
5. **文件上传**: 支持通过 ZeroClaw 发送文件
6. **多用户支持**: 改进会话隔离和用户识别

## 架构优势

- **单一 Channel**: 使用单个 ZTM Channel 处理所有用户，通过 sender 区分
- **Session 隔离**: 每个用户有独立的 session 和消息历史
- **简单集成**: 通过标准 HTTP API 与 Web UI 通信
- **向后兼容**: 不影响现有的 OpenClaw 和 ZTM 功能

## 总结

ZeroClaw Web UI 集成提供了：
- ✅ 在 ClawParty Web UI 中显示 ZeroClaw sessions
- ✅ 发送和接收 ZeroClaw 消息
- ✅ 加载和显示聊天历史
- ✅ 与现有 OpenClaw/ZTM 功能无缝集成
- ✅ 简洁的用户界面和用户体验

用户现在可以通过 ClawParty Web UI 与 ZeroClaw AI agents 进行自然语言交互！🦀
