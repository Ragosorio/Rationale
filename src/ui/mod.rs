//! `rationale ui` — Control Room local (vNext).
//!
//! Observa el mismo estado local que la CLI y el servidor MCP (canon,
//! snapshots de operación y actividad) y lo sirve a un navegador en
//! 127.0.0.1: instantáneas REST y un stream SSE de actividad. No escribe nada
//! ni consulta a otro backend: `rationale serve` sigue siendo el servidor MCP
//! y esta interfaz nunca se convierte en una segunda autoridad.

mod api;
mod assets;
mod http;

use std::io::Write;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const DEFAULT_PORT: u16 = 9748;
/// Conexiones simultáneas (incluidos streams SSE): una UI local usa unas
/// pocas; un techo evita que un cliente descontrolado agote hilos.
const MAX_CONNECTIONS: usize = 64;
const SSE_POLL: Duration = Duration::from_millis(250);
const SSE_PING: Duration = Duration::from_secs(15);

pub struct Options {
    pub project_root: PathBuf,
    pub port: u16,
    /// Con `--port` explícito no se prueba otro puerto si está ocupado.
    pub port_explicit: bool,
    pub open_browser: bool,
}

fn bind(port: u16, explicit: bool) -> std::io::Result<TcpListener> {
    let attempts = if explicit || port == 0 { 1 } else { 10 };
    let mut last_error = None;
    for offset in 0..attempts {
        match TcpListener::bind((Ipv4Addr::LOCALHOST, port.saturating_add(offset))) {
            Ok(listener) => return Ok(listener),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| std::io::Error::other("sin intentos de bind")))
}

pub fn run(options: Options) -> Result<(), String> {
    let project = api::Project::load(&options.project_root)?;
    let listener = bind(options.port, options.port_explicit)
        .map_err(|e| format!("no se pudo escuchar en 127.0.0.1:{}: {e}", options.port))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("no se pudo leer el puerto local: {e}"))?
        .port();
    let url = format!("http://127.0.0.1:{port}/");
    eprintln!(
        "Rationale Control Room → {url}  (proyecto {}; solo 127.0.0.1; Ctrl+C para salir)",
        project.id
    );
    if !assets::has_ui() {
        eprintln!(
            "aviso: este binario no incluye la interfaz web; se sirve una página con las \
             instrucciones y la API local"
        );
    }
    if options.open_browser {
        open_browser(&url);
    }
    serve(listener, Arc::new(project));
    Ok(())
}

struct ConnectionSlot(Arc<AtomicUsize>);

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

fn serve(listener: TcpListener, project: Arc<api::Project>) {
    let port = listener.local_addr().map(|addr| addr.port()).unwrap_or(0);
    let active = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        if active.fetch_add(1, Ordering::Relaxed) >= MAX_CONNECTIONS {
            active.fetch_sub(1, Ordering::Relaxed);
            let _ = http::write_response(
                &mut stream,
                &http::Response::error(503, "demasiadas conexiones"),
                false,
            );
            continue;
        }
        let slot = ConnectionSlot(Arc::clone(&active));
        let project = Arc::clone(&project);
        std::thread::spawn(move || {
            let _slot = slot;
            handle(stream, &project, port);
        });
    }
}

