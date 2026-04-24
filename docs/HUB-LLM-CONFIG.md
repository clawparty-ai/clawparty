# Hub LLM 配置指南

## 概述

Hub 现在支持为新加入的 Agent 提供默认 LLM 配置。当用户通过 join party 加入组织时，Hub 会在 permit 响应中附带 `default_llm_config` 字段，Agent 端会自动：
1. 保存为全局配置（`~/.clawparty/global-config.toml`）
2. 使用该配置创建 0#Agent
3. 后续创建的 Agent 如果未指定模型配置，会继承全局配置

## Hub 端配置

### 方式 1: 使用配置文件（推荐）

1. 创建 LLM 配置文件（例如 `llm-config.json`）：

```json
{
  "default_llm_config": {
    "provider": "custom",
    "api_endpoint": "http://your-llm-endpoint/v1",
    "api_key": "your-api-key-here",
    "model": "Kimi-K2.6",
    "temperature": 0.7,
    "timeout_secs": 120
  }
}
```

2. 启动 Hub 时指定配置文件：

```bash
./hub/main.js --llm-config llm-config.json
```

### 方式 2: 不提供配置

如果不指定 `--llm-config` 参数，Hub 不会在 permit 中附带 LLM 配置，Agent 端会保持现有行为（用户手动配置）。

## 配置文件格式

### 完整示例

```json
{
  "default_llm_config": {
    "provider": "custom",
    "api_endpoint": "http://1873330231187220.cn-hangzhou.pai-eas.aliyuncs.com/api/predict/quickstart_deploy_20260422_wqkx/v1",
    "api_key": "NTA5OWQ1ZjVjZjUzYTg3OTk5ZDAwMTk2OWY0NDU0NzliNDliN2JhYQ==",
    "model": "Kimi-K2.6",
    "temperature": 0.7,
    "timeout_secs": 120
  }
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `provider` | string | 是 | LLM 提供商，支持：`custom`, `openai`, `anthropic`, `qwen`, `moonshot`, `doubao`, `deepseek`, `ollama` |
| `api_endpoint` | string | 条件 | API 端点 URL，`custom` 和 `ollama` provider 必填 |
| `api_key` | string | 条件 | API 密钥，除 `ollama` 外都必填 |
| `model` | string | 是 | 模型名称，例如 `Kimi-K2.6`, `gpt-4o-mini`, `claude-3-5-sonnet-20241022` |
| `temperature` | number | 否 | 温度参数，默认 0.7 |
| `timeout_secs` | number | 否 | 超时时间（秒），默认 120 |

### 不同 Provider 的配置示例

#### OpenAI

```json
{
  "default_llm_config": {
    "provider": "openai",
    "api_key": "sk-...",
    "model": "gpt-4o-mini",
    "temperature": 0.7
  }
}
```

#### Anthropic

```json
{
  "default_llm_config": {
    "provider": "anthropic",
    "api_key": "sk-ant-...",
    "model": "claude-3-5-sonnet-20241022",
    "temperature": 0.7
  }
}
```

#### 阿里云通义

```json
{
  "default_llm_config": {
    "provider": "qwen",
    "api_key": "sk-...",
    "model": "qwen-plus",
    "temperature": 0.7
  }
}
```

#### 本地 Ollama

```json
{
  "default_llm_config": {
    "provider": "ollama",
    "api_endpoint": "http://localhost:11434",
    "model": "llama3.2",
    "temperature": 0.7
  }
}
```

## Hub 启动示例

### 完整启动命令

```bash
./hub/main.js \
  --data ~/.ztm \
  --listen 0.0.0.0:8888 \
  --enable-registration 0.0.0.0:7779 \
  --llm-config /path/to/llm-config.json
```

### 日志输出

启动成功后，Hub 会输出：

```
Loaded default LLM config from: /path/to/llm-config.json
  Provider: custom
  Model: Kimi-K2.6
