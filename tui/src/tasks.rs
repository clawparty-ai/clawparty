use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;

use crate::db;
use crate::proxy::box_body;
use crate::wiki;

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

// ── Batch Refresh: frontend sends AI-analyzed changes, tui persists atomically ──

#[derive(Debug, serde::Deserialize)]
struct BatchRefreshRequest {
    agent_name: String,
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    last_analyzed_at: f64,
    #[serde(default)]
    changes: Vec<TaskChange>,
}

#[derive(Debug, serde::Deserialize)]
struct TaskChange {
    #[serde(rename = "type")]
    change_type: String,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    data: Option<db::TaskUpdate>,
    #[serde(default)]
    new_status: Option<String>,
    #[serde(default)]
    new_progress: Option<i32>,
    #[serde(default)]
    result_summary: Option<String>,
}

/// POST /api/tasks/batch-refresh
/// Frontend sends AI-analyzed changes; tui persists them atomically to clawparty.db
/// and writes TASKS.md.
pub async fn batch_refresh(data_dir: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    // Ensure db is initialized
    if let Err(e) = db::init_clawparty_db(data_dir) {
        eprintln!("[Tasks] Failed to init clawparty.db: {}", e);
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }

    // Parse request
    let req: BatchRefreshRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    let agent_name = &req.agent_name;
    let group_id = req.group_id.as_deref();

    // Open DB
    // Run DB operations in a closure so conn/tx are dropped before any .await
    let db_result: Result<(usize, usize, usize, Vec<String>), String> = (|| {
        let mut conn = rusqlite::Connection::open(format!("{}/clawparty.db", data_dir))
            .map_err(|e| format!("DB open: {}", e))?;
        let _ = conn.execute_batch("PRAGMA busy_timeout = 5000;");
        let tx = conn.transaction().map_err(|e| format!("Transaction: {}", e))?;

        let mut created = 0usize;
        let mut updated = 0usize;
        let mut summary_updates = 0usize;
        let mut failed_tasks: Vec<String> = Vec::new();

        for change in &req.changes {
            match change.change_type.as_str() {
                "create" => {
                    if let Some(ref data) = change.data {
                        let mut task_data: db::TaskUpdate = (*data).clone();
                        task_data.agent_name = Some(agent_name.clone());
                        task_data.group_id = req.group_id.clone();
                        if task_data.task_id.is_none() {
                            let ts = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs_f64();
                            task_data.task_id = Some(format!("ai-task-{}-{}", ts as i64, rand::random::<u16>()));
                        }
                        if task_data.status.is_none() { task_data.status = Some("pending".to_string()); }
                        if task_data.progress.is_none() { task_data.progress = Some(0); }
                        if task_data.priority.is_none() { task_data.priority = Some("normal".to_string()); }

                        match db::create_task(&tx, &task_data) {
                            Ok(_) => created += 1,
                            Err(e) => { eprintln!("[Tasks] Create failed: {}", e); failed_tasks.push(format!("create: {}", e)); }
                        }
                    }
                }
                "update" => {
                    if let Some(task_id) = &change.task_id {
                        let mut updates = db::TaskUpdate::default();
                        if let Some(st) = &change.new_status { updates.status = Some(st.clone()); }
                        if let Some(pr) = change.new_progress { updates.progress = Some(pr); }
                        if let Some(sm) = &change.result_summary { updates.result_summary = Some(sm.clone()); }
                        if let Some(data) = &change.data {
                            if data.status.is_some() { updates.status = data.status.clone(); }
                            if data.progress.is_some() { updates.progress = data.progress; }
                            if data.result_summary.is_some() { updates.result_summary = data.result_summary.clone(); }
                            if data.prompt.is_some() { updates.prompt = data.prompt.clone(); }
                            if data.description.is_some() { updates.description = data.description.clone(); }
                        }
                        match db::update_task(&tx, task_id, &updates) {
                            Ok(Some(_)) => updated += 1,
                            Ok(None) => { failed_tasks.push(format!("update {}: not found", task_id)); }
                            Err(e) => { eprintln!("[Tasks] Update failed: {}", e); failed_tasks.push(format!("update {}: {}", task_id, e)); }
                        }
                    }
                }
                "summary" => {
                    if let Some(task_id) = &change.task_id {
                        if let Some(summary) = &change.result_summary {
                            let upd = db::TaskUpdate { result_summary: Some(summary.clone()), ..Default::default() };
                            if let Ok(Some(_)) = db::update_task(&tx, task_id, &upd) { summary_updates += 1; }
                        }
                    }
                }
                _ => { eprintln!("[Tasks] Unknown change type: {}", change.change_type); }
            }
        }

        if let Err(e) = db::set_analysis_log(&tx, agent_name, group_id, req.last_analyzed_at) {
            eprintln!("[Tasks] Failed to set analysis log: {}", e);
        }

        tx.commit().map_err(|e| format!("Commit failed: {}", e))?;
        Ok((created, updated, summary_updates, failed_tasks))
    })();

    let (created, updated, summary_updates, failed_tasks) = match db_result {
        Ok(r) => r,
        Err(msg) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &msg),
    };

    // Write TASKS.md
    let task_save_result = save_tasks_md(data_dir, agent_name, group_id).await;
    let tasks_saved = task_save_result.is_ok();
    if let Err(e) = &task_save_result {
        eprintln!("[Tasks] Failed to save TASKS.md: {}", e);
    }

    ok_response(&serde_json::json!({
        "message": "Tasks refreshed",
        "agent_name": agent_name,
        "group_id": group_id,
        "created": created,
        "updated": updated,
        "summaries": summary_updates,
        "failed": failed_tasks.len(),
        "failed_details": failed_tasks,
        "tasks_saved": tasks_saved,
        "last_analyzed_at": req.last_analyzed_at,
    }))
}

