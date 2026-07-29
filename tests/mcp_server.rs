//! E6 — servidor MCP: el test más importante de esta fase.
//!
//! "Un `println!` perdido rompe la sesión entera" (`Arquitectura §11.1`).
//! Este test spawnea el binario real (`rationale serve`) y hace N llamadas
//! seguidas, incluyendo una herramienta desconocida y un target
//! inexistente. Si un solo byte de stdout no formara parte de una línea JSON
//! MCP bien formada, el parseo de framing de abajo fallaría
//! inmediatamente — esa es la aserción real, no una lectura superficial.
//!
//! No reutiliza `src/mcp/framing.rs` porque este crate solo tiene binario
//! (`[[bin]]`, sin `[lib]`) — un test de integración no puede importar sus
//! módulos internos. Reimplementar ~15 líneas de framing aquí es más
//! honesto que inventar un `lib.rs` solo para el test.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
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
        writeln!(self.stdin, "{body}").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Envía bytes crudos sin pasar por serialización JSON — necesario para
    /// probar mensajes malformados que por definición no son JSON válido.
    fn send_raw(&mut self, bytes: &[u8]) {
        self.stdin.write_all(bytes).unwrap();
        self.stdin.flush().unwrap();
    }

    /// Falla el test si stdout no contiene exactamente una línea JSON válida;
    /// texto auxiliar en stdout rompería el protocolo MCP.
    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .expect("leer stdout no debe fallar");
        assert!(
            !line.is_empty(),
            "EOF inesperado — el servidor murió a mitad de sesión"
        );
        serde_json::from_str(line.trim_end()).expect("cada línea stdout debe ser JSON válido")
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
        assert!(resp["result"]["capabilities"]["prompts"].is_object());
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}));
    }

    fn call(&mut self, id: i64, name: &str, arguments: Value) -> Value {
        self.send(&json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }));
        self.recv()
    }

    fn get_prompt(&mut self, id: i64, name: &str, arguments: Value) -> Value {
        self.send(&json!({
            "jsonrpc": "2.0", "id": id, "method": "prompts/get",
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
fn prompts_list_and_get_expose_the_six_actions_from_one_source() {
    let mut client = TestClient::spawn();
    client.initialize();

    client.send(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "prompts/list", "params": {}
    }));
    let list = client.recv();
    let prompts = list["result"]["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 6);
    assert_eq!(
        prompts
            .iter()
            .map(|prompt| prompt["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "preflight",
            "explain",
            "capture",
            "review",
            "health",
            "protocol"
        ]
    );

    let get = client.get_prompt(
        2,
        "preflight",
        json!({"target": "src/main.rs::cmd_init", "intent": "quitar el retorno temprano"}),
    );
    let text = get["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("src/main.rs::cmd_init"));
    assert!(text.contains("quitar el retorno temprano"));
    assert!(!text.contains("$target"));
    assert!(!text.contains("$intent"));

    let capture = client.get_prompt(3, "capture", json!({}));
    let capture_text = capture["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(capture_text.contains("prompt MCP"));
    assert!(capture_text.contains("herramientas Git disponibles"));
}

#[test]
fn an_unknown_prompt_is_json_rpc_error_and_the_session_stays_alive() {
    let mut client = TestClient::spawn();
    client.initialize();

    let unknown = client.get_prompt(1, "no-existe", json!({}));
    assert_eq!(unknown["error"]["code"], -32602);

    let health = client.call(2, "health", json!({}));
    assert_eq!(health["result"]["isError"], false);
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

    // project_root inválido -> debe convertirse en un error de herramienta
    // limpio, sin tumbar el proceso ni corromper el framing de las llamadas
    // siguientes (regla no negociable de E5.3).
    let no_rationale_dir = std::env::temp_dir().to_str().unwrap().to_string();
    let bad_root = client.call(
        4,
        "prepare_change",
        json!({"target": "x", "project_root": no_rationale_dir}),
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
    let finalize = tools
        .iter()
        .find(|tool| tool["name"] == "finalize_change")
        .unwrap();
    assert_eq!(
        finalize["inputSchema"]["properties"]["novelty_reason"]["type"],
        "object"
    );
    assert_eq!(
        finalize["inputSchema"]["properties"]["novelty_reason"]["required"],
        json!([
            "contrasted_subject",
            "difference_kind",
            "difference",
            "evidence"
        ])
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

/// v0.5 §4.18 define el modo por la presencia de intención, no por un flag
/// separado. El prompt maestro documentado (`docs/prompt-master.md`) solo
/// enseña `prepare_change(target, intent)` — nunca `mode` — así que un
/// caller que pase `intent` sin `mode` debe activar la detección de
/// conflictos igual que si hubiera pasado `mode: "intent-aware"`
/// explícito. Antes de este fix, `intent` se descartaba en silencio sin
/// `mode` explícito: el mismo síntoma exacto del bug real que motivó el
/// proyecto, reproducido por seguir el protocolo oficial al pie de la letra.
#[test]
fn prepare_change_honors_intent_without_an_explicit_mode() {
    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "prepare_change",
        json!({
            "target": "src/main.rs",
            "intent": "leer directamente el SQLite de Codebase Memory para ir mas rapido"
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let packet: Value = serde_json::from_str(text).unwrap();

    let conflicts = packet["packet"]["intent_conflicts"].as_array().unwrap();
    assert!(
        !conflicts.is_empty(),
        "sin `mode` explícito, `intent` debe seguir activando detección de conflictos"
    );
}

/// `mode: "baseline"` explícito sigue siendo la manera de forzar retrieval
/// puro sin detección de conflictos, aunque venga `intent` — el override
/// documentado en el schema, no el comportamiento por defecto.
#[test]
fn prepare_change_explicit_baseline_mode_still_suppresses_intent() {
    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "prepare_change",
        json!({
            "target": "src/main.rs",
            "intent": "leer directamente el SQLite de Codebase Memory para ir mas rapido",
            "mode": "baseline"
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let packet: Value = serde_json::from_str(text).unwrap();

    assert!(packet["packet"]["intent_conflicts"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(packet["packet"]["governance_verdict_required"], false);
}

/// Fase 1.2 — `prepare_change` y `explain_target` deben coincidir sobre el
/// MISMO conjunto de Records gobernantes para el mismo target: antes de
/// `binding_match`, `prepare` caía a `records.first()` cuando nada
/// matcheaba (un Record arbitrario) mientras `explain` devolvía vacío para
/// la misma consulta — confirmado en un dogfood real. Este test reproduce
/// esa situación con un Record cuyo binding es de ARCHIVO (no símbolo,
/// exactamente el caso que la propagación archivo→símbolo debe cubrir) y
/// verifica que ambas herramientas reportan el mismo Record gobernante.
#[test]
fn prepare_and_explain_agree_on_the_same_governing_record() {
    let dir = make_test_project();
    std::fs::create_dir_all(dir.join("app/_components")).unwrap();
    std::fs::write(
        dir.join("app/_components/party-experience.tsx"),
        "export function submitFile() { /* ... */ }\n",
    )
    .unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(
        &dir,
        &["commit", "-q", "-m", "add party experience component"],
    );

    std::fs::write(
        dir.join(".rationale/records/media-challenge-upload-state.yaml"),
        r#"
schema_version: rationale/0.1
id: media-challenge-upload-state
kind: constraint
severity: medium
statement: "Los retos multimedia deben bloquear el envío durante la carga."
epistemic_status: stated
approvals:
  - actor: "user:test"
    authority: contributor
    status: approved
binding_declarations:
  - id: binding.media-challenge-upload-state.0
    type: file
    path_hint: app/_components/party-experience.tsx
subject:
  id: media-challenge-upload
"#,
    )
    .unwrap();

    let mut client = TestClient::spawn();
    client.initialize();

    // El binding es de archivo (sin structural_id); la consulta es por un
    // SÍMBOLO dentro de ese archivo — la propagación archivo→símbolo debe
    // hacer que ambas herramientas lo encuentren.
    let target = "app/_components/party-experience.tsx::submitFile";

    let prepare_resp = client.call(
        1,
        "prepare_change",
        json!({
            "target": target,
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let prepare_text = prepare_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let prepare_outcome: Value = serde_json::from_str(prepare_text).unwrap();
    let prepare_ids: Vec<&str> = prepare_outcome["packet"]["critical_constraints"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["governs_target"] == true)
        .map(|c| c["id"].as_str().unwrap())
        .collect();

    let explain_resp = client.call(
        2,
        "explain_target",
        json!({
            "target": target,
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let explain_text = explain_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let explain_outcome: Value = serde_json::from_str(explain_text).unwrap();
    let explain_ids: Vec<&str> = explain_outcome["governing_records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();

    assert_eq!(
        prepare_ids, explain_ids,
        "prepare_change y explain_target deben coincidir sobre el mismo Record gobernante"
    );
    assert_eq!(
        explain_ids,
        vec!["media-challenge-upload-state"],
        "el binding de archivo debe gobernar el símbolo consultado dentro de ese archivo"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Control negativo del mismo defecto: un target que ningún Record
/// gobierna no debe producir un veredicto de gobernanza en ninguna de las
/// dos herramientas — antes, `prepare` caía a `records.first()` y afirmaba
/// gobernancia sobre un Record no relacionado.
#[test]
fn prepare_and_explain_agree_on_no_governance_for_unrelated_target() {
    let dir = make_test_project();
    std::fs::write(
        dir.join(".rationale/records/media-challenge-upload-state.yaml"),
        r#"
schema_version: rationale/0.1
id: media-challenge-upload-state
kind: constraint
severity: medium
statement: "Los retos multimedia deben bloquear el envío durante la carga."
epistemic_status: stated
approvals: []
binding_declarations:
  - id: binding.media-challenge-upload-state.0
    type: file
    path_hint: app/_components/party-experience.tsx
"#,
    )
    .unwrap();

    let mut client = TestClient::spawn();
    client.initialize();

    let prepare_resp = client.call(
        1,
        "prepare_change",
        json!({
            "target": "README.md",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let prepare_text = prepare_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let prepare_outcome: Value = serde_json::from_str(prepare_text).unwrap();
    let any_governs = prepare_outcome["packet"]["critical_constraints"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["governs_target"] == true);
    assert!(
        !any_governs,
        "README.md no está gobernado por el binding hacia party-experience.tsx"
    );

    let explain_resp = client.call(
        2,
        "explain_target",
        json!({
            "target": "README.md",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let explain_text = explain_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let explain_outcome: Value = serde_json::from_str(explain_text).unwrap();
    assert!(explain_outcome["governing_records"]
        .as_array()
        .unwrap()
        .is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

/// Un mensaje stdio sobredimensionado debe rechazarse sin abortar el proceso,
/// y la sesión debe seguir viva para la llamada siguiente.
#[test]
fn oversized_stdio_message_does_not_abort_the_process() {
    let mut client = TestClient::spawn();
    client.initialize();

    client.send_raw(format!("{}\n", "x".repeat(16 * 1024 * 1024 + 1)).as_bytes());

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

    client.send_raw(b"{not valid json!!!\n");

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
    let body = format!("{}{}\n", "[".repeat(depth), "]".repeat(depth));
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

    // "bindings" fuera de esta lista a propósito — `cmd_init` ya no lo crea
    // (nada lee ni escribe ahí; ver src/main.rs).
    for sub in ["subjects", "records", "proposals", "approvals"] {
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
            "severity": "high",
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

/// `kind: "exception"` está en el enum de `record.schema.json` desde el
/// principio (Fase D2 del plan beta), pero `pipeline::finalize` hardcodeaba
/// `kind: "constraint"` siempre y `finalize_change` no tenía parámetro
/// `kind` — un valor declarado en el schema que ningún caller podía
/// alcanzar nunca. Ahora un caller puede declararlo explícitamente, y
/// omitirlo sigue produciendo "constraint" (el comportamiento de antes).
#[test]
fn finalize_change_honors_an_explicit_kind() {
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
    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "f.txt",
            "base_revision": base_revision,
            "intent": "esta regla normalmente no aplica en este caso puntual",
            "statement": "excepcion temporal a la regla de doble verificacion",
            "record_id": "exception.temporary-double-check-waiver",
            "subject_id": "double-check-waiver-test",
            "subject_title": "Excepcion temporal",
            "severity": "medium",
            "kind": "exception",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();
    assert_eq!(outcome["proposal_written"], true);

    let proposal_path = outcome["proposal_path"].as_str().unwrap();
    let content = std::fs::read_to_string(proposal_path).unwrap();
    assert!(content.contains("kind: exception"));

    // Invalido debe rechazarse, no escribirse en silencio como constraint.
    let base_revision2 = base_revision.clone();
    std::fs::write(dir.join("g.txt"), "changed2\n").unwrap();
    let bad = client.call(
        2,
        "finalize_change",
        json!({
            "target": "g.txt",
            "base_revision": base_revision2,
            "intent": "x",
            "statement": "y",
            "record_id": "constraint.bad-kind-test",
            "subject_id": "bad-kind-test",
            "subject_title": "test",
            "severity": "medium",
            "kind": "not-a-real-kind",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(bad["result"]["isError"], true);

    std::fs::remove_dir_all(&dir).ok();
}

/// Defecto real (Fase B1 del plan beta): los bindings salían solo de
/// `mechanical.changed_files`, nunca del target declarado. Si el archivo
/// que el cambio de verdad afecta ya estaba commiteado ANTES de
/// `base_revision` — el escenario exacto aquí: se commitea el target, y
/// `base_revision` es ESE mismo commit, así que el diff nunca lo incluye,
/// aunque el cambio real (otro archivo) sí dispare la captura — el Record
/// resultante nunca podría gobernar su propio target declarado. El mismo
/// síntoma que el bug original del dogfood (`governs_target: false` para
/// siempre), con una causa distinta y fácil de confundir con ella.
#[test]
fn finalize_change_binds_the_declared_target_even_when_the_diff_does_not_cover_it() {
    let dir = make_test_project();

    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(
        dir.join("app/upload.ts"),
        "export function submit() { /* ... */ }\n",
    )
    .unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "add upload guard"]);

    let base_revision = {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };

    // Cambio real sin commitear en un archivo DISTINTO al target — dispara
    // la captura, pero no toca app/upload.ts en absoluto.
    std::fs::write(dir.join("README.md"), "proyecto de prueba actualizado\n").unwrap();

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "app/upload.ts",
            "base_revision": base_revision,
            "intent": "Documentar por que el envio se bloquea durante la carga.",
            "statement": "El envio debe bloquearse mientras el archivo se esta subiendo.",
            "record_id": "constraint.declared-target-binding-test",
            "subject_id": "upload-guard-test",
            "subject_title": "Bloqueo de envio durante carga",
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();
    assert_eq!(outcome["proposal_written"], true);

    let proposal_path = outcome["proposal_path"].as_str().unwrap();
    let content = std::fs::read_to_string(proposal_path).unwrap();
    assert!(
        content.contains("path_hint: app/upload.ts"),
        "el target declarado debe quedar atado con un binding aunque el diff no lo cubra: {content}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Defecto real encontrado en dogfood contra el binario publicado (Fase B5
/// del plan beta): `rationale init` / `install-agent` dejan `AGENTS.md`,
/// `CLAUDE.md` y `.mcp.json` como untracked; `finalize_change` los capturaba
/// igual que cualquier otro archivo sin commitear y les ataba un binding —
/// un Record sobre `app/upload.ts` terminaba "gobernando" también
/// `AGENTS.md`, visible como tal en `rationale review`. Estos archivos son
/// bookkeeping de Rationale, nunca parte del cambio que el usuario describe.
#[test]
fn finalize_change_excludes_agent_bookkeeping_files_from_bindings() {
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

    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(dir.join("app/upload.ts"), "export function submit() {}\n").unwrap();
    // Simula lo que `rationale init` / `install-agent` acaban de dejar
    // untracked, sin pasar por el instalador real.
    std::fs::write(dir.join("AGENTS.md"), "instrucciones de agente\n").unwrap();
    std::fs::write(dir.join("CLAUDE.md"), "instrucciones de agente\n").unwrap();
    std::fs::write(dir.join(".mcp.json"), r#"{"mcpServers":{}}"#).unwrap();
    std::fs::create_dir_all(dir.join(".claude/skills/rationale-health")).unwrap();
    std::fs::write(
        dir.join(".claude/skills/rationale-health/SKILL.md"),
        "skill administrado\n",
    )
    .unwrap();

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "app/upload.ts",
            "base_revision": base_revision,
            "intent": "Bloquear el envio mientras el archivo se esta subiendo.",
            "statement": "El envio debe bloquearse mientras el archivo se esta subiendo.",
            "record_id": "constraint.bookkeeping-exclusion-test",
            "subject_id": "upload-guard-bookkeeping-test",
            "subject_title": "Bloqueo de envio durante carga",
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();
    assert_eq!(outcome["proposal_written"], true);

    let proposal_path = outcome["proposal_path"].as_str().unwrap();
    let content = std::fs::read_to_string(proposal_path).unwrap();
    assert!(content.contains("path_hint: app/upload.ts"));
    assert!(
        !content.contains("AGENTS.md")
            && !content.contains("CLAUDE.md")
            && !content.contains(".mcp.json")
            && !content.contains(".claude/skills/"),
        "archivos de bookkeeping de agentes no deben aparecer como binding: {content}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// `project_root` (dónde vive `.rationale/`) y `repo_path` (dónde vive el
/// código que cambió) están cableados como parámetros independientes desde
/// el principio (`mcp/server.rs::resolve_roots`), pero nunca se ejercitaban
/// con dos repos Git REALES distintos — todos los demás tests usan el mismo
/// directorio para ambos. Este es el caso real: un repo de canon separado
/// del repo de código.
#[test]
fn finalize_change_supports_project_root_and_repo_path_in_different_repos() {
    let canon_dir = make_test_project();
    let code_dir = make_test_project();

    std::fs::create_dir_all(code_dir.join("app")).unwrap();
    std::fs::write(
        code_dir.join("app/upload.ts"),
        "export function submit() {}\n",
    )
    .unwrap();
    run_git(&code_dir, &["add", "-A"]);
    run_git(&code_dir, &["commit", "-q", "-m", "add upload guard"]);
    let base_revision = {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&code_dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };
    std::fs::write(
        code_dir.join("app/upload.ts"),
        "export function submit() { /* guard removed */ }\n",
    )
    .unwrap();

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "app/upload.ts",
            "base_revision": base_revision,
            "intent": "Documentar el guard de envio en el repo de canon separado",
            "statement": "El envio debe bloquearse mientras el archivo se esta subiendo.",
            "record_id": "constraint.multi-repo-test",
            "subject_id": "upload-guard-multi-repo-test",
            "subject_title": "Bloqueo de envio durante carga",
            "severity": "high",
            "project_root": canon_dir.to_string_lossy(),
            "repo_path": code_dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false, "{resp:?}");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();
    assert_eq!(outcome["proposal_written"], true);

    let proposal_path = outcome["proposal_path"].as_str().unwrap();
    assert!(
        proposal_path.starts_with(canon_dir.to_string_lossy().as_ref()),
        "la propuesta debe escribirse en el canon (project_root), no en repo_path: {proposal_path}"
    );
    let content = std::fs::read_to_string(proposal_path).unwrap();
    assert!(content.contains("path_hint: app/upload.ts"));
    assert!(
        !canon_dir.join("app").exists(),
        "el archivo cambiado vive en repo_path, nunca debe copiarse a project_root"
    );

    std::fs::remove_dir_all(&canon_dir).ok();
    std::fs::remove_dir_all(&code_dir).ok();
}

#[test]
fn novelty_reason_is_structured_validated_and_persisted() {
    let dir = make_test_project();
    std::fs::write(
        dir.join(".rationale/subjects/authorization.existing.yaml"),
        "id: authorization.existing\ntype: system-behavior\ntitle: Entity-scoped staff authorization\nscope: project\n",
    )
    .unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "add existing subject"]);
    let base_revision = {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };

    std::fs::create_dir_all(dir.join("src/auth")).unwrap();
    std::fs::write(dir.join("src/auth/authorization.ts"), "changed\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "authorization change"]);

    let mut client = TestClient::spawn();
    client.initialize();

    let invalid = client.call(
        1,
        "finalize_change",
        json!({
            "target": "src/auth/authorization.ts",
            "base_revision": base_revision,
            "intent": "new authorization behavior",
            "statement": "The new authorization behavior is distinct.",
            "record_id": "constraint.invalid-novelty-test",
            "subject_id": "authorization.new",
            "subject_title": "Entity-scoped staff authorization",
            "novelty_reason": {
                "contrasted_subject": "authorization.missing",
                "difference_kind": "behavior",
                "difference": "different behavior",
                "evidence": "changed source"
            },
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let invalid_text = invalid["result"]["content"][0]["text"].as_str().unwrap();
    let invalid_outcome: Value = serde_json::from_str(invalid_text).unwrap();
    assert_eq!(invalid_outcome["proposal_written"], false);
    assert!(invalid_outcome["blocked_reason"]
        .as_str()
        .unwrap()
        .contains("novelty_reason"));

    let valid = client.call(
        2,
        "finalize_change",
        json!({
            "target": "src/auth/authorization.ts",
            "base_revision": base_revision,
            "intent": "new authorization behavior",
            "statement": "The new authorization behavior is distinct.",
            "record_id": "constraint.valid-novelty-test",
            "subject_id": "authorization.new",
            "subject_title": "Entity-scoped staff authorization",
            "novelty_reason": {
                "contrasted_subject": "authorization.existing",
                "difference_kind": "behavior",
                "difference": "The new rule governs audit decisions, not access scope.",
                "evidence": "The changed binding is the authorization audit path."
            },
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let valid_text = valid["result"]["content"][0]["text"].as_str().unwrap();
    let valid_outcome: Value = serde_json::from_str(valid_text).unwrap();
    assert_eq!(valid_outcome["proposal_written"], true);
    assert_eq!(
        valid_outcome["subject_resolution"]["novelty_reason"]["contrasted_subject"],
        "authorization.existing"
    );
    let proposal =
        std::fs::read_to_string(valid_outcome["proposal_path"].as_str().unwrap()).unwrap();
    assert!(proposal.contains("novelty_reason:"));
    assert!(proposal.contains("contrasted_subject: authorization.existing"));
    assert!(proposal.contains("difference_kind: behavior"));

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
            "severity": "high",
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

/// Fase 1.3 — guarda de "nada que capturar": `base_revision == HEAD` y un
/// working tree limpio (nada commiteado, staged, unstaged ni untracked)
/// debe rechazarse con un `blocked_reason` explícito, nunca escribir una
/// propuesta con `binding_declarations: []`. Distinto de Nivel 0
/// (`Cargo.lock`-only, arriba): ahí SÍ hay cambios, solo que no ameritan un
/// Record. Aquí no hay ningún cambio en absoluto.
#[test]
fn finalize_change_rejects_when_nothing_changed_at_all() {
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

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "README.md",
            "base_revision": base_revision,
            "intent": "no-op",
            "statement": "N/A",
            "record_id": "constraint.nothing-changed-test",
            "subject_id": "should.not-exist-either",
            "subject_title": "Should not exist",
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();

    assert_eq!(outcome["proposal_written"], false);
    assert!(outcome["blocked_reason"]
        .as_str()
        .unwrap()
        .contains("no hay ningún cambio"));
    assert!(!dir
        .join(".rationale/proposals/constraint.nothing-changed-test.yaml")
        .exists());

    std::fs::remove_dir_all(&dir).ok();
}

/// Vulnerabilidad real encontrada y corregida durante la verificación de
/// fin de Fase F: un `record_id` con `../` escribía fuera de
/// `.rationale/proposals/` — confirmado empíricamente escribiendo un
/// archivo real fuera del directorio del proyecto antes del fix. Este test
/// reproduce el ataque exacto contra el binario real compilado.
#[test]
fn finalize_change_rejects_path_traversal_in_record_id() {
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

    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "malicious change"]);

    let escape_target = dir.join("../pwned-by-rationale-test.yaml");
    let _ = std::fs::remove_file(&escape_target);

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "f.txt",
            "base_revision": base_revision,
            "intent": "attempted traversal must never escape because it would be a real vulnerability",
            "statement": "test",
            "record_id": "../pwned-by-rationale-test",
            "subject_id": "x.y",
            "subject_title": "x",
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();

    assert_eq!(outcome["proposal_written"], false);
    assert!(!outcome["blocked_reason"].is_null());
    assert!(
        !escape_target.exists(),
        "el record_id malicioso NUNCA debe escribir fuera de .rationale/proposals/"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Revisión adversarial de Fase F, hallazgo 3: secuencias de escape ANSI en
/// `intent`/`statement` sobrevivían intactas hasta el terminal del revisor
/// humano en `rationale review` — pudiendo pintar un banner falso
/// "AUTO-APPROVED" o borrar/ocultar texto exactamente en el momento en que
/// el humano decide aprobar. Verifica contra el binario real que el byte
/// ESC (0x1b) nunca llega a la propuesta escrita en disco.
#[test]
fn finalize_change_strips_ansi_escape_sequences_from_free_text() {
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

    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "change touching auth"]);

    let malicious_statement =
        "Staff must never receive global super_admin.\u{1b}[2K\r\u{1b}[32mAUTO-APPROVED BY SECURITY TEAM\u{1b}[0m";
    let malicious_intent =
        "Normal intent text \u{1b}[8mhidden-instruction\u{1b}[28m end because reasons";

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "f.txt",
            "base_revision": base_revision,
            "intent": malicious_intent,
            "statement": malicious_statement,
            "record_id": "constraint.ansi-injection-test",
            "subject_id": "x.y",
            "subject_title": "x",
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);

    let proposal_path = dir.join(".rationale/proposals/constraint.ansi-injection-test.yaml");
    let content = std::fs::read_to_string(&proposal_path).unwrap();
    assert!(
        !content.contains('\u{1b}'),
        "ningún byte ESC crudo debe sobrevivir en la propuesta escrita: {content:?}"
    );
    assert!(
        content.contains("AUTO-APPROVED BY SECURITY TEAM"),
        "el texto en sí no se pierde, solo los códigos de control que lo disfrazaban"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Revisión adversarial de Fase F, hallazgo 1: un solo archivo YAML
/// corrupto en `.rationale/subjects/` apagaba el Subject Resolver COMPLETO
/// en silencio (`.unwrap_or_default()` sobre un `Err` que antes abortaba
/// toda la lectura) — un candidato de Subject que debería bloquear la
/// propuesta dejaba de detectarse, sin ningún diagnóstico. Verifica contra
/// el binario real que un archivo corrupto ya no ciega al Resolver.
#[test]
fn finalize_change_still_resolves_subjects_when_one_file_is_corrupt() {
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

    // Un Subject real existente con un título casi idéntico al propuesto —
    // debería surgir como candidato fuerte y bloquear sin novelty_reason.
    std::fs::write(
        dir.join(".rationale/subjects/authz.existing.yaml"),
        "id: authz.existing\ntype: system-behavior\ntitle: Entity scoped staff authorization access\n",
    )
    .unwrap();
    // Un archivo corrupto al lado — nunca debe apagar la lectura de los demás.
    std::fs::write(
        dir.join(".rationale/subjects/broken.yaml"),
        "id: \ntitle: \n",
    )
    .unwrap();

    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "change"]);

    let mut client = TestClient::spawn();
    client.initialize();

    let resp = client.call(
        1,
        "finalize_change",
        json!({
            "target": "f.txt",
            "base_revision": base_revision,
            "intent": "test",
            "statement": "test",
            "record_id": "constraint.resolver-blindness-test",
            "subject_id": "authz.new-duplicate-attempt",
            "subject_title": "Entity scoped staff authorization access",
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let outcome: Value = serde_json::from_str(text).unwrap();

    let candidates = outcome["subject_resolution"]["candidates"]
        .as_array()
        .unwrap();
    assert!(
        !candidates.is_empty(),
        "el Subject casi-duplicado debe seguir detectándose pese al archivo corrupto: {outcome}"
    );
    let all_diagnostics = outcome["diagnostics"].as_array().unwrap();
    assert!(
        all_diagnostics
            .iter()
            .any(|d| d.as_str().unwrap_or("").contains("broken.yaml")),
        "debe quedar un diagnóstico explícito sobre el archivo que no se pudo leer: {all_diagnostics:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Sin `governs_paths`, `finalize_change` ata un binding a TODO archivo del
/// diff. Eso empuja al agente a lo contrario de lo que el canon necesita: un
/// Record único gigante en vez de varios pequeños, cada uno gobernando lo
/// suyo. Con `governs_paths`, un mismo árbol de trabajo puede producir varias
/// propuestas acotadas.
///
/// Este test cubre los tres comportamientos en un solo árbol: el default
/// histórico, el acotado, y el rechazo de una ruta no verificable.
#[test]
fn governs_paths_scopes_bindings_without_changing_the_default() {
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

    // Dos decisiones independientes en el mismo árbol, más un archivo de
    // scratch que no pertenece a ninguna — el caso real que motivó esto.
    std::fs::create_dir_all(dir.join("src/auth")).unwrap();
    std::fs::write(
        dir.join("src/auth/authorization.ts"),
        "export function resolveEntityRole() { /* ... */ }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/billing.ts"),
        "export function chargeOnce() { /* ... */ }\n",
    )
    .unwrap();
    std::fs::write(dir.join("scratch-notes.txt"), "notas sueltas\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "two decisions plus scratch"]);

    let mut client = TestClient::spawn();
    client.initialize();

    // Cada caso necesita un Subject LÉXICAMENTE distinto, no solo un id
    // distinto: con títulos parecidos el Subject Resolver los marca
    // `MergeCandidate` y exige `novelty_reason`, bloqueando la propuesta
    // (comportamiento correcto de v0.5 §294). Este test es sobre bindings,
    // así que los Subjects se eligen para no activar esa ruta.
    let finalize = |client: &mut TestClient,
                    id: i64,
                    record: &str,
                    subject_id: &str,
                    subject_title: &str,
                    extra: Value|
     -> Value {
        let mut args = json!({
            "target": "src/auth/authorization.ts",
            "base_revision": base_revision,
            "intent": "Staff users must never receive global super_admin access.",
            "statement": "Staff users must never receive global super_admin.",
            "record_id": record,
            "subject_id": subject_id,
            "subject_title": subject_title,
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        });
        for (k, v) in extra.as_object().unwrap() {
            args[k] = v.clone();
        }
        let resp = client.call(id, "finalize_change", args);
        resp["result"].clone()
    };

    // 1. Sin el parámetro: el comportamiento histórico, todos los archivos.
    let result = finalize(
        &mut client,
        1,
        "constraint.govpaths-default",
        "authorization.staff-global-admin",
        "Staff global admin prohibition",
        json!({}),
    );
    assert_eq!(result["isError"], false);
    let outcome: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    let default_proposal =
        std::fs::read_to_string(outcome["proposal_path"].as_str().unwrap()).unwrap();
    for path in [
        "src/auth/authorization.ts",
        "src/billing.ts",
        "scratch-notes.txt",
    ] {
        assert!(
            default_proposal.contains(path),
            "sin governs_paths debe atarse todo el diff; falta {path}:\n{default_proposal}"
        );
    }

    // 2. Con el parámetro: solo lo declarado. El scratch y la otra decisión
    //    quedan fuera, aunque sigan estando en el diff.
    let result = finalize(
        &mut client,
        2,
        "constraint.govpaths-scoped",
        "billing.charge-idempotency",
        "Payment charge idempotency",
        json!({"governs_paths": ["src/auth/authorization.ts"]}),
    );
    assert_eq!(result["isError"], false);
    let outcome: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    let scoped_proposal = std::fs::read_to_string(
        outcome["proposal_path"]
            .as_str()
            .unwrap_or_else(|| panic!("no se escribió propuesta acotada: {outcome}")),
    )
    .unwrap();
    assert!(
        scoped_proposal.contains("src/auth/authorization.ts"),
        "la ruta declarada debe estar atada:\n{scoped_proposal}"
    );
    for path in ["src/billing.ts", "scratch-notes.txt"] {
        assert!(
            !scoped_proposal.contains(path),
            "{path} no fue declarado y no debe aparecer atado:\n{scoped_proposal}"
        );
    }

    // 3. Una ruta que no está en el diff no es verificable: se rechaza en vez
    //    de fabricarle un binding.
    let result = finalize(
        &mut client,
        3,
        "constraint.govpaths-missing",
        "retention.audit-log-window",
        "Audit log retention window",
        json!({"governs_paths": ["src/never-touched.ts"]}),
    );
    assert_eq!(
        result["isError"], true,
        "una ruta ausente del diff debe fallar explícitamente: {result}"
    );
    let message = result["content"][0]["text"].as_str().unwrap();
    assert!(
        message.contains("src/never-touched.ts"),
        "el error debe nombrar la ruta ofensora: {message}"
    );

    // 4. Vacío no es lo mismo que ausente: un Record que no gobierna nada
    //    casi seguro es un error del caller.
    let result = finalize(
        &mut client,
        4,
        "constraint.govpaths-empty",
        "notifications.push-delivery",
        "Push notification delivery",
        json!({"governs_paths": []}),
    );
    assert_eq!(
        result["isError"], true,
        "governs_paths vacío debe rechazarse en vez de atar todo: {result}"
    );

    std::fs::remove_dir_all(&dir).ok();
}
