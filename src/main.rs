//! Rationale — vertical slice (Fase D).
//!
//! Arquitectura_Conceptual_v0.1.md §23, Fase D: exactamente
//!   init -> leer Record canónico -> resolver target -> consultar
//!   Codebase Memory -> verificar revisión -> devolver una constraint
//!   compacta.
//! No más que esto — el resto (FTS, budget multi-constraint, capture,
//! assessments persistidos) pertenece a Fase E/F.

mod agents;
mod assessment;
mod binding_match;
mod cache;
mod capture;
mod configuration;
mod doctor;
mod evaluation;
mod mascot;
mod mcp;
mod pipeline;
mod project;
mod prompts;
mod providers;
mod retrieval;
mod review;
mod revision;
mod signals;
mod storage;
mod subjects;

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");

    if command.is_empty() || command == "-h" || command == "--help" {
        print_usage();
        return;
    }
    if command == "-V" || command == "--version" {
        println!("rationale {}", env!("RATIONALE_BUILD_VERSION"));
        return;
    }

    let command_args = &args[2..];
    if wants_help(command_args)
        && matches!(
            command,
            "init"
                | "health"
                | "prepare"
                | "serve"
                | "review"
                | "review-record"
                | "install-agent"
                | "uninstall-agent"
                | "update"
                | "doctor"
        )
    {
        print_command_help(command);
        return;
    }

    if let Err(error) = validate_command_args(command, command_args) {
        eprintln!("{error}");
        std::process::exit(2);
    }

    match command {
        "init" => cmd_init(command_args),
        "health" => cmd_health(command_args),
        "prepare" => cmd_prepare(command_args),
        "serve" => mcp::server::run(),
        "review" => cmd_review(command_args),
        "review-record" => cmd_review_record(command_args),
        "install-agent" => cmd_install_agent(command_args),
        "uninstall-agent" => cmd_uninstall_agent(command_args),
        "update" => cmd_update(command_args),
        "doctor" => cmd_doctor(command_args),
        _ => {
            eprintln!("comando desconocido: {command}");
            print_usage();
            std::process::exit(2);
        }
    }
}

fn print_usage() {
    println!(
        "Uso: rationale <init|health|prepare|serve|review|review-record|install-agent|uninstall-agent|update|doctor> [opciones]"
    );
    println!("  rationale --help");
    println!("  rationale --version");
    println!("  rationale init [--skip-agent-config] [--no-mascot]");
    println!("  rationale health [--project-root <path>] [--no-mascot]");
    println!(
        "  rationale prepare <target-spec> [--project-root <path>] [--repo-path <path>] [--intent \"texto\"] [--no-mascot]"
    );
    println!(
        "  rationale serve   # servidor MCP (prepare_change, explain_target, health, finalize_change)"
    );
    println!(
        "  rationale review [--project-root <path>] [--no-mascot]   # confirma propuestas pendientes, una a la vez"
    );
    println!(
        "  rationale review-record <record-id> [--project-root <path>] [--no-mascot]   # lifecycle humano de un Record aprobado"
    );
    println!(
        "  rationale install-agent [--project-root <path>] [--dry-run] [--global-only] [--no-mascot]   # registra el MCP y las instrucciones de invocación en los agentes detectados"
    );
    println!(
        "  rationale uninstall-agent [--project-root <path>] [--global-only] [--no-mascot]   # revierte exactamente lo que install-agent escribió"
    );
    println!("  rationale update   # descarga e instala la última Release disponible");
    println!(
        "  rationale doctor [--project-root <path>] [--check] [--repair] [--json]   # detecta (y opcionalmente repara) defectos de integridad en el canon"
    );
}

fn print_command_help(command: &str) {
    match command {
        "init" => println!(
            "Uso: rationale init [--skip-agent-config] [--no-mascot]\n\nCrea el canon .rationale/ del proyecto."
        ),
        "health" => println!(
            "Uso: rationale health [--project-root <path>] [--no-mascot]\n\nReporta el estado del proyecto y del proveedor."
        ),
        "prepare" => println!(
            "Uso: rationale prepare <target-spec> [--project-root <path>] [--repo-path <path>] [--intent \"texto\"] [--no-mascot]\n\nCompila un ContextPacket antes de un cambio."
        ),
        "serve" => println!(
            "Uso: rationale serve\n\nInicia el servidor MCP persistente por stdin/stdout. El proceso permanece abierto esperando mensajes del agente."
        ),
        "review" => println!(
            "Uso: rationale review [--project-root <path>] [--no-mascot]\n\nRevisa propuestas pendientes con confirmación humana."
        ),
        "review-record" => println!(
            "Uso: rationale review-record <record-id> [--project-root <path>] [--no-mascot]\n\nEjecuta lifecycle sobre un Record aprobado."
        ),
        "install-agent" => println!(
            "Uso: rationale install-agent [--project-root <path>] [--dry-run] [--global-only] [--no-mascot]\n\nRegistra MCP e instrucciones de invocación de forma idempotente."
        ),
        "uninstall-agent" => println!(
            "Uso: rationale uninstall-agent [--project-root <path>] [--global-only] [--no-mascot]\n\nRevierte exactamente lo que install-agent escribió."
        ),
        "update" => println!(
            "Uso: rationale update\n\nDescarga e instala la última Release mediante el helper instalado junto al binario."
        ),
        "doctor" => println!(
            "Uso: rationale doctor [--project-root <path>] [--check] [--repair] [--json]\n\nDetecta severidades inválidas, Records sin bindings, path_hint rotos, Subjects colgantes y Records sin aprobación. Solo lectura por defecto; --check sale con código 1 si hay hallazgos; --repair pide confirmación por hallazgo."
        ),
        _ => unreachable!("solo se solicita ayuda para comandos conocidos"),
    }
}

