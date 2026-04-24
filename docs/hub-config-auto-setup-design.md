# Hub 配置自动获取与 0#Agent 自动创建技术方案

## 1. 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                         用户旅程                                  │
├─────────────────────────────────────────────────────────────────┤
│ 1. 启动软件                                                       │
│ 2. Join Party (输入 hub URL)                                     │
│ 3. ✨ 自动获取 hub 默认 LLM 配置                                  │
│ 4. ✨ 自动创建 0#Agent (使用 hub 配置)                            │
│ 5. ✨ 将 hub 配置设为全局默认                                     │
│ 6. 用户创建更多 agent (自动继承全局配置)                          │
│ 7. 开始聊天                                                       │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      系统架构图                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────┐  HTTP GET   ┌──────────┐  Permit +  ┌──────────┐ │
│  │          │ ───────────> │          │  Config    │          │ │
│  │  Client  │              │   Hub    │ ─────────> │  Client  │ │
│  │          │ <─────────── │          │            │          │ │
│  └──────────┘  Permit +    └──────────┘            └──────────┘ │
│                Config                                             │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Client 端处理流程                                          │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │ 1. 解析 permit 中的 default_llm_config                    │   │
│  │ 2. 保存到 ~/.clawparty/global-config.toml                │   │
│  │ 3. 调用 createAgent("0#Agent", config)                   │   │
│  │ 4. 自动启动 0#Agent                                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## 2. Hub 配置文件设计

### 2.1 Hub 启动配置文件

**文件路径**: `~/.clawparty-hub/hub-config.toml`

```toml
[hub]
name = "ClawParty Hub"
listen = "0.0.0.0:7779"

[default_llm_config]
provider = "custom"
api_endpoint = "http://1873330231187220.cn-hangzhou.pai-eas.aliyuncs.com/api/predict/quickstart_deploy_20260422_wqkx/v1"
api_key = "NTA5OWQ1ZjVjZjUzYTg3OTk5ZDAwMTk2OWY0NDU0NzliNDliN2JhYQ=="
model = "Kimi-K2.6"
temperature = 0.7
timeout_secs = 120

# 可选：为不同 provider 提供预设
[llm_presets.openai]
provider = "openai"
model = "gpt-4o-mini"
# api_key 留空，由用户自行配置

[llm_presets.anthropic]
provider = "anthropic"
model = "claude-3-5-haiku-20241022"
```

### 2.2 Hub 启动参数支持

```bash
# 方式 1: 使用配置文件
ztm-hub --config hub-config.toml

# 方式 2: 命令行参数覆盖
ztm-hub --config hub-config.toml \
  --default-llm-provider custom \
  --default-llm-endpoint "http://..." \
  --default-llm-key "xxx" \
  --default-llm-model "Kimi-K2.6"
```

## 3. Hub API 设计

### 3.1 Permit 响应扩展

**现有 `/invite` 端点返回格式扩展**:

```json
{
  "UserName": "jose-arcadio-buendia",
  "EpName": "jose-arcadio-buendia-lobster",
  "Permit": {
    "ca": "-----BEGIN CERTIFICATE-----...",
    "agent": {
      "certificate": "-----BEGIN CERTIFICATE-----...",
      "privateKey": "-----BEGIN PRIVATE KEY-----..."
    },
    "bootstraps": ["hub.example.com:7777"],
    "default_llm_config": {
      "provider": "custom",
      "api_endpoint": "http://...",
      "api_key": "xxx",
      "model": "Kimi-K2.6",
      "temperature": 0.7,
      "timeout_secs": 120
    }
  }
}
```

**关键点**:
- `default_llm_config` 字段嵌入到 permit 中
- Hub 在生成 permit 时从配置文件读取并注入
- 如果 Hub 未配置，该字段为 `null` 或不存在

### 3.2 Hub 端实现 (伪代码)

```javascript
// hub/main.js
function handleInvite(req) {
  // ... 现有逻辑 ...
  
  // 读取 hub 配置
  var hubConfig = loadHubConfig()
  var defaultLLMConfig = hubConfig?.default_llm_config || null
  
  var permit = {
    ca: ca,
    agent: {
      certificate: cert,
      privateKey: key
    },
    bootstraps: bootstraps,
    default_llm_config: defaultLLMConfig  // ✨ 新增字段
  }
  
  return {
    UserName: userName,
    EpName: epName,
    Permit: JSON.stringify(permit)
  }
}
```

## 4. ClawParty Agent 改造

### 4.1 全局配置文件

**文件路径**: `~/.clawparty/global-config.toml`

