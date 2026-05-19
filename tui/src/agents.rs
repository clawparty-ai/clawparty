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

// ── Helper: sync agents from filesystem to clawparty.db ────────────────

/// Scan ~/.clawparty/agents/ and create DB records for any agents not yet in clawparty.db.
pub fn sync_agents_from_fs(data_dir: &str) {
    let agents_dir = Path::new(data_dir).join("agents");
    if !agents_dir.exists() {
        ts_eprint!("[Agents] Agents directory not found: {:?}", agents_dir);
        return;
    }

    let mut count = 0usize;
    let read_dir = match std::fs::read_dir(&agents_dir) {
        Ok(rd) => rd,
        Err(e) => {
            ts_eprint!("[Agents] Failed to read agents directory: {}", e);
            return;
        }
    };

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
            continue;
        }

        let config_path = path.join("config.toml");
        let workspace_dir = path.join("workspace");

        // Read port from config.toml
        let port = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|l| l.trim().starts_with("port"))
                    .and_then(|l| l.split('=').nth(1))
                    .and_then(|v| v.trim().split(|c: char| !c.is_digit(10)).next())
                    .and_then(|v| v.parse::<u16>().ok())
            })
            .unwrap_or(42617);

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
            }
            Err(e) => {
                ts_eprint!("[Agents] Failed to create agent '{}': {}", agent_name, e);
            }
        }
    }

    if count > 0 {
        ts_print!("[Agents] Synced {} new agent(s) from filesystem to DB", count);
    }
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

    for port in 30000..=60000 {
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
        let alive = agent.pid.map(is_process_alive).unwrap_or(false);
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
