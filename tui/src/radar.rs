use std::path::PathBuf;

use hyper::body::{Bytes, Incoming};
use hyper::{Response, StatusCode, header};
use http_body_util::BodyExt;
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
    tokio::fs::create_dir_all(workspace.join("radar").join("targets")).await.ok();
    tokio::fs::create_dir_all(workspace.join("radar").join("scans")).await.ok();
}

/// POST /api/radar/{agent}/init
pub async fn init(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let scans_index = workspace.join("radar").join("scans").join("index.md");
    if !scans_index.exists() {
        let content = "# 扫描总览\n\n初始化于 ".to_string()
            + &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            + "\n\n该目录下放置搜索条件（扫描配置）文件。\n";
        tokio::fs::write(&scans_index, content).await.ok();
    }

    let discoveries = workspace.join("radar").join("discoveries.md");
    if !discoveries.exists() {
        let content = "# 发现日志\n\n初始化于 ".to_string()
            + &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            + "\n\n记录通过扫描发现的新目标候选。\n";
        tokio::fs::write(&discoveries, content).await.ok();
    }

    ok_response(&serde_json::json!({
        "message": "Radar initialized",
        "path": workspace.join("radar").to_string_lossy().to_string()
    }))
}

#[derive(serde::Serialize)]
struct TargetEntry {
    name: String,
    status: String,
    created_at: String,
    last_log_at: String,
    log_entries: usize,
}

#[derive(serde::Serialize)]
struct TargetListResponse {
    agent: String,
    targets: Vec<TargetEntry>,
}

/// Parse YAML frontmatter from a markdown string. Returns a simple key-value map.
fn parse_frontmatter(content: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let content = content.trim();
    if !content.starts_with("---") {
        return map;
    }
    if let Some(end) = content[3..].find("---").map(|i| i + 3) {
        let front = &content[3..3 + end - 3];
        for line in front.lines() {
            if let Some(eq) = line.find(':') {
                let key = line[..eq].trim().to_string();
                let value = line[eq + 1..].trim().to_string();
                map.insert(key, value);
            }
        }
    }
    map
}

/// GET /api/radar/{agent}/targets
pub async fn list_targets(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let targets_dir = workspace.join("radar").join("targets");
    ensure_radar_dir(&workspace).await;

    let mut targets = Vec::new();
    let mut read_dir = match tokio::fs::read_dir(&targets_dir).await {
        Ok(rd) => rd,
        Err(_) => {
            return ok_response(&TargetListResponse {
                agent: agent_name.to_string(),
                targets: vec![],
            });
        }
    };

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        if !entry.metadata().await.map(|m| m.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let info_path = entry.path().join("info.md");
        let log_path = entry.path().join("log.md");

        let mut status = "active".to_string();
        let mut created_at = String::new();
        if let Ok(content) = tokio::fs::read_to_string(&info_path).await {
            let fm = parse_frontmatter(&content);
            if let Some(s) = fm.get("status") {
                status = s.clone();
            }
            if let Some(c) = fm.get("created_at") {
                created_at = c.clone();
            }
        }

        let last_log_at = tokio::fs::metadata(&log_path).await
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                let secs = d.as_secs();
                let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
                    .unwrap_or_default();
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_default();

        let log_entries = tokio::fs::read_to_string(&log_path).await
            .map(|c| c.lines().filter(|l| l.starts_with("## ")).count())
            .unwrap_or(0);

        targets.push(TargetEntry {
            name,
            status,
            created_at,
            last_log_at,
            log_entries,
        });
    }

    // Sort by name for deterministic order
    targets.sort_by(|a, b| a.name.cmp(&b.name));

    ok_response(&TargetListResponse {
        agent: agent_name.to_string(),
        targets,
    })
}

/// GET /api/radar/{agent}/targets/{name}
pub async fn get_target_info(data_dir: &str, agent_name: &str, target_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if target_name.contains("..") || target_name.contains('/') || target_name.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Invalid target name");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let info_path = workspace.join("radar").join("targets").join(target_name).join("info.md");
    match tokio::fs::read_to_string(&info_path).await {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from(content)))
                .unwrap()
        }
        Err(_) => error_response(StatusCode::NOT_FOUND, "Target not found"),
    }
}

/// GET /api/radar/{agent}/targets/{name}/log
pub async fn get_target_log(data_dir: &str, agent_name: &str, target_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if target_name.contains("..") || target_name.contains('/') || target_name.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Invalid target name");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let log_path = workspace.join("radar").join("targets").join(target_name).join("log.md");
    match tokio::fs::read_to_string(&log_path).await {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from(content)))
                .unwrap()
        }
        Err(_) => error_response(StatusCode::NOT_FOUND, "Log not found"),
    }
}

