use std::process::Command;

fn unique_temp_project(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rationale-cli-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn run(project: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rationale"))
        .current_dir(project)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_is_successful_and_non_mutating_for_every_command() {
    let project = unique_temp_project("help");
    let commands: &[&[&str]] = &[
        &["--help"],
        &["init", "--help"],
        &["health", "--help"],
        &["prepare", "--help"],
        &["serve", "--help"],
        &["review", "--help"],
        &["review-record", "--help"],
        &["install-agent", "--help"],
        &["uninstall-agent", "--help"],
        &["update", "--help"],
        &["doctor", "--help"],
    ];

    for args in commands {
        let output = run(&project, args);
        assert!(
            output.status.success(),
            "help debe salir con 0 para {args:?}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("Uso:"),
            "help debe imprimir uso para {args:?}"
        );
    }

    assert!(!project.join(".rationale").exists());
    assert!(!project.join("AGENTS.md").exists());
    assert!(!project.join("CLAUDE.md").exists());
    assert!(!project.join(".mcp.json").exists());
    assert!(!project.join(".rationale-local").exists());
    std::fs::remove_dir_all(project).ok();
}

#[test]
fn invalid_project_root_is_a_clean_cli_error() {
    let project = unique_temp_project("invalid-root");
    // Un directorio real que existe pero no tiene `.rationale/` — el propio
    // temp dir del sistema sirve en macOS, Linux y Windows sin asumir `/tmp`
    // literal, que no existe como tal en Windows.
    let no_rationale_dir = std::env::temp_dir();
    let no_rationale_dir = no_rationale_dir.to_str().unwrap();
    let cases: &[&[&str]] = &[
        &["health", "--project-root", no_rationale_dir],
        &["prepare", "src/main.rs", "--project-root", no_rationale_dir],
        &["review", "--project-root", no_rationale_dir],
        &[
            "review-record",
            "record-id",
            "--project-root",
            no_rationale_dir,
        ],
        &["install-agent", "--project-root", no_rationale_dir],
        &["uninstall-agent", "--project-root", no_rationale_dir],
        &["doctor", "--project-root", no_rationale_dir],
    ];
    for args in cases {
        let output = run(&project, args);
        assert!(!output.status.success(), "debe rechazar {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("no se encontró .rationale"),
            "stderr: {stderr}"
        );
        assert!(
            !stderr.contains("panicked"),
            "no debe hacer panic: {stderr}"
        );
    }
    std::fs::remove_dir_all(project).ok();
}

#[test]
fn version_is_available_without_a_project() {
    let project = unique_temp_project("version");
    let output = run(&project, &["--version"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("rationale "));
    std::fs::remove_dir_all(project).ok();
}

#[test]
fn unknown_agent_options_fail_before_touching_project_files() {
    let project = unique_temp_project("unknown-option");

    for args in [
        ["install-agent", "--unknown"],
        ["uninstall-agent", "--unknown"],
    ] {
        let output = run(&project, &args);
        assert!(!output.status.success(), "debe rechazar {args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("opción desconocida"),
            "debe explicar la opción inválida para {args:?}"
        );
    }

    let output = run(&project, &["install-agent", "--project-root"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("falta un valor"));

    let output = run(&project, &["install-agent", "--project-root", "--dry-run"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("falta un valor"));

    assert!(!project.join(".rationale").exists());
    assert!(!project.join("AGENTS.md").exists());
    assert!(!project.join("CLAUDE.md").exists());
    assert!(!project.join(".mcp.json").exists());
    assert!(!project.join(".rationale-local").exists());
    std::fs::remove_dir_all(project).ok();
}