fn wants_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "-h" || arg == "--help")
}

fn validate_command_args(command: &str, args: &[String]) -> Result<(), String> {
    match command {
        "init" => validate_flags(command, args, &["--skip-agent-config", "--no-mascot"], &[]),
        "health" => validate_flags(command, args, &["--no-mascot"], &["--project-root"]),
        "prepare" => validate_flags(
            command,
            args,
            &["--no-mascot"],
            &["--project-root", "--repo-path", "--intent"],
        ),
        "serve" => validate_flags(command, args, &[], &[]),
        "review" => validate_flags(command, args, &["--no-mascot"], &["--project-root"]),
        "review-record" => validate_flags(command, args, &["--no-mascot"], &["--project-root"]),
        "install-agent" => validate_flags(
            command,
            args,
            &[
                "--dry-run",
                "--global-only",
                "--no-mascot",
                "--refresh-skills",
            ],
            &["--project-root"],
        ),
        "uninstall-agent" => validate_flags(
            command,
            args,
            &["--global-only", "--no-mascot"],
            &["--project-root"],
        ),
        "update" => validate_flags(command, args, &[], &[]),
        "doctor" => validate_flags(
            command,
            args,
            &["--check", "--repair", "--json"],
            &["--project-root"],
        ),
        _ => Ok(()),
    }
}

fn validate_flags(
    command: &str,
    args: &[String],
    bare_flags: &[&str],
    value_flags: &[&str],
) -> Result<(), String> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if !arg.starts_with('-') {
            index += 1;
            continue;
        }
        if bare_flags.contains(&arg.as_str()) {
            index += 1;
            continue;
        }
        if value_flags.contains(&arg.as_str()) {
            if index + 1 >= args.len() || args[index + 1].starts_with('-') {
                return Err(format!("{command}: falta un valor para la opción {arg}"));
            }
            index += 2;
            continue;
        }
        return Err(format!("{command}: opción desconocida {arg}"));
    }
    Ok(())
}

fn parse_flag(args: &[String], flag: &str) -> Option<PathBuf> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
}

fn parse_string_flag(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// El primer argumento posicional, saltando correctamente cualquier flag de
/// `value_flags` junto con el valor que le sigue. Defecto real: tomar "el
/// primer arg que no empieza con `--`" (sin saber cuáles llevan valor) hacía
/// que `review-record --project-root /repo mi-record` atara `record_id` al
/// VALOR del flag (`/repo`), no al id real — solo funcionaba en el orden
/// documentado por casualidad.
fn first_positional_arg<'a>(args: &'a [String], value_flags: &[&str]) -> Option<&'a String> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if value_flags.contains(&arg.as_str()) {
            index += 2;
            continue;
        }
        if !arg.starts_with('-') {
            return Some(arg);
        }
        index += 1;
    }
    None
}

fn resolve_project_root(args: &[String]) -> Result<PathBuf, String> {
    let start = match parse_flag(args, "--project-root") {
        Some(path) => path,
        None => std::env::current_dir().map_err(|e| format!("no se pudo determinar cwd: {e}"))?,
    };
    configuration::find_project_root(&start).ok_or_else(|| {
        format!(
            "no se encontró .rationale/ desde {}; usa --project-root con la raíz de un proyecto inicializado",
            start.display()
        )
    })
}

fn fail<T>(error: impl std::fmt::Display) -> T {
    eprintln!("error: {error}");
    std::process::exit(1);
}

