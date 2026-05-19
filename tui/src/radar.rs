use std::path::PathBuf;

use hyper::body::{Bytes, Incoming};
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;

use crate::proxy::box_body;
use crate::wiki::get_agent_workspace;

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

// ── YAML frontmatter parsing ────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct ChannelRaw {
    #[serde(rename = "type")]
    channel_type: String,
    location: String,
}

#[derive(Debug, serde::Deserialize)]
struct TargetRaw {
    id: Option<String>,
    name: String,
    description: Option<String>,
    spec: Option<serde_yaml::Value>,
    channels: Option<Vec<ChannelRaw>>,
    #[serde(rename = "source_probe")]
    source_probe: Option<String>,
    status: Option<String>,
    #[serde(rename = "created_at")]
    created_at: Option<String>,
    #[serde(rename = "last_scan")]
    last_scan: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TargetsYaml {
    targets: Vec<TargetRaw>,
}

#[derive(Debug, serde::Serialize)]
struct SpecEntry {
    key: String,
    value: String,
}

#[derive(Debug, serde::Serialize)]
struct ChannelJson {
    #[serde(rename = "type")]
    channel_type: String,
    location: String,
}

#[derive(Debug, serde::Serialize)]
struct TargetJson {
    id: Option<String>,
    name: String,
    description: Option<String>,
    #[serde(rename = "spec")]
    spec_entries: Vec<SpecEntry>,
    #[serde(rename = "specLabel")]
    spec_label: String,
    channels: Vec<ChannelJson>,
    #[serde(rename = "channelLabel")]
    channel_label: String,
    #[serde(rename = "source_probe")]
    source_probe: Option<String>,
    status: String,
    #[serde(rename = "created_at")]
    created_at: Option<String>,
    #[serde(rename = "last_scan")]
    last_scan: Option<String>,
}

fn convert_spec(value: &serde_yaml::Value) -> Vec<SpecEntry> {
    let mut entries = Vec::new();
    match value {
        serde_yaml::Value::Mapping(m) => {
            for (k, v) in m.iter() {
                let key = k.as_str().unwrap_or("").to_string();
                let val = match v {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => v.as_str().unwrap_or("").to_string(),
                };
                entries.push(SpecEntry { key, value: val });
            }
        }
        serde_yaml::Value::String(s) => {
            // Try to parse "key: value; key2: value2" format
            for part in s.split(';') {
                let trimmed = part.trim();
                if let Some(pos) = trimmed.find(':') {
                    let key = trimmed[..pos].trim().to_string();
                    let value = trimmed[pos + 1..].trim().to_string();
                    entries.push(SpecEntry { key, value });
                }
            }
        }
        _ => {}
    }
    entries
}

fn normalize_status(status: &Option<String>) -> String {
    match status.as_deref() {
        Some("monitoring") | Some("active") | Some("running") => "active".to_string(),
        Some("paused") => "paused".to_string(),
        Some(s) => s.to_string(),
        None => "active".to_string(),
    }
}

fn parse_targets_md(content: &str) -> Vec<TargetJson> {
    // Extract YAML frontmatter between --- markers
    let yaml_str = if content.starts_with("---\n") || content.starts_with("---\r\n") {
        let after_first = content.find("---\n").map(|i| i + 4)
            .or_else(|| content.find("---\r\n").map(|i| i + 5));
        if let Some(start) = after_first {
            let remaining = &content[start..];
            if let Some(end) = remaining.find("\n---") {
                Some(&remaining[..end])
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let yaml_content = match yaml_str {
        Some(y) => y,
        None => return Vec::new(),
    };

    let parsed: Result<TargetsYaml, _> = serde_yaml::from_str(yaml_content);
    let targets_raw = match parsed {
        Ok(t) => t.targets,
        Err(e) => {
            ts_eprint!("[Radar] Failed to parse YAML frontmatter: {}", e);
            return Vec::new();
        }
    };

    targets_raw.into_iter().map(|t| {
        let spec_entries = t.spec.as_ref().map(|v| convert_spec(v)).unwrap_or_default();
        let spec_label = spec_entries.iter()
            .map(|e| e.value.as_str())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        
        let channels: Vec<ChannelJson> = t.channels.map(|chs| {
            chs.into_iter().map(|c| ChannelJson {
                channel_type: c.channel_type,
                location: c.location,
            }).collect()
        }).unwrap_or_default();
        
        let channel_label = channels.iter()
            .map(|c| c.channel_type.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        
        TargetJson {
            id: t.id,
            name: t.name,
            description: t.description,
            spec_entries,
            spec_label,
            channels,
            channel_label,
            source_probe: t.source_probe,
            status: normalize_status(&t.status),
            created_at: t.created_at,
            last_scan: t.last_scan,
        }
    }).collect()
}

/// Ensure the radar directory tree exists for an agent.
async fn ensure_radar_dir(workspace: &PathBuf) {
    tokio::fs::create_dir_all(workspace.join("radar").join("logs")).await.ok();
}

/// POST /api/radar/{agent}/init
pub async fn init(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let probes = workspace.join("radar").join("probes.md");
    if !probes.exists() {
        let content = "# Probes\n\nInitialized at ".to_string()
            + &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            + "\n\nThis file lists all active probes.\n";
        tokio::fs::write(&probes, content).await.ok();
    }

    let targets_md = workspace.join("radar").join("targets.md");
    if !targets_md.exists() {
        let content = "# Targets\n\nInitialized at ".to_string()
            + &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            + "\n\nThis file lists all known targets.\n";
        tokio::fs::write(&targets_md, content).await.ok();
    }

    ok_response(&serde_json::json!({
        "message": "Radar initialized",
        "path": workspace.join("radar").to_string_lossy().to_string()
    }))
}

/// GET /api/radar/{agent}/targets-md
pub async fn get_targets_md(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let path = workspace.join("radar").join("targets.md");
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from(content)))
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from("# Targets\n\nNo targets configured yet.\n")))
                .unwrap()
        }
    }
}

