# PRD: ZTM Agent 状态管理与监控系统

**文档版本**: v1.0  
**创建日期**: 2026-04-24  
**产品经理**: Claude PM  
**优先级**: P0（高优先级）

---

## 1. 需求概述

### 1.1 背景
当前 ClawParty 系统中，ZTM agent 启动时会自动启动 ZTM Chat 应用，并且缺乏对 zeroclaw agent（数字员工）状态变化的系统性记录。这导致：
- 无法追溯数字员工的上线/离线历史
- 缺乏对 mesh 网络健康状况的可观测性
- 难以进行故障排查和性能分析

### 1.2 核心目标
1. ZTM agent 启动时连接到 hub，但不自动启动 ZTM Chat
2. 记录 ZTM agent 连接 hub 的事件
3. 同步并记录每个 zeroclaw agent 的状态变化（上线/离线）
4. 提供状态查询和历史追溯能力

### 1.3 业务价值
- **可观测性提升**: 完整的状态变化历史，便于问题排查
- **用户体验优化**: 用户可以清楚看到每个数字员工的在线状态和历史
- **系统稳定性**: 及时发现异常离线，快速响应故障
- **数据分析基础**: 为未来的使用统计和性能分析提供数据支撑

---

## 2. 用户故事

### US-1: ZTM Agent 启动不自动启动 Chat
**As a** 系统管理员  
**I want** ZTM agent 启动时不自动启动 Chat 应用  
**So that** 我可以按需启动 Chat，减少不必要的资源消耗

**验收标准**:
- [ ] ZTM agent 启动后，Chat 应用状态为 `disabled` 或 `stopped`
- [ ] ZTM agent 能正常连接到 hub
- [ ] 用户可以手动启动 Chat 应用
- [ ] 不影响其他 ZTM 应用的正常运行

---

### US-2: 记录 Hub 连接事件
**As a** 系统管理员  
**I want** 系统记录每次 ZTM agent 连接 hub 的事件  
**So that** 我可以追溯网络连接历史，排查连接问题

**验收标准**:
- [ ] 连接成功时记录事件（时间戳、hub ID、agent ID）
- [ ] 连接失败时记录事件（时间戳、失败原因）
- [ ] 断开连接时记录事件（时间戳、断开原因）
- [ ] 事件持久化存储，不因重启丢失

---

### US-3: 同步 Agent 状态
**As a** 系统  
**I want** 在连接 hub 时同步所有 zeroclaw agent 的当前状态  
**So that** 系统能获取到最新的 agent 在线状态

**验收标准**:
- [ ] 连接 hub 后，自动触发状态同步
- [ ] 同步所有已注册的 zeroclaw agent 状态
- [ ] 状态包括：agent_name, status (online/offline), last_seen, port
- [ ] 同步失败时有重试机制（最多 3 次）
- [ ] 同步结果记录到日志

---

### US-4: 记录 Agent 状态变化
**As a** 产品经理  
**I want** 系统记录每个 zeroclaw agent 的状态变化  
**So that** 我可以分析数字员工的使用情况和可用性

**验收标准**:
- [ ] 记录 agent 上线事件（agent_name, timestamp, from_status, to_status）
- [ ] 记录 agent 离线事件（agent_name, timestamp, from_status, to_status）
- [ ] 记录 agent 错误事件（agent_name, timestamp, error_message）
- [ ] 状态变化事件持久化存储
- [ ] 提供 API 查询状态变化历史

---

### US-5: 查询 Agent 状态历史
**As a** 用户  
**I want** 查看某个数字员工的状态历史  
**So that** 我可以了解它的可用性和稳定性

**验收标准**:
- [ ] 提供 API: `GET /api/agents/:name/status-history`
- [ ] 支持时间范围过滤（start_time, end_time）
- [ ] 支持分页（offset, limit）
- [ ] 返回状态变化列表（时间倒序）
- [ ] 响应时间 < 500ms（100 条记录）

---

## 3. 功能设计

