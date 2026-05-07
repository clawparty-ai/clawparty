# ZTM Filesystem (ztm fs) 架构设计文档

## 概述

ZTM Filesystem (ztm fs) 是 ClawParty/ZTM 系统中的分布式文件系统组件，提供跨多个 agent 节点的文件存储、同步和共享功能。它支持用户私有空间、共享空间和应用专属空间，通过 Hub 中心节点实现文件元数据和内容的同步。

## 核心架构组件

### 1. 本地文件系统层 (`agent/fs.js`)

**职责**：
- 管理本地文件存储
- 维护文件元数据（路径、哈希、大小、时间戳）
- 提供文件的增删改查操作

**数据结构**：
```javascript
pathMap: {
  "/path/to/file": {
    pathname: "/path/to/file",
    hash: "sha256_hash",
    size: 1024,
    time: 1723012345678
  }
}

hashMap: {
  "sha256_hash": { /* 同上 */ }
}
```

**存储格式**：
- 文件内容：`<storeDir>/<hash>`
- 元数据文件：`<storeDir>/<hash>.meta` (JSON格式)
- 元数据包含：`pathname`, `timestamp`, `deleted` (可选)

### 2. Mesh 网络集成层 (`agent/mesh.js`)

**职责**：
- 与 Hub 节点通信
- 管理文件系统广告和同步
- 处理跨节点文件操作

**关键功能**：
- `advertiseFilesystem()`: 向 Hub 广播本地文件列表
- `discoverFiles()`: 从 Hub 获取其他节点的文件列表
- `syncFile()`: 从其他节点同步文件内容
- `watchFile()`: 监听文件变更通知

### 3. Hub 服务端 (`hub/main.js`)

**职责**：
- 维护全局文件元数据
- 处理文件变更通知
- 管理访问控制列表 (ACL)
- 协调文件同步

**文件元数据结构**：
```javascript
files = {
  "/path/to/file": {
    "#": "sha256_hash",      // 文件内容哈希
    "$": 1024,               // 文件大小
    "T": 1723012345678,      // 文件时间戳
    "+": 1723012345679,      // 元数据更新时间
    "@": ["endpoint_id1", "endpoint_id2"]  // 拥有文件内容的端点列表
  }
}
```

## 文件系统目录结构

```
/users/<username>/                # 用户私有空间
/shared/<username>/               # 用户共享空间
/apps/<provider>/<appname>/users/<username>/   # 应用用户私有空间
/apps/<provider>/<appname>/shared/<username>/  # 应用用户共享空间
```

## 数据流与同步机制

### 1. 文件发布流程

```
Agent A (发布文件)
    │
    ├─ 1. fs.write(path, data) → 本地存储文件
    │
    ├─ 2. advertiseFilesystem() → 向 Hub 发送文件元数据
    │
    └─ 3. Hub.updateFileInfo() → 更新全局文件元数据
            │
            └─ 4. 广播变更通知 → 其他 Agent 收到通知
```

### 2. 文件同步流程

```
Agent B (需要同步文件)
    │
    ├─ 1. discoverFiles() → 从 Hub 获取文件列表
    │
    ├─ 2. findFile(path) → 从 Hub 获取文件详细信息
    │       │
    │       └─ 返回：哈希、大小、时间戳、源端点列表
    │
    ├─ 3. downloadFile(ep, hash) → 从源端点下载文件内容
    │       │
    │       └─ 通过 Hub 中继：GET /api/endpoints/{ep}/file-data/{hash}
    │
    └─ 4. fs.write(path, data, time) → 本地存储并保持时间戳
```

### 3. 实时变更通知机制

**SSE (Server-Sent Events) 管道**：
- `GET /api/filesystem/events?since=<timestamp>`: 轮询文件变更
- `GET /api/filesystem?since=<timestamp>&wait`: 长轮询等待变更

**Agent 端监听**：
- `startSSEClient()`: 启动轮询客户端
- `notifySSEWatchers()`: 通知应用层文件变更
- `fsWatchers`: 维护监听器列表