```toml
# 全局默认 LLM 配置
[llm]
provider = "custom"
api_endpoint = "http://..."
api_key = "xxx"
model = "Kimi-K2.6"
temperature = 0.7
timeout_secs = 120

[metadata]
source = "hub"  # 来源: hub | user
hub_url = "https://join.clawparty.ai"
updated_at = 1745678901
```

### 4.2 Join Party 流程改造

**文件**: `agent/main.js` 中的 `/api/join-party` 端点

```javascript
'/api/join-party': {
  'POST': function (_, req) {
    // ... 现有逻辑 ...
    
    return regAgent.request('POST', urlBase + '/invite', headers, inviteBody).then(
      function (res) {
        // ... 现有解析逻辑 ...
        
        var parsedPermit = JSON.parse(permitData)
        
        // ✨ 新增：提取并保存全局配置
        var defaultLLMConfig = parsedPermit.default_llm_config
        if (defaultLLMConfig) {
          console.info('[join-party] Received default LLM config from hub')
          saveGlobalConfig(defaultLLMConfig, regUrl)
        }
        
        // ✨ 新增：自动创建 0#Agent
        if (defaultLLMConfig) {
          console.info('[join-party] Creating 0#Agent with hub config')
          try {
            api.createAgent('0#Agent', '0#Agent', defaultLLMConfig, 'System agent created from hub config')
            console.info('[join-party] 0#Agent created successfully')
            
            // 自动启动 0#Agent
            api.startAgent('0#Agent')
            console.info('[join-party] 0#Agent started')
          } catch (e) {
            console.error('[join-party] Failed to create 0#Agent:', e)
            // 不阻塞 join party 流程
          }
        }
        
        // ... 现有返回逻辑 ...
      }
    )
  }
}

function saveGlobalConfig(llmConfig, hubUrl) {
  var globalConfigPath = os.path.join(rootDir, 'global-config.toml')
  var content = `
[llm]
provider = "${llmConfig.provider || 'openai'}"
api_endpoint = "${llmConfig.api_endpoint || ''}"
api_key = "${llmConfig.api_key || ''}"
model = "${llmConfig.model || 'gpt-4o-mini'}"
temperature = ${llmConfig.temperature || 0.7}
timeout_secs = ${llmConfig.timeout_secs || 120}

[metadata]
source = "hub"
hub_url = "${hubUrl}"
updated_at = ${Date.now() / 1000}
`
  os.write(globalConfigPath, content)
  console.info('[join-party] Global config saved to:', globalConfigPath)
}
```

### 4.3 创建 Agent API 改造

**文件**: `agent/api.js` 中的 `createAgent` 函数

```javascript
function createAgent(agentName, displayName, modelConfig, description) {
  console.log('[AGENT] Creating agent: ' + agentName)
  
  // ✨ 如果未提供 modelConfig，尝试从全局配置读取
  if (!modelConfig || !modelConfig.api_key) {
    console.log('[AGENT] No model config provided, loading from global config')
    var globalConfig = loadGlobalConfig()
    if (globalConfig && globalConfig.llm) {
      modelConfig = {
        provider: globalConfig.llm.provider,
        api_endpoint: globalConfig.llm.api_endpoint,
        api_key: globalConfig.llm.api_key,
        model: globalConfig.llm.model,
        temperature: globalConfig.llm.temperature,
        timeout_secs: globalConfig.llm.timeout_secs
      }
      console.log('[AGENT] Loaded model config from global config')
    }
  }
  
  // ... 现有逻辑 ...
}

function loadGlobalConfig() {
  var globalConfigPath = os.path.join(rootDir, 'global-config.toml')
  try {
    var content = os.read(globalConfigPath).toString()
    return parseToml(content)  // 需要实现 TOML 解析
  } catch (e) {
    console.log('[AGENT] No global config found')
    return null
  }
}
```

## 5. 数据库 Schema

### 5.1 agents 表 (已存在，无需修改)

```sql
CREATE TABLE IF NOT EXISTS agents (
  agent_name      TEXT PRIMARY KEY,
  display_name    TEXT,
  description     TEXT,
  directory       TEXT NOT NULL,
  config_path     TEXT NOT NULL,
  workspace_dir   TEXT NOT NULL,
  port            INTEGER NOT NULL,
  pid             INTEGER,
  status          TEXT NOT NULL DEFAULT 'stopped',
  created_at      REAL NOT NULL,
  updated_at      REAL NOT NULL,
  config_json     TEXT,
  error_msg       TEXT
)
```

### 5.2 新增：global_config 表 (可选，用于历史记录)