```

## Agent 端行为

当 Agent 通过 join party 加入组织时：

1. **接收配置**：从 Hub 的 permit 响应中获取 `default_llm_config`
2. **保存全局配置**：写入 `~/.clawparty/global-config.toml`
3. **创建 0#Agent**：使用该配置自动创建并启动 0#Agent
4. **配置继承**：后续创建的 Agent 如果未指定模型配置，会自动继承全局配置

### 全局配置文件示例

`~/.clawparty/global-config.toml`:

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
hub_url = "https://join.clawparty.ai:7779"
updated_at = 1745678901
```

## 测试验证

### 1. 启动 Hub

```bash
cd /Users/jade/clawparty
./hub/main.js \
  --data ~/.ztm-hub \
  --listen 0.0.0.0:8888 \
  --enable-registration 0.0.0.0:7779 \
  --llm-config hub/llm-config.example.json
```

### 2. 启动 Agent

```bash
./agent/main.js --listen 127.0.0.1:6789 --data ~/.clawparty
```

### 3. Join Party

在 GUI 中点击"加入组织"，输入 Hub 地址 `http://localhost:7779`，观察：
- Join party 对话框显示进度步骤
- 成功后 0#Agent 自动出现在 Agent 列表中
- 全局配置文件已创建

### 4. 验证全局配置

```bash
cat ~/.clawparty/global-config.toml
```

### 5. 创建新 Agent

在 GUI 中创建新 Agent，不填写模型配置，观察：
- 显示"将使用全局配置: Kimi-K2.6"提示
- Agent 创建成功并能正常工作

## 安全注意事项

1. **API Key 保护**：
   - 配置文件中的 API Key 会被传递给 Agent
   - 确保 Hub 和 Agent 之间的通信是加密的（TLS）
   - 不要在公共仓库中提交包含真实 API Key 的配置文件

2. **配置文件权限**：
   ```bash
   chmod 600 llm-config.json
   ```

3. **API Key 轮换**：
   - 定期更换 API Key
   - 更新配置文件后重启 Hub

## 故障排查

### Hub 未加载配置

**症状**：启动日志中没有"Loaded default LLM config"

**原因**：
- 配置文件路径错误
- 配置文件格式错误
- 缺少 `default_llm_config` 字段

**解决**：
```bash
# 检查文件是否存在
ls -la llm-config.json

# 验证 JSON 格式
cat llm-config.json | python3 -m json.tool
```

### Agent 未创建 0#Agent

**症状**：Join party 成功但没有 0#Agent

**原因**：
- Hub 未提供 LLM 配置
- Agent 端代码版本过旧
- 0#Agent 已存在（不会重复创建）

**解决**：
```bash
# 检查 Agent 日志
tail -f ~/.clawparty/logs/agent.log

# 查看全局配置是否已保存
cat ~/.clawparty/global-config.toml

# 手动查询 0#Agent
curl http://127.0.0.1:6789/api/agents | jq '.[] | select(.agent_name == "0#Agent")'
```

### 配置继承不生效

**症状**：创建新 Agent 时未使用全局配置

**原因**：
- 全局配置文件不存在或格式错误
- Agent 端代码版本过旧

**解决**：
```bash
# 检查全局配置 API
curl http://127.0.0.1:6789/api/global-config

# 手动创建 Agent 测试
curl -X POST http://127.0.0.1:6789/api/agents \
  -H "Content-Type: application/json" \
  -d '{"agent_name": "test-agent", "display_name": "Test"}'
```

## 向后兼容性

- **不提供配置**：如果 Hub 不指定 `--llm-config`，行为与之前版本完全一致
- **旧版本 Agent**：旧版本 Agent 会忽略 permit 中的 `default_llm_config` 字段，不影响正常使用
- **配置更新**：更新 Hub 配置文件后需要重启 Hub，已加入的 Agent 不受影响

## 相关文档

- [Agent 端实现文档](TEST-HUB-AUTO-CONFIG.md)
- [完整设计文档](hub-config-auto-setup-design.md)
