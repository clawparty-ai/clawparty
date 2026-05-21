# ZeroClaw Prompt Flow 时序图

本文档描述 zeroclaw 在 `ws/chat` 收到用户消息后，构建 LLM 请求的两个核心步骤。

---

## 时序图一：build_system_prompt() — 系统提示构建

```mermaid
sequenceDiagram
    participant Client as WebSocket Client
    participant WS as ws.rs:handle_socket
    participant Agent as Agent::turn_streamed
    participant BSP as Agent::build_system_prompt
    participant Ctx as PromptContext
    participant SPB as SystemPromptBuilder
    participant IS as IdentitySection
    participant PS as PersonalitySystem
    participant FS as FileSystem (workspace)

    Client->>WS: {"type":"message","content":"..."}
    WS->>Agent: turn_streamed(user_message, event_tx)
    
    rect rgb(230, 245, 255)
        Note over Agent: 第1步：构建 System Prompt<br/>(仅当 history 为空时执行)
        Agent->>BSP: build_system_prompt()
        
        BSP->>Ctx: 组装 PromptContext
        Note right of Ctx: workspace_dir, model_name,<br/>tools, skills, identity_config,<br/>security_summary, autonomy_level
        
        BSP->>SPB: build(&ctx)
        
        loop 遍历 9 个 Section
            SPB->>SPB: section.build(ctx)
            
            alt IdentitySection (第2个)
                SPB->>IS: build(ctx)
                IS->>IS: 检查 AIEOS 配置?
                alt AIEOS 已配置
                    IS->>IS: 加载 identity.json
                else 回退到 OpenClaw
                    IS->>PS: load_personality(workspace_dir)
                    PS->>FS: 读取 SOUL.md
                    PS->>FS: 读取 IDENTITY.md
                    PS->>FS: 读取 USER.md
                    PS->>FS: 读取 AGENTS.md
                    PS->>FS: 读取 TOOLS.md
                    PS->>FS: 读取 HEARTBEAT.md
                    PS->>FS: 读取 BOOTSTRAP.md
                    PS->>FS: 读取 MEMORY.md
                    Note right of PS: 每个文件最多 20K 字符<br/>缺失/空文件跳过
                    FS-->>PS: 返回各文件内容
                    PS-->>IS: PersonalityProfile
                end
                IS-->>SPB: "### SOUL.md\n<内容>\n\n### AGENTS.md\n..."
            else 其他 Section
                Note right of SPB: DateTime, ToolHonesty,<br/>Tools, Safety, Skills,<br/>Workspace, Runtime, ChannelMedia
            end
        end
        
        SPB-->>BSP: 完整 system prompt 字符串
        BSP-->>Agent: Result<String>
        Agent->>Agent: history.push(system_message)
    end
```

**关键代码路径**:
- `agent.rs:1004-1009` — 判断 history 为空，触发 build
- `agent.rs:629-644` — 组装 PromptContext，调用 builder
- `prompt.rs:46-59` — 9 个 Section 的定义
- `prompt.rs:91-123` — IdentitySection 加载 personality 文件
- `personality.rs:15-24` — 硬编码的 8 个 .md 文件列表
- `personality.rs:85-120` — 逐文件读取、截断、渲染

---

## 时序图二：memory_loader.load_context() — 记忆上下文加载

```mermaid
sequenceDiagram
    participant Client as WebSocket Client
    participant WS as ws.rs:handle_socket
    participant Agent as Agent::turn_streamed
    participant ML as DefaultMemoryLoader
    participant Mem as Memory Backend<br/>(SQLite/Vector/Qdrant)
    participant Decay as decay::apply_time_decay
    participant Agent2 as Agent::turn_streamed (续)

    Client->>WS: {"type":"message","content":"今天北京天气如何"}
    WS->>Agent: turn_streamed(user_message, event_tx)
    
    Note over Agent: 第1步已完成<br/>(system prompt 已加入 history)
    
    rect rgb(255, 245, 230)
        Note over Agent: 第2步：加载记忆上下文
        Agent->>ML: load_context(memory, user_message, session_id)
        
        ML->>Mem: recall(user_message, limit=5, session_id, None, None)
        Note right of Mem: 基于用户消息做语义搜索<br/>召回最相关的记忆条目
        Mem-->>ML: Vec<MemoryEntry> (最多5条)
        
        ML->>Decay: apply_time_decay(&mut entries, half_life_days)
        Note right of Decay: 越旧的记忆分数越低<br/>(指数衰减)
        
        loop 过滤与组装
            ML->>ML: is_assistant_autosave_key?
            alt 是 autosave
                ML->>ML: continue (跳过)
            else 检查相关性分数
                ML->>ML: score >= min_relevance_score (0.4)?
                alt 分数过低
                    ML->>ML: continue (跳过)
                else 通过过滤
                    ML->>ML: 追加到 context 字符串
                end
            end
        end
        
        alt 无有效记忆
            ML-->>Agent: "" (空字符串)
            Agent2->>Agent2: enriched = "[2026-05-21 10:30:00 CST] 今天北京天气如何"
        else 有有效记忆
            ML-->>Agent: "[Memory context]\n- key: content\n...\n[/Memory context]\n\n"
            Agent2->>Agent2: enriched = "[Memory context]...\n\n[2026-05-21 10:30:00 CST] 今天北京天气如何"
        end
        
        Agent->>Agent: history.push(user_message)
        Note right of Agent: 用户消息被包装后<br/>加入对话历史
    end
```

**关键代码路径**:
- `agent.rs:1012-1020` — 调用 memory_loader.load_context()
- `memory_loader.rs:40-79` — DefaultMemoryLoader 实现
- `memory_loader.rs:46-48` — 语义搜索 recall (limit=5)
- `memory_loader.rs:54` — 时间衰减
- `memory_loader.rs:56-69` — 过滤规则 (autosave/min_score)
- `agent.rs:1034-1039` — 时间戳前缀 + 合并到用户消息

---

## 两步对比

| | build_system_prompt() | memory_loader.load_context() |
|--|----------------------|------------------------------|
| **触发条件** | 仅首次（history 为空） | 每次消息都执行 |
| **数据源** | 文件系统（.md 文件） | 记忆后端（语义搜索） |
| **确定性** | 固定内容（除非文件修改） | 动态召回（与查询相关） |
| **输出位置** | System Message（role=system） | 用户消息前缀 |
| **可控性** | 目前无配置可过滤 | 通过 limit/min_score 可调 |
| **文件位置** | `personality.rs:15-24` | `memory_loader.rs:15-35` |