```sql
CREATE TABLE IF NOT EXISTS global_config_history (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  source      TEXT NOT NULL,  -- 'hub' | 'user'
  hub_url     TEXT,
  config_json TEXT NOT NULL,
  created_at  REAL NOT NULL
)
```

## 6. 前端交互流程

### 6.1 Join Party 成功后的 UI 反馈

**文件**: `chat-gui/src/components/ChatSidebar.vue`

```javascript
const handleJoinParty = async () => {
  // ... 现有逻辑 ...
  
  try {
    const response = await joinParty(joinPartyUrl.value, joinPartyUserName.value.trim() || undefined)
    
    // ✨ 新增：显示配置获取和 0#Agent 创建状态
    joinPartySuccess.value = '成功加入组织！正在配置 0#Agent...'
    
    // 等待 0#Agent 创建完成
    await new Promise(resolve => setTimeout(resolve, 2000))
    
    // 刷新 zAgents 列表
    await fetchZAgents()
    
    // 自动切换到 zAgents 视图
    activeOrg.value = 'zagents'
    
    joinPartySuccess.value = '✅ 加入成功！0#Agent 已就绪'
    
    setTimeout(() => {
      closeJoinParty()
    }, 1500)
  } catch (err) {
    // ... 错误处理 ...
  }
}
```

### 6.2 创建新 Agent 时的配置继承提示

```vue
<template>
  <div class="model-config-section">
    <div class="global-config-hint" v-if="hasGlobalConfig">
      ℹ️ 已检测到全局配置（来自 Hub），留空将自动继承
    </div>
    <button class="model-config-toggle" @click="showModelConfig = !showModelConfig">
      <span class="toggle-icon">{{ showModelConfig ? '▾' : '▸' }}</span>
      模型配置（可选，留空使用全局配置）
    </button>
    <!-- ... 现有字段 ... -->
  </div>
</template>

<script setup>
const hasGlobalConfig = ref(false)

onMounted(async () => {
  // 检查是否有全局配置
  try {
    const res = await fetch('/api/global-config')
    if (res.ok) {
      hasGlobalConfig.value = true
    }
  } catch {}
})
</script>
```

## 7. 配置文件生成

### 7.1 zeroclaw config.toml 生成逻辑

**文件**: `agent/api.js` 中的 `applyModelConfig` 函数 (已存在，需增强)

```javascript
function applyModelConfig(templateContent, modelConfig) {
  var provider = modelConfig.provider || 'openai'
  var model = modelConfig.model || 'gpt-4o-mini'
  var apiKey = modelConfig.api_key
  var apiEndpoint = modelConfig.api_endpoint
  var temperature = modelConfig.temperature || 0.7
  var timeoutSecs = modelConfig.timeout_secs || 120

  var lines = templateContent.split('\n')
  var result = []

  for (var i = 0; i < lines.length; i++) {
    var line = lines[i]
    
    if (line.startsWith('api_key = ')) {
      result.push('api_key = "' + apiKey + '"')
    } else if (line.startsWith('default_provider = ')) {
      if (provider === 'custom' && apiEndpoint) {
        result.push('default_provider = "custom:' + apiEndpoint + '"')
      } else {
        result.push('default_provider = "' + provider + '"')
      }
    } else if (line.startsWith('default_model = ')) {
      result.push('default_model = "' + model + '"')
    } else if (line.startsWith('default_temperature = ')) {
      result.push('default_temperature = ' + temperature)
    } else if (line.startsWith('provider_timeout_secs = ')) {
      result.push('provider_timeout_secs = ' + timeoutSecs)
    } else {
      result.push(line)
    }
  }

  return result.join('\n')
}
```

### 7.2 全局配置与 Agent 配置合并

```javascript
function mergeConfigs(globalConfig, agentConfig) {
  // agentConfig 优先级高于 globalConfig
  return {
    provider: agentConfig?.provider || globalConfig?.provider,
    api_endpoint: agentConfig?.api_endpoint || globalConfig?.api_endpoint,
    api_key: agentConfig?.api_key || globalConfig?.api_key,
    model: agentConfig?.model || globalConfig?.model,
    temperature: agentConfig?.temperature || globalConfig?.temperature,
    timeout_secs: agentConfig?.timeout_secs || globalConfig?.timeout_secs
  }
}
```

## 8. 错误处理

### 8.1 Hub 配置获取失败

