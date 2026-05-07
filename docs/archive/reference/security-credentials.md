# 敏感信息管理指南

本文档说明如何在 ClawParty 项目中正确处理敏感信息（API keys、tokens、证书等），避免泄露到 Git 仓库。

---

## 一、基本原则

1. **永远不要提交真实的 API keys、tokens、密码到 Git**
2. **使用 .example 文件作为模板**，真实配置文件加入 `.gitignore`
3. **使用环境变量**传递敏感信息
4. **配置文件中使用明显的占位符**，如 `your-api-key-here`、`CHANGE_ME`

---

## 二、文件分类

### 应该提交到 Git（模板/示例）

- ✅ `*.example` 文件 - 配置模板
- ✅ `.env.example` - 环境变量模板
- ✅ 包含占位符的配置文件（如 `setup.sh` 中的 fallback 配置）
- ✅ 文档和说明

### 不应该提交到 Git（本地配置）

- ❌ `.env` - 真实环境变量
- ❌ `.env.local` - 本地环境变量
- ❌ `*.local.sh` - 本地测试脚本
- ❌ `*.local.toml` - 本地配置文件
- ❌ `*.key`, `*.pem` - 证书和密钥文件
- ❌ `credentials.json` - 凭证文件

---

## 三、配置文件使用流程

### 1. 测试环境配置

**步骤：**

```bash
# 1. 复制模板文件
cd tests/acl-local/
cp config.example.toml config.local.toml

# 2. 编辑 config.local.toml，填入真实的 API key
vim config.local.toml

# 3. 运行测试（setup.sh 会优先使用 config.local.toml）
./setup.sh
```

**说明：**
- `config.example.toml` - 提交到 Git 的模板
- `config.local.toml` - 本地使用，已在 `.gitignore` 中排除

### 2. 环境变量配置

**步骤：**

```bash
# 1. 复制环境变量模板
cd tests/
cp .env.example .env.local

# 2. 编辑 .env.local，填入真实的 API keys
vim .env.local

# 3. 加载环境变量
source .env.local

# 4. 运行测试
./acl-local/setup.sh
```

### 3. ZeroClaw 配置

**全局配置：** `~/.zeroclaw/config.toml`

```toml
# 正确的配置格式
default_provider = "openai"
default_model = "gpt-4o-mini"
api_key = "sk-your-real-api-key-here"  # 真实 key

[model_providers]
# 自定义 provider 配置
```

**Agent 配置：** `~/.clawparty/agents/<name>/config.toml`

创建新 agent 时，会自动从 `~/.zeroclaw/config.toml` 复制配置。

---

## 四、.gitignore 规则

项目已配置以下 `.gitignore` 规则保护敏感信息：

```gitignore
# 环境变量
.env
.env.local
.env.*.local

# 本地配置
*.local.sh
*.local.toml
config.local.*

# 证书和密钥
*.key
*.pem
*.crt
credentials.json
.secret_key

# ZeroClaw 配置目录
.zeroclaw/*

# 测试临时文件
tests/tmp/
tests/acl-local/tmp/
```

---

## 五、常见场景

### 场景 1：创建新的测试脚本

**错误做法：**
```bash
# ❌ 直接在脚本中硬编码 API key
API_KEY="sk-real-key-here"
```

**正确做法：**
```bash
# ✅ 使用环境变量
API_KEY="${ZEROCLAW_API_KEY:-your-api-key-here}"

# 或者在脚本开头说明
# Configuration:
# Set ZEROCLAW_API_KEY environment variable before running
```

### 场景 2：添加新的配置文件

**步骤：**

1. 创建 `.example` 模板文件（提交到 Git）
2. 在 `.gitignore` 中添加真实配置文件的规则
3. 在文档中说明如何使用

**示例：**

```bash
# 1. 创建模板
cat > myconfig.example.toml <<EOF
# Example configuration
api_key = "your-api-key-here"
EOF

# 2. 更新 .gitignore
echo "myconfig.toml" >> .gitignore
echo "myconfig.local.toml" >> .gitignore

# 3. 在 README 中说明
# "Copy myconfig.example.toml to myconfig.local.toml and fill in your API key"
```

### 场景 3：修复已泄露的敏感信息

如果不小心提交了敏感信息：

```bash
# 1. 立即修改文件，替换为占位符
vim leaked-file.sh

# 2. Amend 最近的 commit
git add leaked-file.sh
git commit --amend --no-edit

# 3. Force push 覆盖远程历史
git push --force origin branch-name

# 4. 轮换泄露的 API key（重要！）
# 去服务提供商控制台重新生成新的 key
```

---

## 六、占位符规范

使用清晰明确的占位符，避免被误认为是真实值：

**推荐的占位符：**
- `your-api-key-here`
- `CHANGE_ME_BEFORE_PRODUCTION`
- `sk-...` (对于 OpenAI 格式的 key)
- `https://your-endpoint.com`

**不推荐的占位符：**
- `mytoken123` - 看起来像真实 token
- `test` - 太简单，容易被忽略
- `xxx` - 不够明确

---

## 七、检查清单

在提交代码前，检查：

- [ ] 没有硬编码的 API keys
- [ ] 没有真实的服务端点 URL（除非是公开服务）
- [ ] 配置文件使用了明显的占位符
- [ ] 敏感文件已加入 `.gitignore`
- [ ] 提供了 `.example` 模板文件
- [ ] 文档中说明了如何配置

---

## 八、相关文件

- `.gitignore` - Git 忽略规则
- `tests/.env.example` - 测试环境变量模板
- `tests/acl-local/config.example.toml` - 测试配置模板
- `zeroclaw/.env.example` - ZeroClaw 环境变量模板
- `docs/reference/pitfalls.md` - 常见问题和解决方案

---

## 九、紧急联系

如果发现敏感信息泄露：

1. **立即通知团队** - 在团队群中告知
2. **轮换凭证** - 去服务提供商控制台重新生成
3. **修复 Git 历史** - 使用 `git commit --amend` + `git push --force`
4. **检查影响范围** - 确认是否有其他地方使用了相同的凭证

---

## 十、最佳实践

1. **开发环境使用测试 key**，生产环境使用独立的 key
2. **定期轮换 API keys**（建议每 3-6 个月）
3. **使用最小权限原则**，只授予必要的权限
4. **监控 API key 使用情况**，及时发现异常
5. **代码审查时特别注意**敏感信息

---

## 参考资料

- [GitHub: Removing sensitive data from a repository](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository)
- [OWASP: Secrets Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
