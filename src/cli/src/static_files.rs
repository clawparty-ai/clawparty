use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode, header};
use rust_embed::RustEmbed;

/// Embedded frontend GUI files (compiled from chat-gui Vue build output).
#[derive(RustEmbed)]
#[folder = "gui/"]
struct GuiAssets;

/// Build a response body from embedded file bytes.
fn body(bytes: Bytes) -> crate::proxy::BoxBody {
    Full::new(bytes)
        .map_err(|never| match never {})
        .boxed()
}

/// Guess Content-Type from file extension.
fn guess_mime(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".html") || path == "/" || path.is_empty() {
        "text/html"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else if path.ends_with(".ttf") {
        "font/ttf"
    } else {
        "application/octet-stream"
    }
}

/// Serve a static file from the embedded GUI assets.
/// Falls back to index.html for SPA routing (non-API, non-file paths).
pub async fn serve<B>(
    req: Request<B>,
) -> anyhow::Result<Response<crate::proxy::BoxBody>> {
    let path = req.uri().path();
    let method = req.method();

    // Strip leading slash
    let clean_path = if path.starts_with('/') {
        &path[1..]
    } else {
        path
    };

    // Try exact match first
    if let Some(file) = GuiAssets::get(clean_path) {
        let mime = guess_mime(clean_path);
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .body(body(Bytes::from(file.data.to_vec())))?);
    }

    // For directory paths, try index.html inside
    let index_path = if clean_path.is_empty() || clean_path.ends_with('/') {
        format!("{}index.html", clean_path)
    } else {
        format!("{}/index.html", clean_path)
    };
    if let Some(file) = GuiAssets::get(&index_path) {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(body(Bytes::from(file.data.to_vec())))?);
    }

    // SPA fallback: serve index.html for non-file GET requests (let Vue Router handle)
    if *method == hyper::Method::GET {
        if let Some(file) = GuiAssets::get("index.html") {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(body(Bytes::from(file.data.to_vec())))?);
        }
    }

    // Genuine 404
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(body(Bytes::from_static(b"Not Found")))?)
}
