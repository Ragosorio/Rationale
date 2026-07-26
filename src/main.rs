//! Rationale — vertical slice (Fase D).
//!
//! Arquitectura_Conceptual_v0.1.md §23, Fase D: exactamente
//!   init -> leer Record canónico -> resolver target -> consultar
//!   Codebase Memory -> verificar revisión -> devolver una constraint
//!   compacta.
//! No más que esto — el resto (FTS, budget multi-constraint, capture,
//! assessments persistidos) pertenece a Fase E/F.

mod assessment;
mod cache;
mod capture;
mod configuration;
mod evaluation;
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
        "init" => cmd_init(),
        "health" => cmd_health(&args[2..]),
        "prepare" => cmd_prepare(&args[2..]),
        "serve" => mcp::server::run(),
        "review" => cmd_review(&args[2..]),
        _ => {
            eprintln!("Uso: rationale <init|health|prepare|serve|review> [opciones]");
            eprintln!("  rationale init");
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

fn cmd_init() {
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

    let pending = review::list_pending(&config.rationale_dir);
    if pending.is_empty() {
        println!("No hay propuestas pendientes en .rationale/proposals/.");
        return;
    }

    let reviewer_actor = git_reviewer_actor(&project_root);
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
            "\nEscribe '{word}' para aprobar tal cual, 'c' para corregir el statement antes de aprobar, 'r' para rechazar, o cualquier otra cosa para saltar:"
        );

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            eprintln!("no se pudo leer stdin — saltando el resto de propuestas");
            break;
        }
        let input = input.trim().to_string();

        let decision = if input == word {
            match review::approve(&config.rationale_dir, proposal, None, &reviewer_actor) {
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
