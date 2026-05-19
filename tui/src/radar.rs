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
