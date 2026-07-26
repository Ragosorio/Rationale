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
mod cache;
mod capture;
mod configuration;
mod evaluation;
mod mascot;
mod mcp;
mod pipeline;
mod project;
mod providers;
mod retrieval;
mod review;
mod revision;
mod signals;
mod storage;
mod subjects;

use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");

    match command {
        "init" => cmd_init(&args[2..]),
        "health" => cmd_health(&args[2..]),
        "prepare" => cmd_prepare(&args[2..]),
        "serve" => mcp::server::run(),
        "review" => cmd_review(&args[2..]),
        "review-record" => cmd_review_record(&args[2..]),
        "install-agent" => cmd_install_agent(&args[2..]),
        "uninstall-agent" => cmd_uninstall_agent(&args[2..]),
        _ => {
            eprintln!(
                "Uso: rationale <init|health|prepare|serve|review|review-record|install-agent|uninstall-agent> [opciones]"
            );
            eprintln!("  rationale init [--skip-agent-config]");
            eprintln!("  rationale health [--project-root <path>]");
            eprintln!(
                "  rationale prepare <target-spec> [--project-root <path>] [--repo-path <path>] [--intent \"texto\"]"
            );
            eprintln!(
                "  rationale serve   # servidor MCP (prepare_change, explain_target, health, finalize_change)"
            );
            eprintln!(
                "  rationale review [--project-root <path>]   # confirma propuestas pendientes, una a la vez"
            );
            eprintln!(
                "  rationale review-record <record-id> [--project-root <path>]   # lifecycle humano de un Record aprobado"
            );
            eprintln!(
                "  rationale install-agent [--project-root <path>] [--dry-run] [--global-only]   # registra el MCP y las instrucciones de invocación en los agentes detectados"
            );
            eprintln!(
                "  rationale uninstall-agent [--project-root <path>]   # revierte exactamente lo que install-agent escribió"
            );
            std::process::exit(1);
        }
    }
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

fn cmd_init(args: &[String]) {
    let cwd = std::env::current_dir().expect("cwd");
    let rationale_dir = cwd.join(".rationale");
    if rationale_dir.exists() {
        println!(
            "{{\"status\":\"already-initialized\",\"path\":\"{}\"}}",
            rationale_dir.display()
        );
        return;
    }
    for sub in [
        "subjects",
        "records",
        "bindings",
        "approvals",
        "schemas",
        "migrations",
    ] {
        std::fs::create_dir_all(rationale_dir.join(sub)).expect("crear subdirectorio");
    }
    println!(
        "{{\"status\":\"initialized\",\"path\":\"{}\"}}",
        rationale_dir.display()
    );

    // Todo lo que sigue es decoración y conveniencia para humanos — va a
    // stderr para no tocar el contrato de stdout de arriba.
    eprintln!("{}", mascot::art(mascot::Mood::Happy));
    eprintln!("  Soy Chestie — cuido las vallas de este proyecto: por qué existe el código, no solo qué hace.");

    let skip_agent_config = args.iter().any(|a| a == "--skip-agent-config")
        || std::env::var("RATIONALE_SKIP_AGENT_CONFIG").as_deref() == Ok("1");
    if skip_agent_config {
        return;
    }

    eprintln!();
    eprintln!("{}", mascot::art(mascot::Mood::Searching));
    eprintln!("  Buscando agentes de código en este proyecto para avisarles de mí...");
    let rationale_local = find_rationale_local(&cwd);
    let binary_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rationale"));
    match agents::install(&cwd, &rationale_local, &binary_path, false) {
        Ok(report) if report.detected.is_empty() => {
            eprintln!(
                "  No encontré ningún agente conocido todavía — corre 'rationale install-agent' cuando tengas uno."
            );
        }
        Ok(report) => {
            eprintln!("{}", mascot::art(mascot::Mood::Happy));
            eprintln!("  Avisé a: {}", report.detected.join(", "));
            for line in &report.actions {
                eprintln!("  - {line}");
            }
        }
        Err(error) => {
            eprintln!("  advertencia: no pude avisar a los agentes automáticamente: {error}");
        }
    }
}

