use std::collections::HashMap;
use std::path::PathBuf;

use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;
use regex::Regex;

use crate::proxy::box_body;

/// Get the workspace directory for an agent from the ZTM DB.
/// Falls back to inferring from the agents/ directory if not in DB.
pub(crate) fn get_agent_workspace(data_dir: &str, agent_name: &str) -> anyhow::Result<PathBuf> {
    let db_path = format!("{}/ztm.db", data_dir);
    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
        if let Ok(dir) = conn.query_row(
            "SELECT workspace_dir FROM agents WHERE agent_name = ?1 AND deleted = 0",
            [agent_name],
            |row| row.get::<_, String>(0),
        ) {
            return Ok(PathBuf::from(dir));
        }
    }
    
    // Fallback: check agents/ directory directly
    let agents_dir = PathBuf::from(data_dir).join("agents");
    let agent_dir = agents_dir.join(agent_name);
    if agent_dir.exists() {
        // Prefer workspace/ subdirectory over agent root
        let workspace_dir = agent_dir.join("workspace");
        if workspace_dir.exists() {
            return Ok(workspace_dir);
        }
        return Ok(agent_dir);
    }
    
    // Also check .zeroclaw for 0#Agent (legacy path)
    if agent_name == "0#Agent" {
        let zeroclaw_dir = PathBuf::from(data_dir).join(".zeroclaw");
        if zeroclaw_dir.exists() {
            let zw_workspace = zeroclaw_dir.join("workspace");
            if zw_workspace.exists() {
                return Ok(zw_workspace);
            }
            return Ok(zeroclaw_dir);
        }
    }
    
    anyhow::bail!("Agent '{}' not found in database or filesystem", agent_name)
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
    ts_eprint!("[Wiki::graph] agent={}, wiki_dir={:?}", agent_name, wiki_dir);

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
    let mut scanned_files = 0usize;
    let mut skipped_empty = 0usize;
    let mut table_edge_count = 0usize;
    while let Some((dir, category)) = dirs.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) => {
                ts_eprint!("[Wiki::graph] read_dir failed for {:?}: {}", dir, e);
                continue;
            }
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
                    "characters" => "entity",
                    "bridges" | "episodes" | "plot" => "page",
                    "lore" => "concept",
                    _ => category,
                };
                ts_eprint!("[Wiki::graph] push dir {:?} category={}", path, sub_category);
                dirs.push((path, sub_category));
            } else if name.ends_with(".md") {
                scanned_files += 1;
                let page_name = name.trim_end_matches(".md");
                let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
                if content.is_empty() {
                    skipped_empty += 1;
                    ts_eprint!("[Wiki::graph] skip empty file {:?}", path);
                    continue;
                }
                ts_eprint!("[Wiki::graph] scan {} bytes from {:?}", content.len(), path);
                let page_id = get_node_id(page_name, category);

                // Skip links from index pages (they link to everything and clutter the graph)
                if page_name == "index" {
                    continue;
                }

                // Parse [[WikiLink]]
                for cap in wiki_re.captures_iter(&content) {
                    if let Some(m) = cap.get(1) {
                        let target_name = m.as_str().trim();
                        let target_id = get_node_id(target_name, "page");
                        links.push(serde_json::json!({
                            "source": page_id,
                            "target": target_id,
                            "edge_type": "wiki-link"
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
                            "target": target_id,
                            "edge_type": "md-link"
                        }));
                    }
                }

                // Parse ## 人物关系 table rows as graph edges
                if let Some(rel_start) = content.find("## 人物关系") {
                    let after_header = &content[rel_start + "## 人物关系".len()..];
                    let section_end = after_header.find("\n## ").unwrap_or(after_header.len());
                    let section = &after_header[..section_end];

                    for line in section.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || !trimmed.starts_with('|') {
                            continue;
                        }
                        if trimmed.contains("---|---") || trimmed.contains("------") {
                            continue;
                        }
                        if trimmed.contains("关系") && trimmed.contains("对象") {
                            continue;
                        }

                        let cells: Vec<&str> = trimmed.split('|').collect();
                        if cells.len() < 3 {
                            continue;
                        }
                        let relation_type = cells[1].trim();
                        let target_names = cells[2].trim();

                        if relation_type.is_empty() || target_names.is_empty() {
                            continue;
                        }

                        for target_name in target_names.split('/') {
                            let name = target_name.trim();
                            if name.is_empty() {
                                continue;
                            }
                            let clean_name = name
                                .trim_start_matches("[[")
                                .trim_end_matches("]]")
                                .trim();
                            let clean_name = clean_name
                                .split('\u{ff08}').next().unwrap_or(clean_name)
                                .trim();
                            let target_id = get_node_id(clean_name, "page");
                            links.push(serde_json::json!({
                                "source": page_id,
                                "target": target_id,
                                "edge_type": relation_type
                            }));
                            table_edge_count += 1;
                        }
                    }
                }
            }
        }
    }

    ts_eprint!(
        "[Wiki::graph] done: {} nodes, {} links ({} table-relations, scanned {} files, {} empty)",
        nodes.len(), links.len(), table_edge_count, scanned_files, skipped_empty
    );

    ok_response(&serde_json::json!({
        "nodes": nodes,
        "links": links
    }))
}

