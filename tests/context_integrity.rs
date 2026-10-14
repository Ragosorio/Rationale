//! Integridad del contexto sobre el proyecto sintético `parcel`
//! (`eval/fixtures/parcel/`): lo mismo que mide `eval/run_eval.py`, como
//! aserciones que fallan ante una regresión.
//!
//! Historia del fixture: commit inicial con todo el canon; un segundo commit
//! retira `src/comunicaciones/` sin tocar sus Records; el árbol de trabajo
//! añade `splitShipment`, que el índice estructural (el grafo del fixture)
//! todavía no conoce.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/eval/fixtures/parcel");

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn project(&self) -> PathBuf {
        self.root.join("parcel")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn graph(&self) -> PathBuf {
        Path::new(FIXTURE).join("graph.json")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "owner")
        .env("GIT_AUTHOR_EMAIL", "owner@example.invalid")
        .env("GIT_COMMITTER_NAME", "owner")
        .env("GIT_COMMITTER_EMAIL", "owner@example.invalid")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git debe estar disponible");
    assert!(status.success(), "git {args:?} falló");
}

fn fixture() -> Fixture {
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "rationale-integrity-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let fixture = Fixture { root };
    let project = fixture.project();
    copy_dir(&Path::new(FIXTURE).join("project"), &project);
    std::fs::create_dir_all(fixture.home()).unwrap();
    git(&project, &["init", "-q"]);
    git(&project, &["config", "user.name", "owner"]);
    git(&project, &["config", "user.email", "owner@example.invalid"]);
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "-q", "-m", "initial"]);
    std::fs::remove_dir_all(project.join("src/comunicaciones")).unwrap();
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "-q", "-m", "retire comunicaciones"]);
    let service = project.join("src/shipping/service.ts");
    let mut source = std::fs::read_to_string(&service).unwrap();
    source.push_str(
        "\nexport function splitShipment(shipmentId: string, parts: number) {\n  return Array.from({ length: parts }, (_, i) => `${shipmentId}-${i + 1}`);\n}\n",
    );
    std::fs::write(service, source).unwrap();
    fixture
}

struct Client {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
}

impl Client {
    fn start(fixture: &Fixture) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .args(["serve", "--client", "claude-code"])
            .current_dir(fixture.project())
            .env("HOME", fixture.home())
            .env(
                "RATIONALE_PROVIDER",
                format!("fixture:{}", fixture.graph().display()),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("rationale serve debe arrancar");
        let stdin = child.stdin.take().unwrap();
        let reader = BufReader::new(child.stdout.take().unwrap());
        let mut client = Client {
            child,
            stdin,
            reader,
            next_id: 1,
        };
        client.request(
            "initialize",
            json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}),
        );
        client
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        writeln!(self.stdin, "{body}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim_end()).expect("una línea JSON por respuesta")
    }

    /// Respuesta de la herramienta: el JSON parseado y el texto completo que
    /// recibe el agente (sobre el que se mide el presupuesto).
    fn call(&mut self, tool: &str, arguments: Value) -> (Value, String) {
        let response = self.request("tools/call", json!({"name": tool, "arguments": arguments}));
        assert_eq!(response["result"]["isError"], false, "{tool}: {response}");
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        (serde_json::from_str(&text).unwrap(), text)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Ids que la respuesta presenta como autoridad vigente.
fn authority(tool: &str, response: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    let mut collect = |list: &Value, key: &str| {
        for item in list.as_array().into_iter().flatten() {
            ids.push(item[key].as_str().unwrap_or_default().to_string());
        }
    };
    if tool == "explain_target" {
        collect(&response["governing_records"], "id");
    } else {
        let packet = &response["packet"];
        collect(&packet["critical_constraints"], "id");
        collect(&packet["decisions"], "id");
        collect(&packet["relationships"], "record_id");
        collect(&packet["inherited_constraints"], "id");
    }
    ids.sort();
    ids.dedup();
    ids
}

fn history_ids(tool: &str, response: &Value) -> Vec<(String, String)> {
    let list = if tool == "explain_target" {
        &response["history"]
    } else {
        &response["packet"]["history"]
    };
    list.as_array()
        .into_iter()
        .flatten()
        .map(|item| {
            (
                item["id"].as_str().unwrap().to_string(),
                item["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn superseded_records_are_history_never_authority() {
    let fixture = fixture();
    let mut client = Client::start(&fixture);

    let (explain, _) = client.call(
        "explain_target",
        json!({"target": "src/shipping/rates.ts::quoteRate"}),
    );
    let governing = authority("explain_target", &explain);
    assert!(governing.contains(&"decision.rate-cache-ttl-60s".to_string()));
    assert!(!governing.contains(&"decision.rate-cache-ttl-5m".to_string()));
    let history = history_ids("explain_target", &explain);
    assert!(history.contains(&("decision.rate-cache-ttl-5m".into(), "superseded".into())));
    assert_eq!(
        explain["history"][0]["replaced_by"], "decision.rate-cache-ttl-60s",
        "{explain}"
    );
    assert!(explain["history"][0].get("statement").is_none());

    // Reemplazado solo por el `supersedes` del Record nuevo (fusión de ramas
    // que no actualizó el archivo viejo) y un Record revocado.
    let (retry, _) = client.call(
        "explain_target",
        json!({"target": "src/shipping/service.ts::retryLabel", "include_history": true}),
    );
    let governing = authority("explain_target", &retry);
    assert!(!governing.contains(&"decision.label-retry-fixed-3x".to_string()));
    assert!(!governing.contains(&"constraint.no-weekend-pickups".to_string()));
    let old = retry["history"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["id"] == "decision.label-retry-fixed-3x")
        .expect("the replaced decision stays as history");
    assert_eq!(old["status"], "superseded");
    assert_eq!(
        old["replaced_by"],
        "decision.label-retry-exponential-backoff"
    );
    assert!(
        old["statement"].is_string(),
        "include_history returns the text"
    );

    let (prepare, _) = client.call(
        "prepare_change",
        json!({"target": "src/secrets/rotation.ts::rotateKeys", "intent": "Rotate legacy KMS keys in place"}),
    );
    let served = authority("prepare_change", &prepare);
    assert!(served.contains(&"constraint.rotate-secrets-30d".to_string()));
    assert!(!served.contains(&"constraint.rotate-secrets-90d".to_string()));
    let current = prepare["packet"]["critical_constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "constraint.rotate-secrets-30d")
        .unwrap();
    assert_eq!(
        current["replaces"],
        json!(["constraint.rotate-secrets-90d"])
    );
}
