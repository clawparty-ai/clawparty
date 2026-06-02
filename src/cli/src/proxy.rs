use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use sha1::{Digest, Sha1};
use sha2::Sha256;
use base64::Engine;
use rusqlite::OptionalExtension;

use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Request, Response, StatusCode, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::net::TcpListener;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// Type alias for HTTP body
pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

/// Generate or load a self-signed certificate pair.
fn ensure_cert(cert_dir: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let expanded = cert_dir.replace("~", &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    let dir = std::path::Path::new(&expanded);
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        let cert = std::fs::read(&cert_path)?;
        let key = std::fs::read(&key_path)?;
        return Ok((cert, key));
    }

    std::fs::create_dir_all(dir)?;

    let cert = rcgen::generate_simple_self_signed(vec!["clawparty.local".into(), "localhost".into()])?;
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    std::fs::write(&cert_path, &cert_pem)?;
    std::fs::write(&key_path, &key_pem)?;

    ts_eprint!(
        "[Proxy] Generated self-signed certificate at {}",
        cert_path.display()
    );
    ts_eprint!("[Proxy] Add {} to browser trust store if needed", cert_path.display());

    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

fn load_tls_config(cert_pem: &[u8], key_pem: &[u8]) -> anyhow::Result<ServerConfig> {
    let cert_chain: Vec<CertificateDer<'static>> = certs(&mut &cert_pem[..])
        .collect::<Result<Vec<_>, _>>()?;

    let mut key_reader = &key_pem[..];
    let mut keys = pkcs8_private_keys(&mut key_reader);
    let key = match keys.next() {
        Some(Ok(k)) => PrivateKeyDer::try_from(k)?,
        _ => anyhow::bail!("No valid private key found in PEM"),
    };

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)?;

    Ok(config)
}

/// Build a boxed body from bytes.
pub(crate) fn box_body(bytes: Bytes) -> BoxBody {
    Full::new(bytes).map_err(|never| match never {}).boxed()
}

/// Check if a request is a WebSocket upgrade.
fn is_websocket_request(req: &Request<Incoming>) -> bool {
    req.headers()
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        == Some("websocket")
}

/// Clone relevant headers from incoming request for the backend request.
fn clone_headers(
    src: &Request<Incoming>,
    dst: hyper::http::request::Builder,
) -> hyper::http::request::Builder {
    let mut builder = dst;
    for (name, value) in src.headers() {
        let name_str = name.as_str();
        if name_str.eq_ignore_ascii_case("connection")
            || name_str.eq_ignore_ascii_case("keep-alive")
            || name_str.eq_ignore_ascii_case("proxy-connection")
            || name_str.eq_ignore_ascii_case("transfer-encoding")
            || name_str.eq_ignore_ascii_case("upgrade")
            || name_str.eq_ignore_ascii_case("sec-websocket-key")
            || name_str.eq_ignore_ascii_case("sec-websocket-accept")
            || name_str.eq_ignore_ascii_case("sec-websocket-extensions")
            || name_str.eq_ignore_ascii_case("sec-websocket-protocol")
            || name_str.eq_ignore_ascii_case("sec-websocket-version")
            || name_str.eq_ignore_ascii_case("accept-encoding")
            || name_str.eq_ignore_ascii_case("host")
        {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
}

/// Look up an agent's port from clawparty.db.
fn get_agent_port_clawparty(data_dir: &str, agent_name: &str) -> Option<u16> {
    crate::db::get_agent_port(data_dir, agent_name)
}

/// Resolve the backend target URI for HTTP requests.
///
/// Routing rules:
/// - /api/zeroclaw/sessions/{id}      → http://127.0.0.1:{port}/api/sessions/{id}
/// - /api/zeroclaw/sessions/{id}/chat → http://127.0.0.1:{port}/api/sessions/{id}/chat
/// - /api/zeroclaw/messages?agent=...&session=...
///                                      → http://127.0.0.1:{port}/api/sessions/{session}/messages (zeroclaw)
///                                      OR http://127.0.0.1:{port}/session/{session}/message (opencode)
/// - /api/zeroclaw/health             → http://127.0.0.1:{port}/api/health (zeroclaw)
///                                      OR http://127.0.0.1:{port}/global/health (opencode)
/// - /api/zeroclaw/* (catch-all)      → http://127.0.0.1:{port}/api/* (zeroclaw)
/// - everything else                  → http://127.0.0.1:6789
async fn resolve_http_backend(req: &Request<Incoming>) -> anyhow::Result<Uri> {
    let path_and_query = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");

    if path_and_query.starts_with("/api/zeroclaw/") {
        let remainder = &path_and_query["/api/zeroclaw".len()..];
        let target = if remainder.starts_with("/messages") {
            let mut agent_name = String::new();
            let mut session = "me".to_string();
            if let Some(idx) = remainder.find('?') {
                let query_str = &remainder[idx + 1..];
                for (key, value) in url::form_urlencoded::parse(query_str.as_bytes()) {
                    if key == "agent" {
                        agent_name = value.to_string();
                    } else if key == "session" {
                        session = value.to_string();
                    }
                }
            }
            let port = if agent_name.is_empty() {
                42617
            } else if let Some(data_dir) = DATA_DIR.get() {
                get_agent_port_clawparty(data_dir, &agent_name).unwrap_or(42617)
            } else {
                42617
            };

            let is_opencode = if let Some(data_dir) = DATA_DIR.get() {
                crate::db::get_agent(data_dir, &agent_name)
                    .ok()
                    .flatten()
                    .map(|a| a.engine == "opencode")
                    .unwrap_or(false)
            } else {
                false
            };

            if is_opencode {
                if session == "me" {
                    if agent_name == "0#Agent" || agent_name.is_empty() {
                        session = OPENCODE_SESSION.get().cloned().unwrap_or_else(|| "me".to_string());
                    } else {
                        // Get or create a session on this agent's own OpenCode server
                        if let Some(sid) = get_or_create_opencode_session(port, "ClawParty Agent").await {
                            session = sid;
                        }
                    }
                }
                format!("http://127.0.0.1:{}/session/{}/message?limit=50", port, session)
            } else {
                format!("http://127.0.0.1:{}/api/sessions/{}/messages", port, session)
            }
        } else if remainder.starts_with("/sessions") && remainder.contains("/chat") {
            let port = 42617;
            format!("http://127.0.0.1:{}/api{}", port, remainder)
        } else if remainder == "/health" || remainder == "/health/" {
            let is_opencode = ENGINE.get().map(|e| e == "opencode").unwrap_or(false);
            if is_opencode {
                "http://127.0.0.1:42617/global/health".to_string()
            } else {
                "http://127.0.0.1:42617/api/health".to_string()
            }
        } else if remainder.starts_with("/sessions") {
            let is_opencode = ENGINE.get().map(|e| e == "opencode").unwrap_or(false);
            if is_opencode {
                format!("http://127.0.0.1:42617/session{}", &remainder["/sessions".len()..])
            } else {
                format!("http://127.0.0.1:42617/api{}", remainder)
            }
        } else {
            format!("http://127.0.0.1:42617/api{}", remainder)
        };
        Ok(target.parse()?)
    } else {
        let target = format!("http://127.0.0.1:6789{}", path_and_query);
        Ok(target.parse()?)
    }
}

/// Proxy an HTTP request to the backend and return the response.
async fn proxy_http(req: Request<Incoming>) -> anyhow::Result<Response<BoxBody>> {
    let backend_uri = resolve_http_backend(&req).await?;
    let req_path = req.uri().path().to_string();
    let method = req.method().clone();

    let body_bytes = req.collect().await?.to_bytes();
    let backend_url = backend_uri.to_string();

    let reqwest_client = reqwest::Client::new();
    let resp = match method {
        hyper::Method::GET => {
            reqwest_client.get(&backend_url).send().await
        }
        hyper::Method::POST => {
            reqwest_client.post(&backend_url)
                .body(body_bytes.to_vec())
                .send().await
        }
        hyper::Method::DELETE => {
            reqwest_client.delete(&backend_url)
                .send().await
        }
        _ => {
            reqwest_client.get(&backend_url).send().await
        }
    };

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            ts_eprint!("[Proxy] Backend request failed: {}", e);
            let mut resp = Response::new(box_body(Bytes::from(
                "Backend service unavailable".to_string(),
            )));
            *resp.status_mut() = StatusCode::BAD_GATEWAY;
            return Ok(resp);
        }
    };

    let status = StatusCode::from_u16(resp.status().as_u16())?;
    let collected = Bytes::from(resp.bytes().await?.to_vec());

    let is_opencode_messages = req_path.ends_with("/messages") 
        && req_path.starts_with("/api/zeroclaw/");

    if is_opencode_messages && status.is_success() {
        if let Ok(transformed) = transform_opencode_messages(&collected) {
            let mut builder = Response::builder()
                .status(status)
                .header(header::CONTENT_TYPE, "application/json");
            return Ok(builder.body(box_body(transformed))?);
        }
    }

    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json");
    Ok(builder.body(box_body(collected))?)
}

