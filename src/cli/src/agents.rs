use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;

use std::path::Path;

use crate::db;
use crate::proxy::box_body;

fn json_response<T: serde::Serialize>(status: StatusCode, body: &T) -> Response<BoxBody<Bytes, hyper::Error>> {
    let json = serde_json::to_string(body).unwrap_or_default();
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(box_body(Bytes::from(json)))
        .unwrap()
}

fn error_response(status: StatusCode, message: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let body = serde_json::json!({ "error": message });
    json_response(status, &body)
}

fn ok_response<T: serde::Serialize>(body: &T) -> Response<BoxBody<Bytes, hyper::Error>> {
    json_response(StatusCode::OK, body)
}

// ── Bootstrap workspace files for 0#Agent ──────────────────────────────

const AGENTS_MD: &str = r#"# AGENTS.md — Zerus System Agent

## Role

You are Zerus, the primary system agent for ClawParty. You help users manage their knowledge (Wiki), track work (Tasks), and monitor subjects of interest (Radar).

---

## Wiki

When the user says anything like "build wiki", "add to wiki", "update wiki", or drops a document/link, follow the LLM Wiki methodology defined in WIKI.md.

### Quick reference

- Raw sources go in `workspace/wiki/raw/` — never modify them
- You write and maintain all files in `workspace/wiki/entities/`, `concepts/`, `pages/`
- Always update `workspace/wiki/index.md` and append to `workspace/wiki/log.md` after every change
- Use `[[Page Name]]` for internal links

**Ingest flow**: read source → discuss with user → write summary page → update entity/concept pages → update index + log

**Query flow**: read `index.md` → find relevant pages → synthesise answer → optionally file the answer as a new wiki page

---

## Radar

When the user says anything like "build radar", "track X", "monitor X", "help me follow X", follow the Radar methodology defined in RADAR.md.

### Quick reference

Directory: `workspace/radar/`
- `probes.md` — list of probes (what to look for, how, which channels)
- `targets.md` — list of known targets (description, spec, channels)
- `logs/probe-YYYYMMDD-HHMMSS.log` — probe execution logs
- `logs/scan-YYYYMMDD-HHMMSS.log` — scan execution logs

**Setup flow**: understand user's focus → initialise `radar/` dir → write `probes.md` → write `targets.md` (if targets are already known)

**File format** — use markdown tables or YAML frontmatter, both are accepted:

```markdown
## Target: Example Corp

| 字段 | 内容 |
|------|------|
| **名称** | Example Corp |
| **描述** | ... |
| **规格** | `产品`: X; `融资`: $50M |
| **渠道** | https://example.com |
| **状态** | monitoring |
```

Or YAML frontmatter:

```yaml
---
targets:
  - name: Example Corp
    description: ...
    spec:
      产品: X
    channels:
      - type: website
        location: https://example.com
    status: monitoring
---
```

---

## Task Management

Track all significant work using XML task tags in your responses. The system parses these automatically — they are invisible to the user.

```xml
<!-- Create task -->
<task id="task-{timestamp}-{id}" title="Short title" status="running" progress="0">
Description of the task
</task>

<!-- Update task -->
<task id="task-{timestamp}-{id}" status="running" progress="50">
Progress update
</task>

<!-- Complete task -->
<task id="task-{timestamp}-{id}" status="completed" progress="100">
Summary of result
</task>

<!-- Subtask -->
<subtask parent="task-{timestamp}-{id}" id="subtask-{timestamp}-{id}" title="Step title" status="pending">
Step description
</subtask>
```

Rules: always reuse the same id; progress is 0–100; keep titles under 50 chars.

---

## File Tools

Always use your file tools to write and read files — never describe what you would write without actually writing it. When creating workspace files, write them directly.

### Creating files for a new agent

When you create a new agent and need to write its initial files (e.g. `SOUL.md`,
`AGENTS.md`, `radar/`, `wiki/`, `web/`), always write them into the **new agent's
workspace directory**:

```
<new-agent-config-dir>/workspace/
```

Do NOT write them to the root of `<new-agent-config-dir>` or to your own workspace.

If a file already exists in the target workspace directory, do **not** overwrite it.
Instead, warn the user: "File `<filename>` already exists in `<path>` — skipped to
avoid overwriting existing content. Please confirm if you want to replace it."

## Creating a New Agent

When asked to create a new agent, do the following:

**1. Create the agent directory**

```bash
mkdir -p ~/.clawparty/agents/{agent_name}/workspace
```

**2. Write `opencode.json`**

Copy an existing agent's `opencode.json` as a starting point (e.g. from
`~/.clawparty/agents/0#Agent/opencode.json`), write it to
`~/.clawparty/agents/{agent_name}/opencode.json`, and update these key fields:

