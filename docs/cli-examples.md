# clawparty CLI 命令大全

本文档提供了 clawparty CLI 所有命令的详细说明和使用示例。

## 目录

- [基础命令](#基础命令)
- [服务管理](#服务管理)
- [网格管理](#网格管理)
- [查询命令](#查询命令)
- [文件管理](#文件管理)
- [日志与监控](#日志与监控)
- [应用调用](#应用调用)
- [上下文前缀](#上下文前缀)
- [特殊服务测试](#特殊服务测试)
- [完整示例](#完整示例)

## 基础命令

### `version` - 打印版本信息

```sh
# 显示基本信息
clawparty version

# 输出 JSON 格式
clawparty version --json
```

### `completion` - 生成 Shell 补全脚本

```sh
# 生成 bash 补全脚本
clawparty completion bash

# 生成 zsh 补全脚本
clawparty completion zsh
```

### `config` - 查看或设置配置

```sh
# 查看当前配置
clawparty config

# 设置 Agent 地址
clawparty config --agent 192.168.1.100:7777

# 设置默认网格
clawparty config --mesh my-mesh
```

### `ca` - CA 证书管理

```sh
# 导入 CA 证书
clawparty ca import --data ~/.clawparty --cert ca.crt --key ca.key

# 导出 CA 证书
clawparty ca export --data ~/.clawparty --cert ca.crt --key ca.key
```

### `root` - 生成 root 用户许可

```sh
# 生成 root 许可文件
clawparty root --names hub.example.com:8888 --permit root.json

# 使用外部 CA 服务
clawparty root --names hub.example.com:8888 --ca https://ca.example.com:9999

# 使用 PQC 签名算法
clawparty root --names hub.example.com:8888 --pqc-signature ML-DSA-44
```

### `identity` - 打印当前身份

```sh
# 获取当前 Agent 的公钥
clawparty identity

# 保存到文件
clawparty identity > pub.key
```

### `label` - 管理 endpoint 标签

```sh
# 查看当前标签
clawparty label

# 添加标签
clawparty label --add production us-east-1

# 删除标签
clawparty label --delete us-east-1
```

## 服务管理

### `start` - 启动后台服务

```sh
# 启动 Hub
clawparty start hub --listen 0.0.0.0:8888 --names hub.example.com:8888

# 启动 Agent
clawparty start agent --listen 0.0.0.0:7777

# 启动特定应用
clawparty start app tunnel

# 使用高级选项启动 Hub
clawparty start hub \
  --listen 0.0.0.0:8888 \
  --names hub.example.com:8888 \
  --max-agents 1000 \
  --max-sessions 5000 \
  --pqc-key-exchange ML-KEM-512
```

### `stop` - 停止后台服务

```sh
# 停止 Hub
clawparty stop hub

# 停止 Agent
clawparty stop agent

# 停止应用
clawparty stop app tunnel
```

### `run` - 以前台模式运行

```sh
# 以前台模式运行 Hub
clawparty run hub --listen 0.0.0.0:8888 --names hub.example.com:8888

# 以前台模式运行 Agent
clawparty run agent --listen 127.0.0.1:7777

# 运行 Agent 并自动加入网格
clawparty run agent --listen 127.0.0.1:7777 --join my-mesh --join-as ep-1 --permit root.json

# 启用 P2P 连接
clawparty run agent --enable-p2p --p2p-port 17778
```

## 网格管理

### `invite` - 发放许可

```sh
# 为用户生成许可
clawparty invite test-user --identity pub.key --permit test.json

# 输出到标准输出
clawparty invite test-user --identity pub.key
```

### `evict` - 撤销许可

```sh
# 撤销用户许可
clawparty evict test-user
```

### `join` - 加入网格

```sh
# 使用许可文件加入网格
clawparty join my-mesh --as ep-1 --permit root.json

# 简短形式
clawparty join my-mesh --as ep-1 -p root.json
```

### `leave` - 离开网格

```sh
# 离开网格
clawparty leave my-mesh
```

### `enable` - 启用网格或应用

```sh
# 启用网格
clawparty enable mesh my-mesh

# 使用新许可启用网格
clawparty enable mesh my-mesh --permit new-permit.json

# 启用应用
clawparty enable app tunnel
```

### `disable` - 禁用网格或应用

```sh
# 禁用网格
clawparty disable mesh my-mesh

# 禁用网格并擦除许可
clawparty disable mesh my-mesh --erase-permit

# 禁用应用
clawparty disable app tunnel
```

### `attach` - 连接到 Hub

```sh
# 连接到指定的 Hub
clawparty attach hub.example.com:8888
```

## 查询命令

### `get` - 列出对象

```sh
# 列出所有网格
clawparty get mesh

# 列出所有 Hub
clawparty get hub

# 列出所有 endpoint
clawparty get ep

# 列出特定网格的 endpoint
clawparty mesh my-mesh get ep

# 列出所有文件
clawparty get file

# 显示文件哈希
clawparty get file --hash

# 列出所有应用
clawparty get app

# 列出特定应用
clawparty get app tunnel
```

### `describe` - 查看详细信息

```sh
# 查看网格详情
clawparty describe mesh my-mesh

# 查看 Hub 详情
clawparty describe hub hub.example.com:8888

# 查看 endpoint 详情
clawparty describe ep ep-1

# 查看文件详情
clawparty describe file /path/to/file

# 查看应用详情
clawparty describe app tunnel
```

## 文件管理

### `download` - 下载应用或文件

```sh
# 下载应用
clawparty download app tunnel

# 下载文件
clawparty download file /path/to/file

# 下载文件到指定位置
clawparty download file /path/to/file -o ./local-file.txt
```

### `erase` - 删除应用或文件

```sh
# 删除应用
clawparty erase app tunnel

# 删除文件
clawparty erase file /path/to/file
```

### `publish` - 发布应用或文件

```sh
# 发布应用
clawparty publish app tunnel

# 发布文件
clawparty publish file /path/to/file --input ./local-file.txt

# 从标准输入发布
cat data.txt | clawparty publish file /data/input.txt --input -
```

### `unpublish` - 取消发布

```sh
# 取消发布应用
clawparty unpublish app tunnel

# 取消发布文件
clawparty unpublish file /path/to/file
```

## 日志与监控

### `log` - 查看日志

```sh
# 查看应用日志
clawparty log app tunnel

# 查看 endpoint 日志
clawparty log ep ep-1

# 查看 Hub 日志
clawparty log hub hub.example.com:8888
```

### `stats` - 查看统计信息

```sh
# 查看所有 endpoint 统计
clawparty stats ep

# 查看特定 endpoint 统计
clawparty stats ep ep-1
```

### `ping` - 发送 ping 测试

```sh
# ping 所有 endpoint
clawparty ping ep

# ping 特定 endpoint
clawparty ping ep ep-1
```

## 应用调用

clawparty 应用作为独立命令调用，而不是通过 `clawparty app` 子命令。

### 内置应用

```sh
# Tunnel 应用 - 建立安全隧道
clawparty tunnel help
clawparty tunnel get outbound
clawparty tunnel open outbound tcp/my-tunnel --targets 192.168.1.10:80
clawparty tunnel open inbound tcp/my-tunnel --listen 8080

# Proxy 应用 - SOCKS/HTTP 代理
clawparty proxy help
clawparty proxy config --set-listen 0.0.0.0:1080
clawparty proxy config --add-target 0.0.0.0/0 '*'

# Terminal 应用 - 远程终端
clawparty terminal help
clawparty terminal config --add-user root
clawparty terminal open ep-2

# Script 应用 - 远程脚本执行
clawparty script help
clawparty script hello.js

# Cloud 应用 - 分布式文件共享（如果可用）
clawparty cloud help
clawparty cloud ls /users/root
clawparty cloud upload /users/root/file.txt
clawparty cloud download /users/root/file.txt
```

### 应用管理命令

```sh
# 启动应用
clawparty start app tunnel

# 停止应用
clawparty stop app tunnel

# 启用应用
clawparty enable app tunnel

# 禁用应用
clawparty disable app tunnel

# 查看应用列表
clawparty get app

# 查看应用详情
clawparty describe app tunnel

# 下载应用
clawparty download app tunnel

# 删除应用
clawparty erase app tunnel

# 发布应用
clawparty publish app tunnel

# 取消发布应用
clawparty unpublish app tunnel

# 查看应用日志
clawparty log app tunnel
```

## 上下文前缀

### `clawparty ep` - 在特定 endpoint 上执行

```sh
# 在特定 endpoint 上执行命令
clawparty ep ep-1 get file

# 在特定网格的 endpoint 上执行
clawparty ep my-mesh/ep-1 tunnel help

# 在本地 endpoint 上执行
clawparty ep local tunnel get outbound
```

### `clawparty mesh` - 在特定网格上执行

```sh
# 在特定网格上执行命令
clawparty mesh my-mesh get ep

# 在特定网格上执行应用
clawparty mesh my-mesh tunnel help
```

## 特殊服务测试

### `try` - 测试服务

```sh
# 测试 OpenClaw 服务
clawparty try openclaw \
  --mesh-name my-mesh \
  --user-name test-user \
  --ep-name test-ep \
  --pass-key secret123

# 测试 OpenClaw 作为出站端点
clawparty try openclaw \
  --mesh-name my-mesh \
  --user-name test-user \
  --ep-name test-ep \
  --targets 192.168.1.10:80,192.168.1.11:8080

# 测试 OpenClaw 作为入站端点
clawparty try openclaw \
  --mesh-name my-mesh \
  --user-name test-user \
  --ep-name test-ep \
  --master ep-office \
  --listen 8080,8081
```

## 完整示例

### 示例 1：基本网格设置

```sh
# 1. 启动 Hub
clawparty start hub --listen 0.0.0.0:8888 --names 1.2.3.4:8888 --permit root.json

# 2. 在第一个 Agent 上启动并加入网格
clawparty start agent
clawparty join my-mesh --as ep-1 --permit root.json

# 3. 在第二个 Agent 上启动并加入网格
clawparty start agent --listen 127.0.0.1:7778
clawparty identity > pub.key
# 将 pub.key 复制到第一个 Agent
# 在第一个 Agent 上：
clawparty invite ep-2 --identity pub.key --permit ep2.json
# 将 ep2.json 复制到第二个 Agent
# 在第二个 Agent 上：
clawparty join my-mesh --as ep-2 --permit ep2.json

# 4. 验证连接
clawparty get mesh
clawparty get ep
```

### 示例 2：隧道配置

```sh
# 1. 在目标 endpoint 上创建出站隧道
clawparty tunnel open outbound tcp/web-servers --targets 192.168.1.10:80 --targets 192.168.1.11:8080

# 2. 在源 endpoint 上创建入站隧道
clawparty tunnel open inbound tcp/web-servers --listen 8080

# 3. 测试隧道
curl localhost:8080
```

### 示例 3：代理配置

```sh
# 1. 在转发端点配置代理
clawparty proxy config --add-target 0.0.0.0/0 '*'

# 2. 在监听端点配置代理
clawparty proxy config --set-listen 0.0.0.0:1080

# 3. 使用代理
curl --proxy http://localhost:1080 http://internal-service.local
```

### 示例 4：远程终端访问

```sh
# 1. 在目标 endpoint 配置用户权限
clawparty terminal config --add-user admin

# 2. 从源 endpoint 连接
clawparty terminal open target-ep

# 3. 执行远程命令
uname -a
ls -la
```

### 示例 5：分布式文件共享（Cloud 应用）

```sh
# 1. 配置本地目录
clawparty cloud config --local-dir ~/clawpartyCloud

# 2. 查看云端文件
clawparty cloud ls /users/root

# 3. 上传文件
clawparty cloud upload /users/root/document.pdf

# 4. 下载文件
clawparty cloud download /users/root/document.pdf

# 5. 设置自动镜像
clawparty cloud config --add-mirror /users/root

# 6. 共享文件
clawparty cloud share /users/root/shared --set-all readonly
```

## 故障排除

### 常见问题

1. **连接失败**：检查 `clawparty config` 中的 Agent 地址是否正确
2. **许可无效**：确保使用正确的许可文件且未过期
3. **应用未找到**：使用 `clawparty get app` 检查应用是否可用
4. **端口冲突**：使用 `--listen` 选项指定不同的端口

### 调试命令

```sh
# 检查网格状态
clawparty get mesh

# 查看 endpoint 详情
clawparty describe ep $(clawparty get ep | awk 'NR==2{print $1}')

# 检查应用状态
clawparty get app

# 查看日志
clawparty log ep
```

## 环境变量

- `clawparty_AGENT`：设置默认 Agent 地址
- `clawparty_MESH`：设置默认网格名称

```sh
export clawparty_AGENT=192.168.1.100:7777
export clawparty_MESH=my-mesh
```

## 配置文件

配置存储在 `~/.clawparty.conf` 中，格式为 JSON：

```json
{
  "agent": "127.0.0.1:7777",
  "mesh": "my-mesh"
}
```

## 相关文档

- [clawparty 架构概念](./Architecture-Concepts.md)
- [ZT-App 开发指南](./ZT-App.md)
- [Agent API 文档](./Agent-API.md)
- [构建说明](./Build.md)