/// Helper: convert a single file from raw/ to pages/ using zeroclaw LLM.
async fn convert_file(raw_path: &std::path::Path, pages_dir: &std::path::Path, filename: &str) -> anyhow::Result<String> {
    // Check file size (max 500KB for LLM context)
    let metadata = tokio::fs::metadata(raw_path).await?;
    if metadata.len() > 500 * 1024 {
        anyhow::bail!("File too large for conversion (max 500KB)");
    }

    // Read file content as text
    let content = tokio::fs::read_to_string(raw_path).await?;
    let content = if content.len() > 100_000 {
        content[..100_000].to_string() + "\n\n...[内容过长，已截断]"
    } else {
        content
    };

    let prompt = format!(
        "请将以下文件内容转换为 markdown 格式。保持原有的结构和信息，添加适当的标题、列表、代码块等 markdown 语法。文件名为: {}\n\n```\n{}\n```",
        filename, content
    );

    // Call zeroclaw API
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "message": prompt });
    
    let result = client
        .post("http://127.0.0.1:42617/api/sessions/me/chat")
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to LLM: {}", e))?;

    if !result.status().is_success() {
        let status = result.status();
        let err_body = result.text().await.unwrap_or_default();
        anyhow::bail!("LLM service returned {}: {}", status, err_body);
    }

    let json: serde_json::Value = result.json().await
        .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {}", e))?;
    let md_content = json["response"].as_str().unwrap_or("").to_string();

    if md_content.is_empty() {
        anyhow::bail!("LLM returned empty response");
    }

    // Save as markdown in pages/
    tokio::fs::create_dir_all(pages_dir).await?;
    let md_filename = filename.rsplit_once('.')
        .map(|(name, _)| format!("{}.md", name))
        .unwrap_or_else(|| format!("{}.md", filename));
    let md_path = pages_dir.join(&md_filename);
    tokio::fs::write(&md_path, md_content).await?;

    Ok(md_filename)
}

/// Scan pages/ and build index of all markdown files.
async fn ingest_pages(pages_dir: &std::path::Path, wiki_dir: &std::path::Path) -> anyhow::Result<(Vec<serde_json::Value>, usize)> {
    let mut pages: Vec<serde_json::Value> = Vec::new();
    let mut total_links = 0usize;
    let wiki_link_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    
    if let Ok(mut entries) = tokio::fs::read_dir(pages_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") {
                continue;
            }
            let path = entry.path();
            let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            if content.is_empty() {
                continue;
            }
            
            // Extract title from first H1
            let mut title = name.trim_end_matches(".md").to_string();
            for line in content.lines() {
                if line.starts_with("# ") {
                    title = line[2..].trim().to_string();
                    break;
                }
            }
            
            // Extract summary (first non-empty non-header paragraph)
            let mut summary = String::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("[") {
                    summary = trimmed.chars().take(120).collect();
                    break;
                }
            }
            
            // Count wiki links
            let links: Vec<String> = wiki_link_re.captures_iter(&content)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
                .collect();
            total_links += links.len();
            
            pages.push(serde_json::json!({
                "name": name,
                "title": title,
                "summary": summary,
                "links": links,
                "size": content.len()
            }));
        }
    }
    
    // Save index
    let index_path = wiki_dir.join("index.json");
    let index = serde_json::json!({
        "pages": &pages,
        "total_pages": pages.len(),
        "total_links": total_links,
        "updated_at": chrono::Utc::now().to_rfc3339()
    });
    let _ = tokio::fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap_or_default()).await;
    
    Ok((pages, total_links))
}

