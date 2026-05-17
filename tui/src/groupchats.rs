use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;

use crate::db;
use crate::db::GroupChatRecord;
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

// ── GET /api/groupchats ─────────────────────────────────────────────────

pub async fn list_group_chats(data_dir: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    match db::list_group_chats(data_dir) {
        Ok(chats) => ok_response(&chats),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── POST /api/groupchats ────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct CreateGroupChatRequest {
    group_id: String,
    group_name: String,
    #[serde(default)]
    owner_agent: String,
    #[serde(default)]
    members: Vec<String>,
}

pub async fn create_group_chat(data_dir: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    let req: CreateGroupChatRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    if req.group_id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "group_id is required");
    }
    if req.group_name.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "group_name is required");
    }
    if req.owner_agent.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "owner_agent is required");
    }
    if req.members.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "members is required (at least one)");
    }

    match db::create_group_chat(data_dir, &req.group_id, &req.group_name, &req.owner_agent, &req.members) {
        Ok(gc) => json_response(StatusCode::CREATED, &gc),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── GET /api/groupchats/{id} ────────────────────────────────────────────

pub async fn get_group_chat(data_dir: &str, group_id: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    match db::get_group_chat(data_dir, group_id) {
        Ok(Some(gc)) => ok_response(&gc),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "Group chat not found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── PUT /api/groupchats/{id} ────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct UpdateGroupChatRequest {
    #[serde(default)]
    group_name: Option<String>,
    #[serde(default)]
    members: Option<Vec<String>>,
}

pub async fn update_group_chat(data_dir: &str, group_id: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    let req: UpdateGroupChatRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    match db::update_group_chat(
        data_dir,
        group_id,
        req.group_name.as_deref(),
        req.members.as_deref(),
    ) {
        Ok(()) => ok_response(&serde_json::json!({ "status": "updated", "group_id": group_id })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── DELETE /api/groupchats/{id} ────────────────────────────────────────

pub async fn delete_group_chat(data_dir: &str, group_id: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    match db::delete_group_chat(data_dir, group_id) {
        Ok(()) => ok_response(&serde_json::json!({ "status": "deleted", "group_id": group_id })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── GET /api/groupchats/{id}/members ────────────────────────────────────

pub async fn get_members(data_dir: &str, group_id: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    match db::get_group_chat(data_dir, group_id) {
        Ok(Some(gc)) => ok_response(&serde_json::json!({ "members": gc.members })),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "Group chat not found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── POST /api/groupchats/{id}/members ───────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct AddMemberRequest {
    agent_name: String,
}

pub async fn add_member(data_dir: &str, group_id: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    let req: AddMemberRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    let mut gc = match db::get_group_chat(data_dir, group_id) {
        Ok(Some(g)) => g,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Group chat not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    };

    let agent_name = req.agent_name;
    if !gc.members.contains(&agent_name) {
        gc.members.push(agent_name.clone());
        match db::update_group_chat(data_dir, group_id, None, Some(&gc.members)) {
            Ok(()) => ok_response(&serde_json::json!({ "status": "added", "group_id": group_id, "agent": agent_name })),
            Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
        }
    } else {
        ok_response(&serde_json::json!({ "status": "already_member", "group_id": group_id, "agent": agent_name }))
    }
}

// ── DELETE /api/groupchats/{id}/members/{agent} ─────────────────────────

pub async fn remove_member(data_dir: &str, group_id: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);
    let mut gc = match db::get_group_chat(data_dir, group_id) {
        Ok(Some(g)) => g,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Group chat not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    };

    gc.members.retain(|m| m != agent_name);
    match db::update_group_chat(data_dir, group_id, None, Some(&gc.members)) {
        Ok(()) => ok_response(&serde_json::json!({ "status": "removed", "group_id": group_id, "agent": agent_name })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

// ── GET /api/groupchats/{id}/messages ───────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct GroupChatMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "_agentName")]
    agent_name: Option<String>,
}

pub async fn get_messages(data_dir: &str, group_id: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);

    match db::get_group_chat(data_dir, group_id) {
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Group chat not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
        Ok(Some(_gc)) => {}
    }

    let logs = match db::get_chat_log(data_dir, "group_local", group_id, 500, &["user", "response", "system"]) {
        Ok(l) => l,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    };

    let mut messages: Vec<GroupChatMessage> = Vec::new();
    for log in logs.into_iter().rev() {
        let role = if log.msg_type == "user" {
            "user"
        } else if log.msg_type == "system" {
            "system"
        } else {
            "assistant"
        };
        let created_at = if log.time > 0.0 {
            let d = chrono::DateTime::from_timestamp(log.time as i64, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            Some(d)
        } else {
            None
        };
        let agent_name = if log.msg_type == "user" {
            None
        } else {
            Some(log.sender.clone())
        };
        messages.push(GroupChatMessage {
            role: role.to_string(),
            content: log.content.unwrap_or_default(),
            created_at,
            agent_name,
        });
    }

    ok_response(&serde_json::json!({ "messages": messages }))
}

// ── POST /api/groupchats/{id}/messages ──────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct PostMessageRequest {
    #[serde(default)]
    sender: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    msg_type: String,
}

pub async fn post_message(data_dir: &str, group_id: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let _ = db::init_clawparty_db(data_dir);

    let gc = match db::get_group_chat(data_dir, group_id) {
        Ok(Some(g)) => g,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "Group chat not found"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    };

    let req: PostMessageRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    let members_json = serde_json::to_string(&gc.members).unwrap_or_default();
    let msg_type = if req.msg_type.is_empty() { "user" } else { &req.msg_type };

    match db::log_chat(
        data_dir,
        "",
        "group_local",
        group_id,
        Some(&gc.group_name),
        Some(&gc.owner_agent),
        &req.sender,
        "message",
        Some(&req.content),
        Some(&members_json),
        Some(group_id),
        false,
        msg_type,
    ) {
        Ok(()) => Response::builder()
            .status(StatusCode::CREATED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(box_body(Bytes::from(r#"{"status":"created"}"#)))
            .unwrap(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}
