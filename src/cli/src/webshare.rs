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

/// Guess MIME type from filename extension (same logic as PipyJS mimeType).
fn guess_mime(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html"
    } else if lower.ends_with(".js") {
        "application/javascript"
    } else if lower.ends_with(".css") {
        "text/css"
    } else if lower.ends_with(".json") {
        "application/json"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else if lower.ends_with(".woff2") {
        "font/woff2"
    } else if lower.ends_with(".woff") {
        "font/woff"
    } else if lower.ends_with(".ttf") {
        "font/ttf"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".txt") {
        "text/plain"
    } else if lower.ends_with(".xml") {
        "application/xml"
    } else if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".zip") {
        "application/zip"
    } else if lower.ends_with(".gz") {
        "application/gzip"
    } else {
        "application/octet-stream"
    }
}

#[derive(serde::Serialize)]
struct FileEntry {
    name: String,
    size: u64,
    mtime: u64,
    #[serde(rename = "type")]
    file_type: String,
}

#[derive(serde::Serialize)]
struct ListResponse {
    agent: String,
    path: String,
    files: Vec<FileEntry>,
}

#[derive(serde::Serialize)]
struct UploadResponse {
    status: u16,
    message: String,
    path: String,
}

/// GET /api/webshare/{agent}/list?path={subPath}
pub async fn list_files(data_dir: &str, agent_name: &str, sub_path: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    // Security: prevent directory traversal
    if sub_path.contains("..") {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let web_dir = workspace.join("web").join(sub_path);

    // Ensure web_dir is still under workspace/web (canonicalize to resolve symlinks)
    let canonical_web_base = match tokio::fs::canonicalize(&workspace.join("web")).await {
        Ok(p) => p,
        Err(_) => {
            // web/ does not exist yet, create it
            let _ = tokio::fs::create_dir_all(workspace.join("web")).await;
            workspace.join("web")
        }
    };

    let canonical_target = match tokio::fs::canonicalize(&web_dir).await {
        Ok(p) => p,
        Err(_) => {
            // target path does not exist yet, create web dir if needed
            let _ = tokio::fs::create_dir_all(&web_dir).await;
            match tokio::fs::canonicalize(&web_dir).await {
                Ok(p) => p,
                Err(_) => return ok_response(&ListResponse {
                    agent: agent_name.to_string(),
                    path: sub_path.to_string(),
                    files: vec![],
                }),
            }
        }
    };

    if !canonical_target.starts_with(&canonical_web_base) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    // If target is not a directory, fall back to base web dir
    let is_dir = match tokio::fs::metadata(&canonical_target).await {
        Ok(m) => m.is_dir(),
        Err(_) => false,
    };

    let read_dir = if is_dir {
        canonical_target.clone()
    } else {
        canonical_web_base.clone()
    };

    let mut entries = match tokio::fs::read_dir(&read_dir).await {
        Ok(rd) => rd,
        Err(_) => {
            return ok_response(&ListResponse {
                agent: agent_name.to_string(),
                path: sub_path.to_string(),
                files: vec![],
            });
        }
    };

    let mut files: Vec<FileEntry> = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden files (same convention as PipyJS, though PipyJS doesn't explicitly skip them)
        if name.starts_with('.') {
            continue;
        }
        let meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            files.push(FileEntry {
                name,
                size: 0,
                mtime: 0,
                file_type: "dir".to_string(),
            });
        } else if meta.is_file() {
            let mtime = meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push(FileEntry {
                name,
                size: meta.len(),
                mtime,
                file_type: "file".to_string(),
            });
        }
    }

    ok_response(&ListResponse {
        agent: agent_name.to_string(),
        path: sub_path.to_string(),
        files,
    })
}

/// GET /api/webshare/{agent}/file/{filename}?path={subPath}
pub async fn read_file(data_dir: &str, agent_name: &str, filename: &str, sub_path: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    // Security checks
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden");
    }
    if sub_path.contains("..") {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let web_dir = workspace.join("web").join(sub_path);
    let file_path = web_dir.join(filename);

    // Ensure file_path is under workspace/web
    let canonical_web_base = match tokio::fs::canonicalize(&workspace.join("web")).await {
        Ok(p) => p,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "File not found"),
    };
    let canonical_file = match tokio::fs::canonicalize(&file_path).await {
        Ok(p) => p,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "File not found"),
    };

    if !canonical_file.starts_with(&canonical_web_base) {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let data = match tokio::fs::read(&canonical_file).await {
        Ok(d) => d,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "File not found"),
    };

    let mime = guess_mime(filename);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(box_body(Bytes::from(data)))
        .unwrap()
}