fn cmd_init(args: &[String]) {
    let cwd =
        std::env::current_dir().unwrap_or_else(|e| fail(format!("no se pudo determinar cwd: {e}")));
    let rationale_dir = cwd.join(".rationale");
    if rationale_dir.exists() {
        print_status_json("already-initialized", &rationale_dir);
    } else {
        // `bindings/` deliberadamente NO está en esta lista: ningún código lee
        // o escribe ahí — los bindings viven embebidos en
        // `Record.binding_declarations` (ver `.rationale/bindings/README.md`
        // en proyectos que ya lo tenían). Mantenerlo era una falsa afordancia
        // que costó tiempo real de diagnóstico durante un dogfood.
        for sub in ["subjects", "records", "approvals", "schemas", "migrations"] {
            std::fs::create_dir_all(rationale_dir.join(sub))
                .unwrap_or_else(|e| fail(format!("no se pudo crear .rationale/{sub}: {e}")));
        }
        print_status_json("initialized", &rationale_dir);
    }

    // Todo lo que sigue es decoración y conveniencia para humanos — va a
    // stderr para no tocar el contrato de stdout de arriba.
    mascot::print_stderr(args, mascot::Mood::Happy);
    if mascot::stderr_enabled(args) {
        eprintln!("  Soy Chestie — cuido las vallas de este proyecto: por qué existe el código, no solo qué hace.");
    }

    let skip_agent_config = args.iter().any(|a| a == "--skip-agent-config")
        || std::env::var("RATIONALE_SKIP_AGENT_CONFIG").as_deref() == Ok("1");
    if skip_agent_config {
        return;
    }

    if mascot::stderr_enabled(args) {
        eprintln!();
    }
    mascot::print_stderr(args, mascot::Mood::Searching);
    if mascot::stderr_enabled(args) {
        eprintln!("  Buscando agentes de código en este proyecto para avisarles de mí...");
    }
    let rationale_local = find_rationale_local(&cwd);
    let binary_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rationale"));
    match agents::install(&cwd, &rationale_local, &binary_path, false, false) {
        Ok(report) if report.detected.is_empty() => {
            if mascot::stderr_enabled(args) {
                eprintln!(
                    "  No encontré ningún agente conocido todavía — corre 'rationale install-agent' cuando tengas uno."
                );
            }
        }
        Ok(report) => {
            mascot::print_stderr(args, mascot::Mood::Happy);
            if mascot::stderr_enabled(args) {
                eprintln!("  Avisé a: {}", report.detected.join(", "));
                for line in &report.actions {
                    eprintln!("  - {line}");
                }
            }
        }
        Err(error) => {
            if mascot::stderr_enabled(args) {
                eprintln!("  advertencia: no pude avisar a los agentes automáticamente: {error}");
            }
        }
    }
}

fn cmd_health(args: &[String]) {
    let project_root = resolve_project_root(args).unwrap_or_else(fail);

    let mut provider = providers::ProviderHandle::spawn();
    let outcome = pipeline::health(&project_root, &mut provider).unwrap_or_else(fail);

    match outcome.provider_status {
        providers::ProviderStatus::Successful => {
            mascot::print_stderr(args, mascot::Mood::Happy);
        }
        providers::ProviderStatus::Degraded | providers::ProviderStatus::Unavailable => {
            let message = outcome
                .provider_error
                .as_deref()
                .unwrap_or("El proveedor necesita atención; el canon local sigue disponible.");
            mascot::print_guarding_stderr(args, message);
        }
    }

    let mut payload = serde_json::json!({
        "project_id": outcome.project_id,
        "project_root": outcome.project_root.display().to_string(),
        "git_revision": outcome.git_revision,
        "working_tree_dirty": outcome.working_tree_dirty,
    });
    let object = payload.as_object_mut().expect("payload es un objeto");
    match &outcome.provider_error {
        Some(e) => {
            object.insert("provider_status".into(), "unreachable".into());
            object.insert("provider_error".into(), e.as_str().into());
        }
        None => {
            let status = match outcome.provider_status {
                providers::ProviderStatus::Successful => "successful",
                providers::ProviderStatus::Degraded => "degraded",
                providers::ProviderStatus::Unavailable => "unavailable",
            };
            object.insert("provider_status".into(), status.into());
            object.insert(
                "provider_coverage".into(),
                format!("{:?}", outcome.provider_coverage).into(),
            );
        }
    }
    println!("{payload}");
}

/// Contrato JSON de una línea de `init`.
///
/// Se serializa con `serde_json`, nunca interpolando la ruta en un literal:
/// en Windows `path` contiene `\`, que no es un escape JSON válido, y el
/// contrato salía corrupto — dos tests de `tests/cli.rs` fallaban solo en esa
/// plataforma con `Error("invalid escape")`. Escapar a mano habría sido
/// reimplementar mal lo que `serde_json` ya hace.
fn print_status_json(status: &str, path: &Path) {
    println!("{}", status_json(status, path));
}

fn status_json(status: &str, path: &Path) -> String {
    serde_json::json!({ "status": status, "path": path.display().to_string() }).to_string()
}