/// Handle a WebSocket upgrade by proxying to the correct backend.
/// For zeroclaw agents: bridges to zeroclaw WS backend.
/// For opencode agents: bridges to opencode SSE + HTTP.
async fn proxy_websocket(
    mut req: Request<Incoming>,
) -> anyhow::Result<Response<BoxBody>> {
    let path_and_query = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/").to_string();

    let mut agent_name = String::new();
    let mut session_id = String::new();
    {
        let query = req.uri().query().unwrap_or("");
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if key == "agent" {
                agent_name = value.to_string();
            } else if key == "session_id" {
                session_id = value.to_string();
            }
        }
    }

    let target_port = if agent_name.is_empty() {
        42617u16
    } else if let Some(data_dir) = DATA_DIR.get() {
        get_agent_port_clawparty(data_dir, &agent_name).unwrap_or(42617)
    } else {
        42617u16
    };

    let is_opencode = if agent_name.is_empty() {
        ENGINE.get().map(|e| e == "opencode").unwrap_or(false)
    } else if let Some(data_dir) = DATA_DIR.get() {
        crate::db::get_agent(data_dir, &agent_name)
            .ok()
            .flatten()
            .map(|a| a.engine == "opencode")
            .unwrap_or(false)
    } else {
        false
    };

    log::debug!("[Proxy][WS] Upgrade request: {} -> port {} (opencode: {})", path_and_query, target_port, is_opencode);

    // Collect upgrade future from the request before building response
    let frontend_upgrade = hyper::upgrade::on(&mut req);

    // Copy relevant WS headers from original request
    let mut sec_key_raw = None;
    let mut sec_protocol = None;
    let mut sec_version = None;
    if let Some(v) = req.headers().get("sec-websocket-key") {
        sec_key_raw = Some(v.to_str().unwrap_or("").to_string());
    }
    if let Some(v) = req.headers().get("sec-websocket-protocol") {
        sec_protocol = Some(v.clone());
    }
    if let Some(v) = req.headers().get("sec-websocket-version") {
        sec_version = Some(v.clone());
    }

    let target_port_clone = target_port;

    // Map session_id to actual opencode session for opencode agents
    if is_opencode {
        if agent_name == "0#Agent" || agent_name.is_empty() {
            session_id = OPENCODE_SESSION.get().cloned().unwrap_or_else(|| "me".to_string());
        } else if session_id == "me" {
            // Individual (zAgent) chat — use main agent session
            let session = get_or_create_opencode_session(target_port, "ClawParty Agent").await;
            session_id = session.unwrap_or_else(|| "me".to_string());
        } else {
            // Group chat — create a dedicated session per group
            let title = format!("ClawParty Group - {}", session_id);
            let session = get_or_create_opencode_session(target_port, &title).await;
            session_id = session.unwrap_or_else(|| "me".to_string());
        }
    }

    if is_opencode {
        let agent = agent_name.clone();
        let sid = session_id.clone();
        tokio::spawn(async move {
            match frontend_upgrade.await {
                Ok(upgraded) => {
                    let frontend_io = TokioIo::new(upgraded);
                    if let Err(e) = bridge_opencode_sse(frontend_io, target_port_clone, &agent, &sid).await {
                        ts_eprint!("[Proxy] OpenCode SSE bridge error: {}", e);
                    }
                }
                Err(e) => {
                    log::debug!("[Proxy][WS] Frontend upgrade failed: {}", e);
                }
            }
        });
    } else {
        let ws_url = format!("ws://127.0.0.1:{target_port}{path_and_query}");
        let sec_protocol_for_spawn = sec_protocol.clone();

        tokio::spawn(async move {
            match frontend_upgrade.await {
                Ok(upgraded) => {
                    let frontend_io = TokioIo::new(upgraded);
                    if let Err(e) = bridge_websocket(frontend_io, &ws_url, sec_protocol_for_spawn).await {
                        ts_eprint!("[Proxy] WebSocket bridge error: {}", e);
                    }
                }
                Err(e) => {
                    log::debug!("[Proxy][WS] Frontend upgrade failed: {}", e);
                }
            }
        });
    }

    // Compute Sec-WebSocket-Accept per RFC 6455
    let sec_accept = sec_key_raw.map(|key| {
        let mut hasher = Sha1::new();
        hasher.update(key.as_bytes());
        hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let accept = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());
        log::debug!("[Proxy][WS] Computed sec-websocket-accept: {}", accept);
        accept
    });

    // Build 101 Switching Protocols response
    let mut builder = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "upgrade");

    if let Some(accept) = sec_accept {
        builder = builder.header("sec-websocket-accept", accept);
    }
    if let Some(proto) = sec_protocol {
        builder = builder.header("sec-websocket-protocol", proto);
    }
    if let Some(ver) = sec_version {
        builder = builder.header("sec-websocket-version", ver);
    }

    log::debug!("[Proxy][WS] Returning 101 Switching Protocols");
    Ok(builder.body(box_body(Bytes::new()))?)
}