/// Generate TASKS.md and write to agent workspace.
async fn save_tasks_md(data_dir: &str, agent_name: &str, group_id: Option<&str>) -> anyhow::Result<()> {
    let tasks = db::get_tasks(data_dir, agent_name, group_id)?;
    if tasks.is_empty() {
        return Ok(());
    }

    let now = chrono::Local::now();
    let ts = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let total = 0usize;
    let completed = 0usize;
    let running = 0usize;
    let pending = 0usize;
    let failed = 0usize;

    fn count_status(list: &[db::Task], totals: &mut (usize, usize, usize, usize, usize)) {
        for t in list {
            totals.0 += 1;
            match t.status.as_str() {
                "completed" => totals.1 += 1,
                "running" => totals.2 += 1,
                "pending" => totals.3 += 1,
                "failed" => totals.4 += 1,
                _ => {}
            }
            count_status(&t.subtasks, totals);
        }
    }
    count_status(&tasks, &mut (total, completed, running, pending, failed));

    fn render_tasks(list: &[db::Task], depth: usize) -> String {
        let indent = "  ".repeat(depth);
        let mut md = String::new();
        for t in list {
            let checkbox = if t.status == "completed" { "[x]" } else { "[ ]" };
            md.push_str(&format!("{}- {} #{} {} ({}%{}%)\n",
                indent,
                checkbox,
                t.task_number.unwrap_or(0),
                t.title,
                t.status,
                t.progress
            ));
            if let Some(desc) = &t.description {
                let clean = desc.replace('\n', " ").chars().take(200).collect::<String>();
                md.push_str(&format!("{}  - 描述: {}\n", indent, clean));
            }
            if let Some(summary) = &t.result_summary {
                let clean = summary.replace('\n', " ").chars().take(200).collect::<String>();
                md.push_str(&format!("{}  - 结果: {}\n", indent, clean));
            }
            if !t.subtasks.is_empty() {
                md.push_str(&render_tasks(&t.subtasks, depth + 1));
            }
        }
        md
    }

    let gid_str = group_id.map(|g| format!(" (group={})", g)).unwrap_or_default();
    let content = format!(
        "# 任务清单 for {}{}\n\n生成时间: {}\n\n## 概览\n- 总计: {}\n- 已完成: {}\n- 进行中: {}\n- 待开始: {}\n- 失败: {}\n\n## 任务列表\n\n{}\n",
        agent_name, gid_str, ts, total, completed, running, pending, failed, render_tasks(&tasks, 0)
    );

    // Write to workspace via existing save_workspace_file logic
    wiki::save_workspace_file(data_dir, agent_name, "TASKS.md", Bytes::from(content)).await;

    Ok(())
}

