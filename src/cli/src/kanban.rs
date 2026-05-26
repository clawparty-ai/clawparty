use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;

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

// Default kanban config (same as ztm agent)
fn default_kanban_config(agent_name: &str, group_id: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "agent_name": agent_name,
        "group_id": group_id,
        "name": "默认看板",
        "prompt": "",
        "config": {
            "charts": [
                { "id": "status", "type": "doughnut", "title": "状态分布", "enabled": true, "prompt": "" },
                { "id": "trend", "type": "line", "title": "近7天趋势", "enabled": true, "prompt": "" },
                { "id": "agent", "type": "bar", "title": "Agent分布", "enabled": true, "prompt": "" },
                { "id": "duration", "type": "bar", "title": "耗时统计", "enabled": true, "prompt": "" }
            ]
        }
    })
}

// ── GET /api/kanban?agent=...&group=... ────────────────────────────────

pub async fn get_kanban(data_dir: &str, agent_name: Option<&str>, group_id: Option<&str>) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);

    let agent_name = match agent_name {
        Some(a) => a,
        None => return error_response(StatusCode::BAD_REQUEST, "agent parameter required"),
    };

    match db::get_kanban_config(data_dir, agent_name, group_id) {
        Ok(Some(config)) => ok_response(&serde_json::json!({
            "agent_name": config.agent_name,
            "group_id": config.group_id,
            "name": config.name,
            "prompt": config.prompt,
            "config": config.config,
            "created_at": config.created_at,
            "updated_at": config.updated_at,
        })),
        Ok(None) => ok_response(&default_kanban_config(agent_name, group_id)),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── PUT /api/kanban ────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct UpdateKanbanRequest {
    agent_name: String,
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    config: Option<serde_json::Value>,
}

pub async fn update_kanban(data_dir: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);

    let req: UpdateKanbanRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    if req.agent_name.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "agent_name is required");
    }

    match db::set_kanban_config(
        data_dir,
        &req.agent_name,
        req.group_id.as_deref(),
        req.name.as_deref(),
        req.prompt.as_deref(),
        req.config.as_ref(),
    ) {
        Ok(config) => ok_response(&serde_json::json!({
            "agent_name": config.agent_name,
            "group_id": config.group_id,
            "name": config.name,
            "prompt": config.prompt,
            "config": config.config,
            "updated_at": config.updated_at,
        })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}
