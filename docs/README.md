# ClawParty 文档索引

本目录采用如下约定：

- 文件命名：全小写 + 连字符
- 分类：通过子目录承担（`howto/`、`design/`、`reference/`）
- 中英文双语：英文为基础名 `name.md`，中文加后缀 `name.zh.md`，两个文件成对出现

```
docs/
├── howto/        操作指南（用户视角）
├── design/       设计与架构文档
├── reference/    参考资料：API、CLI、配置、安全、踩坑
└── archive/      历史/阶段性文档（命名风格不统一、可能过时，仅作参考）
```

> 早期文档因风格不一（部分英文、部分中文、未成对）已统一归档至 [archive/](archive/)。新文档请按上述命名规则生成中英双版本。

## How To（操作指南）

| 文档 | 说明 |
|------|------|
| [howto/admin-hub.md](howto/admin-hub.md) / [.zh](howto/admin-hub.zh.md) | 管理员部署与管理 Hub |
| [howto/admin-users.md](howto/admin-users.md) / [.zh](howto/admin-users.zh.md) | 管理员邀请用户与用户生命周期管理 |
| [howto/user-install.md](howto/user-install.md) / [.zh](howto/user-install.zh.md) | 用户安装桌面客户端 |
| [howto/user-join.md](howto/user-join.md) / [.zh](howto/user-join.zh.md) | 用户加入 ClawParty mesh |

## Archive（归档）

以下为早期单语版本与开发阶段快照，仅作历史参考。最新信息请以代码、`tests/` 目录与 commit 历史为准。

### archive/howto/

| 文件 | 主语言 | 说明 |
|------|--------|------|
| [build.md](archive/howto/build.md) | EN | 构建指南 |
| [user.md](archive/howto/user.md) | 中文 | 用户手册 |
| [chat.md](archive/howto/chat.md) | EN | Chat App 使用 |
| [cloud.md](archive/howto/cloud.md) | EN | Cloud App 使用 |
| [tunnel.md](archive/howto/tunnel.md) | EN | Tunnel App 使用 |
| [proxy.md](archive/howto/proxy.md) | EN | Proxy App 使用 |
| [payment.md](archive/howto/payment.md) | EN | Payment App 使用 |
| [hub.md](archive/howto/hub.md) | 中文 | Hub 使用 |
| [tui.md](archive/howto/tui.md) | EN | TUI 使用 |
| [pqc.md](archive/howto/pqc.md) | EN | 后量子密码学 |

### archive/design/

| 文件 | 主语言 | 说明 |
|------|--------|------|
| [architecture-concepts.md](archive/design/architecture-concepts.md) | EN | 整体架构概念 |
| [agent-lifecycle.md](archive/design/agent-lifecycle.md) | 中文 | Agent 生命周期管理 |
| [chat-filter-chain.md](archive/design/chat-filter-chain.md) | EN | 聊天过滤链设计 |
| [payment.md](archive/design/payment.md) | EN | 支付功能设计 |
| [ztm-filesystem.md](archive/design/ztm-filesystem.md) | 中文 | ZTM 文件系统设计 |
| [hub-config-auto-setup.md](archive/design/hub-config-auto-setup.md) | 中文 | Hub 配置自动获取与 0#Agent 自动创建 |

### archive/reference/

| 文件 | 主语言 | 说明 |
|------|--------|------|
| [api-agent.md](archive/reference/api-agent.md) | EN | Agent API 参考 |
| [cli-overview.md](archive/reference/cli-overview.md) | EN | CLI 总览 |
| [cli-examples.md](archive/reference/cli-examples.md) | 中文 | CLI 命令大全与示例 |
| [hub-llm-config.md](archive/reference/hub-llm-config.md) | 中文 | Hub LLM 配置参考 |
| [security-credentials.md](archive/reference/security-credentials.md) | 中文 | 凭证与安全 |
| [pitfalls.md](archive/reference/pitfalls.md) | 中文 | 常见踩坑与解决方案 |

### 其他归档

ZeroClaw 集成阶段快照：
- [zeroclaw-implementation.md](archive/zeroclaw-implementation.md)
- [zeroclaw-integration-complete.md](archive/zeroclaw-integration-complete.md)
- [zeroclaw-progress.md](archive/zeroclaw-progress.md)
- [zeroclaw-webui-integration.md](archive/zeroclaw-webui-integration.md)

旧测试方案：
- [test-chat.md](archive/test-chat.md)
- [test-cloud.md](archive/test-cloud.md)
- [test-tunnel.md](archive/test-tunnel.md)
- [test-proxy.md](archive/test-proxy.md)
- [test-payment.md](archive/test-payment.md)
- [test-user.md](archive/test-user.md)
- [test-hub-auto-config.md](archive/test-hub-auto-config.md)

## 相关子项目文档

- [chat-gui/README.md](../chat-gui/README.md) - 聊天 GUI（Vue 3 + Tauri）
- [agent/apps/picoclaw/README.md](../agent/apps/picoclaw/README.md) - PicoClaw
- [chat-gui/src-ios/ios.md](../chat-gui/src-ios/ios.md) - iOS 端说明

## 新增文档规则

1. 选定主目录（`howto/` / `design/` / `reference/`），文件名全小写连字符
2. 同时创建英文版 `name.md` 与中文版 `name.zh.md`，内容对等
3. 两版本文件顶部互相添加链接：英文版指向 `[中文](name.zh.md)`，中文版指向 `[English](name.md)`
4. 在本 README 索引中加入条目
5. 若涉及代码示例或命令，两份文件保持完全一致；仅自然语言部分翻译
