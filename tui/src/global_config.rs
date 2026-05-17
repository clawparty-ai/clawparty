use hyper::body::Bytes;
use hyper::{Response, StatusCode, header};
use http_body_util::combinators::BoxBody;

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

fn global_config_path(data_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(data_dir).join("global-config.toml")
}

/// Ensure default global-config.toml exists
fn ensure_default(data_dir: &str) -> anyhow::Result<String> {
    let path = global_config_path(data_dir);
    if path.exists() {
        return Ok(std::fs::read_to_string(&path)?);
    }
    let default = r#"api_key = ""
api_url = ""
default_provider = "openai"
default_model = "gpt-4o-mini"
default_temperature = 0.7
provider_timeout_secs = 120

[gateway]
require_pairing = false
"#;
    std::fs::write(&path, default)?;
    Ok(default.to_string())
}

/// Parse simple TOML into JSON-like Value
fn parse_toml(content: &str) -> serde_json::Value {
    let mut result = serde_json::Map::new();
    let mut current_section: Option<String> = None;
    let mut llm = serde_json::Map::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = Some(line[1..line.len()-1].to_string());
            continue;
        }
        if let Some(eq_idx) = line.find('=') {
            let key = line[..eq_idx].trim();
            let value = line[eq_idx+1..].trim();
            let parsed = if value.starts_with('"') && value.ends_with('"') {
                serde_json::Value::String(value[1..value.len()-1].to_string())
            } else if let Ok(num) = value.parse::<f64>() {
                serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or(0.into()))
            } else if value == "true" {
                serde_json::Value::Bool(true)
            } else if value == "false" {
                serde_json::Value::Bool(false)
            } else {
                serde_json::Value::String(value.to_string())
            };
            // Flatten top-level keys into llm
            let section = current_section.as_deref().unwrap_or("");
            if section == "metadata" || section.starts_with("metadata.") {
                if !result.contains_key("metadata") {
                    result.insert("metadata".to_string(), serde_json::Value::Object(serde_json::Map::new()));
                }
                if let Some(serde_json::Value::Object(ref mut m)) = result.get_mut("metadata") {
                    m.insert(key.to_string(), parsed);
                }
            } else if section.is_empty() {
                llm.insert(key.to_string(), parsed);
            } else {
                // Other sections
                if !result.contains_key(section) {
                    result.insert(section.to_string(), serde_json::Value::Object(serde_json::Map::new()));
                }
                if let Some(serde_json::Value::Object(ref mut m)) = result.get_mut(section) {
                    m.insert(key.to_string(), parsed);
                }
            }
        }
    }

    result.insert("llm".to_string(), serde_json::Value::Object(llm));
    serde_json::Value::Object(result)
}

/// Generate TOML from config object
fn generate_toml(config: &serde_json::Value) -> String {
    let mut lines: Vec<String> = Vec::new();
    let llm = config.get("llm").and_then(|v| v.as_object()).cloned().unwrap_or_default();

    let api_key = llm.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let api_url = llm.get("api_endpoint").and_then(|v| v.as_str())
        .or_else(|| llm.get("api_url").and_then(|v| v.as_str()))
        .unwrap_or("");
    let provider = llm.get("provider").and_then(|v| v.as_str())
        .unwrap_or_else(|| llm.get("default_provider").and_then(|v| v.as_str()).unwrap_or("openai"));
    let model = llm.get("model").and_then(|v| v.as_str())
        .unwrap_or_else(|| llm.get("default_model").and_then(|v| v.as_str()).unwrap_or("gpt-4o-mini"));
    let temperature = llm.get("temperature").and_then(|v| v.as_f64())
        .or_else(|| llm.get("default_temperature").and_then(|v| v.as_f64()))
        .unwrap_or(0.7);
    let timeout = llm.get("timeout_secs").and_then(|v| v.as_f64())
        .or_else(|| llm.get("provider_timeout_secs").and_then(|v| v.as_f64()))
        .unwrap_or(120.0);

    lines.push(format!(r#"api_key = "{}""#, api_key));
    lines.push(format!(r#"api_url = "{}""#, api_url));
    lines.push(format!(r#"default_provider = "{}""#, provider));
    lines.push(format!(r#"default_model = "{}""#, model));
    lines.push(format!("default_temperature = {}", temperature));
    lines.push(format!("provider_timeout_secs = {}", timeout));
    lines.push(String::new());

    if let Some(metadata) = config.get("metadata").and_then(|v| v.as_object()) {
        lines.push("[metadata]".to_string());
        if let Some(v) = metadata.get("source").and_then(|v| v.as_str()) {
            lines.push(format!(r#"source = "{}""#, v));
        }
        if let Some(v) = metadata.get("hub_url").and_then(|v| v.as_str()) {
            lines.push(format!(r#"hub_url = "{}""#, v));
        }
        if let Some(v) = metadata.get("updated_at").and_then(|v| v.as_f64()) {
            lines.push(format!("updated_at = {}", v));
        }
        lines.push(String::new());
    }

    lines.push("[gateway]".to_string());
    lines.push("require_pairing = false".to_string());
    lines.push(String::new());

    lines.join("\n")
}

fn strip_api_key(config: &serde_json::Value) -> serde_json::Value {
    let mut safe = config.clone();
    if let Some(serde_json::Value::Object(ref mut llm)) = safe.get_mut("llm") {
        llm.remove("api_key");
    }
    safe
}

// ── GET /api/global-config ──────────────────────────────────────────────

pub async fn get_global_config(data_dir: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let content = match ensure_default(data_dir) {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to read config: {}", e)),
    };

    let config = parse_toml(&content);
    ok_response(&strip_api_key(&config))
}

// ── PUT /api/global-config ────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct UpdateGlobalConfigRequest {
    llm: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
}

pub async fn update_global_config(data_dir: &str, body_bytes: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    let req: UpdateGlobalConfigRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e)),
    };

    let llm = match req.llm {
        Some(v) => v,
        None => return error_response(StatusCode::BAD_REQUEST, "missing llm config"),
    };

    let mut config = parse_toml(&ensure_default(data_dir).unwrap_or_default());

    // Update llm
    if let Some(ref mut cfg) = config.as_object_mut() {
        cfg.insert("llm".to_string(), llm);
        if let Some(meta) = req.metadata {
            cfg.insert("metadata".to_string(), meta);
        } else if !cfg.contains_key("metadata") {
            cfg.insert("metadata".to_string(), serde_json::json!({
                "source": "hub",
                "hub_url": "",
                "updated_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
            }));
        } else if let Some(serde_json::Value::Object(ref mut meta)) = cfg.get_mut("metadata") {
            meta.insert("updated_at".to_string(), serde_json::json!(
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()
            ));
        }
    }

    let toml = generate_toml(&config);
    let path = global_config_path(data_dir);
    if let Err(e) = std::fs::write(&path, toml) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to write config: {}", e));
    }

    ok_response(&serde_json::json!({
        "message": "Global config updated",
        "config": strip_api_key(&config)
    }))
}