```javascript
// 场景 1: Hub 未配置 default_llm_config
if (!parsedPermit.default_llm_config) {
  console.info('[join-party] Hub did not provide default LLM config')
  joinPartySuccess.value = '✅ 加入成功！请手动配置 Agent'
  // 不创建 0#Agent，用户需手动创建
  return
}

// 场景 2: Hub 配置格式错误
try {
  validateLLMConfig(parsedPermit.default_llm_config)
} catch (e) {
  console.error('[join-party] Invalid LLM config from hub:', e)
  joinPartyError.value = '⚠️ Hub 配置格式错误，请联系管理员'
  return
}
```

### 8.2 0#Agent 创建失败

```javascript
try {
  api.createAgent('0#Agent', '0#Agent', defaultLLMConfig, 'System agent')
  api.startAgent('0#Agent')
} catch (e) {
  console.error('[join-party] Failed to create 0#Agent:', e)
  
  // 记录错误但不阻塞 join party
  db.logApi(
    clientIp,
    'POST /api/join-party',
    {},
    '',
    {},
    JSON.stringify({ warning: '0#Agent creation failed', error: e.message })
  )
  
  // 提示用户手动创建
  joinPartySuccess.value = '✅ 加入成功！0#Agent 创建失败，请手动创建'
}
```

### 8.3 配置格式不兼容

```javascript
function validateLLMConfig(config) {
  if (!config) throw 'Config is null'
  if (!config.provider) throw 'Missing provider'
  if (!config.model) throw 'Missing model'
  if (!config.api_key && config.provider !== 'ollama') {
    throw 'Missing api_key for provider: ' + config.provider
  }
  if (config.provider === 'custom' && !config.api_endpoint) {
    throw 'Missing api_endpoint for custom provider'
  }
}
```

## 9. 实施步骤

### Phase 1: Hub 端改造 (1-2 天)
1. ✅ 设计 hub-config.toml 格式
2. ✅ 修改 Hub 启动脚本支持配置文件
3. ✅ 修改 `/invite` 端点，在 permit 中注入 `default_llm_config`
4. ✅ 测试 Hub 配置读取和注入

### Phase 2: ClawParty Agent 改造 (2-3 天)
1. ✅ 实现 `saveGlobalConfig` 函数
2. ✅ 修改 `/api/join-party` 端点，解析并保存全局配置
3. ✅ 修改 `/api/join-party` 端点，自动创建 0#Agent
4. ✅ 修改 `createAgent` 函数，支持从全局配置继承
5. ✅ 新增 `/api/global-config` 端点（可选）
6. ✅ 测试配置继承逻辑

### Phase 3: 前端改造 (1-2 天)
1. ✅ 修改 `handleJoinParty` 函数，显示配置获取进度
2. ✅ 修改创建 Agent 对话框，显示全局配置提示
3. ✅ 测试 UI 交互流程

### Phase 4: 集成测试 (1 天)
1. ✅ 端到端测试：启动 Hub → Join Party → 验证 0#Agent
2. ✅ 测试配置继承：创建新 Agent → 验证配置
3. ✅ 测试错误场景：Hub 无配置、配置格式错误等

### Phase 5: 文档和部署 (1 天)
1. ✅ 编写 Hub 配置文档
2. ✅ 编写用户使用文档
3. ✅ 更新部署脚本

## 10. 风险点和注意事项

### 10.1 安全风险
- **API Key 泄露**: Hub 配置文件中的 API Key 需要加密存储
- **配置注入攻击**: 验证 Hub 返回的配置格式，防止恶意注入
- **权限控制**: 0#Agent 不可删除，需在前端和后端双重校验

### 10.2 兼容性风险
- **旧版本 Hub**: 如果 Hub 未升级，不会返回 `default_llm_config`，需兼容处理
- **配置格式变更**: zeroclaw config.toml 格式可能变化，需版本检测

### 10.3 用户体验风险
- **配置覆盖**: 用户手动修改全局配置后，再次 join party 是否覆盖？建议提示用户
- **0#Agent 命名冲突**: 如果用户已有名为 "0#Agent" 的 agent，需处理冲突

### 10.4 性能风险
- **Agent 启动时间**: 0#Agent 启动可能需要 2-5 秒，需异步处理，不阻塞 join party
- **并发创建**: 多个用户同时 join party，Hub 需支持并发

## 11. 未来扩展

### 11.1 多配置支持
- Hub 可提供多个 LLM 配置预设（如 openai、anthropic、本地模型）
- 用户在 join party 时选择使用哪个预设

### 11.2 配置热更新
- Hub 配置变更后，已加入的用户可通过 API 拉取最新配置
- 提供 `/api/hub/config/sync` 端点

### 11.3 配置模板市场
- Hub 可托管多个配置模板（如"高性能"、"低成本"、"本地化"）
- 用户可在创建 Agent 时选择模板

