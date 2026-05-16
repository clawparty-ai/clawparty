use std::collections::HashMap;
use std::path::PathBuf;

use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;
use regex::Regex;

use crate::proxy::box_body;

/// Get the workspace directory for an agent from the ZTM DB.
fn get_agent_workspace(data_dir: &str, agent_name: &str) -> anyhow::Result<PathBuf> {
    let db_path = format!("{}/ztm.db", data_dir);
    let conn = rusqlite::Connection::open(&db_path)?;
    let dir: String = conn.query_row(
        "SELECT workspace_dir FROM agents WHERE agent_name = ?1 AND deleted = 0",
        [agent_name],
        |row| row.get(0),
    )?;
    Ok(PathBuf::from(dir))
}

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

/// POST /api/wiki/{agent}/init
pub async fn init(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let wiki_dir = workspace.join("wiki");
    let dirs = ["raw", "entities", "concepts", "pages"];

    for d in &dirs {
        let _ = tokio::fs::create_dir_all(wiki_dir.join(d)).await;
    }

    let index_path = wiki_dir.join("index.md");
    if !index_path.exists() {
        let content = "# Wiki 目录\n\nWiki 初始化于 ".to_string()
            + &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            + "\n";
        let _ = tokio::fs::write(&index_path, content).await;
    }

    let log_path = wiki_dir.join("log.md");
    if !log_path.exists() {
        let _ = tokio::fs::write(&log_path, "# Wiki 日志\n\n").await;
    }

    let schema_path = wiki_dir.join("schema.md");
    if !schema_path.exists() {
        let schema = r#"# Wiki 维护规范

## 目录结构

- `raw/` — 原始资料（不可变）
- `entities/` — 实体页面（人物、地点、事物）
- `concepts/` — 概念页面（抽象主题）
- `pages/` — 通用页面

## 链接规范

使用 `[[Page Name]]` 创建内部链接。

## 维护要求

1. 每次处理新资料后更新 `index.md`
2. 在 `log.md` 中记录变更
3. 保持交叉引用一致
"#;
        let _ = tokio::fs::write(&schema_path, schema).await;
    }

    ok_response(&serde_json::json!({
        "message": "Wiki initialized",
        "path": wiki_dir.to_string_lossy()
    }))
}

/// GET /api/wiki/{agent}/tree?path={subPath}
pub async fn tree(data_dir: &str, agent_name: &str, sub_path: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let wiki_dir = workspace.join("wiki");
    let target_dir = if sub_path.is_empty() {
        wiki_dir.clone()
    } else {
        wiki_dir.join(sub_path)
    };

    // Security: prevent directory traversal
    if !target_dir.starts_with(&wiki_dir) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let mut files = Vec::new();

    match tokio::fs::read_dir(&target_dir).await {
        Ok(mut entries) => {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let meta = entry.metadata().await.ok();
                if let Some(m) = meta {
                    if m.is_dir() {
                        files.push(serde_json::json!({
                            "name": name,
                            "size": 0,
                            "mtime": 0,
                            "type": "dir"
                        }));
                    } else {
                        files.push(serde_json::json!({
                            "name": name,
                            "size": m.len(),
                            "mtime": m.modified().ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                            "type": "file"
                        }));
                    }
                }
            }
        }
        Err(_) => {}
    }

    ok_response(&serde_json::json!({
        "agent": agent_name,
        "path": sub_path,
        "files": files
    }))
}

/// GET /api/wiki/{agent}/file/{name}?path={subPath}
pub async fn file(data_dir: &str, agent_name: &str, name: &str, sub_path: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let wiki_dir = workspace.join("wiki");

    // Security checks
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden filename");
    }
    if sub_path.contains("..") {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let file_path = if sub_path.is_empty() {
        wiki_dir.join(name)
    } else {
        wiki_dir.join(sub_path).join(name)
    };

    if !file_path.starts_with(&wiki_dir) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .body(box_body(Bytes::from(content)))
                .unwrap()
        }
        Err(_) => error_response(StatusCode::NOT_FOUND, "File not found"),
    }
}

