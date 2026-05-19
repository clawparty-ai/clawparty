use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;

use std::path::Path;

use crate::db;
use crate::db::AgentRecord;
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

"#;

const WIKI_MD_NOTICE: &str = "# WIKI.md\n\nSee this file for the full LLM Wiki methodology.\n";

const RADAR_MD_NOTICE: &str = "# RADAR.md\n\nSee this file for the full Radar methodology.\n";

/// Write bootstrap workspace files for 0#Agent (AGENTS.md, WIKI.md, RADAR.md).
/// Skips any file that already exists.
fn write_zero_agent_bootstrap_files(workspace_dir: &std::path::Path) {
    let files: &[(&str, &str)] = &[
        ("AGENTS.md", AGENTS_MD),
        ("WIKI.md",   WIKI_MD_NOTICE),
        ("RADAR.md",  RADAR_MD_NOTICE),
    ];

    // Read llm-wiki.md from Desktop if available
    let wiki_content = std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join("Desktop").join("llm-wiki.md"))
        .and_then(|p| std::fs::read_to_string(&p).ok());

    // Read radar-design.md from docs/ — try several locations relative to exe
    let radar_content = std::env::current_exe().ok().and_then(|exe| {
        let mut dir = exe.as_path();
        for _ in 0..4 {
            let candidate = dir.join("docs").join("radar-design.md");
            if candidate.exists() {
                return std::fs::read_to_string(&candidate).ok();
            }
            dir = match dir.parent() { Some(p) => p, None => break };
        }
        None
    });

    for (filename, default_content) in files {
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
fn spawn_agent_process(data_dir: &str, agent_name: &str, agent_dir: &str, port: u16) {
    let zeroclaw_bin = match find_zeroclaw_bin() {
        Some(p) => p,
        None => {
            ts_eprint!("[Agents] zeroclaw binary not found, cannot start '{}'", agent_name);
            return;
        }
    };

    // Skip if port already in use (agent may already be running)
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
        Ok(child) => {
            let pid = child.id() as u64;
            ts_print!("[Agents] Started agent '{}' on port {} (pid {})", agent_name, port, pid);
            let _ = db::update_agent_status(data_dir, agent_name, "running", Some(pid), None);
        }
        Err(e) => {
            ts_eprint!("[Agents] Failed to start agent '{}': {}", agent_name, e);
            let _ = db::update_agent_status(data_dir, agent_name, "error", None, Some(&e.to_string()));
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

        // Skip if already in clawparty.db
        if let Ok(Some(_)) = db::get_agent(data_dir, &agent_name) {
            // For 0#Agent: still ensure bootstrap files are present
            if agent_name == "0#Agent" {
                let workspace_dir = path.join("workspace");
                if workspace_dir.exists() {
                    write_zero_agent_bootstrap_files(&workspace_dir);
                }
            }
            continue;
        }

        let config_path = path.join("config.toml");
        let workspace_dir = path.join("workspace");

        // Read port from config.toml; allocate a free port if none found
        let port = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|l| l.trim().starts_with("port"))
                    .and_then(|l| l.split('=').nth(1))
                    .and_then(|v| v.trim().split(|c: char| !c.is_ascii_digit()).next())
                    .and_then(|v| v.parse::<u16>().ok())
            })
            .unwrap_or_else(|| allocate_port(data_dir).unwrap_or(42617));

        let dir_str = path.to_string_lossy().to_string();
        let config_str = config_path.to_string_lossy().to_string();
        let ws_str = workspace_dir.to_string_lossy().to_string();
        let description = format!("Auto-discovered agent: {}", agent_name);

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
        ) {
            Ok(_) => {
                ts_print!("[Agents] Created DB record for agent '{}' on port {}", agent_name, port);
                count += 1;
                // Write bootstrap files for newly discovered 0#Agent
                if agent_name == "0#Agent" && workspace_dir.exists() {
                    write_zero_agent_bootstrap_files(&workspace_dir);
                }
                // Start the agent immediately (skip 0#Agent — managed separately)
                if agent_name != "0#Agent" {
                    spawn_agent_process(data_dir, &agent_name, &dir_str, port);
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

    // Write config.toml
    let config_path = agent_dir.join("config.toml");
    let mut config_content = format!(
        r#"[gateway]
port = {}
require_pairing = false

[agent]
name = "{}"
"#,
        port, agent_name
    );

    // Add model config if provided
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
