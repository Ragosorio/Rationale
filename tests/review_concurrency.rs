//! F8.3: dos procesos reales del SO intentando consumir la misma propuesta.
//! La prueba sincroniza ambos revisores en el prompt humano para que los dos
//! carguen la misma copia antes de competir por el rename de `.in-review/`.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};

struct ReviewProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: ChildStderr,
}

impl ReviewProcess {
    fn spawn(project_root: &PathBuf) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .args(["review", "--project-root"])
            .arg(project_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("debe arrancar rationale review");
        Self {
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            stderr: child.stderr.take().unwrap(),
            child,
        }
    }

    fn wait_for_prompt(&mut self) {
        let mut line = String::new();
        loop {
            line.clear();
            self.stdout.read_line(&mut line).unwrap();
            assert!(
                !line.is_empty(),
                "review terminó antes del prompt: {line:?}"
            );
            if line.contains("cualquier otra cosa para saltar") {
                return;
            }
        }
    }

    fn approve_and_collect(mut self) -> String {
        self.stdin.write_all(b"approve\n").unwrap();
        self.stdin.flush().unwrap();
        drop(self.stdin);
        let mut output = String::new();
        self.stdout.read_to_string(&mut output).unwrap();
        let mut error = String::new();
        self.stderr.read_to_string(&mut error).unwrap();
        let status = self.child.wait().unwrap();
        assert!(status.success());
        format!("{output}{error}")
    }
}

fn temp_project() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rationale-review-process-race-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join(".rationale/proposals")).unwrap();
    std::fs::create_dir_all(dir.join(".rationale/records")).unwrap();
    std::fs::write(
        dir.join(".rationale/config.yaml"),
        "schema_version: rationale/0.1\nproject:\n  id: race-test\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".rationale/proposals/constraint.process-race.yaml"),
        "id: constraint.process-race\nkind: constraint\nseverity: high\nstatement: process claim must be exclusive\nrationale: concurrency test\nepistemic_status: stated\napprovals: []\nbinding_declarations: []\nevidence: []\nrisks: []\nbound_revision: null\nsubject: null\nstatus: pending\n",
    )
    .unwrap();
    dir
}

#[test]
fn exactly_one_real_review_process_claims_a_proposal() {
    let dir = temp_project();
    let mut reviewer_a = ReviewProcess::spawn(&dir);
    let mut reviewer_b = ReviewProcess::spawn(&dir);

    reviewer_a.wait_for_prompt();
    reviewer_b.wait_for_prompt();

    let output_a = reviewer_a.approve_and_collect();
    let output_b = reviewer_b.approve_and_collect();
    let combined = format!("{output_a}\n{output_b}");

    assert_eq!(combined.matches("Aprobado ->").count(), 1, "{combined}");
    assert_eq!(
        combined.matches("error aprobando:").count(),
        1,
        "{combined}"
    );
    let records: Vec<_> = std::fs::read_dir(dir.join(".rationale/records"))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert_eq!(records.len(), 1);
    let record_yaml = std::fs::read_to_string(records[0].path()).unwrap();
    assert!(
        record_yaml.contains("authority: contributor"),
        "{record_yaml}"
    );
    assert!(
        !record_yaml.contains("authority: reviewer"),
        "{record_yaml}"
    );
    assert_eq!(
        std::fs::read_dir(dir.join(".rationale/proposals/.in-review"))
            .unwrap()
            .filter_map(Result::ok)
            .count(),
        0
    );

    std::fs::remove_dir_all(dir).ok();
}