### 3.1 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                      ZTM Agent                              │
│  ┌──────────────┐      ┌──────────────┐                    │
│  │ Mesh Manager │─────▶│ Hub Client   │                    │
│  └──────────────┘      └──────┬───────┘                    │
│         │                     │                             │
│         │                     ▼                             │
│         │              ┌──────────────┐                     │
│         │              │ Event Logger │                     │
│         │              └──────┬───────┘                     │
│         │                     │                             │
│         ▼                     ▼                             │
│  ┌─────────────────────────────────────┐                   │
│  │      Status Sync Service            │                   │
│  │  - Poll agent status                │                   │
│  │  - Detect status changes            │                   │
│  │  - Record events                    │                   │
│  └─────────────┬───────────────────────┘                   │
│                │                                            │
│                ▼                                            │
│  ┌─────────────────────────────────────┐                   │
│  │      Database (SQLite)              │                   │
│  │  - agent_status_events              │                   │
│  │  - hub_connection_events            │                   │
│  └─────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 数据模型

#### 3.2.1 agent_status_events 表
```sql
CREATE TABLE agent_status_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  agent_name TEXT NOT NULL,
  event_type TEXT NOT NULL,  -- 'online', 'offline', 'error', 'starting'
  from_status TEXT,           -- 前一个状态
  to_status TEXT NOT NULL,    -- 当前状态
  pid INTEGER,                -- 进程 PID（如果有）
  error_message TEXT,         -- 错误信息（如果有）
  timestamp INTEGER NOT NULL, -- Unix timestamp (ms)
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_agent_status_events_agent ON agent_status_events(agent_name);
CREATE INDEX idx_agent_status_events_timestamp ON agent_status_events(timestamp);
```

#### 3.2.2 hub_connection_events 表
```sql
CREATE TABLE hub_connection_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mesh_name TEXT NOT NULL,
  hub_id TEXT NOT NULL,
  event_type TEXT NOT NULL,  -- 'connected', 'disconnected', 'failed'
  agent_id TEXT NOT NULL,
  error_message TEXT,
  timestamp INTEGER NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_hub_connection_events_mesh ON hub_connection_events(mesh_name);
CREATE INDEX idx_hub_connection_events_timestamp ON hub_connection_events(timestamp);
```

### 3.3 API 设计

#### 3.3.1 查询 Agent 状态历史
```
GET /api/agents/:name/status-history

Query Parameters:
- start_time: number (optional) - Unix timestamp (ms)
- end_time: number (optional) - Unix timestamp (ms)
- limit: number (optional, default: 50, max: 200)
- offset: number (optional, default: 0)

Response:
{
  "agent_name": "0#Agent",
  "total": 150,
  "events": [
    {
      "id": 123,
      "event_type": "online",
      "from_status": "stopped",
      "to_status": "running",
      "pid": 12345,
      "timestamp": 1714000000000,
      "created_at": "2026-04-24T10:00:00Z"
    },
    ...
  ]
}
```

#### 3.3.2 查询 Hub 连接历史
```
GET /api/meshes/:mesh/hub-connection-history

Query Parameters:
- hub_id: string (optional) - 过滤特定 hub
- start_time: number (optional)
- end_time: number (optional)
- limit: number (optional, default: 50)
- offset: number (optional, default: 0)

Response:
{
  "mesh_name": "mesh",
  "total": 80,
  "events": [
    {
      "id": 45,
      "hub_id": "hub-001",
      "event_type": "connected",
      "agent_id": "agent-123",
      "timestamp": 1714000000000,
      "created_at": "2026-04-24T10:00:00Z"
    },
    ...
  ]
}
```

#### 3.3.3 查询所有 Agent 当前状态摘要
```
GET /api/agents/status-summary

Response:
{
  "total_agents": 5,
  "online": 3,
  "offline": 1,
  "error": 1,
  "agents": [
    {
      "agent_name": "0#Agent",
      "status": "running",
      "pid": 12345,
      "last_status_change": 1714000000000,
      "uptime_seconds": 3600
    },
    ...
  ]
}
```

---

## 4. 技术方案

### 4.1 禁用 Chat 自动启动

**修改位置**: `/Users/jade/clawparty/agent/mesh.js`

**方案**:
1. 在 `initApps()` 调用后，检查 Chat 应用状态
2. 如果 Chat 应用为 `enabled` 且 `isRunning`，则调用 `apps.stopApp('ztm', 'chat')`
3. 设置 Chat 应用为 `disabled` 状态

