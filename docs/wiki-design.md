# Wiki 功能设计文档

## 概述

为每个 zAgent 提供独立的 Wiki 知识库，遵循 LLM Wiki 方法论（三层架构：原始资料、Wiki 内容、维护规范）。

## 设计理念

基于 LLM Wiki 方法论：
- **原始资料层**（raw/）：不可变的源文档，LLM 只读
- **Wiki 内容层**（entities/、concepts/、pages/）：LLM 生成的结构化知识
- **维护规范层**（schema.md）：指导 LLM 如何维护 wiki 的规范文档

## 架构

### 后端（Agent）

#### Wiki 目录结构

```
workspace/wiki/
├── index.md      # 内容目录（自动维护）
├── log.md        # 变更日志（追加写入）
├── schema.md     # Wiki 维护规范
├── raw/          # 原始资料（不可变）
├── entities/     # 实体页面（人物、地点、事物）
├── concepts/     # 概念页面（抽象主题）
└── pages/        # 通用页面
```

#### API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/wiki/{agent}/init` | POST | 初始化 wiki 目录，创建默认模板文件 |
| `/api/wiki/{agent}/tree?path=` | GET | 列出 wiki 目录树 |
| `/api/wiki/{agent}/file/{name}?path=` | GET | 读取 wiki 页面内容 |
| `/api/wiki/{agent}/search?q=` | GET | 搜索 wiki 页面（标题+内容） |
| `/api/wiki/{agent}/graph` | GET | 解析页面间的 `[[链接]]`，返回图数据 |
| `/api/wiki/{agent}/refresh` | POST | 数据刷新（不触发 AI） |

#### 安全

- 路径过滤：禁止 `..` 目录遍历
- 文件名过滤：禁止 `/`、`.` 开头等危险字符

### 前端（Vue 组件）

#### 组件结构

```
ChatMain.vue
├── ChatHeader.vue          # 顶部栏（含 📖 Wiki 按钮）
├── WikiPanel.vue           # Wiki 主面板
│   ├── WikiTreeNode.vue    # 文档树节点
│   ├── marked (渲染)       # Markdown 渲染
│   └── d3-force (图)       # 力导向图物理引擎
├── TaskPanel.vue           # 任务面板
└── WebSharePanel.vue       # 文件共享面板
```

#### WikiPanel 功能

**Tab 1: 📄 文档**
- 左侧：树形文档目录（支持展开/折叠）
- 右侧：Markdown 渲染查看器
- 搜索框：防抖搜索（300ms），实时过滤文档列表
- 全屏模式：⛶ 按钮展开全屏查看

**Tab 2: 🔗 关系图**
- d3-force 物理引擎 + Canvas 2D 渲染
- 节点颜色：entities 绿色、concepts 蓝色、pages 灰色、raw 橙色
- 边：表示页面间的 `[[WikiLink]]` 引用关系
- 交互：力导向布局自动排列

**通用功能**
- 🔄 刷新：旋转动画，重新加载目录树
- 拖拽调整：底部手柄调整面板高度（60-600px）
- 可见性：仅 zAgent 私聊显示

#### Wiki 链接语法

支持 Obsidian 风格的 `[[Page Name]]` 内部链接：
- marked 自定义 tokenizer 解析
- 点击链接自动导航到对应页面
- 同时也支持标准 Markdown 链接 `[text](path.md)`

## 文件清单

### 新增文件

| 文件 | 说明 |
|------|------|
| `chat-gui/src/services/wikiService.js` | Wiki API 服务封装 |
| `chat-gui/src/components/WikiPanel.vue` | Wiki 主面板组件 |
| `chat-gui/src/components/WikiTreeNode.vue` | 文档树节点组件 |

### 修改文件

| 文件 | 修改内容 |
|------|----------|
| `agent/main.js` | 添加 6 个 wiki API 端点 |
| `chat-gui/src/components/ChatHeader.vue` | 添加 📖 Wiki 按钮和 props/emit |
| `chat-gui/src/components/ChatMain.vue` | 集成 WikiPanel，添加状态管理 |
| `chat-gui/src/services/chatService.js` | 导出 wikiService |
| `chat-gui/package.json` | 添加 d3-force 依赖 |

## 依赖

- `marked`（已有）：Markdown 渲染
- `d3-force`（新增）：力导向图物理引擎

## 使用流程

1. **初始化**：首次打开 Wiki Panel 时，后端自动创建 `workspace/wiki/` 目录结构
2. **浏览**：在 📄 文档 Tab 中点击目录树查看页面内容
3. **搜索**：在搜索框输入关键词，实时过滤文档列表
4. **关系图**：切换到 🔗 关系图 Tab 查看页面间的引用关系
5. **刷新**：点击 🔄 刷新按钮重新加载目录和内容

## 未来扩展

- AI 驱动的 wiki 维护（通过聊天交互触发 Ingest/Query/Lint）
- 图片和附件支持
- 版本历史
- 多人协作编辑
