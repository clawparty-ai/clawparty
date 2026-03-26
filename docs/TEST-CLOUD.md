# Cloud App Test Plan

## 概述

Cloud App 是 ClawParty 的分布式文件云存储应用，支持跨端点的文件同步、共享和管理。本文档记录了 Cloud App 的测试方案和测试案例。

## 测试范围

### 1. 基础功能测试
- 端点管理
- 配置管理
- 文件列表
- 文件状态查询

### 2. 文件上传测试
- 单文件上传
- 大文件分块上传
- 重复文件上传

### 3. 文件下载测试
- 单文件下载
- 分块下载
- 并发下载
- 下载恢复
- 下载取消

### 4. 文件管理测试
- 文件删除（unpublish）
- 文件擦除（erase）
- 目录操作

### 5. 访问控制测试
- ACL 设置
- 权限验证
- 跨用户访问

### 6. 自动同步测试
- 自动下载
- 自动上传
- 镜像配置

### 7. CLI 命令测试
- config 命令
- ls 命令
- share 命令
- mirror 命令
- upload/download 命令
- unpublish/erase 命令

---

## 测试案例

### TC-CL-001: 端点配置管理

**前置条件**: Cloud App 已启动

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud config --local-dir ~/testCloud` | 配置成功更新 |
| 2 | 执行 `ztm cloud config` | 显示 localDir 为 ~/testCloud |
| 3 | 调用 GET /api/endpoints/{ep}/config | 返回当前配置 |

---

### TC-CL-002: 文件列表查询

**前置条件**: 存在测试文件

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud ls /` | 显示用户列表 |
| 2 | 执行 `ztm cloud ls /users/{username}` | 显示该用户的文件列表 |
| 3 | 执行 `ztm cloud ls /users/{username} --hash` | 显示文件信息和哈希值 |
| 4 | 调用 GET /api/files/ | 返回根目录信息 |
| 5 | 调用 GET /api/files/users/{username} | 返回用户文件列表 |

---

### TC-CL-003: 文件上传

**前置条件**: 本地存在待上传文件

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud upload /users/{username}/test.txt` | 文件上传成功 |
| 2 | 调用 POST /api/uploads {path: "path"} | 返回 201 |
| 3 | 上传后执行 `ztm cloud ls` | 文件状态变为 synced |
| 4 | 重复上传同一文件 | 返回成功（幂等） |

---

### TC-CL-004: 文件下载

**前置条件**: 远端存在可下载文件

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud download /users/{username}/test.txt` | 下载任务启动 |
| 2 | 执行 `ztm cloud download --list` | 显示下载进度 |
| 3 | 调用 GET /api/downloads | 返回下载列表 |
| 4 | 下载完成后检查本地文件 | 文件完整，哈希匹配 |

---

### TC-CL-005: 分块下载与恢复

**前置条件**: 大文件（>1MB）存在分块

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 开始下载大文件 | 分块下载启动 |
| 2 | 中断下载（模拟网络中断） | 下载暂停 |
| 3 | 重新启动 Cloud App | 自动恢复下载 |
| 4 | 执行 `ztm cloud download --list` | 显示下载进度继续 |

---

### TC-CL-006: 下载取消

**前置条件**: 存在进行中的下载

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 开始下载大文件 | 下载任务启动 |
| 2 | 执行 `ztm cloud download --cancel /path/to/file` | 下载任务取消 |
| 3 | 执行 `ztm cloud download --list` | 该下载不再显示 |
| 4 | 调用 DELETE /api/downloads/{path} | 返回 204 |

---

### TC-CL-007: 文件删除（Unpublish）

**前置条件**: 文件已上传并同步

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud unpublish /users/{username}/test.txt` | 文件从云端删除 |
| 2 | 再次执行 `ztm cloud ls` | 文件不再显示 |
| 3 | 本地文件保留 | 本地文件不受影响 |
| 4 | 调用 DELETE /api/files/{path} | 返回 204 |

---

### TC-CL-008: 文件擦除（Erase）

**前置条件**: 文件已上传

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud erase /users/{username}/test.txt` | 本地文件删除 |
| 2 | 调用 DELETE /api/endpoints/{ep}/files/{path} | 返回 204 |
| 3 | 云端元数据保留 | 其他端点仍可下载 |

---

### TC-CL-009: ACL 权限设置