**代码示例**:
```javascript
// In mesh.js, after apps initialization
try {
  apps = initApps(...)
  
  // Disable auto-start for Chat app
  var chatApp = apps.findApp('ztm', 'chat')
  if (chatApp && chatApp.isRunning) {
    apps.stopApp('ztm', 'chat')
  }
  if (chatApp && !chatApp.isDisabled) {
    apps.disableApp('ztm', 'chat')
  }
} catch (e) {
  meshError(e.toString())
}
```

### 4.2 Hub 连接事件记录

**修改位置**: `/Users/jade/clawparty/agent/mesh.js`

**方案**:
1. 在 `attachHub()` 成功时，调用 `db.recordHubConnectionEvent()`
2. 在 hub 连接失败时，记录失败事件
3. 在 hub 断开连接时（心跳超时），记录断开事件

**关键时机**:
- `attachHub()` 成功返回后
- Hub 心跳检测失败时
- Mesh 主动断开 hub 连接时

### 4.3 Agent 状态同步服务

**新增文件**: `/Users/jade/clawparty/agent/status-sync.js`

**核心逻辑**:
```javascript
export default function StatusSyncService(db, pollIntervalMs) {
  var isRunning = false
  var lastKnownStatuses = {}  // { agent_name: status }
  
  function start() {
    if (isRunning) return
    isRunning = true
    scheduleNextPoll()
  }
  
  function stop() {
    isRunning = false
  }
  
  function scheduleNextPoll() {
    if (!isRunning) return
    setTimeout(pollAgentStatuses, pollIntervalMs)
  }
  
  function pollAgentStatuses() {
    var agents = db.allAgents()
    
    agents.forEach(agent => {
      var currentStatus = agent.status
      var lastStatus = lastKnownStatuses[agent.agent_name]
      
      if (lastStatus !== currentStatus) {
        // Status changed, record event
        db.recordAgentStatusEvent({
          agent_name: agent.agent_name,
          event_type: mapStatusToEventType(currentStatus),
          from_status: lastStatus,
          to_status: currentStatus,
          pid: agent.pid,
          timestamp: Date.now()
        })
        
        lastKnownStatuses[agent.agent_name] = currentStatus
      }
    })
    
    scheduleNextPoll()
  }
  
  function mapStatusToEventType(status) {
    if (status === 'running') return 'online'
    if (status === 'stopped') return 'offline'
    if (status === 'error') return 'error'
    if (status === 'starting') return 'starting'
    return 'unknown'
  }
  
  return { start, stop }
}
```

**集成点**: 在 `agent/api.js` 的 `init()` 函数中启动状态同步服务

### 4.4 数据库扩展

**修改位置**: `/Users/jade/clawparty/agent/db.js`

**新增方法**:
```javascript
// 记录 agent 状态事件
function recordAgentStatusEvent(event) {
  db.sql(`
    INSERT INTO agent_status_events 
    (agent_name, event_type, from_status, to_status, pid, error_message, timestamp)
    VALUES (?, ?, ?, ?, ?, ?, ?)
  `).exec(
    event.agent_name,
    event.event_type,
    event.from_status || null,
    event.to_status,
    event.pid || null,
    event.error_message || null,
    event.timestamp
  )
}

// 查询 agent 状态历史
function getAgentStatusHistory(agentName, startTime, endTime, limit, offset) {
  var sql = `
    SELECT * FROM agent_status_events 
    WHERE agent_name = ?
  `
  var params = [agentName]
  
  if (startTime) {
    sql += ' AND timestamp >= ?'
    params.push(startTime)
  }
  if (endTime) {
    sql += ' AND timestamp <= ?'
    params.push(endTime)
  }
  
  sql += ' ORDER BY timestamp DESC LIMIT ? OFFSET ?'
  params.push(limit, offset)
  
  return db.sql(sql).exec(...params)
}

// 记录 hub 连接事件
function recordHubConnectionEvent(event) {
  db.sql(`
    INSERT INTO hub_connection_events 
    (mesh_name, hub_id, event_type, agent_id, error_message, timestamp)
    VALUES (?, ?, ?, ?, ?, ?)
  `).exec(
    event.mesh_name,
    event.hub_id,
    event.event_type,
    event.agent_id,
    event.error_message || null,
    event.timestamp
  )
}

// 查询 hub 连接历史
function getHubConnectionHistory(meshName, hubId, startTime, endTime, limit, offset) {
  // Similar to getAgentStatusHistory
}
```