fn cmd_prepare(args: &[String]) {
    let target_spec = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| fail("uso: rationale prepare <target-spec>"));

    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let repo_path = parse_flag(args, "--repo-path").unwrap_or_else(|| project_root.clone());
    let intent = parse_string_flag(args, "--intent");

    let mut provider = providers::ProviderHandle::spawn();
    let outcome = pipeline::prepare(
        &pipeline::PrepareRequest {
            target_spec,
            intent,
            project_root: project_root.clone(),
            repo_path,
            budget: retrieval::Budget::default(),
        },
        &mut provider,
    )
    .unwrap_or_else(fail);

    for line in &outcome.diagnostics {
        eprintln!("{line}");
    }

    match outcome.packet.critical_constraints.first() {
        Some(constraint) => {
            mascot::print_guarding_stderr(args, &constraint.statement);
        }
        None => {
            mascot::print_stderr(args, mascot::Mood::Happy);
        }
    }

    let packet_json = serde_json::to_string(&outcome.packet).expect("serialize packet");
    println!("{packet_json}");

    // Instrumentación desde el día uno (Arquitectura §20).
    let rationale_local = find_rationale_local(&project_root);
    let _ = evaluation::record_run(
        &rationale_local,
        &evaluation::RunLog {
            event: "prepare".to_string(),
            timestamp: evaluation::now_iso8601(),
            latency_ms: outcome.latency_ms,
            git_revision: outcome.packet.snapshot.git_revision.clone(),
            consistency: outcome.packet.snapshot.consistency.clone(),
            provider_status: outcome.packet.snapshot.provider_status.clone(),
            provider_coverage: outcome.packet.snapshot.provider_coverage.clone(),
            packet_bytes: packet_json.len(),
        },
    );
}

fn find_rationale_local(project_root: &Path) -> PathBuf {
    // Para el fixture, .rationale-local/ vive junto al repo real de
    // Rationale, no dentro del fixture — evita ensuciar fixtures/ con
    // estado local. Se busca .rationale-local/ subiendo igual que .rationale/.
    let mut current = project_root.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return current.join(".rationale-local");
        }
        if !current.pop() {
            return project_root.join(".rationale-local");
        }
    }
}

/// `rationale review` (Fase F6) — confirmación humana de propuestas, una a
/// la vez. Nunca corre dentro de `rationale serve`: un servidor MCP no
/// puede hacer preguntas interactivas; esta es la única vía que promueve
/// una propuesta a `.rationale/records/` con una `Approval` real.
fn cmd_review(args: &[String]) {
    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let config = configuration::load(&project_root).unwrap_or_else(fail);

    let pending_result = review::list_pending_detailed(&config.rationale_dir);
    for (path, error) in &pending_result.skipped {
        eprintln!(
            "advertencia: propuesta no legible, requiere recuperación manual: {} ({error})",
            path.display()
        );
    }
    mascot::print_stdout(args, mascot::Mood::Base);
    let pending = pending_result.pending;
    if pending.is_empty() {
        println!("No hay propuestas pendientes en .rationale/proposals/.");
        return;
    }

    let reviewer_actor = git_reviewer_actor(&project_root);
    let reviewer_authority = config.authority_for_actor(&reviewer_actor);
    let rationale_local = find_rationale_local(&project_root);
    let stdin = std::io::stdin();

    println!(
        "{} propuesta(s) pendiente(s). Una por pantalla — nunca el YAML completo (v0.5 §15.5).",
        pending.len()
    );
    for proposal in pending {
        let record_id = proposal.record.id.clone();
        let elapsed_ms = proposal
            .proposed_at
            .elapsed()
            .map(|d| d.as_millis())
            .unwrap_or(0);

        println!("\n=== Propuesta: {record_id} ===");
        println!("{}", review::describe_effect(&proposal.record));
        mascot::print_guarding_stdout(args, &proposal.record.statement);

        let word = review::required_confirmation_word(&proposal.record);
        println!(
            "\nActor: {reviewer_actor}\nAutoridad declarada: {}{}\nEscribe '{word}' para aprobar tal cual, 'c' para corregir el statement antes de aprobar, 'r' para rechazar, o cualquier otra cosa para saltar:",
            reviewer_authority.role,
            reviewer_authority
                .domain
                .as_deref()
                .map(|d| format!(" (dominio={d})"))
                .unwrap_or_default()
        );

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            eprintln!("no se pudo leer stdin — saltando el resto de propuestas");
            break;
        }
        let input = input.trim().to_string();

        let decision = if input == word {
            match review::approve(
                &config.rationale_dir,
                proposal,
                None,
                &reviewer_actor,
                reviewer_authority.role,
                reviewer_authority.domain.as_deref(),
            ) {
                Ok(dest) => {
                    mascot::print_stdout(args, mascot::Mood::Happy);
                    println!("Aprobado -> {}", dest.display());
                    "approved"
                }
                Err(e) => {
                    eprintln!("error aprobando: {e}");
                    "error"
                }
            }
        } else if input == "c" {
            println!("Nuevo statement (una línea):");
            let mut new_statement = String::new();
            let _ = stdin.read_line(&mut new_statement);
            let new_statement = new_statement.trim().to_string();

            println!("Escribe '{word}' para confirmar la aprobación con el texto corregido, cualquier otra cosa para abortar:");
            let mut confirm = String::new();
            let _ = stdin.read_line(&mut confirm);
            if confirm.trim() == word {
                match review::approve(
                    &config.rationale_dir,
                    proposal,
                    Some(new_statement),
                    &reviewer_actor,
                    reviewer_authority.role,
                    reviewer_authority.domain.as_deref(),
                ) {
                    Ok(dest) => {
                        mascot::print_stdout(args, mascot::Mood::Happy);
                        println!("Aprobado (corregido) -> {}", dest.display());
                        "approved-corrected"
                    }
                    Err(e) => {
                        eprintln!("error aprobando: {e}");
                        "error"
                    }
                }
            } else {
                println!("Aborted — la propuesta sigue pendiente.");
                "aborted"
            }
        } else if input == "r" {
            match review::reject(&config.rationale_dir, &proposal) {
                Ok(dest) => {
                    println!("Rechazado -> {}", dest.display());
                    "rejected"
                }
                Err(e) => {
                    eprintln!("error rechazando: {e}");
                    "error"
                }
            }
        } else {
            println!("Saltado — la propuesta sigue pendiente.");
            "skipped"
        };

        let _ = review::log_decision(&rationale_local, &record_id, decision, elapsed_ms);
    }
}

