// Rationale — language spike (Rust candidate).
//
// Implementa exactamente las 6 operaciones de
// docs/research/language/spike-protocol.md, ni más ni menos:
//   1. Leer un Record (YAML) desde disco.
//   2. Abrir SQLite (crear, insertar, leer una fila).
//   3. Llamar/mockear un proveedor estructural externo (subprocess, con deadline).
//   4. Verificar una revisión (comparar dos strings).
//   5. Rankear una constraint (ordenar una lista pequeña por campo numérico).
//   6. Emitir JSON a stdout.
//
// Además prueba, fuera del pipeline principal: servidor MCP mínimo (--serve),
// file locking, subprocess con cancelación real por deadline (--demo-timeout).

use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Record {
    id: String,
    kind: String,
    severity: String,
    statement: String,
    subject: String,
    bound_revision: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RankedConstraint {
    id: String,
    severity_weight: i64,
    statement: String,
}

#[derive(Debug, Serialize)]
struct OutputPacket {
    record_id: String,
    revision_consistency: String,
    provider: serde_json::Value,
    top_constraint: RankedConstraint,
    latency_ms: u128,
}

fn severity_weight(s: &str) -> i64 {
    match s {
        "critical" => 100,
        "high" => 75,
        "medium" => 50,
        "low" => 25,
        _ => 0,
    }
}

// --- Operación 1: leer un Record YAML desde disco ---
fn op1_read_record(path: &Path) -> Record {
    let content = fs::read_to_string(path).expect("no se pudo leer el Record YAML");
    serde_yaml::from_str(&content).expect("Record YAML inválido")
}

// --- Operación 2: SQLite — crear, insertar, leer una fila ---
fn op2_sqlite_roundtrip(db_path: &Path, record: &Record) -> String {
    let conn = rusqlite::Connection::open(db_path).expect("no se pudo abrir sqlite");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS records (id TEXT PRIMARY KEY, statement TEXT NOT NULL)",
        [],
    )
    .expect("create table");
    conn.execute(
        "INSERT OR REPLACE INTO records (id, statement) VALUES (?1, ?2)",
        rusqlite::params![record.id, record.statement],
    )
    .expect("insert");
    let statement: String = conn
        .query_row(
            "SELECT statement FROM records WHERE id = ?1",
            rusqlite::params![record.id],
            |row| row.get(0),
        )
        .expect("select");
    statement
}

