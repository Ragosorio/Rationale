use std::process::Command;

fn unique_temp_project() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rationale-empty-project-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn prepare_on_new_project_returns_empty_packet_instead_of_panicking() {
    let project = unique_temp_project();
    let binary = env!("CARGO_BIN_EXE_rationale");
    let init = Command::new(binary)
        .current_dir(&project)
        .arg("init")
        .output()
        .unwrap();
    assert!(init.status.success(), "init stderr: {:?}", init.stderr);

    let prepared = Command::new(binary)
        .current_dir(&project)
        .args(["prepare", "src/main.rs", "--project-root"])
        .arg(&project)
        .arg("--repo-path")
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        prepared.status.success(),
        "prepare debe tolerar un canon vacío; stderr: {:?}",
        prepared.stderr
    );
    let stdout = String::from_utf8(prepared.stdout).unwrap();
    let packet: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(packet["critical_constraints"].as_array().unwrap().len(), 0);
    assert!(String::from_utf8(prepared.stderr)
        .unwrap()
        .contains("no hay Records"));
    std::fs::remove_dir_all(project).ok();
}