fn cmd_health(args: &[String]) {
    let project_root = parse_flag(args, "--project-root")
        .or_else(|| configuration::find_project_root(&std::env::current_dir().unwrap()))
        .expect("no se encontró .rationale/; usa --project-root o corre dentro de un proyecto Rationale");

    let mut provider = providers::ProviderHandle::spawn();
    let outcome = pipeline::health(&project_root, &mut provider);

    let provider_line = match &outcome.provider_error {
        Some(e) => format!("\"provider_status\":\"unreachable\",\"provider_error\":\"{e}\""),
        None => format!(
            "\"provider_status\":\"{}\",\"provider_coverage\":\"{:?}\"",
            match outcome.provider_status {
                providers::ProviderStatus::Successful => "successful",
                providers::ProviderStatus::Degraded => "degraded",
                providers::ProviderStatus::Unavailable => "unavailable",
            },
            outcome.provider_coverage
        ),
    };

    println!(
        "{{\"project_id\":\"{}\",\"project_root\":\"{}\",\"git_revision\":{},\"working_tree_dirty\":{},{}}}",
        outcome.project_id,
        outcome.project_root.display(),
        outcome
            .git_revision
            .map(|h| format!("\"{h}\""))
            .unwrap_or_else(|| "null".to_string()),
        outcome.working_tree_dirty,
        provider_line
    );
}

fn cmd_prepare(args: &[String]) {
    let target_spec = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .expect("uso: rationale prepare <target-spec>");

    let project_root = parse_flag(args, "--project-root")
        .or_else(|| configuration::find_project_root(&std::env::current_dir().unwrap()))
        .expect("no se encontró .rationale/; usa --project-root o corre dentro de un proyecto Rationale");
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
    );

    for line in &outcome.diagnostics {
        eprintln!("{line}");
    }

    match outcome.packet.critical_constraints.first() {
        Some(constraint) => {
            eprintln!("{}", mascot::guarding_with_message(&constraint.statement));
        }
        None => {
            eprintln!("{}", mascot::art(mascot::Mood::Happy));
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
    let project_root = parse_flag(args, "--project-root")
        .or_else(|| configuration::find_project_root(&std::env::current_dir().unwrap()))
        .expect("no se encontró .rationale/; usa --project-root o corre dentro de un proyecto Rationale");
    let config = configuration::load(&project_root).expect("cargar configuración");

    let pending_result = review::list_pending_detailed(&config.rationale_dir);
    for (path, error) in &pending_result.skipped {
        eprintln!(
            "advertencia: propuesta no legible, requiere recuperación manual: {} ({error})",
            path.display()
        );
    }
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

/// `rationale review-record` — mutaciones humanas sobre Records ya
/// canónicos. MCP solo consulta/prepara; esta orden exige un actor declarado
/// por el proyecto y confirma una acción concreta antes de escribir.
fn cmd_review_record(args: &[String]) {
    let record_id = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("uso: rationale review-record <record-id> [--project-root <path>]");
            std::process::exit(1);
        });
    let project_root = parse_flag(args, "--project-root")
        .or_else(|| configuration::find_project_root(&std::env::current_dir().unwrap()))
        .expect("no se encontró .rationale/; usa --project-root o corre dentro de un proyecto Rationale");
    let config = configuration::load(&project_root).expect("cargar configuración");
    let record_path = config
        .rationale_dir
        .join("records")
        .join(format!("{record_id}.yaml"));
    let record = match storage::read_record(&record_path) {
        Ok(record) => record,
        Err(error) => {
            eprintln!("no se pudo leer el Record '{record_id}': {error}");
            return;
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

    let project_root = parse_flag(args, "--project-root")
        .or_else(|| configuration::find_project_root(&std::env::current_dir().unwrap()))
        .expect("no se encontró .rationale/; usa --project-root o corre dentro de un proyecto Rationale");
    let rationale_local = find_rationale_local(&project_root);

    println!("{}", mascot::art(mascot::Mood::Searching));
    println!("Buscando agentes de código en este proyecto...");

    match agents::install(&project_root, &rationale_local, &binary_path, dry_run) {
        Ok(report) => {
            if report.dry_run {
                println!("(dry-run — no se escribió nada)");
            }
            if report.detected.is_empty() {
                println!("{}", mascot::art(mascot::Mood::Base));
                println!("No se detectó ningún agente conocido (claude, codex, cursor-agent).");
            } else {
                println!("{}", mascot::art(mascot::Mood::Happy));
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
    let project_root = parse_flag(args, "--project-root")
        .or_else(|| configuration::find_project_root(&std::env::current_dir().unwrap()))
        .expect("no se encontró .rationale/; usa --project-root o corre dentro de un proyecto Rationale");
    let rationale_local = find_rationale_local(&project_root);

    match agents::uninstall(&project_root, &rationale_local) {
        Ok(actions) => {
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
