//! Fase 1 — reproduce el escenario exacto de un dogfood real (sesión de
//! Codex sobre `klousfriends-memories`, 2026-07): `finalize_change` sobre
//! un archivo editado SIN commitear, aprobación humana vía `rationale
//! review`, y una sesión NUEVA de `prepare_change` con una intención que
//! contradice el Record aprobado. Antes de esta fase, ese último paso
//! respondía "No se detectaron conflictos explícitos" — el fallo que
//! motivó los nueve defectos corregidos en Fase 1.1-1.4. Este test es el
//! contrato: si alguno de los nueve regresiona, este test lo nota primero.
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .arg("serve")
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
const RECORD_ID: &str = "media-challenge-upload-state";
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

    // === Paso 3: sesión MCP A -> finalize_change. ===
    {
        let mut client = TestClient::spawn();
        client.initialize();

        let resp = client.call(
            1,
            "finalize_change",
            json!({
                "target": TARGET,
                "base_revision": base_revision,
                "intent": "Bloquear el envío del reto multimedia mientras la carga está en curso.",
                "statement": "Los retos multimedia deben bloquear el envío durante la carga.",
                "severity": "medium",
                "record_id": RECORD_ID,
                "subject_id": SUBJECT_ID,
                "subject_title": "Estado de carga de retos multimedia",
                "project_root": dir.to_string_lossy(),
                "repo_path": dir.to_string_lossy(),
            }),
        );
        assert_eq!(resp["result"]["isError"], false, "resp: {resp}");
        let outcome = tool_json(&resp);

        assert_eq!(outcome["proposal_written"], true, "outcome: {outcome}");
        assert_eq!(
            outcome["capture"]["verifiability"], "entirely-uncommitted",
            "el archivo se editó sin commitear — nunca debe leerse como si fuera verificable por un tercero"
        );

        let proposal_path = outcome["proposal_path"].as_str().unwrap();
        let content = std::fs::read_to_string(proposal_path).unwrap();
        assert!(content.contains("status: pending"));
        assert!(content.contains("approvals: []"));
        assert!(content.contains("type: file"));
        assert!(
            content.contains("provisional: true"),
            "el binding de un archivo sin commitear debe declararse provisional: {content}"
        );

        // El Subject se materializa en finalize, no en approve (Fase 1.3).
        let subject_path = dir
            .join(".rationale/subjects")
            .join(format!("{SUBJECT_ID}.yaml"));
        assert!(
            subject_path.exists(),
            "el Subject debe materializarse en finalize_change, no esperar a la aprobación"
        );
        let subject_content = std::fs::read_to_string(&subject_path).unwrap();
        assert!(subject_content.contains("unreviewed"));

        assert!(
            !dir.join(".rationale/records")
                .join(format!("{RECORD_ID}.yaml"))
                .exists(),
            "finalize_change nunca escribe en records/ — solo rationale review lo hace"
        );
    } // La sesión A muere aquí — nada debe sobrevivir en memoria.

    // === Paso 5: aprobación humana real vía `rationale review`, un
    // subproceso separado con stdin piped — no un atajo interno. ===
    {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .args(["review", "--project-root"])
            .arg(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("rationale review debe arrancar");
        {
            let stdin = child.stdin.as_mut().unwrap();
            // severity "medium" -> la palabra de confirmación es "approve"
            // (solo "critical" exige "approve-critical").
            writeln!(stdin, "approve").unwrap();
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "rationale review debe salir 0; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Aprobado"), "stdout de review: {stdout}");
    }

    let record_path = dir
        .join(".rationale/records")
        .join(format!("{RECORD_ID}.yaml"));
    assert!(
        record_path.exists(),
        "el Record debe existir en records/ tras aprobar"
    );
    let record_content = std::fs::read_to_string(&record_path).unwrap();
    assert!(record_content.contains("status: approved"));

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
    assert_eq!(assessment["state"]["authority"], "approved");
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

    std::fs::remove_dir_all(&dir).ok();
}