/// Bridge two WebSocket connections (frontend <-> backend).
async fn bridge_websocket(
    frontend: TokioIo<hyper::upgrade::Upgraded>,
    backend_url: &str,
    sec_protocol: Option<hyper::header::HeaderValue>,
) -> anyhow::Result<()> {
    log::debug!("[Proxy][WS] bridge_websocket starting for: {}", backend_url);

    // Generate a random Sec-WebSocket-Key for the backend handshake
    let ws_key = base64::engine::general_purpose::STANDARD.encode(rand::random::<[u8; 16]>());
    log::debug!("[Proxy][WS] Generated backend sec-websocket-key: {}", ws_key);

    let mut backend_req = tokio_tungstenite::tungstenite::handshake::client::Request::builder()
        .uri(backend_url)
        .header("Host", "localhost")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Key", &ws_key)
        .header("Sec-WebSocket-Version", "13");

    if let Some(proto) = sec_protocol {
        let proto_str = proto.to_str().unwrap_or("zeroclaw.v1");
        log::debug!("[Proxy][WS] Backend request sec-protocol: {}", proto_str);
        backend_req = backend_req.header("Sec-WebSocket-Protocol", proto_str);
    } else {
        log::debug!("[Proxy][WS] No sec-protocol for backend request");
    }

    let backend_req = backend_req.body(())?;
    log::debug!("[Proxy][WS] Connecting to backend: {}", backend_url);

    let (backend_ws, backend_resp) = tokio_tungstenite::connect_async(backend_req).await?;
    log::debug!("[Proxy][WS] Backend connected, response status: {:?}", backend_resp.status());

    log::debug!("[Proxy][WS] Waiting for frontend WebSocketStream...");
    let frontend_ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        frontend,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;
    log::debug!("[Proxy][WS] Frontend WebSocketStream ready");

    let (mut frontend_sink, mut frontend_stream) = frontend_ws.split();
    let (mut backend_sink, mut backend_stream) = backend_ws.split();

    let fwd_to_backend = async {
        log::debug!("[Proxy][WS] F->B loop started");
        let mut count = 0;
        while let Some(msg) = frontend_stream.next().await {
            match msg {
                Ok(msg) => {
                    count += 1;
                    let desc = match &msg {
                        tokio_tungstenite::tungstenite::Message::Text(t) => format!("Text({} bytes)", t.len()),
                        tokio_tungstenite::tungstenite::Message::Binary(b) => format!("Binary({} bytes)", b.len()),
                        tokio_tungstenite::tungstenite::Message::Ping(_) => "Ping".to_string(),
                        tokio_tungstenite::tungstenite::Message::Pong(_) => "Pong".to_string(),
                        tokio_tungstenite::tungstenite::Message::Close(c) => format!("Close({:?})", c),
                        tokio_tungstenite::tungstenite::Message::Frame(_) => "Frame".to_string(),
                    };
                    log::debug!("[Proxy][WS] F->B #{}: {}", count, desc);
                    if backend_sink.send(msg).await.is_err() {
                        log::debug!("[Proxy][WS] F->B #{}: backend_sink send error, breaking", count);
                        break;
                    }
                }
                Err(e) => {
                    log::debug!("[Proxy][WS] F->B error: {}", e);
                    break;
                }
            }
        }
        log::debug!("[Proxy][WS] F->B loop ended, total messages: {}", count);
    };

    let fwd_to_frontend = async {
        log::debug!("[Proxy][WS] B->F loop started");
        let mut count = 0;
        while let Some(msg) = backend_stream.next().await {
            match msg {
                Ok(msg) => {
                    count += 1;
                    let desc = match &msg {
                        tokio_tungstenite::tungstenite::Message::Text(t) => format!("Text({} bytes)", t.len()),
                        tokio_tungstenite::tungstenite::Message::Binary(b) => format!("Binary({} bytes)", b.len()),
                        tokio_tungstenite::tungstenite::Message::Ping(_) => "Ping".to_string(),
                        tokio_tungstenite::tungstenite::Message::Pong(_) => "Pong".to_string(),
                        tokio_tungstenite::tungstenite::Message::Close(c) => format!("Close({:?})", c),
                        tokio_tungstenite::tungstenite::Message::Frame(_) => "Frame".to_string(),
                    };
                    log::debug!("[Proxy][WS] B->F #{}: {}", count, desc);
                    if frontend_sink.send(msg).await.is_err() {
                        log::debug!("[Proxy][WS] B->F #{}: frontend_sink send error, breaking", count);
                        break;
                    }
                }
                Err(e) => {
                    log::debug!("[Proxy][WS] B->F error: {}", e);
                    break;
                }
            }
        }
        log::debug!("[Proxy][WS] B->F loop ended, total messages: {}", count);
    };

    tokio::select! {
        _ = fwd_to_backend => {
            log::debug!("[Proxy][WS] fwd_to_backend completed first");
        },
        _ = fwd_to_frontend => {
            log::debug!("[Proxy][WS] fwd_to_frontend completed first");
        },
    }

    log::debug!("[Proxy][WS] bridge_websocket completed");
    Ok(())
}

