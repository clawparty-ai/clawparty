# PRD: zAgent Task 管理系统

**文档版本**: v1.0  
**创建日期**: 2026-04-25  
**优先级**: P1  

---

## 1. 需求概述

### 1.1 背景
当前用户与 zAgent（ZeroClaw AI agent）聊天时，对话是线性的、无结构化信息。当用户让 zAgent "做一件事"时，AI 可能在执行过程中分解出多个子任务，但这些任务信息散落在对话流中，用户无法：
- 一眼看清 AI 当前在做什么
- 了解任务的整体结构和子任务依赖关系
- 追踪任务的执行进度和状态

### 1.2 核心目标
1. **任务自动提取**：当用户指示 zAgent 做事时，AI 自动将用户意图解析为 Task
2. **子任务自省**：zAgent 在执行过程中自动分析并记录 Sub-task
3. **可视化面板**：Web UI 在聊天区上方展示当前 zAgent 的任务树，类似甘特图的层级+依赖视图
4. **状态追踪**：每个任务有明确的状态（pending/running/completed/failed）和进度

### 1.3 业务价值
- **透明度提升**：用户清楚知道 AI 在"忙什么"，减少焦虑
- **可预期性**：通过任务结构了解预期完成时间
- **可追溯性**：任务历史可回溯，便于复盘

---

## 2. 核心概念

| 概念 | 英文 | 定义 |
|------|------|------|
| 任务 | Task | 用户直接指派给 zAgent 的事情，顶层任务 |
| 子任务 | Sub-task | zAgent 在执行 Task 过程中自我分解出的步骤 |
| 任务状态 | Task Status | `pending` → `running` → `completed` / `failed` |
| 任务事件 | Task Event | 状态变更的历史记录 |

### 2.1 任务层级关系
```
Task (用户指派)
├── Sub-task 1 (AI分解)
│   ├── Sub-sub-task 1.1
│   └── Sub-sub-task 1.2
├── Sub-task 2 (AI分解)
│   └── Sub-sub-task 2.1
└── Sub-task 3 (AI分解)
```

### 2.2 任务状态流转
```
        ┌──────────┐
        │  pending │ ← 初始状态
        └────┬─────┘
             │ AI开始执行
             ▼
        ┌──────────┐
        │ running  │ ← 执行中
        └────┬─────┘
       ┌───┴───┐
       ▼       ▼
┌─────────┐ ┌───────┐
│completed│ │ failed│
└─────────┘ └───────┘
```

---

## 3. 数据模型

### 3.1 tasks 表（核心）

```sql
CREATE TABLE IF NOT EXISTS tasks (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id       TEXT NOT NULL UNIQUE,   -- 全局唯一 task ID
  agent_name    TEXT NOT NULL,           -- 所属 zAgent 名称
  parent_id     TEXT,                    -- 父任务 task_id，null 则为顶层 Task
  title         TEXT NOT NULL,           -- 任务标题
  description   TEXT,                    -- 任务描述
  status        TEXT NOT NULL DEFAULT 'pending',
  progress      INTEGER DEFAULT 0,       -- 0-100 进度百分比
  priority      TEXT DEFAULT 'normal',   -- low|normal|high|urgent
  dependencies  TEXT,                    -- JSON 数组
  created_at    REAL NOT NULL,           -- Unix timestamp (seconds)
  updated_at    REAL NOT NULL,
  started_at    REAL,
  completed_at  REAL
);

CREATE INDEX idx_tasks_agent ON tasks(agent_name);
CREATE INDEX idx_tasks_parent ON tasks(parent_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_task_id ON tasks(task_id);
```

### 3.2 task_events 表（审计）

```sql
CREATE TABLE IF NOT EXISTS task_events (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id       TEXT NOT NULL,
  event_type    TEXT NOT NULL,           -- created|started|progress|completed|failed|updated
  from_status   TEXT,
  to_status     TEXT,
  progress      INTEGER,
  message       TEXT,
  timestamp     REAL NOT NULL
);

CREATE INDEX idx_task_events_task ON task_events(task_id);
CREATE INDEX idx_task_events_timestamp ON task_events(timestamp);
```

---

## 4. API 设计