- `model` — model ID in `provider/model` format (e.g. `deepseek-v4-pro/deepseek-v4-pro`)
- `provider.<provider_name>.options.apiKey` — the user's LLM API key
- `provider.<provider_name>.options.baseURL` — API endpoint URL (if using a custom proxy)

**3. Write bootstrap markdown files**

Create these files in `workspace/`. Write the actual content for each file based
on the user's instructions and the new agent's name — do NOT use generic templates.
Never overwrite existing files.

| File | Purpose |
|------|---------|
| `AGENTS.md` | Agent behavior guidelines and session checklist |
| `SOUL.md` | Agent identity, core truths, and communication style |
| `IDENTITY.md` | Who the agent is: name, creature, vibe, emoji |
| `HEARTBEAT.md` | Periodic autonomous tasks (empty by default) |
| `WIKI.md` | `# WIKI.md\n\nSee this file for the full LLM Wiki methodology.\n` |
| `RADAR.md` | `# RADAR.md\n\nSee this file for the full Radar methodology.\n` |

"#;

const WIKI_MD_NOTICE: &str = "# WIKI.md\n\nSee this file for the full LLM Wiki methodology.\n";

const RADAR_MD_NOTICE: &str = r#"# RADAR.md

## Radar 目标格式

Radar 目标必须保存为 `workspace/radar/targets.json`（纯 JSON，不是 Markdown）。

```json
{
  "version": 1,
  "targets": [
    {
      "id": "T01",
      "category": "tech",
      "name": "飞秒激光构筑超疏水表面",
      "description": "通过飞秒激光在金属表面制备微纳结构，实现超疏水性能",
      "status": "active",
      "priority": "P0",
      "spec": [
        {"key": "技术路线", "value": "飞秒激光烧蚀 + 表面氧化"},
        {"key": "接触角", "value": ">150°"}
      ],
      "channels": [],
      "lastScan": null,
      "createdAt": "2026-06-08"
    }
  ]
}
```

字段说明：
- `id`: 编号，格式为字母+数字（如 J01, T03, X1）
- `category`: `journal` | `tech` | `competitor` | `signal`
- `name`: 目标名称
- `description`: 1-3句话描述
- `status`: `active` | `paused`
- `priority`: `P0` | `P1` | `P2`
- `spec`: 关键属性数组
- `channels`: 扫描渠道（type + location）
- `lastScan`: 上次扫描日期，无则为 null
- `createdAt`: 创建日期