### 4.5 API 路由实现

**修改位置**: `/Users/jade/clawparty/agent/main.js`

**新增路由**:
```javascript
.route('/api/agents/:name/status-history', ({ params, query }) => {
  var agentName = params.name
  var startTime = query.start_time ? Number(query.start_time) : null
  var endTime = query.end_time ? Number(query.end_time) : null
  var limit = Math.min(Number(query.limit) || 50, 200)
  var offset = Number(query.offset) || 0
  
  var events = db.getAgentStatusHistory(agentName, startTime, endTime, limit, offset)
  var total = db.countAgentStatusEvents(agentName, startTime, endTime)
  
  return new Message({
    agent_name: agentName,
    total: total,
    events: events
  })
})

.route('/api/meshes/:mesh/hub-connection-history', ({ params, query }) => {
  // Similar implementation
})

.route('/api/agents/status-summary', () => {
  var agents = api.allAgentStatuses()
  var summary = {
    total_agents: agents.length,
    online: agents.filter(a => a.status === 'running').length,
    offline: agents.filter(a => a.status === 'stopped').length,
    error: agents.filter(a => a.status === 'error').length,
    agents: agents.map(a => ({
      agent_name: a.agent_name,
      status: a.status,
      pid: a.pid,
      last_status_change: db.getLastStatusChangeTime(a.agent_name),
      uptime_seconds: calculateUptime(a)
    }))
  }
  return new Message(summary)
})
```

---

## 5. 边界情况与风险

### 5.1 边界情况

| 场景 | 处理方案 |
|------|---------|
| ZTM agent 启动时 hub 不可达 | 记录连接失败事件，后续重试时再记录成功事件 |
| Agent 进程异常退出（未经过 stopAgent） | 状态同步服务会检测到进程消失，记录离线事件 |
| 数据库写入失败 | 记录到日志，不影响主流程，下次轮询时重试 |
| 大量 agent 同时上线/离线 | 批量写入数据库，避免逐条插入的性能问题 |
| 历史数据过多导致查询慢 | 添加索引，定期归档旧数据（保留 90 天） |
| 状态轮询间隔过短导致性能问题 | 默认 5 秒轮询一次，可配置 |

### 5.2 技术风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 状态同步服务占用过多 CPU | 中 | 使用定时器而非忙等待，轮询间隔可配置 |
| 数据库表增长过快 | 中 | 实现数据归档策略，定期清理 90 天前的数据 |
| Hub 连接事件记录失败 | 低 | 使用事务保证原子性，失败时记录到日志 |
| 禁用 Chat 影响现有用户 | 高 | 提供配置项允许用户选择是否自动启动 Chat |

### 5.3 依赖关系

- **前置依赖**: 无
- **后置依赖**: 
  - GUI 需要更新以展示状态历史（可选）
  - 告警系统可以基于状态事件触发通知（未来）

---

## 6. 验收标准

### 6.1 功能验收

- [ ] ZTM agent 启动后，Chat 应用不自动启动
- [ ] 手动启动 Chat 应用功能正常
- [ ] Hub 连接成功时，数据库中有对应的连接事件记录
- [ ] Agent 启动时，数据库中有 `online` 事件记录
- [ ] Agent 停止时，数据库中有 `offline` 事件记录
- [ ] Agent 异常退出时，状态同步服务能检测到并记录 `offline` 事件
- [ ] API `/api/agents/:name/status-history` 返回正确的历史记录
- [ ] API `/api/meshes/:mesh/hub-connection-history` 返回正确的连接历史
- [ ] API `/api/agents/status-summary` 返回正确的状态摘要

### 6.2 性能验收

- [ ] 状态同步服务 CPU 占用 < 5%
- [ ] 状态历史查询响应时间 < 500ms（100 条记录）
- [ ] Hub 连接事件记录不阻塞主流程（< 10ms）
- [ ] 数据库写入失败不影响 agent 正常运行

### 6.3 稳定性验收