/// POST /api/webshare/{agent}/upload?name={filename}&path={subPath}
pub async fn upload_file(data_dir: &str, agent_name: &str, filename: &str, sub_path: &str, body: Bytes) -> Response<BoxBody<Bytes, hyper::Error>> {
    if filename.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "name is required");
    }
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') || filename.starts_with('.') {
        return error_response(StatusCode::FORBIDDEN, "Forbidden filename");
    }
    if sub_path.contains("..") {
        return error_response(StatusCode::FORBIDDEN, "Forbidden path");
    }

    let workspace = match get_agent_workspace(data_dir, agent_name) {
        Ok(w) => w,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "Agent not found"),
    };

    let web_dir = workspace.join("web").join(sub_path);

    // Ensure target directory is under workspace/web
    let canonical_web_base = match tokio::fs::canonicalize(&workspace.join("web")).await {
        Ok(p) => p,
        Err(_) => {
            let _ = tokio::fs::create_dir_all(workspace.join("web")).await;
            match tokio::fs::canonicalize(&workspace.join("web")).await {
                Ok(p) => p,
                Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create directory"),
            }
        }
    };

    if let Ok(canonical_target) = tokio::fs::canonicalize(&web_dir).await {
        if !canonical_target.starts_with(&canonical_web_base) {
            return error_response(StatusCode::FORBIDDEN, "Forbidden path");
        }
    } else {
        // Directory doesn't exist yet, create it
        if let Err(e) = tokio::fs::create_dir_all(&web_dir).await {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to create directory: {}", e));
        }
    }

    let file_path = web_dir.join(filename);
    if let Ok(canonical_file) = tokio::fs::canonicalize(&file_path.parent().unwrap_or(&web_dir)).await {
        if !canonical_file.starts_with(&canonical_web_base) {
            return error_response(StatusCode::FORBIDDEN, "Forbidden path");
        }
    }

    match tokio::fs::write(&file_path, body).await {
        Ok(_) => {
            let return_path = if sub_path.is_empty() {
                filename.to_string()
            } else {
                format!("{}/{}", sub_path, filename)
            };
            ok_response(&UploadResponse {
                status: 200,
                message: "File uploaded successfully".to_string(),
                path: return_path,
            })
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to write file: {}", e)),
    }
}

/// Route dispatcher for /api/webshare/* requests.
/// Returns `Some(Response)` if the path matches a webshare route, else `None`.
pub async fn route(
    data_dir: &str,
    path: &str,
    method: &hyper::Method,
    req: hyper::Request<Incoming>,
) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
    let rest = path.strip_prefix("/api/webshare/")?;

    // Split the rest into segments
    let segments: Vec<&str> = rest.split('/').collect();
    if segments.len() < 2 {
        return None;
    }

    let agent_encoded = segments[0];
    let agent = urlencoding::decode(agent_encoded).unwrap_or_else(|_| agent_encoded.into()).to_string();
    let action = segments[1];

    let query = req.uri().query().unwrap_or("");

    match action {
        "list" if method == hyper::Method::GET => {
            let sub_path = url::form_urlencoded::parse(query.as_bytes())
                .find(|(k, _)| k == "path")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            Some(list_files(data_dir, &agent, &sub_path).await)
        }
        "file" if method == hyper::Method::GET && segments.len() >= 3 => {
            let filename_encoded = segments[2..].join("/");
            let filename = match urlencoding::decode(&filename_encoded) {
                Ok(decoded) => decoded.to_string(),
                Err(_) => filename_encoded,
            };
            let sub_path = url::form_urlencoded::parse(query.as_bytes())
                .find(|(k, _)| k == "path")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            Some(read_file(data_dir, &agent, &filename, &sub_path).await)
        }
        "upload" if method == hyper::Method::POST => {
            let filename = url::form_urlencoded::parse(query.as_bytes())
                .find(|(k, _)| k == "name")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            let sub_path = url::form_urlencoded::parse(query.as_bytes())
                .find(|(k, _)| k == "path")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            let body_bytes = match req.collect().await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    return Some(error_response(StatusCode::BAD_REQUEST, "Failed to read body"));
                }
            };
            Some(upload_file(data_dir, &agent, &filename, &sub_path, body_bytes).await)
        }
        _ => None,
    }
}
