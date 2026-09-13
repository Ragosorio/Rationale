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
    let test_home = project.join(".test-home");
    std::fs::create_dir_all(&test_home).unwrap();
    Command::new(env!("CARGO_BIN_EXE_rationale"))
        .current_dir(project)
        .env("HOME", test_home)
        // Nunca el Codebase Memory real del usuario sobre un repo temporal.
        .env("RATIONALE_PROVIDER", "none")
        .args(args)
        .output()
        .unwrap()
}

fn run_with_path(
    project: &std::path::Path,
    args: &[&str],
    extra_path: &std::path::Path,
) -> std::process::Output {
    let test_home = project.join(".test-home");
    std::fs::create_dir_all(&test_home).unwrap();
    Command::new(env!("CARGO_BIN_EXE_rationale"))
        .current_dir(project)
        // El test debe detectar solo el `claude` falso. Heredar el PATH del
        // host podría ejecutar `codex mcp add` y mutar configuración global.
        .env("PATH", extra_path)
        .env("HOME", test_home)
        .env("RATIONALE_PROVIDER", "none")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn init_on_an_existing_project_configures_a_newly_available_claude_code() {
    let project = unique_temp_project("init-existing-configures-agent");
    let first = run(&project, &["init", "--skip-agent-config", "--no-mascot"]);
    assert!(
        first.status.success(),
        "primer init debe tener éxito: {first:?}"
    );
    assert!(!project.join(".mcp.json").exists());
    assert!(!project.join("CLAUDE.md").exists());

    let fake_bin = project.join("fake-bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    let fake_claude = fake_bin.join("claude");
    std::fs::write(&fake_claude, "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&fake_claude).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_claude, permissions).unwrap();
    }

    let second = run_with_path(&project, &["init", "--no-mascot"], &fake_bin);
    assert!(
        second.status.success(),
        "segundo init debe configurar el agente: {second:?}"
    );
    let stdout = String::from_utf8(second.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout conserva una única línea JSON");
    let response: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(response["status"], "already-initialized");

    assert!(
        !project.join(".mcp.json").exists(),
        "beta.3 no escribe el servidor instalado dentro del repositorio"
    );
    assert!(std::fs::read_to_string(project.join("CLAUDE.md"))
        .unwrap()
        .contains("rationale:begin"));
    for name in [
        "preflight",
        "explain",
        "capture",
        "conflicts",
        "health",
        "protocol",
    ] {
        let skill = project.join(format!(".claude/skills/rationale-{name}/SKILL.md"));
        assert!(skill.is_file(), "falta el skill {}", skill.display());
    }
    assert!(
        !project.join(".claude/skills/rationale-review").exists(),
        "vNext no instala la cola de aprobación como skill"
    );
    // `init` solo configura el proyecto; el registro global es de
    // `install-agent`. En el HOME aislado del test: ruta absoluta del binario
    // y `--client`, para que la sesión MCP sepa qué agente la abrió.
    let install = run_with_path(&project, &["install-agent", "--no-mascot"], &fake_bin);
    assert!(install.status.success(), "install-agent: {install:?}");
    let global: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(project.join(".test-home/.claude.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        global["mcpServers"]["rationale"]["args"],
        serde_json::json!(["serve", "--client", "claude-code"])
    );
    assert!(std::path::Path::new(
        global["mcpServers"]["rationale"]["command"]
            .as_str()
            .unwrap()
    )
    .is_absolute());
    assert!(
        !project.join(".cursor/skills").exists(),
        "los skills no deben escribirse para Cursor"
    );
    assert!(
        project
            .join(".rationale-local/installed-agent-files.json")
            .is_file(),
        "la reinstalación debe registrar archivos reversibles"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn init_on_an_existing_project_still_honors_skip_and_json_stdout() {
    let project = unique_temp_project("init-existing-skip");
    let first = run(&project, &["init", "--skip-agent-config", "--no-mascot"]);
    assert!(
        first.status.success(),
        "primer init debe tener éxito: {first:?}"
    );

    let second = run(&project, &["init", "--skip-agent-config", "--no-mascot"]);
    assert!(second.status.success());
    let stdout = String::from_utf8(second.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout debe contener solo el contrato JSON");
    let response: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(response["status"], "already-initialized");
    assert!(!project.join(".mcp.json").exists());
    assert!(!project.join("CLAUDE.md").exists());

    std::fs::remove_dir_all(project).ok();
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
        &["ui", "--help"],
        &["review", "--help"],
        &["review-record", "--help"],
        &["install-agent", "--help"],
        &["uninstall-agent", "--help"],
        &["update", "--help"],
        &["doctor", "--help"],
        &["pin", "--help"],
        &["unpin", "--help"],
        &["migrate", "--help"],
        &["conflicts", "--help"],
        &["resolve", "--help"],
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
        &["ui", "--no-open", "--project-root", no_rationale_dir],
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
        &["pin", "record-id", "--project-root", no_rationale_dir],
        &["migrate", "--project-root", no_rationale_dir],
        &["conflicts", "--project-root", no_rationale_dir],
        &[
            "resolve",
            "conflict_x",
            "keep-pinned",
            "--project-root",
            no_rationale_dir,
        ],
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

/// Defecto real: `record_id` se tomaba como "el primer arg que no empieza
/// con `--`", sin saber que `--project-root` consume un valor aparte.
/// `review-record --project-root <path> mi-record` ataba `record_id` al
/// VALOR del flag, no al id real — solo funcionaba en el orden documentado
/// por casualidad. Este test pasa el flag ANTES del id, con un
/// `--project-root` real (el proyecto mismo), y confirma que el error
/// menciona el id real, no la ruta del proyecto.
#[test]
fn review_record_parses_the_positional_id_regardless_of_flag_order() {
    let project = unique_temp_project("review-record-flag-order");
    let init = run(&project, &["init"]);
    assert!(init.status.success(), "init debe tener éxito: {init:?}");

    let project_str = project.to_string_lossy().to_string();
    let output = run(
        &project,
        &[
            "review-record",
            "--project-root",
            &project_str,
            "constraint.does-not-exist",
        ],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("constraint.does-not-exist"),
        "el error debe mencionar el record_id real, no la ruta del proyecto: {stderr}"
    );
    assert!(
        !stderr.contains("panicked"),
        "no debe hacer panic: {stderr}"
    );

    std::fs::remove_dir_all(project).ok();
}

/// Defecto real: auditar "quién aprobó y cuándo" exigía leer el YAML a
/// mano — ningún comando imprimía `approvals[]` ni `lifecycle.events[]`.
/// `review-record` ahora los muestra antes del menú de acción, de forma
/// puramente informativa: con stdin cerrado (el caso aquí, `run()` no pasa
/// ningún input) cae al camino de EOF y nunca muta el Record.
#[test]
fn review_record_prints_approvals_and_lifecycle_history_without_mutating() {
    let project = unique_temp_project("audit-view");
    // Identidad Git LOCAL explícita — sin esto, `git_reviewer_actor` cae al
    // config global del host (que en una máquina de desarrollador real no
    // está vacío), haciendo el test no determinista entre entornos.
    Command::new("git")
        .current_dir(&project)
        .args(["init", "-q"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&project)
        .args(["config", "user.name", "Test Reviewer"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&project)
        .args(["config", "user.email", "test@example.com"])
        .status()
        .unwrap();
    let init = run(&project, &["init"]);
    assert!(init.status.success(), "init debe tener éxito: {init:?}");

    std::fs::write(
        project.join(".rationale/config.yaml"),
        "authority:\n  \"user:Test Reviewer <test@example.com>\":\n    role: contributor\n",
    )
    .unwrap();
    let record_yaml = "id: constraint.audit-view-test\nkind: constraint\nseverity: high\nstatement: \"test statement\"\napprovals:\n  - actor: \"user:alice\"\n    authority: contributor\n    status: approved\n    approved_at: \"2026-01-01T00:00:00Z\"\nlifecycle:\n  events:\n    - type: disputed\n      actor: \"user:bob\"\n      authority: architecture-owner\n      reason: \"needs review\"\n      timestamp: \"2026-01-02T00:00:00Z\"\n";
    std::fs::write(
        project.join(".rationale/records/constraint.audit-view-test.yaml"),
        record_yaml,
    )
    .unwrap();

    let output = run(&project, &["review-record", "constraint.audit-view-test"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("actor=user:alice"), "stdout: {stdout}");
    assert!(
        stdout.contains("approved_at=2026-01-01T00:00:00Z"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("disputed actor=user:bob"),
        "stdout: {stdout}"
    );

    let after =
        std::fs::read_to_string(project.join(".rationale/records/constraint.audit-view-test.yaml"))
            .unwrap();
    assert_eq!(
        after, record_yaml,
        "una vista de auditoría nunca debe mutar el Record"
    );

    std::fs::remove_dir_all(project).ok();
}

/// Ambos instaladores (`rationale-uninstall.sh/.ps1`) y `uninstall-agent`
/// *imprimen* la garantía de que `.rationale/` sobrevive, pero nada la
/// probaba end-to-end. Aquí sí: canon real con un Record, `install-agent`
/// para generar el manifest, `uninstall-agent`, y verificar que el canon
/// sigue intacto mientras los archivos que Rationale escribió se revierten.
#[test]
fn uninstall_agent_preserves_the_rationale_canon() {
    let project = unique_temp_project("uninstall-preserves-canon");
    let init = run(&project, &["init"]);
    assert!(init.status.success(), "init debe tener éxito: {init:?}");

    let record_path = project.join(".rationale/records/constraint.canon-survives.yaml");
    let record_yaml = "id: constraint.canon-survives\nkind: constraint\nseverity: high\nstatement: \"el canon debe sobrevivir a uninstall-agent\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n";
    std::fs::write(&record_path, record_yaml).unwrap();

    let install = run(&project, &["install-agent"]);
    assert!(
        install.status.success(),
        "install-agent debe tener éxito: {install:?}"
    );

    let uninstall = run(&project, &["uninstall-agent"]);
    assert!(
        uninstall.status.success(),
        "uninstall-agent debe tener éxito: {uninstall:?}"
    );

    assert!(
        project.join(".rationale").is_dir(),
        ".rationale/ debe sobrevivir a uninstall-agent"
    );
    assert_eq!(
        std::fs::read_to_string(&record_path).unwrap(),
        record_yaml,
        "el Record no debe modificarse ni un byte"
    );

    std::fs::remove_dir_all(project).ok();
}

/// `doctor --check` no tenía prueba de su exit code, ni `--json` de la
/// forma real de su salida — ninguno de los dos es evidente por lectura del
/// código sin correrlo.
#[test]
fn doctor_check_exit_code_and_json_shape() {
    let project = unique_temp_project("doctor-check-json");
    let init = run(&project, &["init"]);
    assert!(init.status.success(), "init debe tener éxito: {init:?}");

    // Canon sano: --check debe salir 0 y no reportar hallazgos.
    let clean = run(&project, &["doctor", "--check"]);
    assert!(
        clean.status.success(),
        "doctor --check debe salir 0 sobre un canon sano: {clean:?}"
    );

    // Escribir un Record con severidad fuera de enum, un finding real y
    // detectable sin depender del working tree.
    std::fs::write(
        project.join(".rationale/records/constraint.dirty.yaml"),
        "id: constraint.dirty\nkind: constraint\nseverity: normal\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
    )
    .unwrap();

    let dirty_check = run(&project, &["doctor", "--check"]);
    assert_eq!(
        dirty_check.status.code(),
        Some(1),
        "doctor --check debe salir 1 cuando hay hallazgos: {dirty_check:?}"
    );

    let json_output = run(&project, &["doctor", "--json"]);
    assert!(json_output.status.success());
    let stdout = String::from_utf8_lossy(&json_output.stdout);
    let report: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("doctor --json debe emitir JSON válido: {e}\n{stdout}"));
    let findings = report["findings"]
        .as_array()
        .expect("el reporte debe tener un array 'findings'");
    assert!(findings
        .iter()
        .any(|f| { f["kind"] == "invalid-severity" && f["record_id"] == "constraint.dirty" }));

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

/// El skill de salud debe mostrar los hallazgos —es su propósito— sin que el
/// cliente lo marque como fallo, y aun así propagar un fallo operativo real.
///
/// Dos defectos reales cerrados aquí. Primero, la inyección usaba
/// `doctor --check`, que sale 1 con hallazgos (contrato de CI, correcto para
/// CI): Claude Code trata cualquier código distinto de cero como "Shell
/// command failed" y descarta el output. Segundo, el intento de traducir ese
/// código con un compuesto de shell (`$?`, `$$`) fue rechazado por Claude Code
/// con "Contains simple_expansion" — su verificador de permisos no puede
/// comprobar estáticamente un comando con expansión de variables, y hace bien.
///
/// La respuesta correcta era no inventar nada: `doctor` sin `--check` ya
/// imprime los hallazgos saliendo 0 y sigue saliendo 1 ante un fallo real.
/// Este test ejercita el snippet LITERAL que genera `skill_content()` —no una
/// copia a mano— y además exige que siga siendo un comando simple.
#[cfg(unix)]
#[test]
fn health_skill_injection_shows_findings_without_failing_but_keeps_real_errors() {
    use std::os::unix::fs::PermissionsExt;

    let project = unique_temp_project("health-skill-injection");
    let init = run(&project, &["init", "--skip-agent-config", "--no-mascot"]);
    assert!(init.status.success(), "init debe tener éxito: {init:?}");

    let fake_bin = project.join("fake-bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    let fake_claude = fake_bin.join("claude");
    std::fs::write(&fake_claude, "").unwrap();
    let mut perms = std::fs::metadata(&fake_claude).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake_claude, perms).unwrap();

    let install = run_with_path(&project, &["install-agent", "--no-mascot"], &fake_bin);
    assert!(
        install.status.success(),
        "install-agent debe tener éxito: {install:?}"
    );

    let skill_md =
        std::fs::read_to_string(project.join(".claude/skills/rationale-health/SKILL.md")).unwrap();
    assert!(
        skill_md.contains("allowed-tools: Bash(rationale doctor:*)"),
        "el skill generado debe declarar el permiso exacto:\n{skill_md}"
    );

    let injected_start = skill_md
        .find("!`")
        .expect("skill debe inyectar un comando bash")
        + 2;
    let rest = &skill_md[injected_start..];
    let injected_end = rest
        .find('`')
        .expect("la inyección debe cerrar con backtick");
    let injected = &rest[..injected_end];
    assert_eq!(
        injected, "rationale doctor",
        "la inyección debe ser el comando simple que allowed-tools autoriza"
    );
    // La regresión concreta: Claude Code rechaza con "Contains
    // simple_expansion" cualquier inyección con expansión de variables,
    // porque no puede comprobarla estáticamente contra el patrón permitido.
    for forbidden in ["$", "&&", ";", "|"] {
        assert!(
            !injected.contains(forbidden),
            "la inyección no puede contener {forbidden:?} — el verificador de \
             permisos del cliente la rechazaría: {injected}"
        );
    }

    // El "rationale" resuelto por PATH dentro de la inyección debe ser el
    // binario real compilado en este test run, no una copia distinta.
    let sh_bin = project.join("sh-bin");
    std::fs::create_dir_all(&sh_bin).unwrap();
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_rationale"), sh_bin.join("rationale")).unwrap();

    // El shim va primero para que "rationale" resuelva a nuestro binario de
    // test, conservando el PATH del sistema para `sh`.
    let injected_path = format!("{}:/bin:/usr/bin", sh_bin.display());
    let run_injected = |cwd: &std::path::Path| -> std::process::Output {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(injected)
            .current_dir(cwd)
            .env("PATH", &injected_path)
            .output()
            .unwrap()
    };

    // Escenario 1: sin hallazgos.
    let clean = run_injected(&project);
    assert!(
        clean.status.success(),
        "sin hallazgos, la inyección debe salir 0: {clean:?}"
    );
    assert!(String::from_utf8_lossy(&clean.stdout).contains("Sin hallazgos"));

    // Escenario 2: con hallazgos — mostrarlos ES el propósito del skill,
    //               así que no puede reportarse como fallo.
    std::fs::write(
        project.join(".rationale/records/constraint.dirty.yaml"),
        "id: constraint.dirty\nkind: constraint\nseverity: normal\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
    )
    .unwrap();
    let with_findings = run_injected(&project);
    assert!(
        with_findings.status.success(),
        "un hallazgo normal no debe marcarse como fallo: {with_findings:?}"
    );
    assert!(
        String::from_utf8_lossy(&with_findings.stdout).contains("hallazgo(s)"),
        "el reporte de hallazgos debe seguir visible: {with_findings:?}"
    );
    assert!(
        String::from_utf8_lossy(&with_findings.stderr).is_empty(),
        "no debe quedar contenido residual en stderr: {with_findings:?}"
    );

    // Escenario 3: fallo operativo real — ningún .rationale/ en el cwd.
    let broken = unique_temp_project("health-skill-injection-broken");
    let real_failure = run_injected(&broken);
    assert!(
        !real_failure.status.success(),
        "un fallo operativo real debe seguir propagándose: {real_failure:?}"
    );
    assert!(
        String::from_utf8_lossy(&real_failure.stderr).contains("error:"),
        "el mensaje de error real debe seguir visible: {real_failure:?}"
    );

    std::fs::remove_dir_all(project).ok();
    std::fs::remove_dir_all(broken).ok();
}

/// Fijar es un acto humano: sin terminal interactiva (el caso de un agente
/// ejecutando comandos), `pin`/`unpin`/`resolve` se niegan antes de tocar el
/// canon, aunque el actor Git esté declarado.
#[test]
fn authority_commands_refuse_without_an_interactive_terminal() {
    let project = unique_temp_project("pin-no-tty");
    assert!(
        run(&project, &["init", "--skip-agent-config", "--no-mascot"])
            .status
            .success()
    );
    let record =
        "id: constraint.pin-me\nkind: constraint\nseverity: high\nstatement: \"x must hold\"\n";
    std::fs::write(
        project.join(".rationale/records/constraint.pin-me.yaml"),
        record,
    )
    .unwrap();

    for args in [
        &["pin", "constraint.pin-me"][..],
        &["unpin", "constraint.pin-me"][..],
        &["resolve", "conflict_x", "adopt-new"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .current_dir(&project)
            .env("HOME", project.join(".test-home"))
            .args(args)
            .stdin(std::process::Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("terminal interactiva"));
    }
    assert_eq!(
        std::fs::read_to_string(project.join(".rationale/records/constraint.pin-me.yaml")).unwrap(),
        record,
        "el Record no cambió"
    );
    std::fs::remove_dir_all(project).ok();
}

/// `rationale migrate` procesa las propuestas pre-vNext sin revisión manual:
/// las válidas quedan canónicas, las ruidosas archivadas con su motivo.
#[test]
fn migrate_canonicalizes_valid_legacy_proposals_and_archives_noise() {
    let project = unique_temp_project("migrate");
    assert!(
        run(&project, &["init", "--skip-agent-config", "--no-mascot"])
            .status
            .success()
    );
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("src/pay.rs"), "fn pay() {}\n").unwrap();
    let proposals = project.join(".rationale/proposals");
    std::fs::create_dir_all(&proposals).unwrap();
    std::fs::write(
        proposals.join("constraint.legacy-valid.yaml"),
        "id: constraint.legacy-valid\nkind: constraint\nseverity: high\nstatement: \"Payments must be idempotent per invoice.\"\nrationale: \"Card networks retry on timeouts, so charges were duplicated.\"\nbinding_declarations:\n  - id: b\n    type: file\n    path_hint: src/pay.rs\nstatus: pending\n",
    )
    .unwrap();
    std::fs::write(
        proposals.join("constraint.legacy-noise.yaml"),
        "id: constraint.legacy-noise\nkind: constraint\nseverity: low\nstatement: \"Updated pay.rs formatting\"\nrationale: \"It looked inconsistent\"\nbinding_declarations:\n  - id: b\n    type: file\n    path_hint: src/pay.rs\nstatus: pending\n",
    )
    .unwrap();

    let dry = run(&project, &["migrate", "--dry-run", "--json"]);
    assert!(dry.status.success(), "{dry:?}");
    assert!(proposals.join("constraint.legacy-valid.yaml").is_file());

    let output = run(&project, &["migrate", "--json"]);
    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["canonicalized"][0]["record_id"],
        "constraint.legacy-valid"
    );
    assert_eq!(
        report["archived"][0]["proposal_id"],
        "constraint.legacy-noise"
    );
    assert_eq!(report["archived"][0]["reason"], "mechanical_noise");
    assert!(project
        .join(".rationale/records/constraint.legacy-valid.yaml")
        .is_file());
    assert!(project
        .join(".rationale/archive/proposals/constraint.legacy-noise.yaml")
        .is_file());
    assert!(!proposals.join("constraint.legacy-valid.yaml").exists());

    let doctor = run(&project, &["doctor", "--json"]);
    let doctor: serde_json::Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert!(
        !doctor["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["kind"] == "pending-legacy-proposals"),
        "tras migrar no quedan propuestas pendientes: {doctor}"
    );
    std::fs::remove_dir_all(project).ok();
}

/// Defecto real (dogfood de vNext, fase 5): con `--project-root .` el
/// pipeline calculaba la ruta del target con un `strip_prefix` literal que
/// fallaba contra una raíz relativa; el proveedor recibía un archivo vacío y
/// la búsqueda por patrón heredada devolvía cualquier nodo con ese nombre
/// (en el índice real, una sección Markdown del sitio).
#[test]
fn prepare_with_a_relative_project_root_resolves_the_exact_symbol() {
    let project = unique_temp_project("relative-root");
    std::fs::create_dir_all(project.join(".rationale/records")).unwrap();
    std::fs::create_dir_all(project.join(".test-home")).unwrap();
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("src/payments.rs"), "pub fn create_link() {}\n").unwrap();
    let fixture = project.join("graph.json");
    std::fs::write(
        &fixture,
        serde_json::json!({
            "nodes": [
                {"file_path": "docs/workflow.md", "qualified_name": "docs.workflow.create_link",
                 "label": "section"},
                {"file_path": "src/payments.rs", "qualified_name": "src.payments.create_link"}
            ],
            "relationships": []
        })
        .to_string(),
    )
    .unwrap();
    let provider = format!("fixture:{}", fixture.display());
    let prepare = |spec: &str| -> serde_json::Value {
        let output = Command::new(env!("CARGO_BIN_EXE_rationale"))
            .current_dir(&project)
            .env("HOME", project.join(".test-home"))
            .env("RATIONALE_PROVIDER", &provider)
            .args(["prepare", spec, "--project-root", ".", "--no-mascot"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };

    let symbol = prepare("src/payments.rs::create_link");
    assert_eq!(
        symbol["resolved_target"], "src.payments.create_link",
        "{symbol}"
    );
    assert_eq!(symbol["snapshot"]["provider_status"], "successful");
    assert_eq!(symbol["structure"]["nodes"][0]["role"], "target");

    let file = prepare("src/payments.rs");
    assert!(file["resolved_target"].is_null(), "{file}");
    assert_eq!(file["snapshot"]["provider_status"], "successful");
    assert!(
        file["warnings"].as_array().unwrap().is_empty(),
        "un target de archivo no produce advertencias espurias: {file}"
    );

    std::fs::remove_dir_all(&project).ok();
}