/// POST /api/radar/{agent}/targets/{name}
pub async fn create_target(data_dir: &str, agent_name: &str, target_name: &str, body: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    if target_name.contains("..") || target_name.contains('/') || target_name.contains('\\') || target_name.starts_with('.') {
        return error_response(StatusCode::FORBIDDEN, "Invalid target name");
    }
    if target_name.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Target name is required");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let target_dir = workspace.join("radar").join("targets").join(target_name);
    if target_dir.exists() {
        // Update existing: write body as info.md
        tokio::fs::write(target_dir.join("info.md"), &body).await.ok();
        return ok_response(&serde_json::json!({ "message": "Target updated", "name": target_name }));
    }

    // Create new target
    tokio::fs::create_dir_all(&target_dir).await.ok();

    // If body is empty, generate default info.md
    if body.is_empty() {
        let default_info = format!(
            "---\nname: {name}\nstatus: active\ncreated_at: {now}\n---\n\n# {name}\n\n",
            name = target_name,
            now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z")
        );
        tokio::fs::write(target_dir.join("info.md"), &default_info).await.ok();
    } else {
        tokio::fs::write(target_dir.join("info.md"), &body).await.ok();
    }

    // Create empty log.md
    let log_header = format!("# 监控日志: {}\n\n", target_name);
    tokio::fs::write(target_dir.join("log.md"), &log_header).await.ok();

    ok_response(&serde_json::json!({ "message": "Target created", "name": target_name }))
}

/// DELETE /api/radar/{agent}/targets/{name}
pub async fn delete_target(data_dir: &str, agent_name: &str, target_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if target_name.contains("..") || target_name.contains('/') || target_name.contains('\\') || target_name.starts_with('.') {
        return error_response(StatusCode::FORBIDDEN, "Invalid target name");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let target_dir = workspace.join("radar").join("targets").join(target_name);
    if !target_dir.exists() {
        return error_response(StatusCode::NOT_FOUND, "Target not found");
    }

    tokio::fs::remove_dir_all(&target_dir).await.ok();
    ok_response(&serde_json::json!({ "message": "Target deleted", "name": target_name }))
}

/// GET /api/radar/{agent}/scans (placeholder)
pub async fn list_scans(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let scans_dir = workspace.join("radar").join("scans");
    ensure_radar_dir(&workspace).await;

    let mut files: Vec<String> = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&scans_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") && name != "index.md" {
                files.push(name.trim_end_matches(".md").to_string());
            }
        }
    }
    files.sort();

    ok_response(&serde_json::json!({
        "agent": agent_name,
        "scans": files
    }))
}

/// GET /api/radar/{agent}/discoveries (placeholder)
pub async fn get_discoveries(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let path = workspace.join("radar").join("discoveries.md");
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from(content)))
                .unwrap()
        }
        Err(_) => {
            // Return empty discoveries
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from("# 发现日志\n\n暂无发现记录。\n")))
                .unwrap()
        }
    }
}

/// Route dispatcher for /api/radar/* requests.
pub async fn route(
    data_dir: &str,
    path: &str,
    method: &hyper::Method,
    req: hyper::Request<Incoming>,
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
        "targets" if method == hyper::Method::GET && segments.len() == 2 => {
            Some(list_targets(data_dir, &agent).await)
        }
        "targets" if segments.len() >= 3 => {
            let target_name_encoded = segments[2];
            let target_name = urlencoding::decode(target_name_encoded).unwrap_or_else(|_| target_name_encoded.into()).to_string();

            // /targets/{name}/log
            if segments.len() >= 4 && segments[3] == "log" {
                if method == hyper::Method::GET {
                    return Some(get_target_log(data_dir, &agent, &target_name).await);
                }
                return None;
            }

            // /targets/{name}
            match method {
                &hyper::Method::GET => Some(get_target_info(data_dir, &agent, &target_name).await),
                &hyper::Method::POST => {
                    let body_bytes = match req.collect().await {
                        Ok(body) => body.to_bytes(),
                        Err(_) => return Some(error_response(StatusCode::BAD_REQUEST, "Failed to read body")),
                    };
                    Some(create_target(data_dir, &agent, &target_name, body_bytes).await)
                }
                &hyper::Method::DELETE => Some(delete_target(data_dir, &agent, &target_name).await),
                _ => None,
            }
        }
        "scans" if method == hyper::Method::GET => {
            Some(list_scans(data_dir, &agent).await)
        }
        "discoveries" if method == hyper::Method::GET => {
            Some(get_discoveries(data_dir, &agent).await)
        }
        _ => None,
    }
}
