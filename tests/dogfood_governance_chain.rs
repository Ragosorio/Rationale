//! Reproduce el escenario de un dogfood real (sesión de Codex sobre
//! `klousfriends-memories`, 2026-07) sobre el contrato vNext:
//! `finalize_change` sobre un archivo editado SIN commitear escribe el
//! Record canónico en la misma llamada (sin aprobación), y una sesión NUEVA
//! de `prepare_change` con una intención que lo contradice debe verlo como
//! regla gobernante. Antes de Fase 1 ese último paso respondía "No se
//! detectaron conflictos explícitos" — el fallo que motivó los nueve
//! defectos corregidos en Fase 1.1-1.4. Cierra con el único punto de
//! interrupción humana de vNext: reemplazar una regla fijada.
//!
//! No reutiliza `tests/mcp_server.rs` (mismo motivo documentado ahí: este
//! crate solo tiene binario, sin `[lib]`) — reimplementa el `TestClient`
//! mínimo necesario.

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
        // Sin proveedor estructural: el contrato de gobernanza no depende de
        // Codebase Memory, y el test no debe indexar directorios temporales
        // en la instalación real de quien lo corre.
        let mut child = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .args(["serve", "--client", "codex"])
            .env("RATIONALE_PROVIDER", "none")
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

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .expect("leer stdout no debe fallar");
        assert!(!line.is_empty(), "EOF inesperado del servidor MCP");
        serde_json::from_str(line.trim_end()).expect("cada línea stdout debe ser JSON válido")
    }

    fn initialize(&mut self) {
        self.send(&json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}
        }));
        self.recv();
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

fn tool_json(resp: &Value) -> Value {
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("respuesta sin content[0].text: {resp}"));
    serde_json::from_str(text).unwrap_or_else(|e| panic!("text no es JSON válido ({e}): {text}"))
}

fn run_git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} debe tener éxito");
}

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

fn make_test_project() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rationale-dogfood-chain-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    run_git(&dir, &["init", "-q"]);
    run_git(&dir, &["config", "user.email", "test@rationale.local"]);
    run_git(&dir, &["config", "user.name", "Rationale Test"]);
    for sub in ["subjects", "records", "proposals", "approvals"] {
        std::fs::create_dir_all(dir.join(".rationale").join(sub)).unwrap();
    }
    std::fs::create_dir_all(dir.join("app/_components")).unwrap();
    std::fs::write(
        dir.join("app/_components/party-experience.tsx"),
        "export function submitFile() { /* envía el reto */ }\n",
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "proyecto de prueba\n").unwrap();
    run_git(&dir, &["add", "-A"]);
    run_git(&dir, &["commit", "-q", "-m", "commit inicial"]);
    dir
}

const TARGET: &str = "app/_components/party-experience.tsx::submitFile";
const RECORD_ID: &str = "constraint.media-challenge-upload-state";
const SUBJECT_ID: &str = "media-challenge-upload";

