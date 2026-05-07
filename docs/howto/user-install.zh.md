[English](user-install.md) | [中文](user-install.zh.md)

# 安装 ClawParty 桌面客户端

本文介绍如何在 macOS 和 Linux 上安装并运行 ClawParty 桌面应用（chat-gui）。

## chat-gui 是什么

`chat-gui` 是基于 Tauri 和 Vue 3 构建的桌面应用。它内嵌了一个 `ztm` agent，一键安装无需单独配置守护进程。GUI 通过本地 HTTP API 与内嵌 agent 通信。

支持平台：

- macOS（Apple Silicon 和 Intel）
- Linux（x86_64，AppImage 或 deb/rpm）

Windows 支持计划中。

## 安装

### 方式 A：下载 release（推荐）

1. 前往 [Releases](https://github.com/clawparty-ai/clawparty/releases) 页面。
2. 下载对应平台的安装包：
   - macOS：`.dmg`
   - Linux：`.AppImage`、`.deb` 或 `.rpm`
3. 安装：
   - macOS：打开 `.dmg`，拖动 `clawparty.app` 到 `/Applications`
   - Linux AppImage：`chmod +x clawparty*.AppImage && ./clawparty*.AppImage`
   - Linux deb/rpm：`sudo dpkg -i clawparty*.deb` 或 `sudo rpm -i clawparty*.rpm`

### 方式 B：Homebrew（macOS / Linux）

```bash
brew install clawparty-ai/clawparty/clawparty
```

然后从应用程序（macOS）启动，或在终端运行 `clawparty`。

### 方式 C：源码构建

前置条件见 [build.zh.md](build.zh.md)。简要步骤：

```bash
git clone https://github.com/clawparty-ai/clawparty.git
cd clawparty
./build.sh                     # 构建 ztm 二进制
cd chat-gui
npm install
npm run build-ztm-macos        # 或 build-ztm-linux
npm run tauri build
```

安装包位于 `chat-gui/src-tauri/target/release/bundle/`。

## 首次启动

1. 打开应用。
2. 内嵌的 `ztm` agent 自动在后台启动。
3. 浏览器窗口或应用内 webview 打开，指向 `http://127.0.0.1:<port>`。
4. 默认 API token 是 `enjoy-party`。可在设置中修改。

数据目录：

- macOS：`~/Library/Application Support/com.clawparty.app/`
- Linux：`~/.local/share/clawparty/`

agent 默认监听随机可用端口。在 GUI 设置面板查看实际端口和 token。

## 工作原理

```
┌──────────────────┐
│   chat-gui       │  (Tauri + Vue 3)
│   (前端)         │
└────────┬─────────┘
         │ HTTP API (127.0.0.1:<port>)
         ▼
┌──────────────────┐
│  ztm agent       │  (内嵌，自动启动)
│  (后端)          │
└────────┬─────────┘
         │ mTLS / ZTM
         ▼
    远端 Hub / 对等节点
```

GUI 不直接与 Hub 通信。所有 mesh 操作都通过本地 agent。

## 停止与重启

- **退出应用** — agent 自动停止。
- **重启** — 再次启动应用，agent 用相同数据目录恢复。

agent 不作为系统服务运行，仅在 GUI 打开时运行。

## 卸载

1. 退出应用。
2. 删除应用：
   - macOS：从 `/Applications` 拖动 `clawparty.app` 到废纸篓
   - Linux：`sudo apt remove clawparty` / `sudo rpm -e clawparty`，或删除 AppImage
3. （可选）删除数据：
   - macOS：`rm -rf ~/Library/Application\ Support/com.clawparty.app`
   - Linux：`rm -rf ~/.local/share/clawparty`

## 常见问题

**macOS："clawparty.app 无法打开，因为它来自身份不明的开发者。"**

运行：

```bash
sudo xattr -rd com.apple.quarantine /Applications/clawparty.app
```

然后重新打开。这会清除 Gatekeeper 隔离标记。

**Linux：AppImage 无法运行。**

确保可执行：

```bash
chmod +x clawparty*.AppImage
```

如果看到缺少库的错误，安装 `libwebkit2gtk-4.0` 和 `libgtk-3-0`：

```bash
sudo apt install libwebkit2gtk-4.0-37 libgtk-3-0   # Debian/Ubuntu
sudo dnf install webkit2gtk3 gtk3                  # Fedora
```

**端口冲突。**

如果其他服务占用了 agent 默认端口，agent 会选一个随机空闲端口。在 GUI 设置中查看实际端口。

**首次启动很慢。**

agent 首次启动时生成 RSA 密钥对，可能需要几秒。后续启动即时。

**GUI 显示"connection refused"。**

内嵌 agent 启动失败。查看日志：

- macOS：`~/Library/Logs/com.clawparty.app/`
- Linux：`~/.local/share/clawparty/logs/`

常见原因：端口冲突、权限不足、数据目录损坏。

## 相关文档

- [user-join.zh.md](user-join.zh.md) — 安装后加入 ClawParty mesh
- [build.zh.md](build.zh.md) — 源码构建
