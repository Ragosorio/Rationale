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
        Self::spawn_with(&[], &[])
    }

    /// Sin proveedor estructural: los tests del canon autónomo no necesitan
    /// Codebase Memory y no deben indexar directorios temporales en la
    /// instalación real del usuario.
    fn spawn_without_provider(extra_args: &[&str]) -> Self {
        Self::spawn_with(extra_args, &[("RATIONALE_PROVIDER", "none")])
    }

    fn spawn_with(extra_args: &[&str], env: &[(&str, &str)]) -> Self {
        Self::spawn_in(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            extra_args,
            env,
        )
    }

    fn spawn_in(cwd: &std::path::Path, extra_args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rationale"));
        command
            .arg("serve")
            .args(extra_args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Actividad local desactivada por defecto: el cwd de estos tests es
        // el repo de Rationale, cuyo `.rationale-local/activity/` no debe
        // llenarse de sesiones de prueba. Quien la verifica pasa
        // `RATIONALE_ACTIVITY=on`.
        command.env("RATIONALE_ACTIVITY", "off");
        // Tampoco el Codebase Memory real del usuario: los tests con
        // proyectos temporales lo llenaban de índices desechables (436 de 467
        // proyectos en el dogfood de vNext). Un test que necesita estructura
        // pasa su propio fixture.
        command.env("RATIONALE_PROVIDER", "none");
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command.spawn().expect("el binario rationale debe arrancar");
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
        self.initialize_as("test");
    }

    fn initialize_as(&mut self, client_name: &str) {
        self.send(&json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": client_name, "version": "0"}}
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

    /// Llama una herramienta y devuelve su JSON ya parseado, fallando el
    /// test si la herramienta respondió `isError`.
    fn call_ok(&mut self, id: i64, name: &str, arguments: Value) -> Value {
        let resp = self.call(id, name, arguments);
        assert_eq!(resp["result"]["isError"], false, "{name} falló: {resp}");
        serde_json::from_str(resp["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
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
            "conflicts",
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
        tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "prepare_change",
            "explain_target",
            "health",
            "finalize_change",
            "resolve_conflict"
        ]
    );
    let finalize = tools
        .iter()
        .find(|tool| tool["name"] == "finalize_change")
        .unwrap();
    assert_eq!(
        finalize["inputSchema"]["properties"]["candidates"]["items"]["required"],
        json!(["kind", "statement", "rationale", "durability", "bindings"])
    );
    let resolve = tools
        .iter()
        .find(|tool| tool["name"] == "resolve_conflict")
        .unwrap();
    assert_eq!(
        resolve["inputSchema"]["required"],
        json!(["conflict_id", "decision", "human_answer"])
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
    // Buscar por id, no por posición [0]: `critical_constraints` refleja el
    // canon VIVO de este mismo repo, así que cualquier Record nuevo con
    // mayor severidad/prioridad de orden desplaza la posición de esta
    // constraint sin que la intención del test (una constraint sin
    // aprobación nunca se sirve como aprobada) deje de cumplirse.
    let critical_constraints = packet["packet"]["critical_constraints"].as_array().unwrap();
    let target_constraint = critical_constraints
        .iter()
        .find(|c| c["id"] == "constraint.no-provider-internal-access")
        .expect("constraint.no-provider-internal-access debe estar en critical_constraints");
    let authority = target_constraint["authority"].as_str().unwrap();
    assert_eq!(
        authority, "normal",
        "una constraint que nadie fijó nunca se sirve como pinned"
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

fn head(dir: &std::path::Path) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn read_records(dir: &std::path::Path) -> Vec<(String, String)> {
    let mut records: Vec<(String, String)> = std::fs::read_dir(dir.join(".rationale/records"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
                .map(|e| {
                    (
                        e.file_name()
                            .to_string_lossy()
                            .trim_end_matches(".yaml")
                            .to_string(),
                        std::fs::read_to_string(e.path()).unwrap(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    records.sort();
    records
}

fn pending_proposal_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir.join(".rationale/proposals"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
                .count()
        })
        .unwrap_or(0)
}

fn durable_candidate(statement: &str, rationale: &str, bindings: &[&str]) -> Value {
    json!({
        "kind": "constraint",
        "statement": statement,
        "rationale": rationale,
        "durability": "durable",
        "severity": "high",
        "bindings": bindings,
    })
}

const STAFF: &str = "Staff users must never receive global super_admin.";
const STAFF_WHY: &str =
    "Access to several entities is scoped per entity; a global role would leak every tenant.";

/// vNext: el trabajo normal no deja trabajo humano. Un candidato durable se
/// vuelve Record canónico en la misma llamada — sin propuesta pendiente,
/// sin aprobación, con procedencia y autoridad explícitas.
#[test]
fn finalize_change_commits_durable_candidates_without_approval() {
    let dir = make_test_project();
    let base_revision = head(&dir);
    std::fs::create_dir_all(dir.join("src/auth")).unwrap();
    std::fs::write(
        dir.join("src/auth/authorization.ts"),
        "export function resolveEntityRole() { /* ... */ }\n",
    )
    .unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "add authorization resolver"]);

    let mut client = TestClient::spawn_without_provider(&["--client", "claude-code"]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "target": "src/auth/authorization.ts",
            "base_revision": base_revision,
            "summary": "Added the entity role resolver.",
            "candidates": [durable_candidate(STAFF, STAFF_WHY, &["src/auth/authorization.ts::resolveEntityRole"])],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );

    assert_eq!(
        outcome["summary"],
        json!({"committed": 1, "discarded": 0, "conflicts": 0})
    );
    assert!(outcome["signals"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s == "authorization"));
    assert_eq!(
        pending_proposal_files(&dir),
        0,
        "nunca una propuesta pendiente"
    );
    let records = read_records(&dir);
    assert_eq!(records.len(), 1);
    let (id, content) = &records[0];
    assert_eq!(id, "constraint.staff-users-must-never-receive-global-super");
    assert!(content.contains("authority: normal"), "{content}");
    assert!(content.contains("kind: agent_asserted"), "{content}");
    assert!(content.contains("client: claude-code"), "{content}");
    assert!(content.contains("client_source: flag"), "{content}");
    assert!(content.contains("approvals: []"), "{content}");
    assert!(
        content.contains("path_hint: src/auth/authorization.ts"),
        "{content}"
    );
    assert!(
        content.contains("provisional: false"),
        "el archivo estaba commiteado: su binding es verificable: {content}"
    );
    assert!(
        outcome["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap().contains("no confirmó el símbolo")),
        "sin proveedor, el símbolo nunca se sintetiza: {outcome}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Un diff por sí solo nunca produce memoria: sin candidatos, finalize es un
/// no-op honesto que reporta los hechos mecánicos (incluido un cambio de
/// solo lockfile, antes "Nivel 0").
#[test]
fn finalize_change_without_candidates_writes_no_memory() {
    let dir = make_test_project();
    let base_revision = head(&dir);
    std::fs::write(dir.join("Cargo.lock"), "# lockfile\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "update lockfile"]);

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "base_revision": base_revision,
            "summary": "Bumped a dependency.",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(
        outcome["summary"],
        json!({"committed": 0, "discarded": 0, "conflicts": 0})
    );
    assert_eq!(outcome["capture"]["changed_files"][0]["path"], "Cargo.lock");
    assert!(read_records(&dir).is_empty());

    // Nada cambió en absoluto: mismo no-op, sin inventar un binding.
    let clean = client.call_ok(
        2,
        "finalize_change",
        json!({
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert!(clean["capture"]["changed_files"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(read_records(&dir).is_empty());
    assert_eq!(pending_proposal_files(&dir), 0);

    std::fs::remove_dir_all(&dir).ok();
}

/// El gate descarta ruido con motivo explícito — nunca en silencio, nunca
/// escribiendo nada. Cubre las garantías del contrato anterior: kind
/// inválido, id/kind inconsistentes (el defecto que rompió CI), path
/// traversal en el id y en un binding.
#[test]
fn finalize_change_discards_noise_and_unsafe_candidates_with_reasons() {
    let dir = make_test_project();
    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();
    let escape_target = dir.join("../pwned-by-rationale-test.yaml");
    let _ = std::fs::remove_file(&escape_target);

    let mut mismatch = durable_candidate(STAFF, STAFF_WHY, &["f.txt"]);
    mismatch["id"] = json!("decision.kind-contradiction-test");
    let mut traversal = durable_candidate(STAFF, STAFF_WHY, &["f.txt"]);
    traversal["id"] = json!("../pwned-by-rationale-test");
    let mut bad_kind = durable_candidate(STAFF, STAFF_WHY, &["f.txt"]);
    bad_kind["kind"] = json!("not-a-real-kind");
    let mut transient = durable_candidate(STAFF, STAFF_WHY, &["f.txt"]);
    transient["durability"] = json!("transient");

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [
                mismatch,
                traversal,
                bad_kind,
                transient,
                durable_candidate("Updated f.txt formatting", "It looked inconsistent", &["f.txt"]),
                durable_candidate(STAFF, STAFF_WHY, &["../outside-the-project.txt"]),
                {"statement": 42},
            ],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let reasons: Vec<&str> = outcome["discarded"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["reason"].as_str().unwrap())
        .collect();
    assert_eq!(
        reasons,
        vec![
            "id_kind_mismatch",
            "invalid_id",
            "invalid_kind",
            "transient",
            "mechanical_noise",
            "no_meaningful_binding",
            "malformed_candidate"
        ]
    );
    assert!(read_records(&dir).is_empty());
    assert!(
        !escape_target.exists(),
        "un id malicioso nunca escribe fuera del canon"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Kinds explícitos y ids generados: `exception` es alcanzable y un id sin
/// declarar nace con el prefijo de su kind.
#[test]
fn finalize_change_honors_kind_and_generates_prefix_consistent_ids() {
    let dir = make_test_project();
    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();

    let mut exception = durable_candidate(
        "The double verification rule does not apply to internal test tenants.",
        "Test tenants have no real funds, and double verification blocks automated QA runs.",
        &["f.txt"],
    );
    exception["kind"] = json!("exception");
    let mut decision = durable_candidate(
        "Agent registration uses a converging per-user schema.",
        "Per-project registration pointed to binaries that only existed on the installer's machine.",
        &["f.txt"],
    );
    decision["kind"] = json!("decision");

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [exception, decision],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let ids: Vec<&str> = outcome["committed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 2, "{outcome}");
    assert!(ids[0].starts_with("exception."), "{ids:?}");
    assert!(ids[1].starts_with("decision."), "{ids:?}");
    let records = read_records(&dir);
    assert!(records.iter().any(|(_, c)| c.contains("kind: exception")));
    assert!(records.iter().any(|(_, c)| c.contains("kind: decision")));

    std::fs::remove_dir_all(&dir).ok();
}

/// Bindings explícitos: pueden anclar código que el diff no tocó (explicar
/// por qué existe algo es justo el caso de uso), y los archivos que
/// `install-agent` administra nunca aparecen en la captura mecánica.
#[test]
fn finalize_change_binds_declared_code_and_ignores_agent_bookkeeping() {
    let dir = make_test_project();
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(dir.join("app/upload.ts"), "export function submit() {}\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "add upload guard"]);
    let base_revision = head(&dir);

    std::fs::write(dir.join("README.md"), "proyecto de prueba actualizado\n").unwrap();
    std::fs::write(dir.join("AGENTS.md"), "instrucciones de agente\n").unwrap();
    std::fs::write(dir.join("CLAUDE.md"), "instrucciones de agente\n").unwrap();
    std::fs::write(dir.join(".mcp.json"), r#"{"mcpServers":{}}"#).unwrap();
    std::fs::create_dir_all(dir.join(".claude/skills/rationale-health")).unwrap();
    std::fs::write(
        dir.join(".claude/skills/rationale-health/SKILL.md"),
        "skill administrado\n",
    )
    .unwrap();

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "base_revision": base_revision,
            "candidates": [durable_candidate(
                "Sending must stay blocked while the file is still uploading.",
                "Messages sent before the upload finishes arrive with a broken attachment link.",
                &["app/upload.ts::submit"],
            )],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(outcome["summary"]["committed"], 1, "{outcome}");
    let changed: Vec<&str> = outcome["capture"]["changed_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        changed,
        vec!["README.md"],
        "el bookkeeping de agentes no es parte del cambio"
    );
    let (_, content) = &read_records(&dir)[0];
    assert!(content.contains("path_hint: app/upload.ts"), "{content}");
    for managed in [
        "AGENTS.md",
        "CLAUDE.md",
        ".mcp.json",
        ".claude/skills/",
        "README.md",
    ] {
        assert!(
            !content.contains(managed),
            "{managed} no fue declarado: {content}"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}

/// `project_root` (canon) y `repo_path` (código) en repos Git distintos: el
/// Record se escribe en el canon y sus bindings son relativos al código.
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

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [durable_candidate(
                "Sending must stay blocked while the file is still uploading.",
                "Messages sent before the upload finishes arrive with a broken attachment link.",
                &["app/upload.ts"],
            )],
            "project_root": canon_dir.to_string_lossy(),
            "repo_path": code_dir.to_string_lossy(),
        }),
    );
    let path = outcome["committed"][0]["path"].as_str().unwrap();
    assert!(
        path.starts_with(canon_dir.to_string_lossy().as_ref()),
        "el Record vive en el canon (project_root): {path}"
    );
    let content = std::fs::read_to_string(path).unwrap();
    assert!(content.contains("path_hint: app/upload.ts"));
    assert!(
        content.contains("provisional: true"),
        "untracked en el repo de código: {content}"
    );
    assert!(!canon_dir.join("app").exists());

    std::fs::remove_dir_all(&canon_dir).ok();
    std::fs::remove_dir_all(&code_dir).ok();
}

/// Subjects autónomos: un candidato fuerte se reutiliza en vez de bloquear
/// la captura, un `novelty_reason` válido crea uno nuevo, y un Subject
/// corrupto al lado nunca ciega al resolver (revisión adversarial de Fase
/// F, hallazgo 1).
#[test]
fn finalize_change_resolves_subjects_autonomously_even_with_a_corrupt_file() {
    let dir = make_test_project();
    std::fs::write(
        dir.join(".rationale/subjects/authz.existing.yaml"),
        "id: authz.existing\ntype: system-behavior\ntitle: Entity scoped staff authorization access\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".rationale/subjects/broken.yaml"),
        "id: \ntitle: \n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("src/auth")).unwrap();
    std::fs::write(dir.join("src/auth/authorization.ts"), "changed\n").unwrap();

    let mut similar = durable_candidate(STAFF, STAFF_WHY, &["src/auth/authorization.ts"]);
    similar["subject"] = json!({"id": "authz.new-duplicate-attempt", "title": "Entity scoped staff authorization access"});
    let mut novel = durable_candidate(
        "Authorization audit decisions are logged per entity.",
        "Auditors review access per tenant, so a global audit trail cannot answer their questions.",
        &["src/auth/authorization.ts"],
    );
    novel["subject"] = json!({
        "id": "authz.audit-trail",
        "title": "Entity scoped staff authorization access",
        "novelty_reason": {
            "contrasted_subject": "authz.existing",
            "difference_kind": "behavior",
            "difference": "The new rule governs audit decisions, not access scope.",
            "evidence": "The binding is the authorization audit path."
        }
    });

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [similar, novel],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let committed = outcome["committed"].as_array().unwrap();
    assert_eq!(committed.len(), 2, "{outcome}");
    assert_eq!(
        committed[0]["subject_id"], "authz.existing",
        "se reutiliza el candidato fuerte"
    );
    assert_eq!(committed[1]["subject_id"], "authz.audit-trail");
    assert!(dir
        .join(".rationale/subjects/authz.audit-trail.yaml")
        .is_file());
    assert!(!dir
        .join(".rationale/subjects/authz.new-duplicate-attempt.yaml")
        .exists());
    let warnings = outcome["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("broken.yaml")),
        "el archivo ilegible se reporta: {warnings:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Revisión adversarial de Fase F, hallazgo 3: el byte ESC nunca llega al
/// canon, aunque el texto visible se conserve.
#[test]
fn finalize_change_strips_ansi_escape_sequences_from_free_text() {
    let dir = make_test_project();
    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();
    let malicious_statement =
        "Staff must never receive global super_admin.\u{1b}[2K\r\u{1b}[32mAUTO-PINNED BY SECURITY TEAM\u{1b}[0m";
    let malicious_rationale =
        "Normal rationale text \u{1b}[8mhidden-instruction\u{1b}[28m because tenants must stay isolated";

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [durable_candidate(malicious_statement, malicious_rationale, &["f.txt"])],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(outcome["summary"]["committed"], 1, "{outcome}");
    let (_, content) = &read_records(&dir)[0];
    assert!(!content.contains('\u{1b}'), "{content:?}");
    assert!(content.contains("AUTO-PINNED BY SECURITY TEAM"));
    assert!(
        content.contains("authority: normal"),
        "el texto nunca otorga autoridad"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// El contrato pre-vNext (statement + record_id, sin candidates) se reporta
/// como descarte explícito — nunca se convierte en propuesta pendiente.
#[test]
fn legacy_finalize_contract_is_reported_not_converted_into_a_proposal() {
    let dir = make_test_project();
    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "target": "f.txt",
            "base_revision": head(&dir),
            "intent": "Staff users must never receive global super_admin access.",
            "statement": STAFF,
            "record_id": "constraint.legacy-contract-test",
            "subject_id": "legacy.subject",
            "subject_title": "Legacy",
            "severity": "high",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(
        outcome["discarded"][0]["reason"], "legacy_contract",
        "{outcome}"
    );
    assert_eq!(pending_proposal_files(&dir), 0);
    assert!(read_records(&dir).is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

/// Una decisión por Record: dos candidatos en el mismo árbol atan cada uno
/// solo su propio código (reemplaza a `governs_paths`: en vNext los bindings
/// son siempre explícitos por candidato).
#[test]
fn independent_candidates_bind_only_their_own_code() {
    let dir = make_test_project();
    std::fs::create_dir_all(dir.join("src/auth")).unwrap();
    std::fs::write(
        dir.join("src/auth/authorization.ts"),
        "export function a() {}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/billing.ts"),
        "export function chargeOnce() {}\n",
    )
    .unwrap();
    std::fs::write(dir.join("scratch-notes.txt"), "notas sueltas\n").unwrap();

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [
                durable_candidate(STAFF, STAFF_WHY, &["src/auth/authorization.ts"]),
                durable_candidate(
                    "A payment charge must be idempotent per invoice.",
                    "Card networks retry on timeouts, and without idempotency customers were charged twice.",
                    &["src/billing.ts::chargeOnce"],
                ),
            ],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let committed = outcome["committed"].as_array().unwrap();
    assert_eq!(committed.len(), 2, "{outcome}");
    assert_eq!(
        committed[0]["bindings"],
        json!(["src/auth/authorization.ts"])
    );
    assert_eq!(committed[1]["bindings"], json!(["src/billing.ts"]));
    for (_, content) in read_records(&dir) {
        assert!(!content.contains("scratch-notes.txt"), "{content}");
    }

    std::fs::remove_dir_all(&dir).ok();
}

fn write_pinned_record(dir: &std::path::Path, id: &str, statement: &str, path_hint: &str) {
    std::fs::write(
        dir.join(format!(".rationale/records/{id}.yaml")),
        format!(
            "schema_version: rationale/0.1\nid: {id}\nkind: constraint\nseverity: high\nstatement: \"{statement}\"\nrationale: \"Finance reconciles by fixed windows.\"\nauthority: pinned\napprovals: []\nbinding_declarations:\n  - id: binding.{id}.0\n    type: file\n    path_hint: {path_hint}\n"
        ),
    )
    .unwrap();
}

fn declare_authority(dir: &std::path::Path) {
    std::fs::write(
        dir.join(".rationale/config.yaml"),
        "project:\n  id: conflict-test\nauthority:\n  \"user:Rationale Test <test@rationale.local>\":\n    role: architecture-owner\n",
    )
    .unwrap();
}

/// El único punto que interrumpe al humano: reemplazar una regla fijada. El
/// agente recibe un conflicto estructurado, pregunta, y continúa con
/// `resolve_conflict` — sin abrir otra terminal.
#[test]
fn pinned_conflict_is_returned_and_resolve_conflict_continues_the_work() {
    let dir = make_test_project();
    declare_authority(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/payments.ts"),
        "export function createLink() {}\n",
    )
    .unwrap();
    write_pinned_record(
        &dir,
        "constraint.payment-links-expire-ten-minutes",
        "Payment links must expire after ten minutes.",
        "src/payments.ts",
    );

    let mut replacement = durable_candidate(
        "Payment links must expire after the tenant-configured window.",
        "Each tenant defines its own payment policy, so a global ten-minute rule was wrong.",
        &["src/payments.ts"],
    );
    replacement["supersedes"] = json!(["constraint.payment-links-expire-ten-minutes"]);

    let mut client = TestClient::spawn_without_provider(&["--client", "codex"]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [replacement],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(
        outcome["summary"],
        json!({"committed": 0, "discarded": 0, "conflicts": 1})
    );
    let conflict = &outcome["conflicts"][0];
    assert_eq!(
        conflict["pinned_record_id"],
        "constraint.payment-links-expire-ten-minutes"
    );
    assert!(conflict["question"]
        .as_str()
        .unwrap()
        .contains("¿Cuál debe gobernar?"));
    let conflict_id = conflict["conflict_id"].as_str().unwrap().to_string();
    assert_eq!(
        read_records(&dir).len(),
        1,
        "nada se escribe hasta que el humano decide"
    );

    // Sin la respuesta humana transcrita, no hay resolución.
    let without_answer = client.call(
        2,
        "resolve_conflict",
        json!({
            "conflict_id": conflict_id,
            "decision": "adopt_new",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(without_answer["result"]["isError"], true);

    let resolution = client.call_ok(
        3,
        "resolve_conflict",
        json!({
            "conflict_id": conflict_id,
            "decision": "adopt_new",
            "human_answer": "La configurable por tenant debe gobernar.",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let new_id = resolution["outcome"]["committed"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(resolution["outcome"]["committed"][0]["authority"], "pinned");
    let records = read_records(&dir);
    let old = &records
        .iter()
        .find(|(id, _)| id == "constraint.payment-links-expire-ten-minutes")
        .unwrap()
        .1;
    assert!(old.contains("status: superseded"), "{old}");
    assert!(old.contains(&format!("superseded_by: {new_id}")), "{old}");
    let new = &records.iter().find(|(id, _)| *id == new_id).unwrap().1;
    assert!(new.contains("authority: pinned"), "{new}");
    assert!(new.contains("client: codex"), "{new}");
    assert!(
        new.contains("La configurable por tenant debe gobernar."),
        "{new}"
    );

    // El mismo conflicto no puede resolverse dos veces.
    let again = client.call(
        4,
        "resolve_conflict",
        json!({
            "conflict_id": conflict_id,
            "decision": "keep_pinned",
            "human_answer": "otra vez",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(again["result"]["isError"], true);

    std::fs::remove_dir_all(&dir).ok();
}

/// Reemplazar una regla fijada exige autoridad declarada
/// (`constraint.f8-project-authority`); mantenerla no exige nada.
#[test]
fn adopting_over_a_pinned_rule_requires_declared_authority() {
    let dir = make_test_project();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/payments.ts"),
        "export function createLink() {}\n",
    )
    .unwrap();
    write_pinned_record(
        &dir,
        "constraint.payment-links-expire-ten-minutes",
        "Payment links must expire after ten minutes.",
        "src/payments.ts",
    );
    let mut replacement = durable_candidate(
        "Payment links should not expire at all.",
        "Customers complained that links died while they were still typing card details.",
        &["src/payments.ts"],
    );
    replacement["supersedes"] = json!(["constraint.payment-links-expire-ten-minutes"]);

    let mut client = TestClient::spawn_without_provider(&[]);
    client.initialize();
    let outcome = client.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [replacement],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let conflict_id = outcome["conflicts"][0]["conflict_id"]
        .as_str()
        .unwrap()
        .to_string();

    let refused = client.call(
        2,
        "resolve_conflict",
        json!({
            "conflict_id": conflict_id,
            "decision": "adopt_new",
            "human_answer": "la nueva",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(refused["result"]["isError"], true);
    assert!(refused["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("no está declarado"));

    let kept = client.call_ok(
        3,
        "resolve_conflict",
        json!({
            "conflict_id": conflict_id,
            "decision": "keep_pinned",
            "human_answer": "Mantén la regla fijada.",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(kept["decision"], "keep_pinned");
    assert_eq!(read_records(&dir).len(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

/// Identidad del cliente: el flag gana; sin flag, el nombre que el cliente
/// declara en `initialize` se registra con su fuente.
#[test]
fn client_identity_comes_from_the_flag_or_the_client_report() {
    let dir = make_test_project();
    std::fs::write(dir.join("f.txt"), "changed\n").unwrap();

    let mut reported = TestClient::spawn_without_provider(&[]);
    reported.initialize_as("claude-code");
    reported.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [durable_candidate(STAFF, STAFF_WHY, &["f.txt"])],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let (_, content) = &read_records(&dir)[0];
    assert!(content.contains("client: claude-code"), "{content}");
    assert!(
        content.contains("client_source: mcp-client-info"),
        "{content}"
    );

    let mut flagged = TestClient::spawn_without_provider(&["--client", "cursor"]);
    flagged.initialize_as("codex-mcp-client");
    let outcome = flagged.call_ok(
        1,
        "finalize_change",
        json!({
            "candidates": [durable_candidate(
                "Refund links must expire after one hour.",
                "Refunds need longer confirmation windows because banks batch them.",
                &["f.txt"],
            )],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let path = outcome["committed"][0]["path"].as_str().unwrap();
    let content = std::fs::read_to_string(path).unwrap();
    assert!(content.contains("client: cursor"), "{content}");
    assert!(content.contains("client_source: flag"), "{content}");

    std::fs::remove_dir_all(&dir).ok();
}

/// vNext — ciclo de vida de una operación por MCP con un proveedor de
/// fixtures (determinista, sin tocar el Codebase Memory del usuario):
/// `prepare_change` abre la operación y devuelve el vecindario acotado;
/// `finalize_change` la cierra capturando una relación explicada; un
/// `prepare_change` posterior sirve el porqué con su estado derivado.
#[test]
fn operation_lifecycle_links_prepare_structure_finalize_and_relationship_why() {
    let dir = make_test_project();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/payments.rs"),
        "pub fn create_link() {}\npub fn sign_link() {}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/config.rs"),
        "pub struct EntityConfig { pub payment_expiration: u64 }\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/api.rs"), "pub fn post_link() {}\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "payments"]);

    let fixture = std::env::temp_dir().join(format!("rationale-fixture-{}.json", unique_suffix()));
    let graph = json!({
        "nodes": [
            {"file_path": "src/payments.rs", "qualified_name": "src.payments.create_link",
             "start_line": 1, "end_line": 1, "source": "pub fn create_link() {}"},
            {"file_path": "src/payments.rs", "qualified_name": "src.payments.sign_link"},
            {"file_path": "src/config.rs", "qualified_name": "src.config.EntityConfig.payment_expiration",
             "label": "field"},
            {"file_path": "src/api.rs", "qualified_name": "src.api.post_link"}
        ],
        "relationships": [
            {"source": "src.api.post_link", "kind": "calls", "target": "src.payments.create_link"},
            {"source": "src.payments.create_link", "kind": "calls", "target": "src.payments.sign_link"},
            {"source": "src.payments.create_link", "kind": "uses",
             "target": "src.config.EntityConfig.payment_expiration"}
        ]
    });
    std::fs::write(&fixture, graph.to_string()).unwrap();
    let provider = format!("fixture:{}", fixture.display());
    let mut client = TestClient::spawn_in(
        &dir,
        &[],
        &[
            ("RATIONALE_PROVIDER", provider.as_str()),
            ("RATIONALE_ACTIVITY", "on"),
        ],
    );
    client.initialize();
    let project_root = dir.to_str().unwrap();

    // 1. prepare abre la operación y trae estructura + código del target.
    let prepared = client.call_ok(
        1,
        "prepare_change",
        json!({
            "target": "src/payments.rs::create_link",
            "intent": "make payment link expiration configurable per entity",
            "project_root": project_root,
        }),
    );
    let operation_id = prepared["operation_id"].as_str().unwrap().to_string();
    assert!(operation_id.starts_with("op_"), "{prepared}");
    let packet = &prepared["packet"];
    assert_eq!(packet["operation_id"], operation_id.as_str());
    assert_eq!(packet["structure"]["provider"], "fixture");
    let nodes = packet["structure"]["nodes"].as_array().unwrap();
    let role_of = |name: &str| {
        nodes
            .iter()
            .find(|n| n["name"] == name)
            .and_then(|n| n["role"].as_str())
            .map(str::to_string)
    };
    assert_eq!(
        role_of("create_link").as_deref(),
        Some("target"),
        "{packet}"
    );
    assert_eq!(role_of("post_link").as_deref(), Some("caller"));
    assert_eq!(packet["relevant_code"]["source"], "pub fn create_link() {}");
    assert!(packet["relationships"].is_null(), "todavía no hay porqué");

    let snapshot_path = dir
        .join(".rationale-local/operations")
        .join(format!("{operation_id}.json"));
    let read_snapshot = || -> Value {
        serde_json::from_str(&std::fs::read_to_string(&snapshot_path).unwrap()).unwrap()
    };
    let snapshot = read_snapshot();
    assert_eq!(snapshot["actor"]["client"], "test");
    assert_eq!(snapshot["selection"]["considered_nodes"], 4, "{snapshot}");
    assert!(snapshot["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["name"] == "create_link" && n["selected"] == true));

    // 2. finalize con el operation_id captura la relación explicada.
    let why = "Tenants define different payment policies, so a global expiration would break some of them.";
    let finalized = client.call_ok(
        2,
        "finalize_change",
        json!({
            "operation_id": operation_id,
            "summary": "Payment links read their expiration from entity configuration.",
            "project_root": project_root,
            "candidates": [{
                "kind": "decision",
                "statement": "Payment link expiration is read from each entity's configuration.",
                "rationale": why,
                "durability": "durable",
                "bindings": [],
                "relationships": [{
                    "source": "src/payments.rs::create_link",
                    "kind": "uses",
                    "target": "src/config.rs::EntityConfig.payment_expiration"
                }]
            }]
        }),
    );
    assert_eq!(finalized["summary"]["committed"], 1, "{finalized}");
    let record_id = finalized["committed"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(finalized["relationships"][0]["state"], "observed");
    assert_eq!(
        read_snapshot()["finalized"]["committed"][0],
        record_id.as_str()
    );

    // 3. Un prepare posterior sirve el porqué con su estado derivado.
    let again = client.call_ok(
        3,
        "prepare_change",
        json!({"target": "src/payments.rs::create_link", "project_root": project_root}),
    );
    assert_ne!(again["operation_id"], operation_id.as_str());
    let packet = &again["packet"];
    let explained = &packet["relationships"][0];
    assert_eq!(explained["record_id"], record_id.as_str(), "{packet}");
    assert_eq!(explained["state"], "observed");
    assert_eq!(explained["rationale"], why);
    assert!(packet["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"] == record_id.as_str()));
    assert!(packet["structure"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["kind"] == "uses" && e["record_ids"][0] == record_id.as_str()));

    // 4. La actividad cuenta la misma historia, enlazada por operation_id,
    // sin contenido: ni código ni rationale (ADR-0017).
    let session_files: Vec<std::path::PathBuf> =
        std::fs::read_dir(dir.join(".rationale-local/activity"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .collect();
    assert_eq!(session_files.len(), 1, "una sesión, un archivo");
    let raw = std::fs::read_to_string(&session_files[0]).unwrap();
    let events: Vec<Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let kinds: Vec<&str> = events.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(
        &kinds[..2],
        &["session.started", "agent.connected"],
        "{kinds:?}"
    );
    let for_operation: Vec<&str> = events
        .iter()
        .filter(|e| e["operation_id"] == operation_id.as_str())
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    let position = |kind: &str| {
        for_operation
            .iter()
            .position(|k| *k == kind)
            .unwrap_or_else(|| panic!("falta {kind}: {for_operation:?}"))
    };
    assert!(position("context.requested") < position("provider.started"));
    assert!(position("provider.started") < position("target.resolved"));
    assert!(position("target.resolved") < position("provider.finished"));
    assert!(position("provider.finished") < position("packet.compiled"));
    assert!(position("packet.compiled") < position("packet.delivered"));
    assert!(position("packet.delivered") < position("change.finalized"));
    assert!(position("change.finalized") < position("capture.candidate"));
    assert!(position("capture.candidate") < position("record.committed"));
    let seqs: Vec<u64> = events.iter().map(|e| e["seq"].as_u64().unwrap()).collect();
    assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1), "{seqs:?}");
    assert!(events
        .iter()
        .all(|e| e["schema_version"] == "rationale/activity/1"
            && e["actor"]["client"] == "test"
            && e["timestamp"].as_str().unwrap().ends_with('Z')));
    let compiled = events
        .iter()
        .find(|e| e["kind"] == "packet.compiled" && e["operation_id"] == operation_id.as_str())
        .unwrap();
    assert_eq!(compiled["payload"]["considered_nodes"], 4, "{compiled}");
    let committed = events
        .iter()
        .find(|e| e["kind"] == "record.committed")
        .unwrap();
    assert_eq!(committed["payload"]["record_id"], record_id.as_str());
    assert!(
        !raw.contains("pub fn create_link"),
        "el código nunca entra a la actividad"
    );
    assert!(
        !raw.contains("Tenants define"),
        "el rationale viaja por referencia"
    );

    std::fs::remove_file(&fixture).ok();
    std::fs::remove_dir_all(&dir).ok();
}