// --- Operación 3: llamar/mockear el proveedor estructural (subprocess) ---
// Con deadline real: si el hijo no responde a tiempo, se mata (no solo se
// abandona el timeout del lado del cliente).
fn op3_call_provider(script: &Path, slow: bool, deadline: Duration) -> serde_json::Value {
    let mut cmd = Command::new(script);
    if slow {
        cmd.arg("slow");
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = cmd.spawn().expect("no se pudo lanzar el proveedor mock");

    let start = Instant::now();
    loop {
        match child.try_wait().expect("try_wait") {
            Some(_status) => break,
            None => {
                if start.elapsed() >= deadline {
                    // Cancelación real: matar el proceso, no solo dejar de esperar.
                    let _ = child.kill();
                    let _ = child.wait();
                    return serde_json::json!({
                        "provider": "mock",
                        "status": "timeout",
                        "coverage": "unknown"
                    });
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    let mut out = String::new();
    child
        .stdout
        .take()
        .expect("stdout")
        .read_to_string(&mut out)
        .expect("read stdout");
    serde_json::from_str(&out).unwrap_or(serde_json::json!({"status": "malformed"}))
}

// --- Operación 4: verificar una revisión (comparar dos strings) ---
fn op4_check_revision(record_revision: &str, provider_revision: &str) -> String {
    if record_revision == provider_revision {
        "exact".to_string()
    } else {
        "structural-index-behind".to_string()
    }
}

// --- Operación 5: rankear una constraint (ordenar por campo numérico) ---
fn op5_rank_constraints(record: &Record) -> RankedConstraint {
    let mut candidates = vec![
        RankedConstraint {
            id: "risk.staff-without-assignment".to_string(),
            severity_weight: severity_weight("medium"),
            statement: "Staff without entity assignments may lose access.".to_string(),
        },
        RankedConstraint {
            id: record.id.clone(),
            severity_weight: severity_weight(&record.severity),
            statement: record.statement.clone(),
        },
        RankedConstraint {
            id: "decision.staff-access-is-entity-scoped".to_string(),
            severity_weight: severity_weight("high"),
            statement: "Staff access must be assigned independently per entity.".to_string(),
        },
    ];
    candidates.sort_by_key(|c| std::cmp::Reverse(c.severity_weight));
    candidates
        .into_iter()
        .next()
        .expect("al menos un candidato")
}

// --- File locking: lock advisorio sobre un archivo de estado ---
// Demuestra la capacidad, no forma parte del pipeline de las 6 operaciones.
#[cfg(unix)]
fn demo_file_lock(path: &Path) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let file = File::create(path)?;
    let fd = file.as_raw_fd();
    let rc = unsafe {
        libc_flock(fd, 2 /* LOCK_EX */ | 4 /* LOCK_NB */)
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

// Evitar una dependencia extra (crate `libc`) solo para flock: se declara el
// símbolo directamente, ya vinculado por defecto en el runtime C de macOS/Linux.
#[cfg(unix)]
extern "C" {
    #[link_name = "flock"]
    fn libc_flock(fd: i32, operation: i32) -> i32;
}

fn run_pipeline(fixtures_dir: &Path, work_dir: &Path, slow_provider: bool) -> OutputPacket {
    let t0 = Instant::now();

    let record = op1_read_record(&fixtures_dir.join("record.yaml"));
    let _stored_statement = op2_sqlite_roundtrip(&work_dir.join("spike.sqlite3"), &record);

    let provider = op3_call_provider(
        &fixtures_dir.join("mock_provider.sh"),
        slow_provider,
        Duration::from_millis(500),
    );
    let provider_revision = provider
        .get("indexed_revision")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let consistency = op4_check_revision(&record.bound_revision, provider_revision);

    let top = op5_rank_constraints(&record);

    OutputPacket {
        record_id: record.id.clone(),
        revision_consistency: consistency,
        provider,
        top_constraint: top,
        latency_ms: t0.elapsed().as_millis(),
    }
}

// --- MCP mínimo: JSON-RPC 2.0 framed con Content-Length sobre stdio ---
// Mismo framing verificado empíricamente contra Codebase Memory
// (docs/research/codebase-memory/11-performance-observations.md).
fn read_mcp_message(stdin: &mut dyn Read) -> Option<serde_json::Value> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if stdin.read(&mut byte).ok()? == 0 {
            return None;
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8_lossy(&header);
    let length: usize = header_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))?
        .split(':')
        .nth(1)?
        .trim()
        .parse()
        .ok()?;
    let mut body = vec![0u8; length];
    stdin.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn write_mcp_message(stdout: &mut dyn Write, value: &serde_json::Value) {
    let body = serde_json::to_string(value).expect("serialize");
    write!(stdout, "Content-Length: {}\r\n\r\n{}", body.len(), body).expect("write");
    stdout.flush().expect("flush");
}

fn serve_mcp(fixtures_dir: &Path, work_dir: &Path) {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();

    while let Some(msg) = read_mcp_message(&mut stdin_lock) {
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();
        match method {
            "initialize" => {
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "rationale-spike-rust", "version": "0.0.0"}
                    }
                });
                write_mcp_message(&mut stdout_lock, &resp);
            }
            "tools/call" => {
                let packet = run_pipeline(fixtures_dir, work_dir, false);
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{"type": "text", "text": serde_json::to_string(&packet).unwrap()}],
                        "isError": false
                    }
                });
                write_mcp_message(&mut stdout_lock, &resp);
            }
            "notifications/initialized" => { /* sin respuesta, es una notificación */ }
            _ => {
                if id.is_some() {
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": {"code": -32601, "message": "method not found"}
                    });
                    write_mcp_message(&mut stdout_lock, &resp);
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = base.join("fixtures");
    let work_dir = base.join("target").join("spike-work");
    fs::create_dir_all(&work_dir).expect("crear work dir");

    if args.iter().any(|a| a == "--serve") {
        serve_mcp(&fixtures_dir, &work_dir);
        return;
    }

    if args.iter().any(|a| a == "--demo-timeout") {
        // Demuestra cancelación real: el proveedor mock duerme 5s, el
        // deadline es 500ms. Se espera provider.status == "timeout" y que el
        // proceso hijo haya sido matado, no abandonado en segundo plano.
        let t0 = Instant::now();
        let provider = op3_call_provider(
            &fixtures_dir.join("mock_provider.sh"),
            true,
            Duration::from_millis(500),
        );
        println!(
            "{}",
            serde_json::json!({"elapsed_ms": t0.elapsed().as_millis(), "provider": provider})
        );
        return;
    }

    if args.iter().any(|a| a == "--demo-lock") {
        #[cfg(unix)]
        {
            let lock_path = work_dir.join("spike.lock");
            match demo_file_lock(&lock_path) {
                Ok(()) => println!("{}", serde_json::json!({"lock": "acquired"})),
                Err(e) => println!(
                    "{}",
                    serde_json::json!({"lock": "failed", "error": e.to_string()})
                ),
            }
        }
        return;
    }

    let packet = run_pipeline(&fixtures_dir, &work_dir, false);
    println!("{}", serde_json::to_string(&packet).expect("serialize"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op4_revision_exact() {
        assert_eq!(op4_check_revision("abc123", "abc123"), "exact");
    }

    #[test]
    fn test_op4_revision_behind() {
        assert_eq!(
            op4_check_revision("abc123", "def456"),
            "structural-index-behind"
        );
    }

    #[test]
    fn test_op5_ranks_critical_highest() {
        let record = Record {
            id: "constraint.test".into(),
            kind: "constraint".into(),
            severity: "critical".into(),
            statement: "test statement".into(),
            subject: "test.subject".into(),
            bound_revision: "abc".into(),
        };
        let top = op5_rank_constraints(&record);
        assert_eq!(top.id, "constraint.test");
        assert_eq!(top.severity_weight, 100);
    }

    // Property-style test (sin dependencia extra): variamos severidad sobre
    // un rango de valores y confirmamos invariante de orden monótono.
    #[test]
    fn test_severity_weight_monotonic_property() {
        let ordered = ["low", "medium", "high", "critical"];
        let weights: Vec<i64> = ordered.iter().map(|s| severity_weight(s)).collect();
        for w in weights.windows(2) {
            assert!(
                w[0] < w[1],
                "severity_weight debe ser estrictamente creciente"
            );
        }
    }

    #[test]
    fn test_op1_reads_fixture_record() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"));
        let record = op1_read_record(&base.join("fixtures").join("record.yaml"));
        assert_eq!(record.id, "constraint.no-global-admin-for-staff");
        assert_eq!(record.severity, "critical");
    }

    #[test]
    fn test_op2_sqlite_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rationale-spike-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let record = Record {
            id: "constraint.sqlite-test".into(),
            kind: "constraint".into(),
            severity: "high".into(),
            statement: "roundtrip statement".into(),
            subject: "test.subject".into(),
            bound_revision: "abc".into(),
        };
        let statement = op2_sqlite_roundtrip(&dir.join("test.sqlite3"), &record);
        assert_eq!(statement, "roundtrip statement");
        fs::remove_dir_all(&dir).ok();
    }
}