## API 接口规范

### Agent API (`/api/meshes/{mesh}/...`)

#### 文件操作

| 端点 | 方法 | 描述 | 参数 |
|------|------|------|------|
| `/api/meshes/{mesh}/files` | GET | 获取文件列表 | `since` (可选): 时间戳 |
| `/api/meshes/{mesh}/files/*` | GET | 获取文件元数据 | `*`: 文件路径 |
| `/api/meshes/{mesh}/files/*` | DELETE | 删除文件 | `*`: 文件路径 |
| `/api/meshes/{mesh}/file-data/*` | GET | 获取文件内容 | `*`: 文件路径 |
| `/api/meshes/{mesh}/file-data/*` | POST | 写入文件内容 | `*`: 文件路径 |
| `/api/meshes/{mesh}/file-data/*` | DELETE | 删除文件内容 | `*`: 文件路径 |
| `/api/meshes/{mesh}/endpoints/{ep}/file-data/{hash}` | GET | 从端点下载文件 | `ep`: 端点ID, `hash`: 文件哈希 |

#### 响应格式

**文件列表响应**：
```json
{
  "/path/to/file": {
    "T": 1723012345678,
    "#": "sha256_hash",
    "$": 1024,
    "+": 1723012345679
  }
}
```

**文件元数据响应**：
```json
{
  "#": "sha256_hash",
  "$": 1024,
  "T": 1723012345678,
  "+": 1723012345679,
  "@": ["endpoint_id1", "endpoint_id2"]
}
```

### Hub API (`/api/...`)

#### 文件系统管理

| 端点 | 方法 | 描述 | 认证 |
|------|------|------|------|
| `/api/filesystem` | GET | 获取全局文件列表 | 无 |
| `/api/filesystem` | POST | 更新文件元数据 | 需要端点会话 |
| `/api/filesystem/events` | GET | 文件变更轮询 | 无 |
| `/api/filesystem/*` | GET | 获取文件详细信息 | 无 |