async fn bridge_opencode_sse(
    frontend: TokioIo<hyper::upgrade::Upgraded>,
    port: u16,
    agent_name: &str,
    session_id: &str,
) -> anyhow::Result<()> {
    use futures_util::{SinkExt, StreamExt};

    let frontend_ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        frontend,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;

    let (mut frontend_sink, mut frontend_stream) = frontend_ws.split();

    let base_url = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    // Emit session_start to frontend
    let start_msg = serde_json::json!({
        "type": "session_start",
        "session_id": session_id
    }).to_string();

    if frontend_sink
        .send(tokio_tungstenite::tungstenite::Message::Text(start_msg.into()))
        .await
        .is_err()
    {
        return Ok(());
    }

    let sse_url = format!("{}/event", base_url);
    let sse_response = client.get(&sse_url).send().await?;

    if !sse_response.status().is_success() {
        ts_eprint!("[Proxy][SSE] Failed to connect to OpenCode SSE: {}", sse_response.status());
        return Ok(());
    }

    let sse_stream = sse_response.bytes_stream();
    tokio::pin!(sse_stream);

    let mut full_response = String::new();
    let mut buffer = String::new();

    let fwd_to_backend = async {
        while let Some(msg) = frontend_stream.next().await {
            if let Ok(msg) = msg {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    log::debug!("[Proxy][SSE] Frontend message: {}", text);

                    let parsed = serde_json::from_str::<serde_json::Value>(&text).ok();
                    let msg_type = parsed.as_ref()
                        .and_then(|p| p.get("type"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("message");

                    if msg_type == "cancel" {
                        let abort_url = format!("{}/session/{}/abort", base_url, session_id);
                        log::debug!("[Proxy][SSE] Canceling session: {}", abort_url);
                        let _ = client.post(&abort_url).send().await;
                        continue;
                    }

                    let content = parsed.as_ref()
                        .and_then(|p| p.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or(&text)
                        .to_string();

                    // Abort any in-progress request to prevent stuck sessions
                    let abort_url = format!("{}/session/{}/abort", base_url, session_id);
                    let _ = client.post(&abort_url).send().await;

                    let send_url = format!("{}/session/{}/message", base_url, session_id);
                    if let Err(e) = client
                        .post(&send_url)
                        .header("Content-Type", "application/json")
                        .json(&serde_json::json!({
                            "parts": [{"type": "text", "text": content}],
                            "agent": "build",
                        }))
                        .send()
                        .await
                    {
                        ts_eprint!("[Proxy][SSE] Failed to send message to OpenCode: {}", e);
                    }
                }
            } else {
                break;
            }
        }
    };

    let sse_to_frontend = async {
        use tokio_tungstenite::tungstenite::Message as WsMsg;
        use futures_util::StreamExt;

        let mut is_busy = false;

        while let Some(chunk) = sse_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let chunk_str = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&chunk_str);

                    while let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();

                        if line.is_empty() || line == ":" {
                            continue;
                        }

                        let data = if line.starts_with("data: ") {
                            &line["data: ".len()..]
                        } else {
                            continue;
                        };

                        let event: serde_json::Value = match serde_json::from_str(data) {
                            Ok(e) => e,
                            Err(_) => continue,
                        };

                        let event_type = event["type"].as_str().unwrap_or("");

                        // Filter events by session_id: the SSE stream broadcasts ALL
                        // sessions' events, but this bridge is for ONE session only.
                        // Skip events belonging to other sessions to avoid cross-talk.
                        let event_session = event["properties"].get("sessionID")
                            .and_then(|v| v.as_str());
                        let is_our_event = event_session.map_or(true, |sid| sid == session_id);

                        match event_type {
                            "message.part.updated" => {
                                if !is_our_event { continue; }
                                let part = &event["properties"]["part"];
                                let part_type = part["type"].as_str().unwrap_or("");

                                match part_type {
                                    "text" => {
                                        let text = part["delta"].as_str()
                                            .or_else(|| part["text"].as_str());
                                        if let Some(t) = text {
                                            if is_busy {
                                                full_response.push_str(t);
                                                let msg = serde_json::json!({
                                                    "type": "chunk",
                                                    "content": t
                                                }).to_string();
                                                if frontend_sink.send(WsMsg::Text(msg.into())).await.is_err() {
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                    "reasoning" => {
                                        let text = part["delta"].as_str()
                                            .or_else(|| part["text"].as_str());
                                        if let Some(t) = text {
                                            if is_busy {
                                                let msg = serde_json::json!({
                                                    "type": "thinking",
                                                    "content": t
                                                }).to_string();
                                                if frontend_sink.send(WsMsg::Text(msg.into())).await.is_err() {
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                    "tool" => {
                                        if let Some(state) = part.get("state") {
                                            if state["status"].as_str() == Some("completed") {
                                                let output = state["output"].as_str().unwrap_or("");
                                                full_response.push_str(output);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "message.part.delta" => {
                                if !is_our_event { continue; }
                                let field = event["properties"]["field"].as_str().unwrap_or("");
                                let delta = event["properties"]["delta"].as_str().unwrap_or("");

                                if delta.is_empty() { continue; }
                                if !is_busy { continue; }

                                match field {
                                    "text" => {
                                        let msg = serde_json::json!({
                                            "type": "chunk",
                                            "content": delta
                                        }).to_string();
                                        if frontend_sink.send(WsMsg::Text(msg.into())).await.is_err() {
                                            return;
                                        }
                                    }
                                    "reasoning" => {
                                        let msg = serde_json::json!({
                                            "type": "thinking",
                                            "content": delta
                                        }).to_string();
                                        if frontend_sink.send(WsMsg::Text(msg.into())).await.is_err() {
                                            return;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "message.updated" => {},
                            "session.status" => {
                                if !is_our_event { continue; }
                                let status = event["properties"]["status"]["type"].as_str().unwrap_or("");
                                match status {
                                    "busy" => is_busy = true,
                                    "idle" => {
                                        is_busy = false;
                                        if !full_response.is_empty() {
                                            let msg = serde_json::json!({
                                                "type": "done",
                                                "full_response": &full_response
                                            }).to_string();
                                            let _ = frontend_sink.send(WsMsg::Text(msg.into())).await;
                                            full_response.clear();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            "session.error" => {
                                if !is_our_event { continue; }
                                let error_obj = &event["properties"]["error"];
                                let error_msg = error_obj["data"]["message"]
                                    .as_str()
                                    .or_else(|| error_obj["message"].as_str())
                                    .unwrap_or("Unknown error");
                                let msg = serde_json::json!({
                                    "type": "error",
                                    "message": error_msg
                                }).to_string();
                                let _ = frontend_sink.send(WsMsg::Text(msg.into())).await;
                            }
                            "server.heartbeat" => {}
                            "server.connected" => {}
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    ts_eprint!("[Proxy][SSE] Stream error: {}", e);
                    break;
                }
            }
        }
    };

    tokio::select! {
        _ = fwd_to_backend => {
            log::debug!("[Proxy][SSE] fwd_to_backend completed");
        },
        _ = sse_to_frontend => {
            log::debug!("[Proxy][SSE] sse_to_frontend completed");
        },
    }

    log::debug!("[Proxy][SSE] bridge_opencode_sse completed");
    Ok(())
}

fn transform_opencode_messages(bytes: &[u8]) -> anyhow::Result<Bytes> {
    let opencode_msgs: Vec<serde_json::Value> = serde_json::from_slice(bytes)?;
    let mut messages: Vec<serde_json::Value> = Vec::new();

    for msg in &opencode_msgs {
        let role = msg["info"]["role"].as_str().unwrap_or("user");
        let created = msg["info"]["time"]["created"].as_f64().unwrap_or(0.0);

        let mut content = String::new();
        if let Some(parts) = msg["parts"].as_array() {
            for part in parts {
                let part_type = part["type"].as_str().unwrap_or("");
                match part_type {
                    "text" | "reasoning" => {
                        if let Some(text) = part["text"].as_str() {
                            if !content.is_empty() {
                                content.push('\n');
                            }
                            content.push_str(text);
                        }
                    }
                    "tool" => {
                        if let Some(tool_name) = part["tool"].as_str() {
                            if !content.is_empty() {
                                content.push('\n');
                            }
                            content.push_str(&format!("[Tool: {}]", tool_name));
                        }
                    }
                    _ => {}
                }
            }
        }

        if !content.is_empty() {
            messages.push(serde_json::json!({
                "content": content,
                "role": role,
                "created_at": created,
            }));
        }
    }

    let result = serde_json::json!({"messages": messages});
    Ok(Bytes::from(serde_json::to_string(&result)?))
}

// ── Authentication helpers (login runs in proxy, not ztm) ──────────────────

/// DB path for authentication (set once at startup).
static DB_PATH: OnceLock<String> = OnceLock::new();

/// Data directory for wiki file operations (set once at startup).
static DATA_DIR: OnceLock<String> = OnceLock::new();

/// Execution engine ("zeroclaw" or "opencode") set once at startup.
static ENGINE: OnceLock<String> = OnceLock::new();

/// Whether the ZTM agent is disabled (--zeroclaw-only / --no-ztm).
static ZEROCLAW_ONLY: OnceLock<bool> = OnceLock::new();

/// Default opencode session ID for 0#Agent (set at startup).
pub static OPENCODE_SESSION: OnceLock<String> = OnceLock::new();

pub fn get_engine() -> String {
    ENGINE.get().cloned().unwrap_or_else(|| "zeroclaw".to_string())
}

pub fn set_engine(engine: &str) {
    let _ = ENGINE.set(engine.to_string());
}

pub fn set_no_ztm(val: bool) {
    let _ = ZEROCLAW_ONLY.set(val);
}

async fn get_or_create_opencode_session(port: u16, title: &str) -> Option<String> {
    let base_url = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();
    let resp = client.get(&format!("{}/session", base_url)).send().await.ok()?;
    if resp.status().is_success() {
        let sessions: Vec<serde_json::Value> = resp.json().await.ok()?;
        let existing = sessions.iter().find(|s| {
            s.get("title").and_then(|t| t.as_str()) == Some(title)
        });
        if let Some(session) = existing {
            if let Some(id) = session["id"].as_str() {
                return Some(id.to_string());
            }
        }
    }
    // No existing session, create one
    let resp = client
        .post(&format!("{}/session", base_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"title": title}))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let result: serde_json::Value = resp.json().await.ok()?;
        return result["id"].as_str().map(|s| s.to_string());
    }
    None
}

fn generate_random_string(len: usize) -> String {
    use rand::Rng;
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let mut rng = rand::rng();
    (0..len).map(|_| chars[rng.random_range(0..chars.len())]).collect()
}

fn is_user_expired(expire: f64) -> bool {
    if expire <= 0.0 {
        false // never expire
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        now > expire
    }
}

fn verify_token(token: &str, db_path: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            ts_eprint!("[Proxy] DB open error: {}", e);
            return false;
        }
    };
    let result: Option<f64> = conn.query_row(
        "SELECT expire FROM users WHERE api_token = ?1",
        [token],
        |row| row.get(0),
    ).optional().unwrap_or(None);

    if let Some(expire) = result {
        !is_user_expired(expire)
    } else {
        false
    }
}

async fn handle_join_party(req: Request<Incoming>) -> anyhow::Result<Response<BoxBody>> {
    use rand::Rng;

    let body_bytes = req.collect().await?.to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    let body: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(_) => {
            let err_body = serde_json::json!({"status":400,"message":"invalid request body"}).to_string();
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(box_body(Bytes::from(err_body)))?);
        }
    };

    let reg_url = body.get("regUrl").and_then(|v| v.as_str()).unwrap_or("https://clawparty.flomesh.io:7779");
    let user_name = body.get("userName").and_then(|v| v.as_str()).unwrap_or("");
    let invite_code = body.get("inviteCode").and_then(|v| v.as_str()).unwrap_or("");

    // Guard: ZTM must be enabled
    if ZEROCLAW_ONLY.get().copied().unwrap_or(false) {
        let err_body = serde_json::json!({"status":503,"message":"ZTM agent not available"}).to_string();
        return Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(header::CONTENT_TYPE, "application/json")
            .body(box_body(Bytes::from(err_body)))?);
    }

    let ztm_base = "http://127.0.0.1:6789";
    let client = reqwest::Client::new();
    let mesh_name = "clawparty";

    // Step 1: Check if already joined
    let meshes_resp = client.get(format!("{}/api/meshes", ztm_base)).send().await;
    if let Ok(resp) = meshes_resp {
        if resp.status().is_success() {
            let meshes: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();
            let already_joined = meshes.iter().any(|m| {
                m.get("name").and_then(|n| n.as_str()) == Some(mesh_name)
            });
            if already_joined {
                let err_body = serde_json::json!({"status":409,"message":"Already joined clawparty, have fun!"}).to_string();
                return Ok(Response::builder()
                    .status(StatusCode::CONFLICT)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(box_body(Bytes::from(err_body)))?);
            }
        }
    }

    // Step 2: Generate names
    const NAMES: &[&str] = &[
        "aureliano", "remedios", "melqulades", "william-wallace", "robert-the-bruce",
        "sitting-bull", "geronimo", "sacagawea", "crazy-horse", "pocahontas",
        "red-cloud", "chief-joseph", "cochise", "thunder-cloud", "morning-star",
        "running-deer", "lone-wolf", "white-buffalo", "red-hawk", "little-wolf",
    ];
    let (final_user_name, ep_name, pass_key) = {
        let mut rng = rand::rng();
        let name = if user_name.is_empty() {
            NAMES[rng.random_range(0..NAMES.len())].to_string()
        } else {
            user_name.to_string()
        };
        let ep = format!("{}-lobster", name);
        let pk: String = (0..16)
            .map(|_| (b'a' + rng.random_range(0..26)) as char)
            .collect();
        (name, ep, pk)
    };

    // Step 3: Get ZTM identity (public key)
    let identity_resp = client
        .get(format!("{}/api/identity", ztm_base))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("ZTM agent not reachable: {}", e))?;
    let public_key = identity_resp.text().await
        .map_err(|e| anyhow::anyhow!("failed to read identity: {}", e))?;

    // Step 4: Request permit from registration server
    let invite_url = if reg_url.ends_with('/') {
        format!("{}invite", reg_url)
    } else {
        format!("{}/invite", reg_url)
    };
    let invite_body = serde_json::json!({
        "PublicKey": public_key,
        "UserName": final_user_name,
        "EpName": ep_name,
        "PassKey": pass_key,
        "InviteCode": invite_code,
    });
    let permit_resp = client
        .post(&invite_url)
        .header("Content-Type", "application/json")
        .json(&invite_body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("permit server request failed: {}", e))?;

    let permit_status = permit_resp.status();
    if !permit_status.is_success() {
        let err_text = permit_resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&err_text)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str().map(String::from)))
            .unwrap_or(err_text);
        return Ok(Response::builder()
            .status(StatusCode::from_u16(permit_status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY))
            .header(header::CONTENT_TYPE, "application/json")
            .body(box_body(Bytes::from(serde_json::json!({"status":permit_status.as_u16(),"message":msg}).to_string())))?);
    }

    let permit_body: serde_json::Value = permit_resp.json().await
        .map_err(|e| anyhow::anyhow!("invalid permit response: {}", e))?;

    // Step 5: Parse double-encoded permit
    let permit_str = permit_body.get("Permit")
        .and_then(|v| v.as_str())
        .or_else(|| permit_body.get("permit").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow::anyhow!("no permit in registration server response"))?;
    let permit: serde_json::Value = serde_json::from_str(permit_str)
        .map_err(|e| anyhow::anyhow!("invalid permit format: {}", e))?;

    let ca = permit.get("ca")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing ca in permit"))?;
    let agent_cert = permit.get("agent")
        .and_then(|v| v.get("certificate"))
        .and_then(|v| v.as_str())
        .or_else(|| permit.get("cert").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow::anyhow!("missing agent certificate in permit"))?;
    let agent_key = permit.get("agent")
        .and_then(|v| v.get("privateKey"))
        .and_then(|v| v.as_str())
        .or_else(|| permit.get("key").and_then(|v| v.as_str()))
        .or_else(|| permit.get("privateKey").and_then(|v| v.as_str()));

    let bootstraps: Vec<String> = permit.get("bootstraps")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let bootstraps = if bootstraps.is_empty() {
        permit.get("hubs")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|h| {
                    h.as_str().or_else(|| h.get("address").and_then(|a| a.as_str()))
                        .map(String::from)
                }).collect::<Vec<String>>()
            }).unwrap_or_default()
    } else {
        bootstraps
    };

    // Step 6: Join mesh via ZTM API
    let mut agent_obj = serde_json::json!({
        "name": ep_name,
        "certificate": agent_cert,
    });
    if let Some(key) = agent_key {
        agent_obj["privateKey"] = serde_json::Value::String(key.to_string());
    }
    let join_body = serde_json::json!({
        "ca": ca,
        "agent": agent_obj,
        "bootstraps": bootstraps,
    });
    let join_resp = client
        .post(format!("{}/api/meshes/{}", ztm_base, mesh_name))
        .header("Content-Type", "application/json")
        .json(&join_body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("failed to join mesh: {}", e))?;

    if !join_resp.status().is_success() {
        let status_code = join_resp.status();
        let err_text = join_resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("failed to join mesh ({}): {}", status_code, err_text));
    }

    let resp_body = serde_json::json!({
        "meshName": mesh_name,
        "userName": final_user_name,
        "epName": ep_name,
    }).to_string();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(box_body(Bytes::from(resp_body)))?)
}

async fn handle_login(req: Request<Incoming>, db_path: &str) -> anyhow::Result<Response<BoxBody>> {
    let body_bytes = req.collect().await?.to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    let body_json: serde_json::Value = serde_json::from_str(&body_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let username = body_json.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = body_json.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if username.is_empty() || password.is_empty() {
        let err_body = r#"{"status":400,"message":"username and password are required"}"#;
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "application/json")
            .body(box_body(Bytes::from(err_body)))?);
    }

    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open DB: {}", e))?;

    let row: Option<(String, String, String, String, f64)> = conn.query_row(
        "SELECT password_hash, salt, api_token, share_token, expire FROM users WHERE username = ?1",
        [username],
        |row| {
            let hash: String = row.get(0)?;
            let salt: String = row.get(1)?;
            let token: String = row.get(2)?;
            let share_token: String = row.get(3)?;
            let expire: f64 = row.get(4)?;
            Ok((hash, salt, token, share_token, expire))
        }
    ).optional()?;

    if let Some((hash, salt, _old_token, share_token, expire)) = row {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", salt, password).as_bytes());
        let computed_hash = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>();

        if computed_hash == hash {
            if is_user_expired(expire) {
                let err_body = r#"{"status":401,"message":"account expired"}"#;
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(box_body(Bytes::from(err_body)))?);
            }

            let new_token = generate_random_string(32);
            let final_share_token = if share_token.is_empty() {
                let st = generate_random_string(32);
                conn.execute(
                    "UPDATE users SET share_token = ?1 WHERE username = ?2",
                    [&st, username],
                ).map_err(|e| anyhow::anyhow!("Failed to update share_token: {}", e))?;
                st
            } else {
                share_token
            };

            conn.execute(
                "UPDATE users SET api_token = ?1 WHERE username = ?2",
                [&new_token, username],
            ).map_err(|e| anyhow::anyhow!("Failed to update token: {}", e))?;

            let role = conn.query_row(
                "SELECT role FROM users WHERE username = ?1",
                [username],
                |row| row.get::<_, String>(0),
            ).unwrap_or_else(|_| "user".to_string());

            let resp_body = serde_json::json!({
                "username": username,
                "role": role,
                "token": new_token,
                "share_token": final_share_token,
            }).to_string();

            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(box_body(Bytes::from(resp_body)))?);
        }
    }

    let err_body = r#"{"status":401,"message":"invalid username or password"}"#;
    Ok(Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "application/json")
        .body(box_body(Bytes::from(err_body)))?)
}