/// GET /api/wiki/{agent}/search?q={query}
pub async fn search(data_dir: &str, agent_name: &str, query: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let wiki_dir = workspace.join("wiki");
    let q = query.to_lowercase();
    if q.is_empty() {
        return ok_response(&serde_json::json!({ "results": [] }));
    }

    let mut results = Vec::new();
    let mut dirs = vec![wiki_dir.clone()];

    while let Some(dir) = dirs.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let meta = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };

            if meta.is_dir() {
                dirs.push(path);
            } else if name.ends_with(".md") {
                let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
                let file_rel = path.strip_prefix(&wiki_dir).unwrap_or(&path).to_string_lossy().to_string();
                let mut title = name.trim_end_matches(".md").to_string();

                for line in content.lines() {
                    if line.starts_with("# ") {
                        title = line[2..].trim().to_string();
                        break;
                    }
                }

                if title.to_lowercase().contains(&q) || content.to_lowercase().contains(&q) {
                    let mut preview = String::new();
                    for line in content.lines() {
                        if line.to_lowercase().contains(&q) {
                            preview = line.trim().chars().take(150).collect();
                            break;
                        }
                    }
                    results.push(serde_json::json!({
                        "name": name,
                        "path": file_rel,
                        "title": title,
                        "preview": preview
                    }));
                }
            }
        }
    }

    ok_response(&serde_json::json!({
        "query": query,
        "results": results
    }))
}

/// GET /api/wiki/{agent}/graph
pub async fn graph(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let wiki_dir = workspace.join("wiki");
    let mut nodes: Vec<serde_json::Value> = Vec::new();
    let mut links: Vec<serde_json::Value> = Vec::new();
    let mut node_map: HashMap<String, usize> = HashMap::new();
    let mut node_index = 0usize;

    let wiki_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let md_link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+\.md)\)").unwrap();

    let mut get_node_id = |name: &str, category: &str| -> usize {
        let key = format!("{}/{}", category, name);
        if let Some(&id) = node_map.get(&key) {
            return id;
        }
        let id = node_index;
        node_map.insert(key, id);
        nodes.push(serde_json::json!({
            "id": id,
            "name": name,
            "category": category
        }));
        node_index += 1;
        id
    };

    let mut dirs = vec![(wiki_dir.clone(), "page")];
    while let Some((dir, category)) = dirs.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let meta = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };

            if meta.is_dir() {
                let sub_category = match name.as_str() {
                    "raw" => "raw",
                    "entities" => "entity",
                    "concepts" => "concept",
                    "pages" => "page",
                    _ => category,
                };
                dirs.push((path, sub_category));
            } else if name.ends_with(".md") {
                let page_name = name.trim_end_matches(".md");
                let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
                if content.is_empty() {
                    continue;
                }
                let page_id = get_node_id(page_name, category);

                // Parse [[WikiLink]]
                for cap in wiki_re.captures_iter(&content) {
                    if let Some(m) = cap.get(1) {
                        let target_name = m.as_str().trim();
                        let target_id = get_node_id(target_name, "page");
                        links.push(serde_json::json!({
                            "source": page_id,
                            "target": target_id
                        }));
                    }
                }

                // Parse [text](path.md)
                for cap in md_link_re.captures_iter(&content) {
                    if let Some(m) = cap.get(2) {
                        let target_path = m.as_str().trim();
                        let target_name = target_path.split('/').last()
                            .unwrap_or(target_path)
                            .trim_end_matches(".md");
                        let target_id = get_node_id(target_name, "page");
                        links.push(serde_json::json!({
                            "source": page_id,
                            "target": target_id
                        }));
                    }
                }
            }
        }
    }

    ok_response(&serde_json::json!({
        "nodes": nodes,
        "links": links
    }))
}

/// POST /api/wiki/{agent}/refresh
pub async fn refresh(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let wiki_dir = workspace.join("wiki");

    // Just verify the wiki directory exists
    match tokio::fs::metadata(&wiki_dir).await {
        Ok(_) => {}
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Wiki not initialized"),
    }

    ok_response(&serde_json::json!({
        "message": "Wiki refreshed",
        "agent": agent_name
    }))
}

/// POST /api/wiki/{agent}/upload?name={filename}
/// Upload a file to wiki/raw/ directory.
pub async fn upload_raw(data_dir: &str, agent_name: &str, filename: &str, data: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    // Security checks
    if filename.is_empty() || filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden filename");
    }

    let raw_dir = workspace.join("wiki").join("raw");

    // Ensure raw directory exists
    if let Err(e) = tokio::fs::create_dir_all(&raw_dir).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to create raw dir: {}", e));
    }

    let file_path = raw_dir.join(filename);

    // Ensure still within raw dir after join
    if !file_path.starts_with(&raw_dir) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    match tokio::fs::write(&file_path, data).await {
        Ok(_) => ok_response(&serde_json::json!({
            "message": "File uploaded",
            "filename": filename,
            "path": format!("raw/{}", filename)
        })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to write file: {}", e)),
    }
}