/// Auditar "quién aprobó y cuándo" exigía leer el YAML a mano — ningún
/// comando imprimía `approvals[]` ni `lifecycle.events[]`; MCP solo expone
/// el string colapsado `authority: approved|revoked|unreviewed`.
/// Read-only: no muta nada, solo hace visible lo que ya está en el Record.
fn print_approvals_and_lifecycle(record: &storage::Record) {
    if record.approvals.is_empty() {
        println!("Approvals: (ninguna)");
    } else {
        println!("Approvals ({}):", record.approvals.len());
        for approval in &record.approvals {
            let approved_at_key = yaml_serde::Value::String("approved_at".to_string());
            let domain_key = yaml_serde::Value::String("domain".to_string());
            let approved_at = approval
                .extra
                .get(&approved_at_key)
                .and_then(|v| v.as_str())
                .unwrap_or("desconocido");
            let domain = approval
                .extra
                .get(&domain_key)
                .and_then(|v| v.as_str())
                .map(|d| format!(" domain={d}"))
                .unwrap_or_default();
            println!(
                "  - actor={} authority={} status={} approved_at={approved_at}{domain}",
                approval.actor, approval.authority, approval.status
            );
        }
    }

    let lifecycle_key = yaml_serde::Value::String("lifecycle".to_string());
    let events_key = yaml_serde::Value::String("events".to_string());
    let events = match record.extra.get(&lifecycle_key) {
        Some(yaml_serde::Value::Mapping(lifecycle)) => match lifecycle.get(&events_key) {
            Some(yaml_serde::Value::Sequence(events)) => Some(events),
            _ => None,
        },
        _ => None,
    };
    match events {
        Some(events) if !events.is_empty() => {
            println!("Historial de lifecycle ({}):", events.len());
            for event in events {
                let yaml_serde::Value::Mapping(event) = event else {
                    continue;
                };
                let field = |name: &str| -> &str {
                    let key = yaml_serde::Value::String(name.to_string());
                    event.get(&key).and_then(|v| v.as_str()).unwrap_or("?")
                };
                println!(
                    "  - {} actor={} authority={} reason=\"{}\" timestamp={}",
                    field("type"),
                    field("actor"),
                    field("authority"),
                    field("reason"),
                    field("timestamp"),
                );
            }
        }
        _ => println!("Historial de lifecycle: (sin eventos)"),
    }
}