### 4.1 任务 CRUD

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/tasks?agent={agentName}` | 获取某 zAgent 的所有任务树 |
| GET | `/api/tasks/{taskId}` | 获取单个任务详情 |
| POST | `/api/tasks` | 创建任务/子任务 |
| PUT | `/api/tasks/{taskId}` | 更新任务（状态、进度、标题等）|
| DELETE | `/api/tasks/{taskId}` | 删除任务及其子任务 |
| GET | `/api/tasks/{taskId}/events` | 获取任务事件历史 |

### 4.2 请求/响应示例

**创建任务** (POST /api/tasks)
```json
{
  "agent_name": "coder-agent",
  "parent_id": null,
  "title": "实现用户登录功能",
  "description": "为Web应用添加JWT登录验证",
  "priority": "high"
}
```

**响应**:
```json
{
  "task_id": "task-20250425120000-abc123",
  "agent_name": "coder-agent",
  "parent_id": null,
  "title": "实现用户登录功能",
  "status": "pending",
  "progress": 0,
  "created_at": 1745563200
}
```

---

## 5. 前端组件设计

### 5.1 TaskPanel.vue

**位置**: `chat-gui/src/components/TaskPanel.vue`

**插入点**: `ChatMain.vue` 中 `<ChatHeader>` 与 `<div class="messages">` 之间

**Props**:
```javascript
{
  agentName: String,      // zAgent 名称
  tasks: Array,           // 任务树数据
  expanded: Boolean       // 是否展开
}
```

**功能**:
- 树形展示任务层级（类似文件浏览器）
- 用进度条显示每个任务的完成度
- 用颜色区隔状态（pending=灰, running=蓝, completed=绿, failed=红）
- 支持点击 task 在消息区高亮相关对话
- 面板可折叠（记在 localStorage）

**样式**:
```css
.task-panel {
  flex-shrink: 0;
  max-height: 240px;
  overflow-y: auto;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-secondary);
}
```

---

## 6. AI 集成方案

### 6.1 核心策略：不修改 Rust 代码

通过 **SOUL.md 系统提示注入** + **WebSocket 标记解析** 实现。

### 6.2 SOUL.md 追加内容

在每个 zAgent 的 `workspace/SOUL.md` 中追加 `## Task Management` 章节（见代码实现中的 `SOUL_TASK_OVERLAY` 常量）。

### 6.3 WebSocket 消息解析流程

```
[ZeroClawWS onmessage]
        |
        ▼
[JSON.parse(event.data)]
        |
        ├── type === 'chunk' ──▶ 检查 content 是否包含 <task> 或 <subtask>
        |                              |
        |                              ▼
        |                        [解析 XML 标记]
        |                              |
        |                              ├── 新任务 ──▶ POST /api/tasks
        |                              ├── 更新任务 ──▶ PUT /api/tasks/{id}
        |                              └── 子任务 ──▶ POST /api/tasks (parent_id set)
        |
        └── type === 'done' ──▶ 标记所有 running 任务为 completed
```

### 6.4 前端标记解析器 (App.vue)

```javascript
const parseTaskTags = (content, agentName) => {
  // 解析 <task id="..." title="..." status="..." progress="N">...</task>
  const taskRegex = /<task\s+id="([^"]+)"\s+title="([^"]*)"(?:\s+status="([^"]*)")?(?:\s+progress="(\d+)")?[^>]*>([\s\S]*?)<\/task>/gi
  
  // 解析 <subtask parent="..." id="..." title="..." status="..." progress="N">...</subtask>
  const subtaskRegex = /<subtask\s+parent="([^"]+)"\s+id="([^"]+)"\s+title="([^"]*)"(?:\s+status="([^"]*)")?(?:\s+progress="(\d+)")?[^>]*>([\s\S]*?)<\/subtask>/gi
  
  // 匹配后调用 taskService.createOrUpdate()
}
```

---

## 7. 后端实现

### 7.1 agent/db.js 新增方法

- `createTask(task)` — 创建任务，自动记录创建事件
- `getTask(taskId)` — 按 task_id 查询
- `updateTask(taskId, updates)` — 更新任务，自动记录事件
- `deleteTask(taskId)` — 删除任务及事件
- `deleteTaskCascade(taskId)` — 级联删除子任务
- `getAgentTasks(agentName)` — 构建任务树
- `recordTaskEvent(...)` — 记录状态变更历史
- `getTaskEvents(taskId)` — 查询事件历史
- `getTaskCount(agentName, status)` — 统计

### 7.2 agent/main.js 新增路由

| 路由 | 方法 | 功能 |
|------|------|------|
| `/api/tasks` | GET/POST | 查询/创建 |
| `/api/tasks/{taskId}` | GET/PUT/DELETE | 详情/更新/删除 |
| `/api/tasks/{taskId}/events` | GET | 事件历史 |

### 7.3 同步到 TASKS.md

每次任务变更后，自动生成 `workspace/TASKS.md` 供 AI 读取上下文。

---

## 8. 前端服务层

### 8.1 chatService.js 新增 taskService

```javascript
export const taskService = {
  getAgentTasks(agentName) { ... },
  createTask(taskData) { ... },
  updateTask(taskId, updates) { ... },
  deleteTask(taskId) { ... },
  getTaskEvents(taskId) { ... }
}
```

### 8.2 ChatMain.vue 集成

- 导入 `TaskPanel` 组件
- 在 `<ChatHeader>` 与 `<div class="messages">` 之间插入 `<TaskPanel>`
- 每 3 秒轮询任务数据（仅对 zAgent 聊天）