/// GET /api/radar/{agent}/targets-json
/// Returns parsed targets from YAML frontmatter as structured JSON.
pub async fn get_targets_json(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let path = workspace.join("radar").join("targets.md");
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => {
            return ok_response(&serde_json::json!({ "targets": Vec::<TargetJson>::new() }));
        }
    };

    let targets = parse_targets_md(&content);
    ok_response(&serde_json::json!({ "targets": targets }))
}

/// GET /api/radar/{agent}/probes
pub async fn get_probes(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let path = workspace.join("radar").join("probes.md");
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from(content)))
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from("# Probes\n\nNo probes configured yet.\n")))
                .unwrap()
        }
    }
}

#[derive(serde::Serialize)]
struct LogEntry {
    name: String,
    log_type: String,
    time: String,
}

/// GET /api/radar/{agent}/logs
pub async fn list_logs(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let logs_dir = workspace.join("radar").join("logs");
    ensure_radar_dir(&workspace).await;

    let mut logs: Vec<LogEntry> = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&logs_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".log") {
                continue;
            }
            let log_type = if name.starts_with("probe-") { "probe".to_string() }
                else if name.starts_with("scan-") { "scan".to_string() }
                else { "other".to_string() };

            let time = entry.metadata().await.ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| {
                    let secs = d.as_secs();
                    let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
                        .unwrap_or_default();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();

            logs.push(LogEntry { name, log_type, time });
        }
    }

    logs.sort_by(|a, b| b.time.cmp(&a.time));

    ok_response(&serde_json::json!({
        "agent": agent_name,
        "logs": logs
    }))
}

/// GET /api/radar/{agent}/logs/{filename}
pub async fn get_log(data_dir: &str, agent_name: &str, filename: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Invalid filename");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let log_path = workspace.join("radar").join("logs").join(filename);
    match tokio::fs::read_to_string(&log_path).await {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(box_body(Bytes::from(content)))
                .unwrap()
        }
        Err(_) => error_response(StatusCode::NOT_FOUND, "Log not found"),
    }
}

/// Route dispatcher for /api/radar/* requests.
pub async fn route(
    data_dir: &str,
    path: &str,
    method: &hyper::Method,
    _req: hyper::Request<Incoming>,
) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
    let rest = path.strip_prefix("/api/radar/")?;
    let segments: Vec<&str> = rest.split('/').collect();
    if segments.len() < 2 {
        return None;
    }

    let agent_encoded = segments[0];
    let agent = urlencoding::decode(agent_encoded).unwrap_or_else(|_| agent_encoded.into()).to_string();
    let action = segments[1];

    match action {
        "init" if method == hyper::Method::POST => {
            Some(init(data_dir, &agent).await)
        }
        "targets-md" if method == hyper::Method::GET => {
            Some(get_targets_md(data_dir, &agent).await)
        }
        "targets-json" if method == hyper::Method::GET => {
            Some(get_targets_json(data_dir, &agent).await)
        }
        "probes" if method == hyper::Method::GET => {
            Some(get_probes(data_dir, &agent).await)
        }
        "logs" if method == hyper::Method::GET && segments.len() == 2 => {
            Some(list_logs(data_dir, &agent).await)
        }
        "logs" if segments.len() >= 3 => {
            let filename = segments[2];
            if method == hyper::Method::GET {
                Some(get_log(data_dir, &agent, filename).await)
            } else {
                None
            }
        }
        _ => None,
    }
}