/// `rationale review-record` — mutaciones humanas sobre Records ya
/// canónicos. MCP solo consulta/prepara; esta orden exige un actor declarado
/// por el proyecto y confirma una acción concreta antes de escribir.
fn cmd_review_record(args: &[String]) {
    let record_id = first_positional_arg(args, &["--project-root"])
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("uso: rationale review-record <record-id> [--project-root <path>]");
            std::process::exit(1);
        });
    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let config = configuration::load(&project_root).unwrap_or_else(fail);
    let record_path = config
        .rationale_dir
        .join("records")
        .join(format!("{record_id}.yaml"));
    let record = match storage::read_record(&record_path) {
        Ok(record) => record,
        Err(error) => {
            eprintln!("no se pudo leer el Record '{record_id}': {error}");
            std::process::exit(1);
        }
    };
    let reviewer_actor = git_reviewer_actor(&project_root);
    let reviewer_authority = config.authority_for_actor(&reviewer_actor);
    if !reviewer_authority.declared {
        eprintln!(
            "actor {reviewer_actor} no está declarado en .rationale/config.yaml; review_record no permite autoelevar autoridad"
        );
        return;
    }

    println!("=== Record: {} ===", record.id);
    println!("Afirmación: {}", record.statement);
    println!("Tipo/severidad: {}/{}", record.kind, record.severity);
    println!(
        "Estado lifecycle: {}",
        storage::lifecycle_status(&record).unwrap_or("active")
    );
    print_approvals_and_lifecycle(&record);
    println!(
        "Actor: {}\nAutoridad declarada: {}{}",
        reviewer_actor,
        reviewer_authority.role,
        reviewer_authority
            .domain
            .as_deref()
            .map(|domain| format!(" (dominio={domain})"))
            .unwrap_or_default()
    );
    println!(
        "Acción [c] corregir, [d] disputar, [r] revocar, [s] superseder, [a] cambiar autoridad, [e] añadir evidencia, [q] salir:"
    );
    let Some(action) = read_interactive_line() else {
        println!("EOF — no se mutó el Record.");
        return;
    };

    let mutation = match action.as_str() {
        "c" => {
            println!("Nuevo statement:");
            let Some(statement) = read_interactive_line() else {
                return;
            };
            println!("Motivo de la corrección:");
            let Some(reason) = read_interactive_line() else {
                return;
            };
            review::RecordMutation::Correct { statement, reason }
        }
        "d" => {
            println!("Motivo de la disputa:");
            let Some(reason) = read_interactive_line() else {
                return;
            };
            review::RecordMutation::Dispute { reason }
        }
        "r" => {
            println!("Motivo de la revocación:");
            let Some(reason) = read_interactive_line() else {
                return;
            };
            review::RecordMutation::Revoke { reason }
        }
        "s" => {
            println!("ID del Record que lo supersede:");
            let Some(replacement_id) = read_interactive_line() else {
                return;
            };
            println!("Motivo del superseder:");
            let Some(reason) = read_interactive_line() else {
                return;
            };
            review::RecordMutation::Supersede {
                replacement_id,
                reason,
            }
        }
        "a" => {
            println!("Actor declarado que recibirá la autoridad:");
            let Some(new_actor) = read_interactive_line() else {
                return;
            };
            let new_authority = config.authority_for_actor(&new_actor);
            if !new_authority.declared {
                eprintln!("ese actor no está declarado; no se permite autoelevación");
                return;
            }
            println!("Motivo del cambio de autoridad:");
            let Some(reason) = read_interactive_line() else {
                return;
            };
            review::RecordMutation::ChangeAuthority {
                new_actor,
                new_role: new_authority.role,
                new_actor_declared: new_authority.declared,
                reason,
            }
        }
        "e" => {
            println!("Ruta de la evidencia (relativa al proyecto):");
            let Some(path) = read_interactive_line() else {
                return;
            };
            println!("Motivo para añadir la evidencia:");
            let Some(reason) = read_interactive_line() else {
                return;
            };
            review::RecordMutation::AddEvidence {
                evidence: storage::Evidence {
                    evidence_type: "review-note".to_string(),
                    path: Some(path),
                    revision: None,
                    verified: false,
                    content_hash: None,
                    visibility: Some("repository".to_string()),
                    extra: yaml_serde::Mapping::new(),
                },
                reason,
            }
        }
        _ => {
            println!("Sin cambios.");
            return;
        }
    };

    println!("Confirma escribiendo APPLY; cualquier otra entrada cancela:");
    if read_interactive_line().as_deref() != Some("APPLY") {
        println!("Cancelado — el Record no cambió.");
        return;
    }
    match review::mutate_record(
        &config.rationale_dir,
        &record_id,
        mutation,
        &reviewer_actor,
        reviewer_authority.role,
        reviewer_authority.declared,
    ) {
        Ok(path) => println!("Lifecycle actualizado atómicamente -> {}", path.display()),
        Err(error) => eprintln!("mutación abortada: {error}"),
    }
}