---

## 9. 技术约束与兼容性

### 9.1 PipyJS 兼容性（agent/ 目录）

- ❌ 禁止使用 `.map()` / `.filter()` / `.forEach()` — 使用 `for` 循环代替
- ❌ 禁止使用 `...` spread 运算符 — 使用 `.bind(n, value)` 链式调用代替
- ❌ 禁止使用箭头函数 `=>` — 使用 `function() {}` 代替
- ❌ 禁止使用 `Array.includes()` — 使用 `indexOf` 代替
- ❌ 禁止使用 `RegExp` API — 使用字符串操作代替

### 9.2 Vue 3 格式化（chat-gui/ 目录）

- 使用 `<script setup>` composition API
- 使用 `ref` / `computed` / `watch`
- 使用 `.then().catch()` 处理异步 API 调用

---

## 10. 实施计划（已执行）

### Phase 1: 数据库层 ✅
- [x] `agent/db.js`：新增 `tasks` 和 `task_events` 表创建
- [x] 实现 `createTask`, `updateTask`, `getAgentTasks`, `deleteTaskCascade`
- [x] 实现 `recordTaskEvent`, `getTaskEvents`
- [x] PipyJS 兼容性修复（禁止 `.map()`, `...` spread，改为 `for` 循环 + `.bind()`）

### Phase 2: 后端 API ✅
- [x] `agent/main.js`：新增 `/api/tasks`, `/api/tasks/{taskId}`, `/api/tasks/{taskId}/events` 路由
- [x] PipyJS 兼容的 POST/PUT/DELETE 实现

### Phase 3: AI 集成 - WebSocket 解析 ✅
- [x] `App.vue` 导入 `taskService`
- [x] 添加 `parseTaskTags()` 函数，解析 `<task>` 和 `<subtask>` XML 标记
- [x] 在 ZeroClaw WebSocket handler 的 `chunk` 事件中调用解析器
- [x] 解析后自动调用 `taskService.createTask` / `updateTask` 持久化

### Phase 4: 前端 TaskPanel 组件 ✅
- [x] 创建 `TaskPanel.vue` 组件（树形任务展示 + 进度条 + 状态颜色）
- [x] 在 `ChatMain.vue` 中插入 TaskPanel（仅对 zAgent 显示）
- [x] 添加任务轮询逻辑（每 3 秒）
- [x] `chatService.js` 新增 `taskService`

### Phase 5: 待完成
- [ ] 集成测试：验证端到端流程
- [ ] SOUL.md Task Management overlay 注入（手动或通过 installer）
- [ ] TASKS.md 同步函数（可选增强）

---

## 11. 变更文件清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `agent/db.js` | 修改 | 新增 tasks/task_events 表创建 + 8 个 CRUD 方法 |
| `agent/main.js` | 修改 | 新增 4 个 /api/tasks/* 路由 |
| `chat-gui/src/App.vue` | 修改 | 导入 taskService + parseTaskTags + 标记解析集成 |
| `chat-gui/src/components/ChatMain.vue` | 修改 | 插入 TaskPanel + 轮询逻辑 |
| `chat-gui/src/components/TaskPanel.vue` | 新增 | 任务树形面板组件 |
| `chat-gui/src/services/chatService.js` | 修改 | 新增 taskService 导出 |

---

## 12. 验收标准

### 功能验收
- [ ] 用户发送指令后，zAgent 能在回复中输出 `<task>` 标记
- [ ] 前端正确解析标记并调用 API 创建/更新任务
- [ ] TaskPanel 展示任务树和进度（状态颜色：pending=灰, running=蓝, completed=绿, failed=红）
- [ ] 任务状态变更时面板实时更新（3 秒轮询）
- [ ] 支持多级子任务（缩进层级显示）

### 性能验收
- [ ] 任务解析不阻塞 WebSocket 消息流（< 50ms）
- [ ] 任务查询 API 响应 < 200ms（100 个任务）
- [ ] TaskPanel 渲染不造成明显卡顿

### 稳定性验收
- [ ] zAgent 重启后任务历史不丢失（SQLite 持久化）
- [ ] 无效/损坏的 task 标记不导致 UI 错误
- [ ] PipyJS 后端无运行时错误

---

## 13. 未来扩展

- **TASKS.md 双写同步**：每次任务变更自动写入 agent workspace/TASKS.md，供 AI 感知上下文
- **任务依赖可视化**：在 TaskPanel 中展示依赖关系连线
- **用户手动任务**：允许用户在面板中手动创建任务（不仅 AI 驱动）
- **任务完成通知**：任务完成时通过 WebSocket 推送通知到前端
- **任务历史时间线**：可查看每个任务的完整事件时间线

---

**状态**: Phase 1-4 已完成 ✅ | 待 Phase 5 集成测试