fn verify_token_or_share(token: &str, db_path: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            ts_eprint!("[Proxy] DB open error: {}", e);
            return false;
        }
    };
    let result: Option<f64> = conn.query_row(
        "SELECT expire FROM users WHERE api_token = ?1 OR share_token = ?1",
        [token],
        |row| row.get(0),
    ).optional().unwrap_or(None);

    if let Some(expire) = result {
        !is_user_expired(expire)
    } else {
        false
    }
}

fn extract_token(req: &Request<Incoming>) -> String {
    // 1. Authorization header
    if let Some(auth) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Bearer ") {
                return auth_str["Bearer ".len()..].to_string();
            }
        }
    }
    // 2. Query parameter ?token=
    if let Some(query) = req.uri().query() {
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if key == "token" {
                return value.to_string();
            }
        }
    }
    String::new()
}

fn decode_agent_name(encoded: &str) -> String {
    urlencoding::decode(encoded)
        .unwrap_or_else(|_| encoded.into())
        .to_string()
}

/// Main request handler.
async fn handle_request(req: Request<Incoming>) -> Result<Response<BoxBody>, Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Login endpoint (no auth required)
    if path == "/api/login" && method == hyper::Method::POST {
        if let Some(db_path) = DB_PATH.get() {
            match handle_login(req, db_path).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    ts_eprint!("[Proxy] Login error: {}", e);
                    let mut resp = Response::new(box_body(Bytes::from("Internal Server Error")));
                    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return Ok(resp);
                }
            }
        } else {
            let mut resp = Response::new(box_body(Bytes::from("Service Unavailable")));
            *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
            return Ok(resp);
        }
    }

    // API routes require token validation
    if path.starts_with("/api/") {
        let token = extract_token(&req);
        if let Some(db_path) = DB_PATH.get() {
            let is_webshare_get = path.starts_with("/api/webshare/") && method == hyper::Method::GET;
            let valid = if is_webshare_get {
                verify_token_or_share(&token, db_path)
            } else {
                verify_token(&token, db_path)
            };
            if !valid {
                let mut resp = Response::new(box_body(Bytes::from(r#"{"status":401,"message":"unauthorized"}"#)));
                *resp.status_mut() = StatusCode::UNAUTHORIZED;
                return Ok(resp);
            }
        } else {
            let mut resp = Response::new(box_body(Bytes::from("Service Unavailable")));
            *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
            return Ok(resp);
        }
    }

    // ── Task API routes — handled locally by Rust (clawparty.db, not ztm.db) ──
    if path.starts_with("/api/tasks") {
        if let Some(data_dir) = DATA_DIR.get() {
            let query = req.uri().query().unwrap_or("");
            let resp = if path == "/api/tasks" && method == hyper::Method::GET {
                let agent_name = url::form_urlencoded::parse(query.as_bytes())
                    .find(|(k, _)| k == "agent")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                let group_id = url::form_urlencoded::parse(query.as_bytes())
                    .find(|(k, _)| k == "group")
                    .map(|(_, v)| v.to_string());
                crate::tasks::list_tasks(data_dir, &agent_name, group_id.as_deref()).await
            } else if path == "/api/tasks" && method == hyper::Method::POST {
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::tasks::create_task(data_dir, body_bytes).await
            } else if path == "/api/tasks/batch-refresh" && method == hyper::Method::POST {
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::tasks::batch_refresh(data_dir, body_bytes).await
            } else if path.starts_with("/api/tasks/") && method == hyper::Method::GET {
                let rest = &path["/api/tasks/".len()..];
                if rest.ends_with("/events") {
                    let task_id = &rest[..rest.len() - "/events".len()];
                    // TODO: implement task events query
                    let mut r = Response::new(box_body(Bytes::from(r#"{"task_id":"","events":[]}"#)));
                    *r.status_mut() = StatusCode::OK;
                    r
                } else {
                    crate::tasks::get_task(data_dir, rest).await
                }
            } else if path.starts_with("/api/tasks/") && method == hyper::Method::PUT {
                let task_id = &path["/api/tasks/".len()..];
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::tasks::update_task(data_dir, task_id, body_bytes).await
            } else if path.starts_with("/api/tasks/") && method == hyper::Method::DELETE {
                let task_id = &path["/api/tasks/".len()..];
                crate::tasks::delete_task(data_dir, task_id).await
            } else {
                Response::builder().status(StatusCode::NOT_FOUND).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Task route not found"}"#))).unwrap()
            };
            return Ok(resp);
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    if path == "/api/task/analysis" {
        if let Some(data_dir) = DATA_DIR.get() {
            let query = req.uri().query().unwrap_or("");
            let resp = if method == hyper::Method::GET {
                let agent_name = url::form_urlencoded::parse(query.as_bytes())
                    .find(|(k, _)| k == "agent")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                let group_id = url::form_urlencoded::parse(query.as_bytes())
                    .find(|(k, _)| k == "group")
                    .map(|(_, v)| v.to_string());
                crate::tasks::get_analysis_log(data_dir, &agent_name, group_id.as_deref()).await
            } else if method == hyper::Method::PUT {
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::tasks::set_analysis_log(data_dir, body_bytes).await
            } else {
                Response::builder().status(StatusCode::METHOD_NOT_ALLOWED).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Method not allowed"}"#))).unwrap()
            };
            return Ok(resp);
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // Wiki API routes — handled locally by Rust (not forwarded to ztm agent)
    if cfg!(debug_assertions) {
        ts_eprint!("[Proxy] Checking path: '{}' for wiki routes", path);
    }
    if path.starts_with("/api/wiki/") {
        if let Some(data_dir) = DATA_DIR.get() {
            let wiki_path = &path["/api/wiki/".len()..];
            if cfg!(debug_assertions) {
                ts_eprint!("[Proxy] wiki_path: '{}'", wiki_path);
            }
            if let Some(slash_idx) = wiki_path.find('/') {
                let agent_name = &wiki_path[..slash_idx];
                let action = &wiki_path[slash_idx + 1..];
                if cfg!(debug_assertions) {
                    ts_eprint!("[Proxy] agent_name: '{}', action: '{}'", agent_name, action);
                }

                // Decode URL-encoded agent name
                let agent_decoded = urlencoding::decode(agent_name).unwrap_or_else(|_| agent_name.into()).to_string();

                let query = req.uri().query().unwrap_or("");
                let resp = match action {
                    "init" if method == hyper::Method::POST => {
                        crate::wiki::init(data_dir, &agent_decoded).await
                    }
                    "tree" if method == hyper::Method::GET => {
                        let sub_path = url::form_urlencoded::parse(query.as_bytes())
                            .find(|(k, _)| k == "path")
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_default();
                        crate::wiki::tree(data_dir, &agent_decoded, &sub_path).await
                    }
                    action if action.starts_with("file/") && method == hyper::Method::GET => {
                        let name_encoded = &action["file/".len()..];
                        let name = urlencoding::decode(name_encoded).unwrap_or_else(|_| name_encoded.into()).to_string();
                        let sub_path = url::form_urlencoded::parse(query.as_bytes())
                            .find(|(k, _)| k == "path")
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_default();
                        crate::wiki::file(data_dir, &agent_decoded, &name, &sub_path).await
                    }
                    "search" if method == hyper::Method::GET => {
                        let q = url::form_urlencoded::parse(query.as_bytes())
                            .find(|(k, _)| k == "q")
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_default();
                        crate::wiki::search(data_dir, &agent_decoded, &q).await
                    }
                    "graph" if method == hyper::Method::GET => {
                        crate::wiki::graph(data_dir, &agent_decoded).await
                    }
                    "refresh" if method == hyper::Method::POST => {
                        crate::wiki::refresh(data_dir, &agent_decoded).await
                    }
                    "convert" if method == hyper::Method::POST => {
                        let filename = url::form_urlencoded::parse(query.as_bytes())
                            .find(|(k, _)| k == "filename")
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_default();
                        crate::wiki::convert(data_dir, &agent_decoded, &filename).await
                    }
                    "upload" if method == hyper::Method::POST => {
                        let name = url::form_urlencoded::parse(query.as_bytes())
                            .find(|(k, _)| k == "name")
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_default();
                        let body_bytes = match req.collect().await {
                            Ok(body) => body.to_bytes(),
                            Err(_) => {
                                let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#)));
                                *resp.status_mut() = StatusCode::BAD_REQUEST;
                                return Ok(resp);
                            }
                        };
                        crate::wiki::upload_raw(data_dir, &agent_decoded, &name, body_bytes).await
                    }
                    _ => {
                        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Wiki route not found"}"#)));
                        *resp.status_mut() = StatusCode::NOT_FOUND;
                        resp
                    }
                };
                return Ok(resp);
            }
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // ── Agent management API routes — handled locally by Rust (clawparty.db) ──
    if path == "/api/agents" || path.starts_with("/api/agents/") || path == "/api/agents/reconcile" {
        if let Some(data_dir) = DATA_DIR.get() {
            let resp = if path == "/api/agents" && method == hyper::Method::GET {
                crate::agents::list_agents(data_dir).await
            } else if path == "/api/agents" && method == hyper::Method::POST {
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::agents::create_agent(data_dir, body_bytes).await
            } else if path == "/api/agents/reconcile" && method == hyper::Method::POST {
                crate::agents::reconcile_agents(data_dir).await
            } else if path.starts_with("/api/agents/") {
                let rest = &path["/api/agents/".len()..];
                // Workspace file write: /api/agents/{name}/workspace/{filename}
                if rest.contains("/workspace/") && method == hyper::Method::POST {
                    if let Some(ws_idx) = rest.find("/workspace/") {
                        let agent_name = urlencoding::decode(&rest[..ws_idx]).unwrap_or_else(|_| rest[..ws_idx].into()).to_string();
                        let filename = urlencoding::decode(&rest[ws_idx + "/workspace/".len()..]).unwrap_or_else(|_| rest[ws_idx + "/workspace/".len()..].into()).to_string();
                        let body_bytes = match req.collect().await {
                            Ok(body) => body.to_bytes(),
                            Err(_) => {
                                let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#)));
                                *resp.status_mut() = StatusCode::BAD_REQUEST;
                                return Ok(resp);
                            }
                        };
                        return Ok(crate::wiki::save_workspace_file(data_dir, &agent_name, &filename, body_bytes).await);
                    }
                }
                if rest.ends_with("/start") && method == hyper::Method::POST {
                    let name = decode_agent_name(&rest[..rest.len() - "/start".len()]);
                    crate::agents::start_agent(data_dir, &name).await
                } else if rest.ends_with("/stop") && method == hyper::Method::POST {
                    let name = decode_agent_name(&rest[..rest.len() - "/stop".len()]);
                    crate::agents::stop_agent(data_dir, &name).await
                } else if rest.ends_with("/status") && method == hyper::Method::GET {
                    let name = decode_agent_name(&rest[..rest.len() - "/status".len()]);
                    crate::agents::get_agent(data_dir, &name).await
                } else if method == hyper::Method::GET {
                    let name = decode_agent_name(rest);
                    crate::agents::get_agent(data_dir, &name).await
                } else if method == hyper::Method::DELETE {
                    let name = decode_agent_name(rest);
                    crate::agents::delete_agent(data_dir, &name).await
                } else {
                    Response::builder().status(StatusCode::NOT_FOUND).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Agent route not found"}"#))).unwrap()
                }
            } else {
                Response::builder().status(StatusCode::NOT_FOUND).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Agent route not found"}"#))).unwrap()
            };
            return Ok(resp);
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // ── Group chat API routes — handled locally by Rust (clawparty.db) ──
    if path == "/api/groupchats" || path.starts_with("/api/groupchats/") {
        if let Some(data_dir) = DATA_DIR.get() {
            let resp = if path == "/api/groupchats" && method == hyper::Method::GET {
                crate::groupchats::list_group_chats(data_dir).await
            } else if path == "/api/groupchats" && method == hyper::Method::POST {
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::groupchats::create_group_chat(data_dir, body_bytes).await
            } else if path.starts_with("/api/groupchats/") {
                let rest = &path["/api/groupchats/".len()..];
                if let Some(slash) = rest.find('/') {
                    let group_id = &rest[..slash];
                    let action = &rest[slash + 1..];
                    if action == "members" && method == hyper::Method::GET {
                        crate::groupchats::get_members(data_dir, group_id).await
                    } else if action == "members" && method == hyper::Method::POST {
                        let body_bytes = match req.collect().await {
                            Ok(body) => body.to_bytes(),
                            Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                        };
                        crate::groupchats::add_member(data_dir, group_id, body_bytes).await
                    } else if action.starts_with("members/") && method == hyper::Method::DELETE {
                        let agent_name = &action["members/".len()..];
                        crate::groupchats::remove_member(data_dir, group_id, agent_name).await
                    } else if action == "messages" && method == hyper::Method::GET {
                        crate::groupchats::get_messages(data_dir, group_id).await
                    } else if action == "messages" && method == hyper::Method::POST {
                        let body_bytes = match req.collect().await {
                            Ok(body) => body.to_bytes(),
                            Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                        };
                        crate::groupchats::post_message(data_dir, group_id, body_bytes).await
                    } else {
                        Response::builder().status(StatusCode::NOT_FOUND).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Group chat route not found"}"#))).unwrap()
                    }
                } else {
                    let group_id = rest;
                    if method == hyper::Method::GET {
                        crate::groupchats::get_group_chat(data_dir, group_id).await
                    } else if method == hyper::Method::PUT {
                        let body_bytes = match req.collect().await {
                            Ok(body) => body.to_bytes(),
                            Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                        };
                        crate::groupchats::update_group_chat(data_dir, group_id, body_bytes).await
                    } else if method == hyper::Method::DELETE {
                        crate::groupchats::delete_group_chat(data_dir, group_id).await
                    } else {
                        Response::builder().status(StatusCode::NOT_FOUND).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Group chat route not found"}"#))).unwrap()
                    }
                }
            } else {
                Response::builder().status(StatusCode::NOT_FOUND).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Group chat route not found"}"#))).unwrap()
            };
            return Ok(resp);
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // Kanban API routes — handled locally by Rust (clawparty.db)
    if path == "/api/kanban" {
        if let Some(data_dir) = DATA_DIR.get() {
            let query = req.uri().query().unwrap_or("");
            let resp = if method == hyper::Method::GET {
                let agent_name = url::form_urlencoded::parse(query.as_bytes())
                    .find(|(k, _)| k == "agent")
                    .map(|(_, v)| v.to_string());
                let group_id = url::form_urlencoded::parse(query.as_bytes())
                    .find(|(k, _)| k == "group")
                    .map(|(_, v)| v.to_string());
                crate::kanban::get_kanban(data_dir, agent_name.as_deref(), group_id.as_deref()).await
            } else if method == hyper::Method::PUT {
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::kanban::update_kanban(data_dir, body_bytes).await
            } else {
                Response::builder().status(StatusCode::METHOD_NOT_ALLOWED).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Method not allowed"}"#))).unwrap()
            };
            return Ok(resp);
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // Global Config API routes — handled locally by Rust
    if path == "/api/global-config" {
        if let Some(data_dir) = DATA_DIR.get() {
            let resp = if method == hyper::Method::GET {
                crate::global_config::get_global_config(data_dir).await
            } else if method == hyper::Method::PUT {
                let body_bytes = match req.collect().await {
                    Ok(body) => body.to_bytes(),
                    Err(_) => return Ok(Response::builder().status(StatusCode::BAD_REQUEST).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Failed to read body"}"#))).unwrap()),
                };
                crate::global_config::update_global_config(data_dir, body_bytes).await
            } else {
                Response::builder().status(StatusCode::METHOD_NOT_ALLOWED).header(header::CONTENT_TYPE, "application/json").body(box_body(Bytes::from(r#"{"error":"Method not allowed"}"#))).unwrap()
            };
            return Ok(resp);
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // Radar API routes — handled locally by Rust (bypass ztm agent)
    if path.starts_with("/api/radar/") {
        if let Some(data_dir) = DATA_DIR.get() {
            if let Some(resp) = crate::radar::route(data_dir, &path, &method, req).await {
                return Ok(resp);
            }
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // WebShare API routes — handled locally by Rust (bypass ztm agent)
    if path.starts_with("/api/webshare/") {
        if let Some(data_dir) = DATA_DIR.get() {
            if let Some(resp) = crate::webshare::route(data_dir, &path, &method, req).await {
                return Ok(resp);
            }
        }
        let mut resp = Response::new(box_body(Bytes::from(r#"{"error":"Service Unavailable"}"#)));
        *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(resp);
    }

    // Join Party API — register this endpoint to a ZTM Hub and join the mesh
    if path == "/api/join-party" && method == hyper::Method::POST {
        match handle_join_party(req).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                ts_eprint!("[Proxy] Join-party error: {}", e);
                let mut resp = Response::new(box_body(
                    Bytes::from(serde_json::json!({"status":500,"message":e.to_string()}).to_string())
                ));
                *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                return Ok(resp);
            }
        }
    }

    // Meshes API — skip backend when ZTM is disabled, otherwise try upstream
    if path.starts_with("/api/meshes") && method == hyper::Method::GET {
        if ZEROCLAW_ONLY.get().copied().unwrap_or(false) {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(box_body(Bytes::from("[]")))
                .unwrap());
        }
        match proxy_http(req).await {
            Ok(resp) => {
                if resp.status() == StatusCode::BAD_GATEWAY {
                    let mut fallback = Response::new(box_body(Bytes::from("[]")));
                    *fallback.status_mut() = StatusCode::OK;
                    Ok(fallback)
                } else {
                    Ok(resp)
                }
            }
            Err(e) => {
                ts_eprint!("[Proxy] Meshes proxy error: {}", e);
                let mut resp = Response::new(box_body(Bytes::from("[]")));
                *resp.status_mut() = StatusCode::OK;
                Ok(resp)
            }
        }
    } else if is_websocket_request(&req) {
        match proxy_websocket(req).await {
            Ok(resp) => Ok(resp),
            Err(e) => {
                ts_eprint!("[Proxy] WebSocket proxy error: {}", e);
                let mut resp = Response::new(box_body(Bytes::from("WebSocket proxy error")));
                *resp.status_mut() = StatusCode::BAD_GATEWAY;
                Ok(resp)
            }
        }
    } else {
        let path = req.uri().path().to_string();
        // API routes go to backend (ztm:6789 or zeroclaw:42617)
        if path.starts_with("/api/") {
            if ZEROCLAW_ONLY.get().copied().unwrap_or(false) && !path.starts_with("/api/zeroclaw/") {
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(box_body(Bytes::from("{}")))
                    .unwrap());
            }
            match proxy_http(req).await {
                Ok(resp) => Ok(resp),
                Err(e) => {
                    ts_eprint!("[Proxy] HTTP proxy error: {}", e);
                    let mut resp = Response::new(box_body(Bytes::from("Proxy error")));
                    *resp.status_mut() = StatusCode::BAD_GATEWAY;
                    Ok(resp)
                }
            }
        } else {
            // Everything else serves embedded GUI static files (SPA fallback)
            match crate::static_files::serve(req).await {
                Ok(resp) => Ok(resp),
                Err(e) => {
                    ts_eprint!("[Proxy] Static file error: {}", e);
                    let mut resp = Response::new(box_body(Bytes::from("Internal Server Error")));
                    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    Ok(resp)
                }
            }
        }
    }
}

/// Run HTTP redirect server (80 -> 443).
async fn run_http_redirect(port: u16) {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            ts_eprint!("[Proxy] WARN: Failed to bind HTTP redirect port {}: {}", port, e);
            return;
        }
    };

    ts_print!("[Proxy] HTTP redirect listening on http://{}", addr);

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                ts_eprint!("[Proxy] HTTP accept error: {}", e);
                continue;
            }
        };

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req: Request<Incoming>| {
                let host = req
                    .headers()
                    .get(header::HOST)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("localhost")
                    .to_string();
                let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
                let location = format!("https://{}{}", host, path);

                async move {
                    let resp = Response::builder()
                        .status(StatusCode::MOVED_PERMANENTLY)
                        .header(header::LOCATION, location)
                        .body(box_body(Bytes::new()))
                        .unwrap();
                    Ok::<_, Infallible>(resp)
                }
            });

            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                ts_eprint!("[Proxy] HTTP redirect connection error from {}: {}", peer_addr, e);
            }
        });
    }
}

/// Run HTTPS proxy server.
async fn run_https_proxy(port: u16, cert_dir: &str) -> anyhow::Result<()> {
    let (cert_pem, key_pem) = ensure_cert(cert_dir)?;
    let tls_config = load_tls_config(&cert_pem, &key_pem)?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            ts_eprint!(
                "[Proxy] WARN: Failed to bind HTTPS proxy port {}: {}. Proxy disabled.",
                port, e
            );
            return Ok(());
        }
    };

    ts_print!("[Proxy] HTTPS proxy listening on https://{}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    ts_eprint!("[Proxy] TLS accept error from {}: {}", peer_addr, e);
                    return;
                }
            };

            let io = TokioIo::new(tls_stream);
            let service = service_fn(handle_request);

            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                ts_eprint!("[Proxy] HTTPS connection error from {}: {}", peer_addr, e);
            }
        });
    }
}

/// Start both HTTP redirect and HTTPS proxy servers.
pub async fn start(https_port: u16, http_port: u16, cert_dir: &str, data_dir: &str, engine: &str) {
    let expanded = data_dir.replace("~", &std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    let _ = DB_PATH.get_or_init(|| format!("{}/clawparty.db", expanded));
    let _ = DATA_DIR.get_or_init(|| expanded.clone());
    let _ = ENGINE.get_or_init(|| engine.to_string());

    let redirect = run_http_redirect(http_port);
    let proxy = run_https_proxy(https_port, cert_dir);

    tokio::select! {
        _ = redirect => {},
        _ = proxy => {},
    }
}