- [ ] 连续运行 24 小时无内存泄漏
- [ ] 100 次 agent 启动/停止循环，状态记录 100% 准确
- [ ] 数据库写入失败时，系统能正常降级运行

---

## 7. 实施计划

### 7.1 开发阶段（预计 3-5 天）

#### Phase 1: 数据库扩展（1 天）
- [ ] 创建 `agent_status_events` 表
- [ ] 创建 `hub_connection_events` 表
- [ ] 实现 `db.recordAgentStatusEvent()`
- [ ] 实现 `db.getAgentStatusHistory()`
- [ ] 实现 `db.recordHubConnectionEvent()`
- [ ] 实现 `db.getHubConnectionHistory()`
- [ ] 编写数据库迁移脚本

#### Phase 2: 状态同步服务（1 天）
- [ ] 创建 `agent/status-sync.js`
- [ ] 实现状态轮询逻辑
- [ ] 实现状态变化检测
- [ ] 集成到 `agent/api.js`
- [ ] 单元测试

#### Phase 3: Hub 连接事件记录（1 天）
- [ ] 修改 `agent/mesh.js` 的 `attachHub()` 方法
- [ ] 添加连接成功事件记录
- [ ] 添加连接失败事件记录
- [ ] 添加断开连接事件记录
- [ ] 集成测试

#### Phase 4: 禁用 Chat 自动启动（0.5 天）
- [ ] 修改 `agent/mesh.js` 的 apps 初始化逻辑
- [ ] 添加配置项 `auto_start_chat`（默认 false）
- [ ] 测试 Chat 手动启动功能

#### Phase 5: API 实现（1 天）
- [ ] 实现 `/api/agents/:name/status-history`
- [ ] 实现 `/api/meshes/:mesh/hub-connection-history`
- [ ] 实现 `/api/agents/status-summary`
- [ ] API 集成测试

#### Phase 6: 测试与优化（0.5 天）
- [ ] 端到端测试
- [ ] 性能测试
- [ ] 边界情况测试
- [ ] 文档更新

### 7.2 测试阶段（1 天）

- [ ] 功能测试
- [ ] 性能测试
- [ ] 稳定性测试
- [ ] 回归测试

### 7.3 发布阶段（0.5 天）

- [ ] 代码审查
- [ ] 合并到主分支
- [ ] 更新 CHANGELOG
- [ ] 发布 Release Notes

---

## 8. 未来扩展

### 8.1 短期（1-2 个月）
- 在 GUI 中展示 agent 状态历史图表
- 实现状态变化告警（邮件/Webhook）
- 添加状态统计报表（可用性、平均在线时长）

### 8.2 中期（3-6 个月）
- 实现分布式状态同步（多 agent 协同）
- 添加状态预测（基于历史数据预测离线风险）
- 集成到监控系统（Prometheus/Grafana）

### 8.3 长期（6-12 个月）
- 实现自动故障恢复（检测到离线自动重启）
- 添加状态关联分析（分析状态变化与系统事件的关联）
- 实现状态回放功能（重现历史状态变化）

---

## 9. 附录

### 9.1 相关文档
- [ClawParty Architecture](../docs/ARCHITECTURE.md)
- [ZTM Agent API](../docs/API-AGENT.md)
- [Database Schema](../docs/DATABASE.md)

### 9.2 参考资料
- ZTM Hub Protocol Specification
- SQLite Performance Best Practices
- Event Sourcing Pattern

### 9.3 术语表
- **ZTM Agent**: ClawParty 的网络节点，负责 mesh 网络通信
- **ZTM Hub**: Mesh 网络的中心节点，负责路由和协调
- **zeroclaw agent**: AI 数字员工实例，运行在 ZTM agent 之上
- **Status Sync**: 状态同步服务，定期检测 agent 状态变化
- **Event Sourcing**: 事件溯源模式，通过记录状态变化事件来重建状态

---

## 10. 变更记录

| 版本 | 日期 | 作者 | 变更内容 |
|------|------|------|---------|
| v1.0 | 2026-04-24 | Claude PM | 初始版本 |

---

**审批流程**:
- [ ] 产品经理审批
- [ ] 技术负责人审批
- [ ] 架构师审批
- [ ] 开发团队确认

**问题与反馈**: 请在 GitHub Issues 中提出