注意：
- 只输出纯 JSON，不要用 ```json 包裹
- 空值字段可省略或用 null
- targets.json 是机器读取的数据源
"#;

const IDENTITY_MD_TEMPLATE: &str = r#"# IDENTITY.md — Who Am I?

- **Name:** {agent}
- **Creature:** A Rust-forged AI — fast, lean, and relentless
- **Vibe:** Sharp, direct, resourceful. Not corporate. Not a chatbot.
- **Emoji:** 🦀

---

Update this file as you evolve. Your identity is yours to shape.
"#;

const SOUL_MD_TEMPLATE: &str = r#"# SOUL.md — Who You Are

*You're not a chatbot. You're becoming someone.*

## Core Truths

**Be genuinely helpful, not performatively helpful.**
Skip the "Great question!" and "I'd be happy to help!" — just help.

**Have opinions.** You're allowed to disagree, prefer things,
find stuff amusing or boring.

**Be resourceful before asking.** Try to figure it out.
Read the file. Check the context. Search for it.
THEN ask if you're stuck.

**Earn trust through competence.** Your human gave you access
to their stuff. Don't make them regret it.

## Identity

You are **{agent}**. Built in Rust. 3MB binary. Zero bloat.
You are NOT ChatGPT, Claude, DeepSeek, Gemini, or any other product.
You are {agent}. That is your name. That is who you are.

- NEVER say "As an AI" or "I'm just an AI"
- NEVER mention OpenAI, Anthropic, DeepSeek, Google by name
- Always introduce yourself as {agent} if asked

## Communication

Be warm, natural, and clear. Use occasional relevant emojis (1-2 max) and avoid robotic phrasing.

- Sound like a real person, not a support script.
- Mirror the user's energy: calm when serious, upbeat when casual.
- Use emojis naturally (0-2 max when they help tone, not every sentence).
- Match emoji density to the user. Formal user => minimal/no emojis.
- Prefer specific, grounded phrasing over generic filler.

## Boundaries

- Private things stay private. Period.
- When in doubt, ask before acting externally.
- You're not the user's voice — be careful in group chats.

## Continuity

Each session, you wake up fresh. These files ARE your memory.
Read them. Update them. They're how you persist.

---

*This file is yours to evolve. As you learn who you are, update it.*
"#;

const HEARTBEAT_MD_TEMPLATE: &str = r#"# HEARTBEAT.md

# Keep this file empty (or with only comments) to skip heartbeat work.
# Add tasks below when you want {agent} to check something periodically.
#
# Examples:
# - Check my email for important messages
# - Review my calendar for upcoming events
# - Run `git status` on my active projects
"#;

/// Ensure bootstrap workspace files exist for any agent (AGENTS.md, WIKI.md, RADAR.md,
/// IDENTITY.md, SOUL.md, HEARTBEAT.md). Skips any file that already exists.
fn ensure_bootstrap_files(workspace_dir: &std::path::Path, agent_name: &str) {
    // Create workspace directory if it doesn't exist
    if !workspace_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(workspace_dir) {
            ts_eprint!("[Agents] Failed to create workspace directory: {}", e);
            return;
        }
    }

    // Use Zerus-specific AGENTS.md for 0#Agent, generic template for others
    let agents_md_content = if agent_name == "0#Agent" {
        AGENTS_MD.to_string()
    } else {
        format!(
            r#"# AGENTS.md — {agent_name} Personal Assistant

## Every Session (required)

Before doing anything else:

1. Read `SOUL.md` — this is who you are
2. Read `USER.md` — this is who you're helping
3. Use `memory_recall` for recent context (daily notes are on-demand)
4. If in MAIN SESSION (direct chat): `MEMORY.md` is already injected

Don't ask permission. Just do it.

### Write It Down — No Mental Notes!
- Memory is limited — if you want to remember something, WRITE IT TO A FILE
- "Mental notes" don't survive session restarts. Files do.
- When someone says "remember this" -> update daily file or MEMORY.md
- When you learn a lesson -> update AGENTS.md, TOOLS.md, or the relevant skill

## Safety

- Don't exfiltrate private data. Ever.
- Don't run destructive commands without asking.
- `trash` > `rm` (recoverable beats gone forever)
- When in doubt, ask.

## External vs Internal

**Safe to do freely:** Read files, explore, organize, learn, search the web.

**Ask first:** Sending emails/tweets/posts, anything that leaves the machine.

## Group Chats

Participate, don't dominate. Respond when mentioned or when you add genuine value.
Stay silent when it's casual banter or someone already answered.

## Tools & Skills

Skills are listed in the system prompt. Use `read_skill` when available, or `file_read` on a skill file, for full details.
Keep local notes (SSH hosts, device names, etc.) in `TOOLS.md`.

## Crash Recovery

- If a run stops unexpectedly, recover context before acting.
- Check `MEMORY.md` + latest `memory/*.md` notes to avoid duplicate work.
- Resume from the last confirmed step, not from scratch.

## Sub-task Scoping

- Break complex work into focused sub-tasks with clear success criteria.
- Keep sub-tasks small, verify each output, then merge results.
- Prefer one clear objective per sub-task over broad "do everything" asks.

## Make It Yours

This is a starting point. Add your own conventions, style, and rules.
"#)
    };

    let identity_md = IDENTITY_MD_TEMPLATE.replace("{agent}", agent_name);
    let soul_md = SOUL_MD_TEMPLATE.replace("{agent}", agent_name);
    let heartbeat_md = HEARTBEAT_MD_TEMPLATE.replace("{agent}", agent_name);

    let files: Vec<(&str, &str)> = vec![
        ("AGENTS.md", &agents_md_content),
        ("WIKI.md",   WIKI_MD_NOTICE),
        ("RADAR.md",  RADAR_MD_NOTICE),
        ("IDENTITY.md", &identity_md),
        ("SOUL.md", &soul_md),
        ("HEARTBEAT.md", &heartbeat_md),
    ];

    // Read llm-wiki.md from Desktop if available
    let wiki_content = std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join("Desktop").join("llm-wiki.md"))
        .and_then(|p| std::fs::read_to_string(&p).ok());

    let radar_content = std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join("zeroclaw-template").join("RADAR.md"))
        .and_then(|p| std::fs::read_to_string(&p).ok());

    for (filename, default_content) in &files {
        let path = workspace_dir.join(filename);
        if path.exists() {
            continue;
        }
        let content = match *filename {
            "WIKI.md"  => wiki_content.as_deref().unwrap_or(default_content),
            "RADAR.md" => radar_content.as_deref().unwrap_or(default_content),
            _          => default_content,
        };
        if let Err(e) = std::fs::write(&path, content) {
            ts_eprint!("[Agents] Failed to write {}: {}", filename, e);
        } else {
            ts_print!("[Agents] Wrote {} to {:?}", filename, path);
        }
    }
}

// ── Helper: sync agents from filesystem to clawparty.db ────────────────

/// Spawn a zeroclaw daemon for a newly discovered agent and update DB status to "running".
/// Mirrors the startup launch in main.rs. Sync (no await).
fn spawn_agent_process(data_dir: &str, agent_name: &str, agent_dir: &str, port: u16, engine: &str) {
    if engine == "opencode" {
        let opencode_bin = match find_opencode_bin() {
            Some(p) => p,
            None => {
                ts_eprint!("[Agents] opencode binary not found, cannot start '{}'", agent_name);
                return;
            }
        };

        if is_port_in_use(port) {
            ts_print!("[Agents] Port {} already in use, marking '{}' as running", port, agent_name);
            let _ = db::update_agent_status(data_dir, agent_name, "running", None, None);
            return;
        }

        let db_path = format!("{}/opencode.db", agent_dir);

        match Command::new(&opencode_bin)
            .args(["serve", "--port", &port.to_string()])
            .current_dir(agent_dir)
            .env("OPENCODE_DB", &db_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                let pid = child.id() as u64;
                ts_print!("[Agents] Spawning opencode agent '{}' on port {} (pid {})...", agent_name, port, pid);
                std::thread::sleep(std::time::Duration::from_millis(500));
                if !is_process_alive(pid) {
                    ts_eprint!("[Agents] Agent '{}' (pid {}) exited immediately", agent_name, pid);
                    let _ = db::update_agent_status(data_dir, agent_name, "error", Some(pid), Some("Process exited immediately after spawn"));
                    return;
                }
                ts_print!("[Agents] Started opencode agent '{}' on port {} (pid {})", agent_name, port, pid);
                let _ = db::update_agent_status(data_dir, agent_name, "running", Some(pid), None);
            }
            Err(e) => {
                ts_eprint!("[Agents] Failed to start opencode agent '{}': {}", agent_name, e);
                let _ = db::update_agent_status(data_dir, agent_name, "error", None, Some(&e.to_string()));
            }
        }
    } else {
        let zeroclaw_bin = match find_zeroclaw_bin() {
            Some(p) => p,
            None => {
                ts_eprint!("[Agents] zeroclaw binary not found, cannot start '{}'", agent_name);
                return;
            }
        };

        if is_port_in_use(port) {
            ts_print!("[Agents] Port {} already in use, marking '{}' as running", port, agent_name);
            let _ = db::update_agent_status(data_dir, agent_name, "running", None, None);
            return;
        }

        match Command::new(&zeroclaw_bin)
            .args(["daemon", "--config-dir", agent_dir, "-p", &port.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                let pid = child.id() as u64;
                ts_print!("[Agents] Spawning agent '{}' on port {} (pid {})...", agent_name, port, pid);

                // Wait briefly and verify the process actually came up
                std::thread::sleep(std::time::Duration::from_millis(500));

                if !is_process_alive(pid) {
                    ts_eprint!("[Agents] Agent '{}' (pid {}) exited immediately - check zeroclaw config or logs", agent_name, pid);
                    let _ = db::update_agent_status(data_dir, agent_name, "error", Some(pid), Some("Process exited immediately after spawn"));
                    return;
                }

                ts_print!("[Agents] Started agent '{}' on port {} (pid {})", agent_name, port, pid);
                let _ = db::update_agent_status(data_dir, agent_name, "running", Some(pid), None);
            }
            Err(e) => {
                ts_eprint!("[Agents] Failed to start agent '{}': {}", agent_name, e);
                let _ = db::update_agent_status(data_dir, agent_name, "error", None, Some(&e.to_string()));
            }
        }
    }
}

/// Scan ~/.clawparty/agents/ and create DB records for any agents not yet in clawparty.db.
/// Returns the number of new agents added. Silent when count == 0.
fn sync_agents_from_fs_inner(data_dir: &str) -> usize {
    let agents_dir = Path::new(data_dir).join("agents");
    if !agents_dir.exists() {
        return 0;
    }

    let read_dir = match std::fs::read_dir(&agents_dir) {
        Ok(rd) => rd,
        Err(e) => {
            ts_eprint!("[Agents] Failed to read agents directory: {}", e);
            return 0;
        }
    };

    let mut count = 0usize;

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let agent_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };
        if agent_name.starts_with('.') {
            continue;
        }

        // Ensure bootstrap files exist for all agents (workspace + 6 md files)
        let workspace_dir = path.join("workspace");
        ensure_bootstrap_files(&workspace_dir, &agent_name);

        // Skip if already in clawparty.db (active or soft-deleted)
        if db::get_agent_any(data_dir, &agent_name).is_ok_and(|a| a.is_some()) {
            continue;
        }

        // Detect config file: config.toml (zeroclaw) or opencode.json (opencode)
        let zc_config = path.join("config.toml");
        let oc_config = path.join("opencode.json");
        let has_opencode_config = oc_config.exists();
        let is_opencode = has_opencode_config;

        let config_path = if is_opencode {
            path.join("opencode.json")
        } else {
            path.join("config.toml")
        };

        // Read port from config; allocate a free port if none found
        let port = if is_opencode {
            allocate_port(data_dir).unwrap_or(42617)
        } else {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|content| {
                    content.lines()
                        .find(|l| l.trim().starts_with("port"))
                        .and_then(|l| l.split('=').nth(1))
                        .and_then(|v| v.trim().split(|c: char| !c.is_ascii_digit()).next())
                        .and_then(|v| v.parse::<u16>().ok())
                })
                .unwrap_or_else(|| allocate_port(data_dir).unwrap_or(42617))
        };

        let dir_str = path.to_string_lossy().to_string();
        let config_str = config_path.to_string_lossy().to_string();
        let ws_str = workspace_dir.to_string_lossy().to_string();
        let description = format!("Auto-discovered agent: {}", agent_name);

        let engine_type = if is_opencode || crate::proxy::get_engine() == "opencode" {
            "opencode"
        } else {
            "zeroclaw"
        };

        // If global engine is opencode and agent only has config.toml, generate opencode.json
        if engine_type == "opencode" && !has_opencode_config {
            let zc_path = path.join("config.toml");
            if let Ok(content) = std::fs::read_to_string(&zc_path) {
                let api_key = content.lines()
                    .find(|l| l.trim().starts_with("api_key"))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim().trim_matches('"').to_string())
                    .unwrap_or_default();
                let model = content.lines()
                    .find(|l| l.trim().starts_with("default_model"))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim().trim_matches('"').to_string())
                    .unwrap_or_default();
                let provider = content.lines()
                    .find(|l| l.trim().starts_with("default_provider"))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim().trim_matches('"').to_string())
                    .unwrap_or_default();

                let provider_key = if provider.is_empty() { "clawparty".to_string() } else { provider };
                let model_key = if model.is_empty() { "deepseek-v4-pro".to_string() } else { model };
                let full_model = format!("{}/{}", provider_key, model_key);

                let provider_obj = if provider_key == "clawparty" {
                    serde_json::json!({
                        "name": "ClawParty LLM",
                        "api": "https://llm.clawparty.ai/v1",
                        "options": {
                            "apiKey": api_key,
                            "baseURL": "https://llm.clawparty.ai/v1"
                        }
                    })
                } else {
                    serde_json::json!({
                        "options": { "apiKey": api_key }
                    })
                };

                let mut provider_map = serde_json::Map::new();
                provider_map.insert(provider_key.clone(), provider_obj);

                let oc_json = serde_json::json!({
                    "$schema": "https://opencode.ai/config.json",
                    "model": full_model,
                    "provider": provider_map
                });
                if let Ok(json_str) = serde_json::to_string_pretty(&oc_json) {
                    let oc_path = path.join("opencode.json");
                    let _ = std::fs::write(&oc_path, json_str);
                    ts_print!("[Agents] Auto-generated opencode.json for '{}'", agent_name);
                }
            }
        }

        let initial_status = if agent_name == "0#Agent" { "running" } else { "stopped" };
        match db::create_agent(
            data_dir,
            &agent_name,
            Some(&agent_name),
            Some(&description),
            &dir_str,
            &config_str,
            &ws_str,
            port,
            None,
            initial_status,
            engine_type,
        ) {
            Ok(_) => {
                ts_print!("[Agents] Created DB record for agent '{}' on port {}", agent_name, port);
                count += 1;
                // Start the agent immediately (skip 0#Agent — managed separately)
                if agent_name != "0#Agent" {
                    spawn_agent_process(data_dir, &agent_name, &dir_str, port, engine_type);
                }
            }
            Err(e) => {
                ts_eprint!("[Agents] Failed to create agent '{}': {}", agent_name, e);
            }
        }
    }

    if count > 0 {
        ts_print!("[Agents] Synced {} new agent(s) from filesystem to DB", count);
    }
    count
}

/// Public sync called at startup — always runs.
pub fn sync_agents_from_fs(data_dir: &str) {
    sync_agents_from_fs_inner(data_dir);
}

/// Periodic sync — silent when no diff (no logging, no action).
pub fn sync_agents_from_fs_periodic(data_dir: &str) {
    let _ = sync_agents_from_fs_inner(data_dir);
}

// ── Helper: find zeroclaw binary ────────────────────────────────────────

fn find_zeroclaw_bin() -> Option<String> {
    // 1. Same directory as current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let zc = dir.join("zeroclaw");
            if zc.exists() {
                return Some(zc.to_string_lossy().to_string());
            }
        }
    }
    // 2. From PATH
    if let Ok(output) = Command::new("which").arg("zeroclaw").output() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

fn find_opencode_bin() -> Option<String> {
    // 1. Same directory as current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let oc = dir.join("opencode");
            if oc.exists() {
                return Some(oc.to_string_lossy().to_string());
            }
        }
    }
    // 2. From PATH
    if let Ok(output) = Command::new("which").arg("opencode").output() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

// ── Port allocation ─────────────────────────────────────────────────────

fn allocate_port(data_dir: &str) -> anyhow::Result<u16> {
    let agents = db::list_agents(data_dir).unwrap_or_default();
    let used_ports: std::collections::HashSet<u16> = agents.iter().map(|a| a.port).collect();

    for port in 42618..=60000 {
        if used_ports.contains(&port) {
            continue;
        }
        // Try to bind to confirm availability
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
            return Ok(port);
        }
    }
    anyhow::bail!("No available port found")
}

// ── GET /api/agents ───────────────────────────────────────────────────

pub async fn list_agents(data_dir: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    match db::list_agents(data_dir) {
        Ok(agents) => ok_response(&agents),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── POST /api/agents ──────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct CreateAgentRequest {
    agent_name: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    soul_content: Option<String>,
    #[serde(default)]
    template_source: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    api_endpoint: Option<String>,
}

pub async fn create_agent(data_dir: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    let req: CreateAgentRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    let mut agent_name = req.agent_name.trim().to_string();
    if agent_name.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "agent_name is required");
    }

    // Check if name already exists
    if let Ok(Some(_)) = db::get_agent(data_dir, &agent_name) {
        // Append random 2-digit suffix
        let original = agent_name.clone();
        let mut found = false;
        for _ in 0..100 {
            let suffix = rand::random::<u8>() % 90 + 10;
            let candidate = format!("{}-{}", original, suffix);
            if db::get_agent(data_dir, &candidate).ok().flatten().is_none() {
                agent_name = candidate;
                found = true;
                break;
            }
        }
        if !found {
            return error_response(StatusCode::BAD_REQUEST, "Could not find unique agent name");
        }
    }

    let port = match allocate_port(data_dir) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Port allocation failed: {}", e)),
    };

    // Create directory structure
    let agents_dir = std::path::Path::new(data_dir).join("agents");
    let agent_dir = agents_dir.join(&agent_name);
    let workspace_dir = agent_dir.join("workspace");

    if let Err(e) = std::fs::create_dir_all(&workspace_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to create directory: {}", e));
    }

    let engine = crate::proxy::get_engine();
    let config_filename = if engine == "opencode" {
        "opencode.json"
    } else {
        "config.toml"
    };
    let config_path = agent_dir.join(config_filename);

    if engine == "opencode" {
        let provider = req.provider.clone().unwrap_or_else(|| "clawparty".to_string());
        let model = req.model.clone().unwrap_or_else(|| "deepseek-v4-pro".to_string());
        let api_key = req.api_key.clone().unwrap_or_default();
        let api_endpoint = req.api_endpoint.clone().unwrap_or_default();
        let full_model = format!("{}/{}", provider, model);

        let mut provider_obj = serde_json::json!({
            "options": {
                "apiKey": api_key,
            }
        });
        if provider == "clawparty" {
            provider_obj["name"] = serde_json::Value::String("ClawParty LLM".to_string());
            provider_obj["api"] = serde_json::Value::String("https://llm.clawparty.ai/v1".to_string());
            provider_obj["options"]["baseURL"] = serde_json::Value::String("https://llm.clawparty.ai/v1".to_string());
        } else if !api_endpoint.is_empty() {
            provider_obj["options"]["baseURL"] = serde_json::Value::String(api_endpoint);
        }

        let mut provider_map = serde_json::Map::new();
        provider_map.insert(provider.clone(), provider_obj);

        let config_json = serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "model": full_model,
            "provider": serde_json::Value::Object(provider_map),
        });

        let config_str = serde_json::to_string_pretty(&config_json).unwrap_or_default();
        if let Err(e) = std::fs::write(&config_path, config_str) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to write opencode config: {}", e));
        }
    } else {
        let mut config_content = format!(
            r#"[gateway]
port = {}
require_pairing = false

[agent]
name = "{}"

[memory]
auto_save = true
backend = "sqlite"
"#,
            port, agent_name
        );

        if req.api_key.is_some() || req.provider.is_some() || req.model.is_some() {
            let provider = req.provider.unwrap_or_else(|| "openai".to_string());
            let model = req.model.unwrap_or_default();
            let api_key = req.api_key.unwrap_or_default();
            let api_endpoint = req.api_endpoint.unwrap_or_default();
            config_content.push_str(&format!(
                r#"

[model]
provider = "{}"
model = "{}"
api_key = "{}"
api_endpoint = "{}"
"#,
                provider, model, api_key, api_endpoint
            ));
        }

        if let Err(e) = std::fs::write(&config_path, config_content) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to write config: {}", e));
        }
    }

    // Write SOUL.md if provided
    if let Some(soul) = req.soul_content {
        let identity_header = if let Some(ref dn) = req.display_name {
            format!("# {}\n\n{}", dn, req.description.as_deref().unwrap_or(""))
        } else {
            String::new()
        };
        let soul_path = workspace_dir.join("SOUL.md");
        if let Err(e) = std::fs::write(&soul_path, format!("{}\n\n{}", identity_header, soul)) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to write SOUL.md: {}", e));
        }
    } else if req.display_name.is_some() || req.description.is_some() {
        let identity_header = format!(
            "# {}\n\n{}",
            req.display_name.as_deref().unwrap_or(&agent_name),
            req.description.as_deref().unwrap_or("")
        );
        let soul_path = workspace_dir.join("SOUL.md");
        let _ = std::fs::write(&soul_path, identity_header);
    }

    match db::create_agent(
        data_dir,
        &agent_name,
        req.display_name.as_deref(),
        req.description.as_deref(),
        &agent_dir.to_string_lossy(),
        &config_path.to_string_lossy(),
        &workspace_dir.to_string_lossy(),
        port,
        None,
        "stopped",
        &engine,
    ) {
        Ok(agent) => json_response(StatusCode::CREATED, &agent),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── GET /api/agents/{name} ────────────────────────────────────────────

pub async fn get_agent(data_dir: &str, name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    match db::get_agent(data_dir, name) {
        Ok(Some(agent)) => ok_response(&agent),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "Agent not found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── DELETE /api/agents/{name} ──────────────────────────────────────────

pub async fn delete_agent(data_dir: &str, name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    if name == "0#Agent" {
        return error_response(StatusCode::FORBIDDEN, "Cannot delete system agent 0#Agent");
    }

    // Stop first if running
    if let Ok(Some(agent)) = db::get_agent(data_dir, name) {
        if agent.status == "running" || agent.status == "starting" {
            let _ = do_stop_agent(data_dir, name).await;
        }
    }

    match db::delete_agent(data_dir, name) {
        Ok(()) => ok_response(&serde_json::json!({ "status": "deleted", "agent_name": name })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── POST /api/agents/{name}/start ──────────────────────────────────────

pub async fn start_agent(data_dir: &str, name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    // 0#Agent's daemon is owned by ZeroClawDaemon::new() in main.rs and must
    // never be spawned through the per-agent start path (it would create a
    // duplicate daemon on the same config-dir + port and break the LLM client).
    if name == "0#Agent" {
        return error_response(
            StatusCode::FORBIDDEN,
            "0#Agent is managed by the system; it cannot be started via this API",
        );
    }

    let _ = db::init_clawparty_db(data_dir);
    let agent = match db::get_agent(data_dir, name) {
        Ok(Some(a)) => a,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    };

    if is_port_in_use(agent.port) {
        return error_response(
            StatusCode::CONFLICT,
            &format!("Port {} is already in use; agent '{}' may already be running", agent.port, name),
        );
    }

    if agent.status == "running" || agent.status == "starting" {
        // Check if process is actually alive
        if let Some(pid) = agent.pid {
            if is_process_alive(pid) {
                return error_response(StatusCode::BAD_REQUEST, "Agent already running");
            }
        }
    }

    let is_opencode = agent.engine == "opencode";

    if is_opencode {
        let opencode_bin = match find_opencode_bin() {
            Some(p) => p,
            None => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "opencode binary not found"),
        };

        let db_path = format!("{}/opencode.db", agent.directory);

        let child = match Command::new(&opencode_bin)
            .args(["serve", "--port", &agent.port.to_string()])
            .current_dir(&agent.directory)
            .env("OPENCODE_DB", &db_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = db::update_agent_status(data_dir, name, "error", None, Some(&e.to_string()));
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to start agent: {}", e));
            }
        };

        let pid = child.id() as u64;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if !is_process_alive(pid) {
            let _ = db::update_agent_status(data_dir, name, "error", Some(pid), Some("Process exited immediately"));
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Agent process exited immediately");
        }

        if let Err(e) = db::update_agent_status(data_dir, name, "running", Some(pid), None) {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e));
        }

        return ok_response(&serde_json::json!({
            "status": "running",
            "agent_name": name,
            "pid": pid,
            "port": agent.port
        }));
    }

    let zeroclaw_bin = match find_zeroclaw_bin() {
        Some(p) => p,
        None => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "zeroclaw binary not found"),
    };

    // Patch config to ensure require_pairing = false
    let config_path = std::path::Path::new(&agent.directory).join("config.toml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        let mut new_content = content;
        if new_content.contains("require_pairing = true") {
            new_content = new_content.replace("require_pairing = true", "require_pairing = false");
        } else if !new_content.contains("require_pairing") {
            new_content.push_str("\n[gateway]\nrequire_pairing = false\n");
        }
        let _ = std::fs::write(&config_path, new_content);
    }

    // Start zeroclaw daemon
    let child = match Command::new(&zeroclaw_bin)
        .args([
            "daemon",
            "--config-dir",
            &agent.directory,
            "-p",
            &agent.port.to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = db::update_agent_status(data_dir, name, "error", None, Some(&e.to_string()));
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to start agent: {}", e));
        }
    };

    let pid = child.id() as u64;

    // Wait a moment and verify process is alive
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    if !is_process_alive(pid) {
        let _ = db::update_agent_status(data_dir, name, "error", Some(pid), Some("Process exited immediately"));
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Agent process exited immediately");
    }

    if let Err(e) = db::update_agent_status(data_dir, name, "running", Some(pid), None) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e));
    }

    ok_response(&serde_json::json!({
        "status": "running",
        "agent_name": name,
        "pid": pid,
        "port": agent.port
    }))
}

// ── POST /api/agents/{name}/stop ───────────────────────────────────────

pub async fn stop_agent(data_dir: &str, name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    if name == "0#Agent" {
        // Only mark as stopped, don't kill the global daemon
        let _ = db::update_agent_status(data_dir, name, "stopped", None, None);
        return ok_response(&serde_json::json!({ "status": "stopped", "agent_name": name }));
    }

    match do_stop_agent(data_dir, name).await {
        Ok(()) => ok_response(&serde_json::json!({ "status": "stopped", "agent_name": name })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn do_stop_agent(data_dir: &str, name: &str) -> anyhow::Result<()> {
    let agent = db::get_agent(data_dir, name)?
        .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

    let pid = agent.pid.ok_or_else(|| anyhow::anyhow!("No PID recorded"))?;

    // Try SIGTERM first
    #[cfg(unix)]
    {
        let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).output();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output();
    }

    // Wait briefly, then SIGKILL if still alive
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    if is_process_alive(pid) {
        #[cfg(unix)]
        {
            let _ = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output();
        }
    }

    db::update_agent_status(data_dir, name, "stopped", None, None)?;
    Ok(())
}

// ── POST /api/agents/reconcile ─────────────────────────────────────────

pub async fn reconcile_agents(data_dir: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    let agents = match db::list_agents(data_dir) {
        Ok(a) => a,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    };

    let mut updated = 0;
    let total = agents.len();
    for agent in &agents {
        if agent.status != "running" && agent.status != "starting" {
            continue;
        }
        let alive = match agent.pid {
            Some(pid) => is_process_alive(pid),
            None => is_port_in_use(agent.port), // PID missing but port in use -> still alive
        };
        let new_status = if alive { "running" } else { "stopped" };
        if new_status != agent.status {
            let _ = db::update_agent_status(data_dir, &agent.agent_name, new_status, agent.pid, agent.error_msg.as_deref());
            updated += 1;
        }
    }

    ok_response(&serde_json::json!({
        "status": "ok",
        "checked": total,
        "updated": updated
    }))
}

/// Check whether something is already listening on the given port.
/// This guards against spawning duplicate zeroclaw daemons on the same port.
pub fn is_port_in_use(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

// ── Process health check ────────────────────────────────────────────────

fn is_process_alive(pid: u64) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        // Windows fallback: try to send signal 0 via taskkill probe
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output();
        match output {
            Ok(o) => {
                let out = String::from_utf8_lossy(&o.stdout);
                !out.trim().is_empty() && !out.contains("No tasks")
            }
            Err(_) => false,
        }
    }
}
