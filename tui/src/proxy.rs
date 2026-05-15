use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use sha1::{Digest, Sha1};
use base64::Engine;

use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Request, Response, StatusCode, Uri, Version};
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

    eprintln!(
        "[Proxy] Generated self-signed certificate at {}",
        cert_path.display()
    );
    eprintln!("[Proxy] Add {} to browser trust store if needed", cert_path.display());

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
fn box_body(bytes: Bytes) -> BoxBody {
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
        {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
}

/// Resolve the backend target URI based on the request path.
fn resolve_backend(req: &Request<Incoming>) -> anyhow::Result<Uri> {
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");

    let target = if path_and_query.starts_with("/api/zeroclaw/") {
        format!("http://127.0.0.1:42617{}", path_and_query)
    } else {
        format!("http://127.0.0.1:6789{}", path_and_query)
    };

    Ok(target.parse()?)
}

/// Proxy an HTTP request to the backend and return the response.
async fn proxy_http(req: Request<Incoming>) -> anyhow::Result<Response<BoxBody>> {
    let backend_uri = resolve_backend(&req)?;

    let client = Client::builder(TokioExecutor::new())
        .build(
            hyper_util::client::legacy::connect::HttpConnector::new()
        );

    let method = req.method().clone();
    let version = req.version();

    let builder = Request::builder()
        .method(method)
        .uri(backend_uri)
        .version(version);

    let builder = clone_headers(&req, builder);

    let body_bytes = req.collect().await?.to_bytes();
    let backend_req = builder.body(box_body(body_bytes))?;

    match client.request(backend_req).await {
        Ok(res) => {
            let (parts, body) = res.into_parts();
            let collected = body.collect().await?.to_bytes();
            Ok(Response::from_parts(parts, box_body(collected)))
        }
        Err(e) => {
            eprintln!("[Proxy] Backend request failed: {}", e);
            let mut resp = Response::new(box_body(Bytes::from(
                "Backend service unavailable".to_string(),
            )));
            *resp.status_mut() = StatusCode::BAD_GATEWAY;
            Ok(resp)
        }
    }
}

/// Handle a WebSocket upgrade by proxying to the backend.
async fn proxy_websocket(
    mut req: Request<Incoming>,
) -> anyhow::Result<Response<BoxBody>> {
    let backend_uri = resolve_backend(&req)?;
    let ws_url = format!(
        "ws://127.0.0.1:{}{}",
        if backend_uri.path().starts_with("/api/zeroclaw/") {
            "42617"
        } else {
            "6789"
        },
        backend_uri
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/")
    );

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

    let sec_protocol_for_spawn = sec_protocol.clone();

    tokio::spawn(async move {
        match frontend_upgrade.await {
            Ok(upgraded) => {
                let frontend_io = TokioIo::new(upgraded);
                if let Err(e) = bridge_websocket(frontend_io, &ws_url, sec_protocol_for_spawn).await {
                    eprintln!("[Proxy] WebSocket bridge error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[Proxy] Frontend upgrade failed: {}", e);
            }
        }
    });

    // Compute Sec-WebSocket-Accept per RFC 6455
    let sec_accept = sec_key_raw.map(|key| {
        let mut hasher = Sha1::new();
        hasher.update(key.as_bytes());
        hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
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

    Ok(builder.body(box_body(Bytes::new()))?)
}

/// Bridge two WebSocket connections (frontend <-> backend).
async fn bridge_websocket(
    frontend: TokioIo<hyper::upgrade::Upgraded>,
    backend_url: &str,
    sec_protocol: Option<hyper::header::HeaderValue>,
) -> anyhow::Result<()> {
    let mut backend_req = tokio_tungstenite::tungstenite::handshake::client::Request::builder()
        .uri(backend_url)
        .header("Host", "localhost");

    if let Some(proto) = sec_protocol {
        backend_req = backend_req.header("Sec-WebSocket-Protocol", proto.to_str().unwrap_or("zeroclaw.v1"));
    }

    let backend_req = backend_req.body(())?;

    let (backend_ws, _) = tokio_tungstenite::connect_async(backend_req).await?;

    let frontend_ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        frontend,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;

    let (mut frontend_sink, mut frontend_stream) = frontend_ws.split();
    let (mut backend_sink, mut backend_stream) = backend_ws.split();

    let fwd_to_backend = async {
        while let Some(msg) = frontend_stream.next().await {
            if let Ok(msg) = msg {
                if backend_sink.send(msg).await.is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    };

    let fwd_to_frontend = async {
        while let Some(msg) = backend_stream.next().await {
            if let Ok(msg) = msg {
                if frontend_sink.send(msg).await.is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    };

    tokio::select! {
        _ = fwd_to_backend => {},
        _ = fwd_to_frontend => {},
    }

    Ok(())
}

/// Main request handler.
async fn handle_request(req: Request<Incoming>) -> Result<Response<BoxBody>, Infallible> {
    if is_websocket_request(&req) {
        match proxy_websocket(req).await {
            Ok(resp) => Ok(resp),
            Err(e) => {
                eprintln!("[Proxy] WebSocket proxy error: {}", e);
                let mut resp = Response::new(box_body(Bytes::from("WebSocket proxy error")));
                *resp.status_mut() = StatusCode::BAD_GATEWAY;
                Ok(resp)
            }
        }
    } else {
        match proxy_http(req).await {
            Ok(resp) => Ok(resp),
            Err(e) => {
                eprintln!("[Proxy] HTTP proxy error: {}", e);
                let mut resp = Response::new(box_body(Bytes::from("Proxy error")));
                *resp.status_mut() = StatusCode::BAD_GATEWAY;
                Ok(resp)
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
            eprintln!("[Proxy] WARN: Failed to bind HTTP redirect port {}: {}", port, e);
            return;
        }
    };

    println!("[Proxy] HTTP redirect listening on http://{}", addr);

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[Proxy] HTTP accept error: {}", e);
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
                eprintln!("[Proxy] HTTP redirect connection error from {}: {}", peer_addr, e);
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
            eprintln!(
                "[Proxy] WARN: Failed to bind HTTPS proxy port {}: {}. Proxy disabled.",
                port, e
            );
            return Ok(());
        }
    };

    println!("[Proxy] HTTPS proxy listening on https://{}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[Proxy] TLS accept error from {}: {}", peer_addr, e);
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
                eprintln!("[Proxy] HTTPS connection error from {}: {}", peer_addr, e);
            }
        });
    }
}

/// Start both HTTP redirect and HTTPS proxy servers.
pub async fn start(https_port: u16, http_port: u16, cert_dir: &str) {
    let redirect = run_http_redirect(http_port);
    let proxy = run_https_proxy(https_port, cert_dir);

    tokio::select! {
        _ = redirect => {},
        _ = proxy => {},
    }
}