// ── Standalone Task CRUD APIs (for frontend to use clawparty.db instead of ztm.db) ──

/// GET /api/tasks?agent={name}&group={id}
pub async fn list_tasks(data_dir: &str, agent_name: &str, group_id: Option<&str>) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Err(e) = db::init_clawparty_db(data_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }
    match db::get_tasks(data_dir, agent_name, group_id) {
        Ok(tasks) => ok_response(&serde_json::json!({
            "agent_name": agent_name,
            "group_id": group_id,
            "tasks": tasks,
        })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

/// GET /api/tasks/{taskId}
pub async fn get_task(data_dir: &str, task_id: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Err(e) = db::init_clawparty_db(data_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }
    let db_path = format!("{}/clawparty.db", data_dir);
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB open: {}", e)),
    };
    match db::get_task(&conn, task_id) {
        Ok(Some(t)) => ok_response(&t),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "Task not found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

/// POST /api/tasks
pub async fn create_task(data_dir: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Err(e) = db::init_clawparty_db(data_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }
    let update: db::TaskUpdate = match serde_json::from_slice(&body_bytes) {
        Ok(u) => u,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };
    let db_path = format!("{}/clawparty.db", data_dir);
    let mut conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB open: {}", e)),
    };
    let tx = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Transaction: {}", e)),
    };
    match db::create_task(&tx, &update) {
        Ok(task) => {
            let _ = tx.commit();
            json_response(StatusCode::CREATED, &task)
        }
        Err(e) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Create failed: {}", e))
        }
    }
}

/// PUT /api/tasks/{taskId}
pub async fn update_task(data_dir: &str, task_id: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Err(e) = db::init_clawparty_db(data_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }
    let update: db::TaskUpdate = match serde_json::from_slice(&body_bytes) {
        Ok(u) => u,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };
    let db_path = format!("{}/clawparty.db", data_dir);
    let mut conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB open: {}", e)),
    };
    let tx = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Transaction: {}", e)),
    };
    match db::update_task(&tx, task_id, &update) {
        Ok(Some(task)) => {
            let _ = tx.commit();
            ok_response(&task)
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "Task not found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Update failed: {}", e)),
    }
}

/// DELETE /api/tasks/{taskId}
pub async fn delete_task(data_dir: &str, task_id: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Err(e) = db::init_clawparty_db(data_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }
    let db_path = format!("{}/clawparty.db", data_dir);
    let mut conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB open: {}", e)),
    };
    let tx = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Transaction: {}", e)),
    };
    match db::delete_task_cascade(&tx, task_id) {
        Ok(true) => {
            let _ = tx.commit();
            let mut resp = Response::new(box_body(Bytes::new()));
            *resp.status_mut() = StatusCode::NO_CONTENT;
            resp
        }
        Ok(false) => error_response(StatusCode::NOT_FOUND, "Task not found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Delete failed: {}", e)),
    }
}

/// GET /api/task/analysis?agent={name}&group={id}
pub async fn get_analysis_log(data_dir: &str, agent_name: &str, group_id: Option<&str>) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Err(e) = db::init_clawparty_db(data_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }
    match db::get_analysis_log(data_dir, agent_name, group_id) {
        Ok(ts) => ok_response(&serde_json::json!({
            "agent_name": agent_name,
            "group_id": group_id,
            "last_analyzed_at": ts,
        })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB error: {}", e)),
    }
}

/// PUT /api/task/analysis
pub async fn set_analysis_log(data_dir: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    if let Err(e) = db::init_clawparty_db(data_dir) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB init failed: {}", e));
    }
    let req: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };
    let agent_name = req["agent_name"].as_str().unwrap_or("");
    let group_id = req["group_id"].as_str();
    let ts = req["last_analyzed_at"].as_f64().unwrap_or(0.0);

    let db_path = format!("{}/clawparty.db", data_dir);
    let mut conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("DB open: {}", e)),
    };
    let tx = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Transaction: {}", e)),
    };
    match db::set_analysis_log(&tx, agent_name, group_id, ts) {
        Ok(_) => {
            let _ = tx.commit();
            ok_response(&serde_json::json!({ "last_analyzed_at": ts }))
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Set failed: {}", e)),
    }
}