/// `rationale install-agent` (Arquitectura §24) — registra el servidor MCP
/// en los agentes detectados y escribe las instrucciones que hacen que se
/// invoque solo. Registrar el MCP no basta: sin instrucciones, un agente
/// con herramientas disponibles no las llama (v0.5 §4.12).
fn cmd_install_agent(args: &[String]) {
    let dry_run = args.iter().any(|a| a == "--dry-run");
    // Resuelve el caso "procedencia desconocida": un skill que existe en disco
    // sin registro en el manifest local. No toca los que sí tienen un hash
    // registrado que no coincide — esos están probadamente editados.
    let refresh_skills = args.iter().any(|a| a == "--refresh-skills");
    let binary_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rationale"));

    if args.iter().any(|a| a == "--global-only") {
        // Solo el registro que no depende de un proyecto (Codex vía CLI
        // global). Lo usa `scripts/rationale-installer.sh` justo tras
        // instalar el binario, antes de que exista ningún `.rationale/`.
        match agents::install_global_only(&binary_path, dry_run) {
            Ok(actions) => {
                for line in &actions {
                    println!("- {line}");
                }
            }
            Err(error) => {
                eprintln!("install-agent --global-only falló: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let rationale_local = find_rationale_local(&project_root);

    mascot::print_stdout(args, mascot::Mood::Searching);
    println!("Buscando agentes de código en este proyecto...");

    match agents::install_global_only(&binary_path, dry_run) {
        Ok(actions) => {
            for line in actions {
                println!("- {line}");
            }
        }
        Err(error) => {
            eprintln!("registro global de agentes falló: {error}");
            std::process::exit(1);
        }
    }

    match agents::install(
        &project_root,
        &rationale_local,
        &binary_path,
        refresh_skills,
        dry_run,
    ) {
        Ok(report) => {
            if report.dry_run {
                println!("(dry-run — no se escribió nada)");
            }
            if report.detected.is_empty() {
                mascot::print_stdout(args, mascot::Mood::Base);
                println!("No se detectó ningún agente conocido (claude, codex, cursor-agent).");
            } else {
                mascot::print_stdout(args, mascot::Mood::Happy);
                println!("Agentes detectados: {}", report.detected.join(", "));
            }
            for line in &report.actions {
                println!("- {line}");
            }
        }
        Err(error) => {
            eprintln!("install-agent falló: {error}");
            std::process::exit(1);
        }
    }
}

/// `rationale uninstall-agent` — revierte exactamente lo que `install-agent`
/// escribió, dejando cualquier contenido previo del usuario intacto.
fn cmd_uninstall_agent(args: &[String]) {
    if args.iter().any(|arg| arg == "--global-only") {
        let binary_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rationale"));
        match agents::uninstall_global_only(&binary_path) {
            Ok(actions) => {
                for line in actions {
                    println!("- {line}");
                }
            }
            Err(error) => {
                eprintln!("uninstall-agent --global-only falló: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let rationale_local = find_rationale_local(&project_root);

    match agents::uninstall(&project_root, &rationale_local) {
        Ok(actions) => {
            mascot::print_stdout(args, mascot::Mood::Happy);
            for line in &actions {
                println!("- {line}");
            }
        }
        Err(error) => {
            eprintln!("uninstall-agent falló: {error}");
            std::process::exit(1);
        }
    }
}

fn cmd_update(args: &[String]) {
    if !args.is_empty() {
        fail::<()>("update no acepta opciones; usa 'rationale update --help'");
    }
    let executable = std::env::current_exe()
        .unwrap_or_else(|e| fail(format!("no se pudo localizar rationale: {e}")));

    #[cfg(unix)]
    let status = {
        let helper = executable.with_file_name("rationale-update");
        if !helper.is_file() {
            fail::<()>(
                "no se encontró el helper de actualización junto al binario; ejecuta 'curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-update.sh | sh' una vez",
            );
        }
        Command::new(helper).status().unwrap_or_else(|e| {
            fail(format!(
                "no se pudo ejecutar el helper de actualización: {e}"
            ))
        })
    };

    #[cfg(windows)]
    let status = {
        let helper = executable.with_file_name("rationale-update.ps1");
        if !helper.is_file() {
            return fail("no se encontró rationale-update.ps1 junto al binario");
        }
        Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(helper)
            .status()
            .unwrap_or_else(|e| {
                fail(format!(
                    "no se pudo ejecutar el helper de actualización: {e}"
                ))
            })
    };

    if !status.success() {
        fail::<()>(format!("la actualización terminó con {status}"));
    }
}

/// `rationale doctor` (Fase 1.5) — detecta defectos de integridad que el
/// productor roto (ya corregido en Fase 1.1-1.4) dejó en `.rationale/`.
/// Solo lectura por defecto; `--repair` exige confirmación humana por
/// hallazgo, nunca en lote (mismo principio que `rationale review`).
fn cmd_doctor(args: &[String]) {
    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let config = configuration::load(&project_root).unwrap_or_else(fail);
    let report = doctor::check(&config.rationale_dir, &project_root);

    if args.iter().any(|a| a == "--json") {
        println!(
            "{}",
            serde_json::to_string(&report)
                .unwrap_or_else(|e| fail(format!("no se pudo serializar el reporte: {e}")))
        );
    } else if report.findings.is_empty() {
        println!(
            "Sin hallazgos — el canon en {} está sano.",
            config.rationale_dir.display()
        );
    } else {
        println!(
            "{} hallazgo(s) en {}:",
            report.findings.len(),
            config.rationale_dir.display()
        );
        for finding in &report.findings {
            println!("- {}", finding.describe());
        }
    }

    let repair_mode = args.iter().any(|a| a == "--repair");
    if !repair_mode {
        if args.iter().any(|a| a == "--check") && !report.findings.is_empty() {
            std::process::exit(1);
        }
        return;
    }

    let reviewer_actor = git_reviewer_actor(&project_root);
    let reviewer_authority = config.authority_for_actor(&reviewer_actor);
    let repairable: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.is_repairable())
        .collect();

    if repairable.is_empty() {
        println!("\nNingún hallazgo de este reporte es reparable automáticamente.");
    } else {
        println!(
            "\n{} hallazgo(s) reparable(s). Uno por pantalla — Actor: {reviewer_actor}, autoridad: {}",
            repairable.len(),
            reviewer_authority.role
        );
        for finding in repairable {
            println!("\n=== {} ===", finding.describe());
            println!("Escribe 'repair' para aplicar, cualquier otra cosa para saltar:");
            let Some(confirm) = read_interactive_line() else {
                println!("EOF — no se repara nada más.");
                break;
            };
            if confirm != "repair" {
                println!("Saltado.");
                continue;
            }

            let result = match finding {
                doctor::Finding::InvalidSeverity { record_id, .. } => {
                    println!(
                        "Nueva severidad para '{record_id}' ({}):",
                        storage::Severity::ALL.join(", ")
                    );
                    let Some(new_severity) = read_interactive_line() else {
                        break;
                    };
                    doctor::repair_severity(
                        &config.rationale_dir,
                        record_id,
                        &new_severity,
                        "corregido por rationale doctor --repair",
                        &reviewer_actor,
                        reviewer_authority.role,
                        reviewer_authority.declared,
                    )
                }
                _ => doctor::repair(&config.rationale_dir, finding, &reviewer_actor),
            };

            match result {
                Ok(path) => println!("Reparado -> {}", path.display()),
                Err(e) => eprintln!("no se pudo reparar: {e}"),
            }
        }
    }

    // `RecordWithoutBindings` nunca entra al bucle automático de arriba
    // (`is_repairable() == false` a propósito: inventar un binding sería
    // peor que ninguno). Este es el único camino para cerrar ese hueco: el
    // HUMANO escribe la ruta y, opcionalmente, el símbolo — nunca Rationale
    // adivinándolo. Sin esto, el canon legado que el productor roto dejó
    // sin bindings (Fase 1) no tenía ninguna salida.
    let unbound: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(f, doctor::Finding::RecordWithoutBindings { .. }))
        .collect();
    if !unbound.is_empty() {
        println!(
            "\n{} Record(s) sin binding_declarations. Uno por pantalla — escribe la ruta real \
             que gobiernan, o 'skip' para dejarlo sin reparar:",
            unbound.len()
        );
        for finding in unbound {
            let doctor::Finding::RecordWithoutBindings { record_id, .. } = finding else {
                continue;
            };
            println!("\n=== {} ===", finding.describe());
            println!("Ruta (repo-relativa) que este Record gobierna, o 'skip':");
            let Some(path_hint) = read_interactive_line() else {
                println!("EOF — no se repara nada más.");
                break;
            };
            if path_hint == "skip" {
                println!("Saltado.");
                continue;
            }
            println!("Símbolo dentro de esa ruta (opcional, Enter para omitir):");
            let symbol = read_interactive_line().filter(|s| !s.is_empty());
            println!("Razón (por qué este binding es correcto):");
            let Some(reason) = read_interactive_line() else {
                break;
            };
            let result = doctor::repair_binding(
                &config.rationale_dir,
                &project_root,
                record_id,
                &path_hint,
                symbol.as_deref(),
                &reason,
                &reviewer_actor,
                reviewer_authority.role,
                reviewer_authority.declared,
            );
            match result {
                Ok(path) => println!("Reparado -> {}", path.display()),
                Err(e) => eprintln!("no se pudo reparar: {e}"),
            }
        }
    }
}

fn read_interactive_line() -> Option<String> {
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).ok()? == 0 {
        return None;
    }
    Some(input.trim().to_string())
}

/// Identidad del revisor humano — reusa `git config user.name`/`user.email`
/// del propio repo, nunca inventa una identidad ni asume "el agente" como
/// aprobador (`Proceso §21`: la aprobación es de un humano).
fn git_reviewer_actor(repo_path: &Path) -> String {
    let name = run_git_config(repo_path, "user.name");
    let email = run_git_config(repo_path, "user.email");
    match (name, email) {
        (Some(n), Some(e)) => format!("user:{n} <{e}>"),
        (Some(n), None) => format!("user:{n}"),
        _ => "user:local-reviewer".to_string(),
    }
}

fn run_git_config(repo_path: &Path, key: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["config", "--get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El contrato JSON de `init` se rompía en Windows porque la ruta se
    /// interpolaba en un literal y `\` no es un escape JSON válido.
    ///
    /// `\` también es un carácter legal en un nombre de archivo Unix, así que
    /// este test reproduce el fallo en cualquier plataforma: con el código
    /// anterior, `from_str` devolvía `Error("invalid escape")`.
    #[test]
    fn init_status_json_survives_a_backslash_in_the_path() {
        let path = Path::new(r"C:\Users\alguien\Desktop\Proyecto\.rationale");
        let raw = status_json("already-initialized", path);

        let parsed: serde_json::Value =
            serde_json::from_str(&raw).expect("el contrato debe ser JSON válido");
        assert_eq!(parsed["status"], "already-initialized");
        assert_eq!(
            parsed["path"].as_str().unwrap(),
            path.display().to_string(),
            "la ruta debe sobrevivir el round-trip sin alterarse"
        );
    }

    #[test]
    fn init_status_json_is_a_single_line() {
        let raw = status_json("initialized", Path::new("/tmp/p/.rationale"));
        assert_eq!(raw.lines().count(), 1, "stdout conserva una única línea");
    }
}