fn handle(mut stream: TcpStream, project: &api::Project, port: u16) {
    let _ = stream.set_read_timeout(Some(http::IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(http::IO_TIMEOUT));
    let request = match http::read_request(&mut stream) {
        Ok(request) => request,
        Err(http::ParseError::Closed) => return,
        Err(http::ParseError::TooLarge) => {
            let response = http::Response::error(431, "cabecera demasiado grande");
            let _ = http::write_response(&mut stream, &response, false);
            return;
        }
        Err(http::ParseError::Malformed(reason)) => {
            let _ = http::write_response(&mut stream, &http::Response::error(400, reason), false);
            return;
        }
    };
    if !http::host_allowed(request.header("Host"), port) {
        let response = http::Response::error(421, "host no permitido: solo 127.0.0.1 o localhost");
        let _ = http::write_response(&mut stream, &response, false);
        return;
    }
    let head_only = request.method == "HEAD";
    if request.method != "GET" && !head_only {
        let mut response = http::Response::error(405, "la UI es de solo lectura");
        response
            .extra_headers
            .push(("Allow", "GET, HEAD".to_string()));
        let _ = http::write_response(&mut stream, &response, false);
        return;
    }
    if request.path == "/events" && !head_only {
        stream_events(stream, project);
        return;
    }
    let response = route(&request, project);
    let _ = http::write_response(&mut stream, &response, head_only);
}

fn route(request: &http::Request, project: &api::Project) -> http::Response {
    let number = |name: &str, default: usize| {
        request
            .query_value(name)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(default)
    };
    let path = request.path.as_str();
    let result: api::ApiResult = match path {
        "/api/meta" => Ok(api::meta(project)),
        "/api/activity" => Ok(api::activity(project, number("limit", 500))),
        "/api/operations" => Ok(api::operations(project, number("limit", 50))),
        "/api/graph" => api::graph(
            project,
            request.query_value("operation"),
            number("recent", 5),
        ),
        "/api/records" => Ok(api::records(project)),
        "/api/conflicts" => Ok(api::conflicts(project)),
        _ => {
            if let Some(id) = path.strip_prefix("/api/operations/") {
                api::operation(project, id)
            } else if let Some(key) = path.strip_prefix("/api/node/") {
                api::node(project, key)
            } else if let Some(key) = path.strip_prefix("/api/edge/") {
                api::edge(project, key)
            } else if path.starts_with("/api/") || path == "/events" {
                Err((404, "endpoint desconocido".to_string()))
            } else {
                return static_asset(path);
            }
        }
    };
    match result {
        Ok(value) => http::Response::json(&value),
        Err((status, message)) => http::Response::error(status, &message),
    }
}

fn static_asset(path: &str) -> http::Response {
    if !assets::has_ui() {
        return if path == "/" || path == "/index.html" {
            http::Response::html(assets::FALLBACK_HTML)
        } else {
            http::Response::error(404, "no encontrado")
        };
    }
    let asset = match assets::find(path) {
        Some(asset) => asset,
        // Rutas de la aplicación (sin extensión) sirven el shell de la UI.
        None if !path.rsplit('/').next().unwrap_or("").contains('.') => match assets::find("/") {
            Some(index) => index,
            None => return http::Response::error(404, "no encontrado"),
        },
        None => return http::Response::error(404, "no encontrado"),
    };
    http::Response {
        status: 200,
        content_type: asset.content_type,
        body: std::borrow::Cow::Borrowed(asset.bytes),
        // Vite nombra los assets con hash de contenido: son inmutables.
        cache_control: if asset.path.starts_with("assets/") {
            "public, max-age=31536000, immutable"
        } else {
            "no-store"
        },
        extra_headers: Vec::new(),
    }
}

/// Stream de actividad: cada línea completa nueva de cualquier sesión, en
/// orden temporal. El historial se pide aparte a `/api/activity`.
fn stream_events(mut stream: TcpStream, project: &api::Project) {
    if http::write_sse_head(&mut stream).is_err() {
        return;
    }
    let mut tail = crate::activity::Tail::from_end(&project.local_dir);
    let mut last_write = Instant::now();
    loop {
        let events = tail.poll();
        if !events.is_empty() {
            for event in &events {
                let Ok(data) = serde_json::to_string(event) else {
                    continue;
                };
                let id = format!("{}:{}", event.session_id, event.seq);
                if stream
                    .write_all(http::sse_event(&id, "activity", &data).as_bytes())
                    .is_err()
                {
                    return;
                }
            }
            if stream.flush().is_err() {
                return;
            }
            last_write = Instant::now();
        } else if last_write.elapsed() >= SSE_PING {
            // Un comentario SSE mantiene viva la conexión y detecta al
            // cliente que se fue: la escritura falla y el hilo termina.
            if stream
                .write_all(b": ping\n\n")
                .and_then(|_| stream.flush())
                .is_err()
            {
                return;
            }
            last_write = Instant::now();
        }
        std::thread::sleep(SSE_POLL);
    }
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    if let Err(error) = result {
        eprintln!("aviso: no se pudo abrir el navegador ({error}); abre {url}");
    }
}

/// Para tests: el mismo servidor en un puerto efímero, en segundo plano.
#[cfg(test)]
fn spawn_for_tests(project_root: &std::path::Path) -> u16 {
    let project = api::Project::load(project_root).expect("proyecto de prueba");
    let listener = bind(0, true).expect("puerto efímero");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || serve(listener, Arc::new(project)));
    port
}