/// POST /api/wiki/{agent}/refresh
/// 1. Convert unconverted non-markdown files in raw/ → pages/
/// 2. Ingest all pages/ files (build index, extract metadata)
pub async fn refresh(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let wiki_dir = workspace.join("wiki");
    let raw_dir = wiki_dir.join("raw");
    let pages_dir = wiki_dir.join("pages");

    // Auto-initialize wiki directories if missing
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

    // Phase 1: Convert non-markdown files in raw/ that don't have a corresponding .md in pages/
    let mut converted = Vec::new();
    let mut failed = Vec::new();

    if let Ok(mut entries) = tokio::fs::read_dir(&raw_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") || name.starts_with('.') {
                continue;
            }
            let md_name = name.rsplit_once('.')
                .map(|(n, _)| format!("{}.md", n))
                .unwrap_or_else(|| format!("{}.md", name));
            let md_path = pages_dir.join(&md_name);
            
            // Skip if already has corresponding .md
            if let Ok(true) = tokio::fs::try_exists(&md_path).await {
                continue;
            }

            let raw_file_path = raw_dir.join(&name);
            match convert_file(&raw_file_path, &pages_dir, &name).await {
                Ok(md_filename) => converted.push(md_filename),
                Err(e) => {
                    ts_eprint!("[Wiki] Convert failed for {}: {}", name, e);
                    failed.push(name);
                }
            }
        }
    }

    // Phase 2: Ingest all pages/ (including newly converted + existing)
    let (pages, total_links) = match ingest_pages(&pages_dir, &wiki_dir).await {
        Ok(result) => result,
        Err(e) => {
            ts_eprint!("[Wiki] Ingest failed: {}", e);
            (Vec::new(), 0)
        }
    };

    ok_response(&serde_json::json!({
        "message": "Wiki refreshed",
        "agent": agent_name,
        "converted": converted,
        "failed": failed,
        "ingested_pages": pages.len(),
        "total_links": total_links
    }))
}

/// POST /api/wiki/{agent}/convert?filename={name}
/// Convert a non-markdown file in wiki/raw/ to markdown using zeroclaw LLM.
pub async fn convert(data_dir: &str, agent_name: &str, filename: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    if filename.is_empty() || filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden filename");
    }

    let raw_path = workspace.join("wiki").join("raw").join(filename);
    if !raw_path.starts_with(workspace.join("wiki").join("raw")) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let pages_dir = workspace.join("wiki").join("pages");

    match convert_file(&raw_path, &pages_dir, filename).await {
        Ok(md_filename) => ok_response(&serde_json::json!({
            "message": "Converted to markdown",
            "filename": md_filename,
            "path": format!("pages/{}", md_filename)
        })),
        Err(e) => ok_response(&serde_json::json!({
            "error": format!("Conversion failed: {}", e),
            "filename": filename,
            "path": null
        })),
    }
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

/// Internal helper: write a file into the agent's workspace directory.
/// Returns Err if the agent is not found or the write fails.
pub async fn write_workspace_file(data_dir: &str, agent_name: &str, filename: &str, data: Bytes) -> anyhow::Result<()> {
    let workspace = get_agent_workspace(data_dir, agent_name)?;

    if filename.is_empty() || filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        anyhow::bail!("Forbidden filename");
    }

    let file_path = workspace.join(filename);
    if !file_path.starts_with(&workspace) {
        anyhow::bail!("Forbidden path");
    }

    tokio::fs::write(&file_path, data).await
        .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
    Ok(())
}

/// POST /api/agents/{agent}/workspace/{filename}
/// Write a file into the agent's workspace directory.
pub async fn save_workspace_file(data_dir: &str, agent_name: &str, filename: &str, data: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    match write_workspace_file(data_dir, agent_name, filename, data).await {
        Ok(_) => ok_response(&serde_json::json!({
            "message": "File saved",
            "filename": filename,
        })),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)),
    }
}
