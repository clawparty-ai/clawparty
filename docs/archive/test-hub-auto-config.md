# Hub 自动配置和 0#Agent 创建功能测试指南

## 功能概述

本次实现了以下功能：

1. **全局配置管理** (`agent/config.js`)
   - 保存全局 LLM 配置到 `~/.clawparty/global-config.toml`
   - 从全局配置文件加载配置
   - 合并全局配置和 Agent 特定配置

2. **0#Agent 自动创建** (`agent/api.js`)
   - 在 join party 成功后自动创建 0#Agent
   - 使用 hub 提供的 LLM 配置
   - 自动启动 0#Agent

3. **Join Party 流程改造** (`agent/main.js`)
   - 解析 permit 中的 `default_llm_config` 字段
   - 保存全局配置
   - 调用 `createZeroAgent()` 创建 0#Agent

4. **全局配置 API**
   - `GET /api/global-config` - 获取全局配置
   - `PUT /api/global-config` - 更新全局配置

## 测试前提

### Hub 端配置

Hub 需要在 permit 响应中包含 `default_llm_config` 字段：

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
      "api_endpoint": "http://1873330231187220.cn-hangzhou.pai-eas.aliyuncs.com/api/predict/quickstart_deploy_20260422_wqkx/v1",
      "api_key": "NTA5OWQ1ZjVjZjUzYTg3OTk5ZDAwMTk2OWY0NDU0NzliNDliN2JhYQ==",
      "model": "Kimi-K2.6",
      "temperature": 0.7,
      "timeout_secs": 120
    }
  }
}
```

## 测试步骤

### 1. 测试 Join Party 自动配置

```bash
# 启动 ClawParty Agent
cd /Users/jade/clawparty
./agent/main.js --listen 127.0.0.1:6789 --data ~/.clawparty

# 在另一个终端，调用 join party API
curl -X POST http://127.0.0.1:6789/api/join-party \
  -H "Content-Type: application/json" \
  -d '{
    "regUrl": "https://your-hub-url:7779",
    "userName": "test-user"
  }'
```

**预期结果**：
- Join party 成功
- 全局配置保存到 `~/.clawparty/global-config.toml`
- 0#Agent 自动创建并启动
- 日志中显示：
  ```
  [join-party] Received default LLM config from hub
  [config] Global config saved to: ~/.clawparty/global-config.toml
  [join-party] Creating 0#Agent with hub config
  [AGENT] 0#Agent created successfully
  [AGENT] 0#Agent started
  ```

### 2. 验证全局配置文件

```bash
cat ~/.clawparty/global-config.toml
```

**预期内容**：
```toml
[llm]
provider = "custom"
api_endpoint = "http://..."
api_key = "xxx"
model = "Kimi-K2.6"
temperature = 0.7
timeout_secs = 120

[metadata]
source = "hub"
hub_url = "https://your-hub-url:7779"
updated_at = 1745678901
```

### 3. 验证 0#Agent 创建

```bash
# 查看所有 agents
curl http://127.0.0.1:6789/api/agents

# 查看 0#Agent 状态
curl http://127.0.0.1:6789/api/agents/0%23Agent/status
```

**预期结果**：
- agents 列表中包含 `0#Agent`
- 状态为 `running`
- 端口已分配

### 4. 测试全局配置 API

```bash
# 获取全局配置
curl http://127.0.0.1:6789/api/global-config

# 更新全局配置
curl -X PUT http://127.0.0.1:6789/api/global-config \
  -H "Content-Type: application/json" \
  -d '{
    "llm": {
      "provider": "openai",
      "api_key": "sk-xxx",
      "model": "gpt-4o-mini",
      "temperature": 0.8,
      "timeout_secs": 60
    },
    "metadata": {
      "hub_url": "https://example.com"
    }
  }'
```

**预期结果**：
- GET 返回全局配置（不包含 api_key）
- PUT 成功更新配置文件

### 5. 测试配置继承

```bash
# 创建新 agent，不提供 model config
curl -X POST http://127.0.0.1:6789/api/agents \
  -H "Content-Type: application/json" \
  -d '{
    "agent_name": "test-agent",
    "display_name": "Test Agent",
    "description": "Test agent for config inheritance"
  }'

# 检查新 agent 的配置文件
cat ~/.clawparty/agents/test-agent/config.toml
```

**预期结果**：
- 新 agent 的配置文件包含全局配置的 LLM 设置
- 日志显示：`[AGENT] Loaded model config from global config`

### 6. 测试 0#Agent 删除保护

```bash
# 尝试删除 0#Agent
curl -X DELETE http://127.0.0.1:6789/api/agents/0%23Agent
```

**预期结果**：
- 返回 400 错误
- 错误信息：`Cannot delete system agent: 0#Agent`

## 错误场景测试

### 场景 1: Hub 未提供 default_llm_config

**测试方法**：使用旧版本 Hub 或 Hub 未配置 LLM

**预期行为**：
- Join party 成功
- 不创建全局配置文件
- 不创建 0#Agent
- 日志显示：`[join-party] Hub did not provide default LLM config`

### 场景 2: 0#Agent 已存在

**测试方法**：第二次 join party（或手动创建 0#Agent 后 join party）

**预期行为**：
- Join party 成功
- 不重复创建 0#Agent
- 日志显示：`[AGENT] 0#Agent already exists, skipping creation`

### 场景 3: 0#Agent 创建失败

**测试方法**：模拟端口分配失败或配置错误

**预期行为**：
- Join party 仍然成功
- 0#Agent 创建失败不阻塞流程
- 日志显示：`[join-party] Failed to create 0#Agent: <error>`

## 代码变更清单

### 新增文件
- `agent/config.js` - 全局配置管理模块

### 修改文件
- `agent/api.js`
  - 导入 `config` 模块
  - 修改 `init()` 初始化 config 模块
  - 修改 `createAgent()` 支持从全局配置继承
  - 新增 `createZeroAgent()` 函数
  - 导出全局配置管理函数

- `agent/main.js`
  - 修改 `/api/join-party` 端点
    - 解析 `default_llm_config`
    - 保存全局配置
    - 创建 0#Agent
  - 新增 `/api/global-config` 端点
    - GET - 获取全局配置
    - PUT - 更新全局配置

## 注意事项

1. **API Key 安全**：全局配置 API 的 GET 响应不包含 api_key，避免泄露
2. **0#Agent 保护**：0#Agent 不可删除，前端和后端都有校验
3. **错误容错**：0#Agent 创建失败不阻塞 join party 流程
4. **配置优先级**：Agent 特定配置 > 全局配置
5. **向后兼容**：Hub 未提供配置时，保持现有流程不变

## 下一步

1. 前端改造：修改 join party 对话框，显示配置获取进度
2. 前端改造：修改创建 agent 对话框，显示全局配置提示
3. 集成测试：端到端测试完整流程
4. 文档更新：更新用户文档和部署文档