**前置条件**: 文件已上传

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud share /path --set-all readonly` | 所有用户可读 |
| 2 | 执行 `ztm cloud share /path --set-block user1` | user1 被阻止访问 |
| 3 | 执行 `ztm cloud share /path --set-readonly user2` | user2 只读访问 |
| 4 | 执行 `ztm cloud share /path` | 显示当前 ACL 配置 |
| 5 | 调用 POST /api/acl/{path} | ACL 设置成功 |

---

### TC-CL-010: ACL 权限验证

**前置条件**: 已设置 ACL

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 被阻止的用户尝试下载文件 | 返回 403 或无法访问 |
| 2 | 只读用户尝试下载文件 | 下载成功 |
| 3 | 只读用户尝试上传文件 | 上传被拒绝 |
| 4 | 调用 GET /api/acl/{path} | 返回 ACL 配置 |

---

### TC-CL-011: 自动下载配置

**前置条件**: 无

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud mirror /users/remoteUser --download on` | 配置自动下载 |
| 2 | 远端用户上传新文件 | 自动开始下载 |
| 3 | 执行 `ztm cloud mirror /users/remoteUser` | 显示 download: on |
| 4 | 执行 `ztm cloud mirror /users/remoteUser --download off` | 关闭自动下载 |

---

### TC-CL-012: 自动上传配置

**前置条件**: 无

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud mirror /users/{username} --upload on` | 配置自动上传 |
| 2 | 本地文件修改 | 自动开始上传 |
| 3 | 执行 `ztm cloud mirror /users/{username}` | 显示 upload: on |
| 4 | 执行 `ztm cloud mirror /users/{username} --upload off` | 关闭自动上传 |

---

### TC-CL-013: 镜像配置查看

**前置条件**: 已配置镜像

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 调用 GET /api/mirrors | 返回所有镜像配置 |
| 2 | 调用 GET /api/mirrors/{path} | 返回指定路径的镜像配置 |
| 3 | 调用 GET /api/endpoints/{ep}/mirrors | 返回指定端点的镜像配置 |

---

### TC-CL-014: 文件流式传输

**前置条件**: 文件已上传

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 调用 GET /api/file-data/{path} | 返回文件流 |
| 2 | 验证 Content-Type | 根据扩展名正确设置 |
| 3 | 验证文件完整性 | 下载的文件哈希匹配 |

---

### TC-CL-015: 分块服务

**前置条件**: 文件有分块

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 调用 GET /api/chunks/users/{username}/{file}?chunk=0 | 返回第一块数据 |
| 2 | 调用 GET /api/chunks/users/{username}/{file}?chunk=1 | 返回第二块数据 |
| 3 | 验证分块哈希 | 分块哈希与元数据匹配 |

---

### TC-CL-016: 并发下载

**前置条件**: 大文件需要分块下载

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 开始下载大文件 | 启动 8 个并发下载通道 |
| 2 | 监控下载进度 | 多个分块同时下载 |
| 3 | 下载完成 | 所有分块合并，文件完整 |

---

### TC-CL-017: 下载输出到指定文件

**前置条件**: 远端存在文件

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud download /remote/path -o /local/output.txt` | 文件保存到指定位置 |
| 2 | 验证输出文件 | 文件内容和哈希正确 |

---

### TC-CL-018: 目录同步

**前置条件**: 目录存在

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 执行 `ztm cloud mirror /dir --download on --upload on` | 双向同步配置 |
| 2 | 本地文件修改 | 自动上传到云端 |
| 3 | 远端文件修改 | 自动下载到本地 |
| 4 | 验证文件状态 | 状态为 synced |

---

### TC-CL-019: 跨端点配置同步

**前置条件**: 多端点环境

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 调用 GET /api/endpoints/{ep}/config | 返回端点配置 |
| 2 | 调用 POST /api/endpoints/{ep}/config | 配置更新成功 |
| 3 | 验证配置应用 | 新配置生效 |

---

### TC-CL-020: 错误处理

**前置条件**: 无

| 步骤 | 操作 | 预期结果 |
|------|------|----------|
| 1 | 访问不存在的文件 | 返回 404 |
| 2 | 无权限访问文件 | 返回 403 |
| 3 | 无效的 API 调用 | 返回适当的错误码 |
| 4 | 下载过程中源端点下线 | 自动尝试其他源 |

---

## 测试环境要求

### 硬件要求
- 至少 2 个端点（模拟分布式环境）
- 足够的磁盘空间用于文件测试

### 软件要求
- ClawParty 运行环境
- ZTM 网络连接

### 测试数据
- 小文件 (<1MB)
- 大文件 (>10MB，用于分块测试)
- 多个文件（用于目录测试）

---

## 测试执行指南

### 单元测试
- 测试各个 API 函数的输入输出
- 验证文件状态计算逻辑
- 测试哈希计算

### 集成测试
- 测试 CLI 命令到 API 的调用链
- 测试跨端点文件同步
- 测试 ACL 权限验证

### 端到端测试
- 完整的文件上传下载流程
- 自动同步场景
- 错误恢复场景

---

## 已知问题和注意事项

1. **分块大小**: 默认为 1MB，修改需要重新编译
2. **并发数**: 默认 8 个并发下载通道
3. **路径处理**: 使用 `os.path.normalize` 统一路径格式
4. **ACL 继承**: 目录 ACL 不自动继承到子文件
