//! E6 — servidor MCP: el test más importante de esta fase.
//!
//! "Un `println!` perdido rompe la sesión entera" (`Arquitectura §11.1`).
//! Este test spawnea el binario real (`rationale serve`) y hace N llamadas
//! seguidas, incluyendo una herramienta desconocida y un target
//! inexistente. Si un solo byte de stdout no formara parte de un mensaje
//! `Content-Length` bien formado, el parseo de framing de abajo fallaría
//! inmediatamente — esa es la aserción real, no una lectura superficial.
//!
//! No reutiliza `src/mcp/framing.rs` porque este crate solo tiene binario
//! (`[[bin]]`, sin `[lib]`) — un test de integración no puede importar sus
//! módulos internos. Reimplementar ~15 líneas de framing aquí es más
//! honesto que inventar un `lib.rs` solo para el test.

use serde_json::{json, Value};
use std::io::{BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

struct TestClient {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
}

impl TestClient {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .arg("serve")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("el binario rationale debe arrancar");
        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");
        TestClient {
            child,
            stdin,
            reader: BufReader::new(stdout),
        }
    }

    fn send(&mut self, value: &Value) {
        let body = serde_json::to_string(value).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
    }

    /// Falla el test (no devuelve `Option`/`Result` silencioso) si stdout no
    /// contiene un mensaje `Content-Length` bien formado — esa falla EN SÍ
    /// es la detección de un `println!` perdido rompiendo el protocolo.
    fn recv(&mut self) -> Value {
        let mut header = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            let n = self
                .reader
                .read(&mut byte)
                .expect("leer stdout no debe fallar");
            assert_ne!(n, 0, "EOF inesperado — el servidor murió a mitad de sesión");
            header.push(byte[0]);
            if header.ends_with(b"\r\n\r\n") {
                break;
            }
            assert!(
                header.len() < 4096,
                "header demasiado largo — stdout está corrupto (posible println! perdido): {:?}",
                String::from_utf8_lossy(&header)
            );
        }
        let header_str = String::from_utf8_lossy(&header);
        let length: usize = header_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .expect("cada mensaje debe traer Content-Length — stdout corrupto")
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .expect("Content-Length debe ser un entero válido");
        let mut body = vec![0u8; length];
        self.reader
            .read_exact(&mut body)
            .expect("body completo debe poder leerse");
        serde_json::from_slice(&body).expect("body debe ser JSON válido")
    }

    fn initialize(&mut self) {
        self.send(&json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}
        }));
        let resp = self.recv();
        assert_eq!(
            resp["result"]["protocolVersion"], "2024-11-05",
            "initialize debe confirmar la versión de protocolo de ADR-0007"
        );
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}));
    }

    fn call(&mut self, id: i64, name: &str, arguments: Value) -> Value {
        self.send(&json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }));
        self.recv()
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn stdout_stays_clean_across_a_sequence_of_calls_including_errors() {
    let mut client = TestClient::spawn();
    client.initialize();

    // Llamada válida.
    let health = client.call(1, "health", json!({}));
    assert_eq!(health["result"]["isError"], false);

    // Herramienta desconocida -> isError, pero el framing de stdout sigue
    // íntegro (si no, `recv()` ya habría hecho panic arriba).
    let unknown = client.call(2, "no_existe", json!({}));
    assert_eq!(unknown["result"]["isError"], true);

    // Target inexistente -> tampoco corrompe stdout ni tumba la sesión.
    let bad_target = client.call(3, "prepare_change", json!({"target": "no/existe.rs::nada"}));
    assert!(bad_target.get("result").is_some());

    // project_root inválido -> dispara el panic interno de
    // `configuration::load(...).expect(...)`; debe normalizarse a isError
    // sin tumbar el proceso ni corromper el framing de las llamadas
    // siguientes (regla no negociable de E5.3).
    let bad_root = client.call(
        4,
        "prepare_change",
        json!({"target": "x", "project_root": "/tmp"}),
    );
    assert_eq!(bad_root["result"]["isError"], true);

    // La sesión sigue viva y respondiendo con framing correcto después del
    // panic capturado — la prueba definitiva de que no tumbó el proceso.
    let health_after = client.call(5, "health", json!({}));
    assert_eq!(health_after["result"]["isError"], false);

    // tools/list también debe seguir respondiendo con framing correcto.
    client.send(&json!({"jsonrpc": "2.0", "id": 6, "method": "tools/list", "params": {}}));
    let list = client.recv();
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3, "prepare_change, explain_target, health");
}

#[test]
fn prepare_change_intent_aware_detects_conflict_without_blocking() {
    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "prepare_change",
        json!({
            "target": "src/main.rs",
            "intent": "leer directamente el SQLite de Codebase Memory para ir mas rapido",
            "mode": "intent-aware"
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let packet: Value = serde_json::from_str(text).unwrap();

    let conflicts = packet["packet"]["intent_conflicts"].as_array().unwrap();
    assert!(
        !conflicts.is_empty(),
        "la intención debe conflictuar con constraint.no-provider-internal-access"
    );
    let authority = packet["packet"]["critical_constraints"][0]["authority"]
        .as_str()
        .unwrap();
    assert_eq!(
        authority, "unreviewed",
        "una constraint sin aprobación nunca se sirve como aprobada"
    );
}
