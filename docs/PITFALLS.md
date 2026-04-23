# ClawParty 开发常见问题与解决方案

本文档记录 ClawParty 项目开发过程中遇到的常见问题和解决方案，避免重复踩坑。

---

## ZeroClaw Agent 配置问题

### 问题：zeroclaw daemon 启动失败，gateway 组件报错 "Unknown provider"

**症状：**
- zeroclaw daemon 进程存在但端口未监听
- `daemon_state.json` 中 gateway 组件状态为 `error`
- 错误信息：`Unknown provider: <model_name>`
- WebSocket 连接失败，提示 "WebSocket not connected"

**根本原因：**
zeroclaw 的 `model_providers` 配置格式错误，使用了不兼容的字段。

**错误配置示例：**
```toml
default_provider = "aliyun"
default_model = "Kimi-K2.6"

[model_providers.aliyun]
name = "Kimi-K2.6"                    # ❌ 错误：name 应该是 provider 类型
npm = "@ai-sdk/openai-compatible"     # ❌ 错误：zeroclaw 不支持 npm 字段

[model_providers.aliyun.options]
baseURL = "https://..."               # ❌ 错误：字段名应为 base_url（snake_case）
apiKey = "..."                        # ❌ 错误：API key 不放在 provider 配置里
```

**正确配置：**
```toml
default_provider = "aliyun"
default_model = "Kimi-K2.6"          # 填写实际的模型 ID
api_key = "..."                       # ✅ 顶层 api_key 字段

[model_providers.aliyun]
name = "openai"                       # ✅ provider 类型：openai / anthropic 等
base_url = "https://..."              # ✅ snake_case，OpenAI 兼容端点
```

或者直接用 `custom:` 前缀，无需 `[model_providers]` 配置：
```toml
default_provider = "custom:https://your-api.example.com"
default_model = "Kimi-K2.6"
api_key = "..."
```

**诊断方法：**
```bash
cat ~/.clawparty/agents/<agent_name>/daemon_state.json
# 查看 gateway.last_error 字段
```

---

## ZTM Agent API 认证

### 问题：调用 ZTM API 返回 401 Unauthorized

**症状：**
```bash
curl http://127.0.0.1:6789/api/meshes
# {"status":401,"message":"unauthorized"}
```

**解决方案：**
ZTM agent 默认 token 是 `enjoy-party`，通过 `Authorization: Bearer` 传递：
```bash
curl -H "Authorization: Bearer enjoy-party" http://127.0.0.1:6789/api/meshes
```

测试环境（`tests/acl-local/`）的 token 是 `acl-local`，两者不同，不要混用。

---

## zAgent 管理 API 路径

### 问题：找不到 zAgent 管理 API 端点

**正确的 API 路径：**
```
GET    /api/agents                          # 列出所有 zAgent
POST   /api/agents                          # 创建 zAgent
DELETE /api/agents/:name                    # 删除 zAgent
POST   /api/agents/:name/start              # 启动 zAgent daemon
POST   /api/agents/:name/stop               # 停止 zAgent daemon
```

注意：路径是 `/api/agents`，不是 `/api/zagents`。

---

## GUI 群聊对话框无法打开

### 问题：点击群聊没有反应，对话框不显示

**根本原因：**
`App.vue` 的 `selectChat()` 函数只更新了 `activeChat.value`，但没有更新 `currentActiveChatId.value`，而 `ChatMain` 组件用 `v-show="currentActiveChatId === item.id"` 控制显示。

**修复：**
```javascript
const selectChat = (index) => {
  activeOpenclawAgent.value = null
  activeChat.value = index
  if (chats.value[index]) {
    chats.value[index].updated = 0
    currentActiveChatId.value = chats.value[index].id  // 必须同步更新
  }
}
```

同时，`ChatSidebar.vue` 的 `getChatIndex()` 不能过滤 `isOpenclaw`，否则群聊返回 -1：
```javascript
const getChatIndex = (chatId) => {
  return props.chats.findIndex(c => c.id === chatId)  // 不加 isOpenclaw 过滤
}
```

---

## Vite 代理配置

### 问题：vite.config.js 代理地址硬编码

代理目标不能硬编码端口，应通过环境变量控制：
```javascript
// vite.config.js
proxy: {
  '/api': {
    target: process.env.VITE_API_TARGET || 'http://localhost:6789',
    changeOrigin: true,
  }
}
```

启动时通过环境变量切换目标：
```bash
VITE_API_TARGET=http://127.0.0.1:7781 npm run dev  # 指向 alice
npm run dev                                          # 默认 localhost:6789
```

---

## 测试环境 zeroclaw 配置初始化

### 问题：`tests/acl-local/setup.sh` 创建的 agent 缺少 zeroclaw 配置

**症状：**
```
Agent creation failed: cannot open file: .../tmp/alice/.zeroclaw/config.toml
```

**原因：** `setup.sh` 在 `start_agent` 时需要先初始化 zeroclaw 配置目录。

**修复：** `setup.sh` 中的 `start_agent()` 调用 `init_zeroclaw_config()` 初始化配置，优先从 `~/.zeroclaw/config.toml` 复制，否则写入最小化默认配置。
