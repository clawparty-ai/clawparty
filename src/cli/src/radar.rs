use std::path::PathBuf;
use std::time::Duration;

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

// ── Probe types (mirrors targets pattern) ───────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct ProbeChannelRaw {
    #[serde(rename = "type")]
    channel_type: String,
    location: String,
    description: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ProbeRaw {
    id: Option<String>,
    name: String,
    description: Option<String>,
    #[serde(rename = "module_ref")]
    module_ref: Option<String>,
    channel: Option<Vec<ProbeChannelRaw>>,
    method: Option<String>,
    keywords: Option<Vec<String>>,
    schedule: Option<String>,
    status: Option<String>,
    #[serde(rename = "created_at")]
    created_at: Option<String>,
    #[serde(rename = "last_run")]
    last_run: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ProbesYaml {
    probes: Vec<ProbeRaw>,
}

#[derive(Debug, serde::Serialize)]
struct ProbeChannelJson {
    #[serde(rename = "type")]
    channel_type: String,
    location: String,
}

#[derive(Debug, serde::Serialize)]
struct ProbeJson {
    id: Option<String>,
    name: String,
    description: Option<String>,
    #[serde(rename = "module_ref")]
    module_ref: Option<String>,
    channels: Vec<ProbeChannelJson>,
    #[serde(rename = "channelLabel")]
    channel_label: String,
    #[serde(rename = "channel_type")]
    channel_type: Option<String>,
    #[serde(rename = "channel_location")]
    channel_location: Option<String>,
    method: Option<String>,
    keywords: Option<Vec<String>>,
    schedule: Option<String>,
    status: String,
    #[serde(rename = "created_at")]
    created_at: Option<String>,
    #[serde(rename = "last_run")]
    last_run: Option<String>,
}

fn parse_probes_md(content: &str) -> Vec<ProbeJson> {
    if content.starts_with("---\n") || content.starts_with("---\r\n") {
        let after_first = content.find("---\n").map(|i| i + 4)
            .or_else(|| content.find("---\r\n").map(|i| i + 5));
        let start = match after_first {
            Some(s) => s,
            None => return parse_probes_table(content),
        };

        let remaining = &content[start..];
        let end = match remaining.find("\n---") {
            Some(e) => e,
            None => return parse_probes_table(content),
        };

        let yaml_str = &remaining[..end];

        let parsed: Result<ProbesYaml, _> = serde_yaml::from_str(yaml_str);
        let probes_raw = match parsed {
            Ok(t) => t.probes,
            Err(e) => {
                ts_eprint!("[Radar] Failed to parse probes YAML: {}", e);
                return parse_probes_table(content);
            }
        };

        return probes_raw.into_iter().map(|p| {
            let channels: Vec<ProbeChannelJson> = p.channel.map(|chs| {
                chs.into_iter().map(|c| ProbeChannelJson {
                    channel_type: c.channel_type,
                    location: c.location,
                }).collect()
            }).unwrap_or_default();

            let channel_label = channels.iter()
                .map(|c| c.channel_type.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            ProbeJson {
                id: p.id,
                name: p.name,
                description: p.description,
                module_ref: p.module_ref,
                channels,
                channel_label,
                channel_type: None,
                channel_location: None,
                method: p.method,
                keywords: p.keywords,
                schedule: p.schedule,
                status: normalize_status(&p.status),
                created_at: p.created_at,
                last_run: p.last_run,
            }
        }).collect();
    }

    parse_probes_table(content)
}

/// Parse probe Markdown tables: expects `### P-XXX：...` sections with `| 字段 | 值 |` tables.
fn parse_probes_table(content: &str) -> Vec<ProbeJson> {
    let mut probes = Vec::new();
    let sections: Vec<&str> = content.split("\n### ").collect();

    for section in &sections {
        let section = section.trim();
        if section.is_empty() { continue; }
        if section.starts_with("# ") { continue; }

        let header_end = section.find('\n').unwrap_or(section.len());
        let header = &section[..header_end].trim();
        let body = &section[header_end..];

        let fields = parse_md_table_fields(body);
        if fields.is_empty() { continue; }

        let id = fields.get("ID").cloned();
        let name = id.clone().unwrap_or_else(|| {
            header.split('：').next().unwrap_or(header).to_string()
        });
        let description = fields.get("描述").cloned();
        let keywords = fields.get("搜索词").map(|kw| {
            kw.split(|c: char| c == ',' || c == '、' || c == ' ')
                .map(|s| s.trim().trim_matches('`'))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        });
        let schedule = fields.get("频率").cloned();
        let channel_type = fields.get("平台").map(|p| {
            p.split(|c: char| c == ' ' || c == '（' || c == '(').next().unwrap_or("").to_string()
        });
        let method_str = fields.get("方法").or_else(|| fields.get("查询模板")).cloned();

        let status = match fields.get("状态").map(|s| s.as_str()) {
            Some("active") | Some("running") => "active".to_string(),
            Some("paused") => "paused".to_string(),
            Some(s) => s.to_string(),
            None => "active".to_string(),
        };

        probes.push(ProbeJson {
            id,
            name,
            description,
            module_ref: None,
            channels: Vec::new(),
            channel_label: channel_type.clone().unwrap_or_default(),
            channel_type,
            channel_location: fields.get("搜索范围").cloned(),
            method: method_str,
            keywords,
            schedule,
            status,
            created_at: None,
            last_run: None,
        });
    }

    probes
}

/// Parse a markdown table with `| **key** | value |` rows into a HashMap.
fn parse_md_table_fields(section: &str) -> std::collections::HashMap<String, String> {
    let mut fields = std::collections::HashMap::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') { continue; }
        if trimmed.contains("---|---") || trimmed.contains("------") { continue; }
        if trimmed.contains("字段") && trimmed.contains("值") { continue; }

        let cells: Vec<&str> = trimmed.split('|').collect();
        if cells.len() < 3 { continue; }

        let key = cells[1].trim().trim_start_matches("**").trim_end_matches("**").trim();
        let value = cells[2].trim();
        if !key.is_empty() && !value.is_empty() {
            fields.insert(key.to_string(), value.to_string());
        }
    }
    fields
}

// ── Target types ────────────────────────────────────────────────────────

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

/// Parse a detailed target markdown file from radrar/targets/*.md.
///
/// Each file has:
///   - H1 title → target name
///   - Blockquote lines (`> ...`) → description
///   - H2/H3 sections with 2-column tables → spec entries (prefixed with section name)
///   - H2/H3 sections with list items (`- ...`) → spec entries
fn parse_detailed_target(content: &str) -> Option<TargetJson> {
    let mut name = String::new();
    let mut description_parts: Vec<String> = Vec::new();
    let mut spec_entries: Vec<SpecEntry> = Vec::new();
    let mut current_section = String::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("# ") && !line.starts_with("## ") {
            name = line[2..].trim().to_string();
            i += 1;
            continue;
        }

        if line.starts_with("## ") {
            current_section = line.trim_start_matches('#').trim().to_string();
            i += 1;
            let (consumed, entries) = parse_section_entries(&lines[i..], &current_section);
            spec_entries.extend(entries);
            i += consumed;
            continue;
        }
        if line.starts_with("### ") {
            current_section = line.trim_start_matches('#').trim().to_string();
            i += 1;
            let (consumed, entries) = parse_section_entries(&lines[i..], &current_section);
            spec_entries.extend(entries);
            i += consumed;
            continue;
        }

        if line.starts_with('>') {
            let text = line[1..].trim();
            if !text.is_empty() {
                description_parts.push(text.to_string());
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    if name.is_empty() {
        return None;
    }

    let description = if description_parts.is_empty() {
        None
    } else {
        Some(description_parts.join("\n"))
    };

    let spec_label = spec_entries.iter()
        .map(|e| e.value.as_str())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    Some(TargetJson {
        id: None,
        name,
        description,
        spec_entries,
        spec_label,
        channels: Vec::new(),
        channel_label: String::new(),
        source_probe: None,
        status: "active".to_string(),
        created_at: None,
        last_scan: None,
    })
}

/// Parse entries after a section heading: tables, multi-column tables, and list items.
/// Returns (lines consumed, spec entries).
fn parse_section_entries(lines: &[&str], section: &str) -> (usize, Vec<SpecEntry>) {
    let mut consumed = 0;
    let mut entries: Vec<SpecEntry> = Vec::new();

    while consumed < lines.len() && lines[consumed].trim().is_empty() {
        consumed += 1;
    }

    if consumed >= lines.len() {
        return (consumed, entries);
    }

    let first_line = lines[consumed].trim();
    if first_line.starts_with('|') && first_line.contains('|') {
        let header_cells: Vec<&str> = first_line.split('|').collect();
        let col_count = header_cells.iter().filter(|c| !c.trim().is_empty()).count();

        if col_count == 2 {
            while consumed < lines.len() {
                let line = lines[consumed].trim();
                if !line.starts_with('|') { break; }
                if line.contains("---|---") || line.contains("------") { consumed += 1; continue; }
                let cells: Vec<&str> = line.split('|').collect();
                if cells.len() < 3 { consumed += 1; continue; }
                let key = cells[1].trim().trim_start_matches("**").trim_end_matches("**").trim();
                let value = cells[2].trim();
                if !key.is_empty() && !value.is_empty() {
                    let prefixed_key = format!("{}-{}", section, key);
                    entries.push(SpecEntry { key: prefixed_key, value: value.to_string() });
                }
                consumed += 1;
            }
        } else {
            let headers: Vec<String> = header_cells.iter()
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect();
            consumed += 1;
            while consumed < lines.len() {
                let line = lines[consumed].trim();
                if line.starts_with('|') && (line.contains("---") || line.contains("------")) {
                    consumed += 1;
                } else {
                    break;
                }
            }
            let mut row_idx = 0;
            while consumed < lines.len() {
                let line = lines[consumed].trim();
                if !line.starts_with('|') { break; }
                let cells: Vec<&str> = line.split('|').collect();
                let values: Vec<&str> = cells.iter()
                    .map(|c| c.trim().trim_start_matches("**").trim_end_matches("**"))
                    .filter(|c| !c.is_empty())
                    .collect();
                if values.is_empty() { consumed += 1; continue; }
                let label = if !headers.is_empty() && values.len() <= headers.len() {
                    values.iter().enumerate()
                        .map(|(j, v)| format!("{}:{}", headers.get(j).unwrap_or(&String::new()), v))
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    values.join(", ")
                };
                let key = format!("{}-row{}", section, row_idx + 1);
                entries.push(SpecEntry { key, value: label });
                consumed += 1;
                row_idx += 1;
            }
        }
        return (consumed, entries);
    }

    if first_line.starts_with('-') {
        let mut items: Vec<String> = Vec::new();
        while consumed < lines.len() {
            let line = lines[consumed].trim();
            if line.starts_with('-') {
                items.push(line[1..].trim().to_string());
                consumed += 1;
            } else if line.is_empty() {
                consumed += 1;
            } else {
                break;
            }
        }
        if !items.is_empty() {
            entries.push(SpecEntry {
                key: section.to_string(),
                value: items.join("\n"),
            });
        }
        return (consumed, entries);
    }

    (consumed, entries)
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

    if let Some(yaml_content) = yaml_str {
        if let Ok(t) = serde_yaml::from_str::<TargetsYaml>(yaml_content) {
            return t.targets.into_iter().map(|t| {
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
            }).collect();
        }
    }

    parse_targets_table(content)
}

fn parse_targets_table(content: &str) -> Vec<TargetJson> {
    let mut targets = Vec::new();
    let sections: Vec<&str> = content.split("\n### ").collect();
    let known_spec_keys = ["ID", "名称", "描述", "状态"];

    for section in &sections {
        let section = section.trim();
        if section.is_empty() { continue; }
        if section.starts_with("# ") { continue; }

        let header_end = section.find('\n').unwrap_or(section.len());
        let header = &section[..header_end].trim();
        let body = &section[header_end..];

        let fields = parse_md_table_fields(body);
        if fields.is_empty() { continue; }

        let id = fields.get("ID").cloned();
        let name = fields.get("名称").cloned().unwrap_or_else(|| {
            let after_colon = header.find(':')
                .or_else(|| header.find('：'))
                .map(|i| header[i + 1..].trim());
            after_colon.unwrap_or(header).to_string()
        });
        let description = fields.get("描述").cloned();

        let mut spec_entries = Vec::new();
        for (key, value) in &fields {
            if !known_spec_keys.contains(&key.as_str()) {
                spec_entries.push(SpecEntry {
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }
        let spec_label = spec_entries.iter()
            .map(|e| e.value.as_str())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join(", ");

        let status = match fields.get("状态").map(|s| s.as_str()) {
            Some("active") | Some("running") => "active".to_string(),
            Some("paused") => "paused".to_string(),
            Some(s) => s.to_string(),
            None => "active".to_string(),
        };

        targets.push(TargetJson {
            id,
            name,
            description,
            spec_entries,
            spec_label,
            channels: Vec::new(),
            channel_label: String::new(),
            source_probe: fields.get("关联 Probe").cloned(),
            status,
            created_at: None,
            last_scan: None,
        });
    }

    targets
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
/// Returns parsed targets from targets.md and individual target/*.md files.
pub async fn get_targets_json(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let path = workspace.join("radar").join("targets.md");
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => String::new(),
    };
    let mut targets = parse_targets_md(&content);
    let mut name_to_index: std::collections::HashMap<String, usize> = targets.iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), i))
        .collect();

    let detailed_dir = workspace.join("radar").join("targets");
    if let Ok(mut rd) = tokio::fs::read_dir(&detailed_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let fname = entry.file_name().to_string_lossy().to_string();
            if !fname.ends_with(".md") {
                continue;
            }
            if let Ok(file_content) = tokio::fs::read_to_string(entry.path()).await {
                if let Some(detailed) = parse_detailed_target(&file_content) {
                    if let Some(&idx) = name_to_index.get(&detailed.name) {
                        targets[idx] = detailed;
                    } else {
                        name_to_index.insert(detailed.name.clone(), targets.len());
                        targets.push(detailed);
                    }
                }
            }
        }
    }

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

/// GET /api/radar/{agent}/probes-json
/// Returns parsed probes from YAML frontmatter as structured JSON.
pub async fn get_probes_json(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    ensure_radar_dir(&workspace).await;

    let path = workspace.join("radar").join("probes.md");
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => {
            return ok_response(&serde_json::json!({ "probes": Vec::<ProbeJson>::new() }));
        }
    };

    let probes = parse_probes_md(&content);
    ok_response(&serde_json::json!({ "probes": probes }))
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

/// POST /api/radar/{agent}/format-targets
/// Reads targets.md and sends it to LLM for formatting into HTML.
pub async fn format_targets(data_dir: &str, agent_name: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let path = workspace.join("radar").join("targets.md");
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "targets.md not found"),
    };

    if content.trim().is_empty() {
        return ok_response(&serde_json::json!({
            "html": "<p class=\"radar-empty-text\">targets.md 为空，无需格式化。</p>",
            "source": "noop"
        }));
    }

    // Check file size (max 500KB for LLM context)
    if content.len() > 500 * 1024 {
        return error_response(StatusCode::PAYLOAD_TOO_LARGE, "targets.md too large for LLM formatting (max 500KB)");
    }

    let prompt = format!(
        "你是一个数据格式化助手。请将以下 targets.md 文件内容格式化为结构化的 HTML 表格。\n\
        要求：\n\
        1. 保持所有原始数据完整，不要遗漏任何一行\n\
        2. 使用 HTML <table> 标签，带有合适的 thead/tbody\n\
        3. 为表格添加现代、简洁的样式（内联 style 或 <style> 标签）\n\
        4. 保留所有 ID、姓名、年龄、位置、状态等列\n\
        5. 对于不同 section（如 ## 球员跟踪目标、## 赛事跟踪目标），用 <h2> 标题分隔\n\
        6. 只输出 HTML 代码片段（不需要 <!DOCTYPE html> 或 <html> 包裹），不需要任何解释文字\n\
        7. 对于线索/待确认类数据，用浅灰背景的表格表示\n\
        \n\
        文件内容：\n\n```markdown\n{}\n```",
        content
    );

    let client = reqwest::Client::new();
    let body = serde_json::json!({ "message": prompt });

    match client
        .post("http://127.0.0.1:42617/api/sessions/me/chat")
        .json(&body)
        .timeout(Duration::from_secs(120))
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                let status = resp.status();
                let err_body = resp.text().await.unwrap_or_default();
                return error_response(status, &format!("LLM service returned {}: {}", status, err_body));
            }
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    let html = json["response"].as_str().unwrap_or("").to_string();
                    if html.is_empty() {
                        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "LLM returned empty response");
                    }
                    ok_response(&serde_json::json!({
                        "html": html,
                        "source": "llm",
                        "agent": "0#Agent"
                    }))
                }
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to parse LLM response: {}", e)),
            }
        }
        Err(e) => error_response(StatusCode::SERVICE_UNAVAILABLE, &format!("Failed to connect to LLM service: {}", e)),
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
        "format-targets" if method == hyper::Method::POST => {
            Some(format_targets(data_dir, &agent).await)
        }
        "probes" if method == hyper::Method::GET && segments.len() == 2 => {
            Some(get_probes(data_dir, &agent).await)
        }
        "probes-json" if method == hyper::Method::GET => {
            Some(get_probes_json(data_dir, &agent).await)
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