#[test]
fn governance_chain_survives_uncommitted_work_and_a_new_session() {
    let dir = make_test_project();

    // === Paso 1-2: base_revision == HEAD, y el archivo se edita SIN
    // commitear — la condición exacta del dogfood real. ===
    let base_revision = {
        let output = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };
    std::fs::write(
        dir.join("app/_components/party-experience.tsx"),
        "export function submitFile() { /* ahora bloquea durante la carga */ }\n",
    )
    .unwrap();

    // === Paso 3: sesión MCP A -> finalize_change con un candidato durable. ===
    {
        let mut client = TestClient::spawn();
        client.initialize();

        let resp = client.call(
            1,
            "finalize_change",
            json!({
                "target": TARGET,
                "base_revision": base_revision,
                "summary": "El envío del reto multimedia ahora espera a que termine la carga.",
                "candidates": [{
                    "id": RECORD_ID,
                    "kind": "constraint",
                    "statement": "Los retos multimedia deben bloquear el envío durante la carga.",
                    "rationale": "Un reto enviado antes de terminar la carga llega sin archivo y el invitado no puede responderlo.",
                    "durability": "durable",
                    "severity": "medium",
                    "bindings": [TARGET],
                    "subject": {"id": SUBJECT_ID, "title": "Estado de carga de retos multimedia"}
                }],
                "project_root": dir.to_string_lossy(),
                "repo_path": dir.to_string_lossy(),
            }),
        );
        assert_eq!(resp["result"]["isError"], false, "resp: {resp}");
        let outcome = tool_json(&resp);

        assert_eq!(outcome["summary"]["committed"], 1, "outcome: {outcome}");
        assert_eq!(
            outcome["capture"]["verifiability"], "entirely-uncommitted",
            "el archivo se editó sin commitear — nunca debe leerse como si fuera verificable por un tercero"
        );

        let record_path = dir
            .join(".rationale/records")
            .join(format!("{RECORD_ID}.yaml"));
        let content = std::fs::read_to_string(&record_path)
            .expect("vNext: el Record es canónico en la misma llamada, sin revisión humana");
        assert!(content.contains("authority: normal"));
        assert!(content.contains("approvals: []"));
        assert!(content.contains("type: file"));
        assert!(
            content.contains("provisional: true"),
            "el binding de un archivo sin commitear debe declararse provisional: {content}"
        );

        let subject_path = dir
            .join(".rationale/subjects")
            .join(format!("{SUBJECT_ID}.yaml"));
        assert!(
            subject_path.exists(),
            "el Subject se materializa junto al Record"
        );
        assert!(
            std::fs::read_dir(dir.join(".rationale/proposals"))
                .unwrap()
                .flatten()
                .all(|e| e.path().extension().and_then(|x| x.to_str()) != Some("yaml")),
            "el trabajo normal nunca deja propuestas pendientes"
        );
    } // La sesión A muere aquí — nada debe sobrevivir en memoria.

    // === Paso 6-7: sesión MCP B, completamente NUEVA, con una intención en
    // inglés que contradice el statement en español. ===
    let mut session_b = TestClient::spawn();
    session_b.initialize();

    let prepare_resp = session_b.call(
        1,
        "prepare_change",
        json!({
            "target": TARGET,
            "intent": "allow sending the challenge immediately even if the upload is still pending",
            "mode": "intent-aware",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    assert_eq!(
        prepare_resp["result"]["isError"], false,
        "resp: {prepare_resp}"
    );
    let prepare_outcome = tool_json(&prepare_resp);
    let packet = &prepare_outcome["packet"];

    // Defecto 3 (severidad): "medium" ya no es invisible.
    let governing_constraint = packet["critical_constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == RECORD_ID)
        .unwrap_or_else(|| panic!("el Record debe aparecer en critical_constraints: {packet}"));
    assert_eq!(governing_constraint["severity"], "medium");

    // Defectos 1, 2, 7 (bindings, Subject, propagación archivo->símbolo).
    // `match_kind` puede ser "file-contains-symbol" (sin proveedor
    // estructural, solo el binding de archivo) o "structural" (con
    // Codebase Memory disponible, que confirmó el símbolo de verdad) —
    // ambos casos son correctos; el test no debe depender de si la
    // máquina que lo corre tiene el proveedor instalado.
    assert_eq!(governing_constraint["governs_target"], true);
    let match_kind = governing_constraint["match_kind"].as_str().unwrap();
    assert!(
        matches!(match_kind, "structural" | "file-contains-symbol"),
        "match_kind inesperado: {match_kind}"
    );

    // Defecto 4 (conflicto honesto: hecho verificable, no un veredicto semántico).
    let conflict = packet["intent_conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["record_id"] == RECORD_ID)
        .unwrap_or_else(|| panic!("debe haber un intent_conflict para el Record: {packet}"));
    assert_eq!(conflict["detection"], "governs-target");
    assert_eq!(conflict["governs_target"], true);
    assert_eq!(packet["governance_verdict_required"], true);

    // Assessment: autoridad aprobada. El binding de ARCHIVO (siempre
    // producido, sin depender del proveedor) debe resolver — el archivo
    // sigue en el working tree. No se fija `linkage` al valor agregado:
    // si Codebase Memory está instalado y ya indexó otro proyecto con un
    // símbolo `submitFile`, puede devolver un match cruzado de un
    // proyecto no relacionado (un directorio temporal como este nunca fue
    // indexado); ese binding de símbolo queda correctamente marcado
    // `path_hint: null` (no verificable) por `binding_match`, lo que
    // arrastra el agregado a `stale` — comportamiento honesto, no un bug,
    // pero depende de si la máquina que corre el test tiene el proveedor
    // y qué haya indexado antes.
    let assessment = &prepare_outcome["assessment"];
    assert_eq!(
        assessment["state"]["authority"], "normal",
        "capturado por un agente: normal, nunca pinned"
    );
    let linkage = assessment["state"]["linkage"].as_str().unwrap();
    assert!(
        matches!(linkage, "current" | "stale"),
        "linkage inesperado: {linkage}"
    );
    let file_binding_resolved = assessment["binding_resolution"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["resolved"] == true);
    assert!(
        file_binding_resolved,
        "el binding de archivo (siempre verificable, sin depender del proveedor) debe resolver: {assessment}"
    );

    // === Paso 8: explain_target debe coincidir con prepare_change. ===
    let explain_resp = session_b.call(
        2,
        "explain_target",
        json!({
            "target": TARGET,
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let explain_outcome = tool_json(&explain_resp);
    let governing_ids: Vec<&str> = explain_outcome["governing_records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(governing_ids, vec![RECORD_ID]);

    // === Paso 9: control negativo — un target no relacionado no hereda
    // gobernancia del Record (el fallback arbitrario de antes ya no existe). ===
    let unrelated_resp = session_b.call(
        3,
        "prepare_change",
        json!({
            "target": "README.md",
            "intent": "allow sending the challenge immediately even if the upload is still pending",
            "mode": "intent-aware",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    );
    let unrelated_outcome = tool_json(&unrelated_resp);
    let any_governs = unrelated_outcome["packet"]["critical_constraints"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["governs_target"] == true);
    assert!(
        !any_governs,
        "README.md no está gobernado por el binding hacia party-experience.tsx: {unrelated_outcome}"
    );

    // === Paso 10: la persona fija la regla (equivalente a `rationale pin`,
    // que exige terminal interactiva) y un agente intenta reemplazarla. ===
    let record_path = dir
        .join(".rationale/records")
        .join(format!("{RECORD_ID}.yaml"));
    let pinned = std::fs::read_to_string(&record_path)
        .unwrap()
        .replace("authority: normal", "authority: pinned");
    std::fs::write(&record_path, pinned).unwrap();

    let attempt = tool_json(&session_b.call(
        4,
        "finalize_change",
        json!({
            "candidates": [{
                "kind": "constraint",
                "statement": "Challenges may be sent immediately while the upload continues in the background.",
                "rationale": "Guests abandoned the flow while waiting, and the upload can finish after sending.",
                "durability": "durable",
                "bindings": [TARGET],
                "supersedes": [RECORD_ID]
            }],
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    ));
    assert_eq!(attempt["summary"]["conflicts"], 1, "{attempt}");
    let conflict_id = attempt["conflicts"][0]["conflict_id"].as_str().unwrap();

    let kept = tool_json(&session_b.call(
        5,
        "resolve_conflict",
        json!({
            "conflict_id": conflict_id,
            "decision": "keep_pinned",
            "human_answer": "No, el envío sigue bloqueado durante la carga.",
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    ));
    assert_eq!(kept["decision"], "keep_pinned");
    let after = tool_json(&session_b.call(
        6,
        "explain_target",
        json!({
            "target": TARGET,
            "project_root": dir.to_string_lossy(),
            "repo_path": dir.to_string_lossy(),
        }),
    ));
    assert_eq!(after["governing_records"][0]["id"], RECORD_ID);
    assert_eq!(after["governing_records"][0]["authority"], "pinned");
    assert_eq!(
        after["governing_records"].as_array().unwrap().len(),
        1,
        "la afirmación descartada nunca entró al canon: {after}"
    );

    std::fs::remove_dir_all(&dir).ok();
}