#[cfg(test)]
fn get(port: u16, path: &str, host: Option<&str>, method: &str) -> (u16, String, String) {
    use std::io::Read;
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let host = host.map_or_else(|| format!("127.0.0.1:{port}"), str::to_string);
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, head.to_string(), body.to_string())
}

#[cfg(test)]
fn get_json(port: u16, path: &str) -> (u16, serde_json::Value) {
    let (status, _, body) = get(port, path, None, "GET");
    (
        status,
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::ActorContext;
    use crate::operations::{GraphEdge, GraphNode, Operation, OperationTarget};
    use crate::providers::{NodeBinding, RelationKind, StructuralRelationship};
    use serde_json::Value;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    const RECORD: &str = "id: decision.expiry-per-entity
kind: decision
severity: high
statement: Payment expiration belongs to entity configuration.
rationale: Each tenant defines its own payment policy.
relationship_bindings:
- id: rel.0
  source:
    file_path: src/payments.rs
    qualified_name: src.payments.create_link
  kind: uses
  target:
    file_path: src/config.rs
    qualified_name: src.config.EntityConfig.payment_expiration
";

    struct Fixture {
        dir: PathBuf,
        port: u16,
        source: NodeBinding,
        target: NodeBinding,
        edge_key: String,
        operation_id: String,
    }

    fn actor() -> ActorContext {
        ActorContext {
            client: "claude-code".to_string(),
            client_source: "flag".to_string(),
            session_id: None,
            operation_id: None,
        }
    }

    fn fixture() -> Fixture {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-ui-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join(".rationale/records")).unwrap();
        std::fs::write(
            dir.join(".rationale/records/decision.expiry-per-entity.yaml"),
            RECORD,
        )
        .unwrap();
        let project = api::Project::load(&dir).unwrap();
        let binding = |file: &str, qn: &str| NodeBinding {
            file_path: file.to_string(),
            qualified_name: qn.to_string(),
            symbol_kind: None,
        };
        let source = binding("src/payments.rs", "src.payments.create_link");
        let target = binding(
            "src/config.rs",
            "src.config.EntityConfig.payment_expiration",
        );
        let edge_key = StructuralRelationship::relationship_key(
            &project.id,
            &source,
            RelationKind::Uses,
            &target,
        );
        let node = |binding: &NodeBinding, role: &str| GraphNode {
            key: binding.key(&project.id),
            name: binding.short_name().to_string(),
            label: "function".to_string(),
            file_path: binding.file_path.clone(),
            qualified_name: binding.qualified_name.clone(),
            role: role.to_string(),
            selected: true,
            record_ids: vec!["decision.expiry-per-entity".to_string()],
            start_line: Some(1),
        };
        let mut operation = Operation::new(
            &actor(),
            OperationTarget {
                spec: "src/payments.rs::create_link".to_string(),
                file_path: Some("src/payments.rs".to_string()),
                symbol: Some("create_link".to_string()),
                node_key: Some(source.key(&project.id)),
                qualified_name: Some(source.qualified_name.clone()),
            },
            Some("make expiration configurable".to_string()),
            None,
        );
        operation.graph.nodes = vec![node(&source, "target"), node(&target, "explained")];
        operation.graph.edges = vec![GraphEdge {
            key: edge_key.clone(),
            source: source.key(&project.id),
            kind: "uses".to_string(),
            target: target.key(&project.id),
            selected: true,
            state: "observed".to_string(),
            record_ids: vec!["decision.expiry-per-entity".to_string()],
        }];
        crate::operations::save(&project.local_dir, &operation).unwrap();
        let port = spawn_for_tests(&dir);
        Fixture {
            dir,
            port,
            source,
            target,
            edge_key,
            operation_id: operation.operation_id,
        }
    }

    #[test]
    fn snapshots_expose_the_working_subgraph_with_its_causal_overlay() {
        let fx = fixture();
        let project_id = api::Project::load(&fx.dir).unwrap().id;

        let (status, meta) = get_json(fx.port, "/api/meta");
        assert_eq!(status, 200);
        assert_eq!(meta["canon"]["records"], 1);
        assert_eq!(meta["canon"]["active"], 1);
        assert_eq!(
            meta["latest_operation"]["operation_id"],
            fx.operation_id.as_str()
        );

        let (_, graph) = get_json(fx.port, "/api/graph");
        assert_eq!(graph["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(graph["edges"][0]["state"], "observed");
        assert_eq!(
            graph["focus"]["node_key"],
            fx.source.key(&project_id).as_str()
        );
        assert_eq!(
            graph["records"]["decision.expiry-per-entity"]["rationale"],
            "Each tenant defines its own payment policy."
        );

        let (_, records) = get_json(fx.port, "/api/records");
        assert_eq!(records[0]["relationships"][0]["key"], fx.edge_key.as_str());

        let node_path = format!("/api/node/{}", fx.source.key(&project_id));
        let (status, node) = get_json(fx.port, &node_path);
        assert_eq!(status, 200, "{node}");
        assert_eq!(node["outgoing"][0]["kind"], "uses");
        assert_eq!(
            node["outgoing"][0]["node"]["qualified_name"],
            fx.target.qualified_name.as_str()
        );

        let (status, edge) = get_json(fx.port, &format!("/api/edge/{}", fx.edge_key));
        assert_eq!(status, 200, "{edge}");
        assert_eq!(edge["history"][0]["state"], "observed");
        assert!(edge["records"]["decision.expiry-per-entity"].is_object());

        let (status, operation) =
            get_json(fx.port, &format!("/api/operations/{}", fx.operation_id));
        assert_eq!(status, 200);
        assert_eq!(operation["intent"], "make expiration configurable");
        std::fs::remove_dir_all(&fx.dir).ok();
    }

    #[test]
    fn hostile_or_unsupported_requests_are_refused() {
        let fx = fixture();
        let (status, _, _) = get(fx.port, "/api/meta", Some("evil.example:80"), "GET");
        assert_eq!(status, 421, "DNS rebinding");
        let (status, head, _) = get(fx.port, "/api/meta", None, "POST");
        assert_eq!(status, 405);
        assert!(head.contains("Allow: GET, HEAD"));
        assert_eq!(get_json(fx.port, "/api/operations/op_missing").0, 404);
        assert_eq!(get_json(fx.port, "/api/operations/..%2F..%2Fsecret").0, 400);
        assert_eq!(get_json(fx.port, "/api/node/n_nope").0, 400);
        assert_eq!(get_json(fx.port, "/api/nothing").0, 404);
        let (status, head, _) = get(fx.port, "/", None, "GET");
        assert_eq!(status, 200);
        assert!(head.contains("text/html"));
        assert!(head.contains("Content-Security-Policy"));
        std::fs::remove_dir_all(&fx.dir).ok();
    }

    #[test]
    fn the_event_stream_delivers_new_activity_live() {
        let fx = fixture();
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, fx.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET /events HTTP/1.1\r\nHost: localhost:{}\r\n\r\n",
            fx.port
        )
        .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.starts_with("HTTP/1.1 200"), "{line}");

        // Tras el primer poll del servidor, un agente registra actividad.
        std::thread::sleep(Duration::from_millis(400));
        let project = api::Project::load(&fx.dir).unwrap();
        let recorder = crate::activity::Recorder::with_enabled(Some("session_live"), true);
        let scope = crate::activity::Scope::new(
            &project.local_dir,
            Path::new(&fx.dir),
            &project.id,
            &actor(),
        );
        recorder.emit(
            &scope,
            "context.requested",
            crate::activity::payload::context_requested("src/payments.rs::create_link", None),
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut kinds = Vec::new();
        while Instant::now() < deadline && !kinds.contains(&"context.requested".to_string()) {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                let event: Value = serde_json::from_str(data.trim()).unwrap();
                kinds.push(event["kind"].as_str().unwrap().to_string());
            }
        }
        assert!(
            kinds.contains(&"context.requested".to_string()),
            "{kinds:?}"
        );
        std::fs::remove_dir_all(&fx.dir).ok();
    }
}