#### 文件数据代理

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/endpoints/{ep}/file-data/{hash}` | GET | 从端点获取文件内容 |
| `/api/file-data/{hash}` | GET | 本地文件内容获取 (Agent内部) |

## 访问控制 (ACL)

### ACL 数据结构

```json
{
  "pathname": "/apps/provider/app/shared/username/file.txt",
  "prefix": "/apps/provider/app/shared/username/",
  "all": "readonly",
  "users": {
    "other_user": "readwrite"
  },
  "since": 1723012345678
}
```

### ACL 检查流程

1. `makeOwnerChecker(username)`: 检查用户是否拥有路径
2. `makeAccessChecker(username)`: 检查用户是否有访问权限
3. 路径前缀匹配：
   - `/users/{username}/`: 用户私有空间
   - `/shared/{username}/`: 用户共享空间
   - `/apps/{provider}/{app}/users/{username}/`: 应用用户私有空间
   - `/apps/{provider}/{app}/shared/{username}/`: 应用用户共享空间

## 文件哈希与验证

### 哈希计算

```javascript
function hash(path, data, size) {
  size = size || data.size
  var h = new crypto.Hash('sha256')
  h.update(data)
  h.update(size.toString())
  h.update(path)
  return h.digest().toString('hex')
}
```

**特点**：
- 使用 SHA-256 算法
- 包含文件内容、大小和路径
- 相同内容不同路径产生不同哈希
- 用于验证文件完整性

## 错误处理与恢复

### 下载失败处理

```javascript
function pickOne() {
  if (sources.length === 0) return null
  var i = Math.floor(Math.random() * sources.length) % sources.length
  var ep = sources.splice(i, 1)[0]
  return downloadFile(ep, hash).then(data => {
    if (!data) {
      logError(`Download of file ${hash} from ep ${ep} is null`)
      return pickOne()  // 尝试下一个源
    }
    if (fs.hash(pathname, data) !== hash) {
      logError(`Download of file ${hash} from ep ${ep} is corrupted`)
      return pickOne()  // 哈希验证失败，尝试下一个源
    }
    // 下载成功
    fs.write(pathname, data, time)
    advertiseFilesystem()
    return data
  }).catch(ret => {
    logError(`Download of file ${hash} from ep ${ep} failed: ${JSON.stringify(ret)}`)
    return pickOne()  // 网络错误，尝试下一个源
  })
}
```

### 文件删除与恢复

- `fs.remove()`: 从本地存储删除文件
- `fs.tombstone()`: 创建删除标记（软删除）
- 删除标记保留元数据，允许其他节点检测删除操作

## 性能优化

### 1. 缓存机制

- `fileList`: 缓存文件列表，避免重复计算
- `hubCache`: 缓存 Hub 连接信息
- `pathMap` 和 `hashMap`: 内存缓存文件元数据

### 2. 增量同步

- `since` 参数支持增量获取文件变更
- 时间戳比较避免不必要的数据传输
- 哈希比较避免重复下载相同文件

### 3. 连接复用

- HTTP/2 多路复用
- 长连接保持
- 连接池管理

## 部署与配置

### 存储目录结构

```
~/.clawparty/
├── ztm.db                    # SQLite 数据库
├── fs/                       # 文件系统存储
│   ├── <hash>               # 文件内容
│   ├── <hash>.meta          # 文件元数据
│   └── ...
├── meshes/                   # Mesh 配置
│   └── <mesh_name>/
│       ├── fs/              # 该 Mesh 的文件系统
│       └── apps/            # 应用存储
└── ...
```

### 关键配置参数

- `--data <dir>`: 数据存储目录
- `--listen <[ip:]port>`: 监听地址
- `--api-token <token>`: API 认证令牌
- `--enable-p2p`: 启用 P2P 直连

## 监控与调试

### 日志记录

- 文件同步操作日志
- 下载失败错误日志
- ACL 检查日志
- SSE 连接状态日志

### 调试端点

- `/api/meshes/{mesh}/log`: Agent 日志
- `/api/meshes/{mesh}/endpoints/{ep}/log`: 远程端点日志
- `/api/meshes/{mesh}/hubs/{id}/log`: Hub 日志

## 扩展性考虑

### 水平扩展

- 支持多个 Hub 节点
- 文件内容多副本存储
- 基于哈希的内容寻址

### 应用集成

- 应用专属文件系统命名空间
- 应用可以管理自己的文件存储
- 支持应用间文件共享

## 安全考虑

### 数据完整性

- SHA-256 哈希验证
- 下载后完整性检查
- 元数据与内容分离

### 访问控制

- 基于路径的 ACL
- 用户身份验证
- 端点会话管理

### 传输安全

- TLS 加密通信
- 证书认证
- 可选 P2P 直连加密

## 总结

ZTM Filesystem 是一个分布式文件系统，通过 Hub 中心节点协调多个 Agent 之间的文件同步。它提供了完整的文件操作 API，支持实时变更通知，具有强大的错误恢复机制和访问控制能力。系统设计考虑了性能、扩展性和安全性，适合在分布式环境中使用。

### 主要特性

1. **分布式架构**：支持多节点文件同步
2. **内容寻址**：基于 SHA-256 哈希的文件标识
3. **实时同步**：SSE 和长轮询机制
4. **访问控制**：基于路径的 ACL 系统
5. **错误恢复**：多源下载和完整性验证
6. **应用隔离**：支持应用专属命名空间

### 典型使用场景

1. **用户文件共享**：跨设备文件同步
2. **应用数据存储**：应用状态和配置存储
3. **分布式协作**：多人协作文件编辑
4. **内容分发**：文件多副本分发

### 技术优势

1. **高性能**：内存缓存和增量同步
2. **高可用**：多副本和错误恢复
3. **可扩展**：支持水平扩展
4. **安全**：TLS 加密和访问控制