---

## 附录 A: 关键代码片段

### A.1 Hub 配置读取 (伪代码)

```javascript
// hub/config.js
function loadHubConfig() {
  var configPath = os.path.join(os.home(), '.clawparty-hub', 'hub-config.toml')
  try {
    var content = os.read(configPath).toString()
    return parseToml(content)
  } catch (e) {
    console.error('[hub] Failed to load config:', e)
    return null
  }
}

function getDefaultLLMConfig() {
  var config = loadHubConfig()
  return config?.default_llm_config || null
}
```

### A.2 TOML 解析 (简化版)

```javascript
// agent/toml-parser.js
function parseToml(content) {
  var result = {}
  var currentSection = result
  var lines = content.split('\n')
  
  for (var i = 0; i < lines.length; i++) {
    var line = lines[i].trim()
    
    // 跳过注释和空行
    if (!line || line.startsWith('#')) continue
    
    // 解析 section
    if (line.startsWith('[') && line.endsWith(']')) {
      var sectionName = line.slice(1, -1)
      var parts = sectionName.split('.')
      currentSection = result
      for (var j = 0; j < parts.length; j++) {
        var part = parts[j]
        if (!currentSection[part]) currentSection[part] = {}
        currentSection = currentSection[part]
      }
      continue
    }
    
    // 解析 key = value
    var eqIndex = line.indexOf('=')
    if (eqIndex > 0) {
      var key = line.slice(0, eqIndex).trim()
      var value = line.slice(eqIndex + 1).trim()
      
      // 去除引号
      if (value.startsWith('"') && value.endsWith('"')) {
        value = value.slice(1, -1)
      }
      
      // 尝试解析数字
      if (!isNaN(value)) {
        value = parseFloat(value)
      }
      
      currentSection[key] = value
    }
  }
  
  return result
}
```

### A.3 0#Agent 特殊处理

```javascript
// agent/api.js
function deleteAgent(agentName) {
  // ✨ 禁止删除 0#Agent
  if (agentName === '0#Agent') {
    console.log('[AGENT] Delete rejected: 0#Agent is a system agent')
    throw 'Cannot delete system agent: 0#Agent'
  }
  
  // ... 现有逻辑 ...
}

// chat-gui/src/components/ChatSidebar.vue
<button
  v-if="agent.agent_name !== '0#Agent'"
  class="agent-action-btn delete-btn"
  @click.stop="handleDeleteZAgent(agent.agent_name)"
  title="Delete Agent"
>
  <!-- Delete icon -->
</button>
```

---

## 附录 B: API 接口定义

### B.1 新增 API: 获取全局配置

```
GET /api/global-config

Response 200:
{
  "llm": {
    "provider": "custom",
    "api_endpoint": "http://...",
    "model": "Kimi-K2.6",
    "temperature": 0.7,
    "timeout_secs": 120
  },
  "metadata": {
    "source": "hub",
    "hub_url": "https://join.clawparty.ai",
    "updated_at": 1745678901
  }
}

Response 404:
{
  "error": "No global config found"
}
```

### B.2 新增 API: 更新全局配置

```
POST /api/global-config

Request Body:
{
  "llm": {
    "provider": "openai",
    "api_key": "sk-xxx",
    "model": "gpt-4o-mini"
  }
}

Response 200:
{
  "message": "Global config updated",
  "config": { ... }
}
```

---

## 总结

本方案通过以下关键设计实现了用户旅程的自动化：

1. **Hub 配置注入**: Hub 在生成 permit 时注入 `default_llm_config`
2. **全局配置管理**: ClawParty Agent 保存全局配置到 `~/.clawparty/global-config.toml`
3. **0#Agent 自动创建**: Join Party 成功后自动创建并启动 0#Agent
4. **配置继承**: 新 Agent 默认继承全局配置，可选覆盖
5. **错误容错**: 各环节失败不阻塞主流程，提供清晰的错误提示

**核心优势**:
- ✅ 用户体验流畅：Join Party → 自动配置 → 立即可用
- ✅ 配置集中管理：Hub 统一管理默认配置，降低用户配置成本
- ✅ 灵活性：支持全局配置 + 自定义配置
- ✅ 向后兼容：旧版本 Hub 不影响现有流程

**实施优先级**:
1. **P0**: Hub 配置注入 + ClawParty 解析保存
2. **P1**: 0#Agent 自动创建 + 配置继承
3. **P2**: 前端 UI 优化 + 错误处理
4. **P3**: 配置管理 API + 高级功能
