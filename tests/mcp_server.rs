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

    /// Envía bytes crudos sin pasar por serialización JSON — necesario para
    /// los ataques de E7 (Content-Length astronómico, body malformado) que
    /// por definición no son JSON válido.
    fn send_raw(&mut self, bytes: &[u8]) {
        self.stdin.write_all(bytes).unwrap();
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
    assert_eq!(
        tools.len(),
        4,
        "prepare_change, explain_target, health, finalize_change"
    );
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

/// E7 hallazgo A1 — reproduce el ataque exacto de la revisión adversarial
/// contra el binario real compilado: un `Content-Length` astronómico antes
/// disparaba `handle_alloc_error` (SIGABRT). Debe rechazarse sin abortar el
/// proceso, y la sesión debe seguir viva para la llamada siguiente.
#[test]
fn astronomical_content_length_does_not_abort_the_process() {
    let mut client = TestClient::spawn();
    client.initialize();

    // Sin body: el límite de tamaño rechaza el header apenas se parsea
    // Content-Length, antes de intentar leer ningún byte del body (por
    // diseño — nunca se asigna memoria para un tamaño fuera de cota).
    client.send_raw(b"Content-Length: 999999999999999999\r\n\r\n");

    // El servidor debe responder con un error de parseo JSON-RPC en vez de
    // morir en silencio — si el proceso hubiera abortado, `recv()` fallaría
    // al leer EOF inesperado.
    let resp = client.recv();
    assert_eq!(resp["error"]["code"], -32700);

    // La sesión sigue viva: una llamada normal después del ataque funciona.
    let health = client.call(99, "health", json!({}));
    assert_eq!(health["result"]["isError"], false);
}

/// E7 hallazgo B — reproduce el ataque exacto: JSON sintácticamente
/// malformado. Antes era indistinguible de EOF y terminaba la sesión
/// persistente completa en silencio (exit 0, sin aviso al cliente).
#[test]
fn malformed_json_does_not_kill_the_persistent_session() {
    let mut client = TestClient::spawn();
    client.initialize();

    let body = b"{not valid json!!!";
    client.send_raw(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    client.send_raw(body);

    let resp = client.recv();
    assert_eq!(
        resp["error"]["code"], -32700,
        "un mensaje malformado debe responder con parse error, no matar la sesión"
    );

    // La prueba definitiva: la sesión sigue viva y respondiendo con framing
    // correcto después del mensaje malformado.
    let health = client.call(100, "health", json!({}));
    assert_eq!(health["result"]["isError"], false);
}

/// E7 hallazgo B (variante) — JSON válido pero por encima del límite de
/// recursión de `serde_json` (128 niveles) tampoco debe matar la sesión.
#[test]
fn deeply_nested_json_does_not_kill_the_persistent_session() {
    let mut client = TestClient::spawn();
    client.initialize();

    let depth = 200;
    let body = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    client.send_raw(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    client.send_raw(body.as_bytes());

    let resp = client.recv();
    assert_eq!(resp["error"]["code"], -32700);

    let health = client.call(101, "health", json!({}));
    assert_eq!(health["result"]["isError"], false);
}

fn run_git(dir: &std::path::Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} debe tener éxito");
}

/// PID + nanos + contador atómico: bajo carga extrema, la resolución real
/// del reloj puede no ser tan fina como promete `as_nanos()` — dos tests en
/// hilos paralelos podrían colisionar en el mismo directorio y correr
/// `git init` concurrente sobre él (mismo bug encontrado y corregido en
/// `src/capture.rs`). El contador lo hace imposible.
fn unique_suffix() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

/// Proyecto Rationale desechable con su propio repo Git — usado por los
/// tests de `finalize_change`, que necesitan un `base_revision` real y un
/// `.rationale/` propio (nunca el del repo de Rationale mismo).
fn make_test_project() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rationale-finalize-test-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    run_git(&dir, &["init", "-q"]);
    run_git(&dir, &["config", "user.email", "test@rationale.local"]);
    run_git(&dir, &["config", "user.name", "Rationale Test"]);

    for sub in ["subjects", "records", "proposals", "approvals", "bindings"] {
        std::fs::create_dir_all(dir.join(".rationale").join(sub)).unwrap();
    }
    std::fs::write(dir.join("README.md"), "proyecto de prueba\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "commit inicial"]);

    dir
}

#[test]
fn finalize_change_writes_pending_proposal_for_high_value_change() {
    let dir = make_test_project();
    let base_revision = {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };

    // Cambio real que toca autorización — debe activar señales y superar
    // Nivel 0.
    std::fs::create_dir_all(dir.join("src/auth")).unwrap();
    std::fs::write(
        dir.join("src/auth/authorization.ts"),
        "export function resolveEntityRole() { /* ... */ }\n",
    )
    .unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "add authorization resolver"]);

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "src/auth/authorization.ts",
            "base_revision": base_revision,
            "intent": "Staff users must never receive global super_admin access.",
            "statement": "Staff users must never receive global super_admin.",
            "record_id": "constraint.no-global-admin-for-staff-test",
            "subject_id": "authorization.entity-scoped-staff-access-test",
            "subject_title": "Entity-scoped staff authorization",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();

    assert_ne!(outcome["level"], "git-only");
    assert_eq!(outcome["proposal_written"], true);
    assert!(outcome["signals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s == "authorization" || s == "normative-language"));

    let proposal_path = outcome["proposal_path"].as_str().unwrap();
    let content = std::fs::read_to_string(proposal_path)
        .expect("la propuesta debe existir en disco como archivo real");
    assert!(content.contains("status: pending"));
    assert!(content.contains("approvals: []"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn finalize_change_skips_proposal_for_mechanical_only_change() {
    let dir = make_test_project();
    let base_revision = {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };

    // Solo un lockfile — Nivel 0, v0.5 §16: no se crea ningún registro.
    std::fs::write(dir.join("Cargo.lock"), "# lockfile\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "update lockfile"]);

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "Cargo.lock",
            "base_revision": base_revision,
            "intent": "Bump a dependency version.",
            "statement": "N/A",
            "record_id": "constraint.should-not-exist-test",
            "subject_id": "should.not-exist-test",
            "subject_title": "Should not exist",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();

    assert_eq!(outcome["level"], "git-only");
    assert_eq!(outcome["proposal_written"], false);
    assert!(outcome["proposal_path"].is_null());
    assert!(!dir
        .join(".rationale/proposals/constraint.should-not-exist-test.yaml")
        .exists());

    std::fs::remove_dir_all(&dir).ok();
}
