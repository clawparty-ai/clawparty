# ClawParty Desktop

macOS 菜单栏应用，用于管理 ClawParty 服务、查看实时日志，以及为每个 Agent 单独配置 LLM。

## 功能

### 系统托盘
- 常驻菜单栏，随时查看和控制服务状态
- 点击龙虾图标展开菜单，快速启停服务

### 主面板
- **状态显示**: 实时显示 ClawParty 运行状态（绿色=运行中，灰色=已停止）
- **启停控制**: 启动 / 停止 ClawParty 主程序
- **Agent 列表**: 从 `~/.clawparty/clawparty.db` 读取所有 Agent，显示名称、状态
- **实时日志**: 
  - 显示 ClawParty 的 stdout/stderr 输出
  - 带时间戳和级别（INFO/ERROR/WARN）
  - 自动滚动，支持手动开关
  - 支持清空日志
- **LLM 配置**: 点击按钮打开 Agent LLM 配置窗口

### 进程检测
- **启动时自动检测**: 检查系统中是否已有 Clawparty 在运行
- **双检测机制**: 
  - 检查进程名（`ps aux | grep clawparty`）
  - 检查端口（`lsof -i :7778`）
- **外部进程管理**: 即使 ClawParty 由其他方式启动（终端、脚本等），也能检测到并可以停止

### Agent LLM 配置
- 从 `clawparty.db` 读取 Agent 列表
- **为每个 Agent 单独配置 LLM 参数**:
  - Provider（OpenAI、OpenRouter、Anthropic 等）
  - Model（预设列表或自定义输入）
  - API Key
  - API URL（可选）
  - Temperature（0.0 - 2.0）
  - Timeout
- 保存到每个 Agent 的独立 `config.toml`

## 项目结构

```
desktop/
├── ClawPartyDesktop/
│   ├── ClawPartyDesktopApp.swift   # 应用入口
│   ├── StatusBarController.swift   # 菜单栏控制器
│   ├── ProcessManager.swift        # 进程管理 + 日志 + 进程检测
│   ├── ConfigManager.swift         # Agent 配置读写 + SQLite 读取
│   ├── LLMConfigView.swift         # LLM 配置界面
│   ├── MainPanelView.swift         # 主面板（Agent列表 + 日志）
│   ├── ContentView.swift           # 视图入口
│   └── Info.plist                  # 应用信息
├── build/
│   └── ClawPartyDesktop.app        # 编译好的 .app 包
├── build.sh                        # 构建脚本
└── README.md                       # 本文件
```

## 系统要求

- macOS 13.0+
- Swift 5.9+
- ClawParty 项目已克隆并编译 (`../bin/clawparty` 存在)

## 构建

```bash
cd desktop
chmod +x build.sh
./build.sh
```

构建完成后，应用位于 `build/ClawPartyDesktop.app`。

## 安装

### 方式一：直接运行

```bash
open build/ClawPartyDesktop.app
```

> **⚠️ 安全提示**: macOS 可能会阻止运行未签名的应用。如果提示"无法验证开发者"，请执行：
> ```bash
> xattr -cr build/ClawPartyDesktop.app
> ```
> 或在 **系统设置 > 隐私与安全性** 中点击 **"仍要打开"**。

### 方式二：安装到 Applications

```bash
cp -r build/ClawPartyDesktop.app /Applications/
```

## 使用

### 菜单栏
1. 启动后，菜单栏会出现一个龙虾图标 🦞
2. 点击图标展开菜单:
   - **打开面板** (`Cmd+O`) - 打开主窗口
   - **启动/停止 ClawParty** - 控制主程序
   - **Agent LLM 配置...** (`Cmd+,`) - 打开配置窗口
   - **退出** (`Cmd+Q`) - 关闭应用并停止服务

### 主面板
1. 点击菜单栏的 **"打开面板"** 或按 `Cmd+O`
2. 面板分为左右两部分:
   - **左侧**: Agent 列表（自动刷新）
   - **右侧**: 实时日志（带时间戳、级别、自动滚动）
3. 点击 **"启动"** / **"停止"** 控制 ClawParty
4. 点击 **"LLM 配置"** 打开配置窗口

### 日志
- 日志区域显示 ClawParty 的所有输出
- 每行包含：`[时间] [级别] 消息`
- 级别颜色：INFO=绿色, ERROR=红色, WARN=橙色, DEBUG=蓝色
- 勾选"自动滚动"保持查看最新日志
- 点击垃圾桶图标清空日志

### 进程检测
- 应用启动时会自动检测是否已有 ClawParty 在运行
- 即使 ClawParty 是由终端或其他方式启动的，也能检测到
- 停止按钮可以终止外部启动的 ClawParty 进程

### Agent LLM 配置
1. 点击 **"Agent LLM 配置..."** 打开配置窗口
2. 下拉选择 Agent（从数据库读取）
3. 配置 LLM 参数后点击 **"保存配置"**
4. 配置会保存到对应 Agent 的 `config.toml`

## 配置文件

应用会自动读取和写入以下配置:

- `~/.clawparty/clawparty.db` - SQLite 数据库，读取 agents 列表
- `~/.clawparty/agents/{agent_name}/config.toml` - 每个 Agent 的独立配置文件

## 注意事项

### 安全与签名

由于这是本地开发构建的应用，没有使用 Apple Developer ID 证书签名，macOS Gatekeeper 可能会阻止运行。解决方法：

1. **构建脚本已自动移除隔离属性**（`xattr -cr`）
2. 如果仍被阻止，执行：
   ```bash
   xattr -cr build/ClawPartyDesktop.app
   ```
3. 或在 **系统设置 > 隐私与安全性 > 安全性** 中点击 **"仍要打开"**

### 其他

- 应用使用 `LSUIElement` 模式，不会在 Dock 显示图标
- 退出应用时会自动停止所有已启动的服务
- 日志输出实时更新，最多保留 1000 条记录

## 自定义二进制路径

如果 ClawParty 项目不在默认位置，请修改 `ProcessManager.swift` 中的路径:

```swift
let repoRoot = "/Users/caishu/github/clawparty"
```

## 许可证

MIT
