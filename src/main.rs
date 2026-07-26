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
mod revision;
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
        _ => {
            eprintln!("Uso: rationale <init|health|prepare|serve> [opciones]");
            eprintln!("  rationale init");
            eprintln!("  rationale health [--project-root <path>]");
            eprintln!(
                "  rationale prepare <target-spec> [--project-root <path>] [--repo-path <path>] [--intent \"texto\"]"
            );
            eprintln!(
                "  rationale serve   # servidor MCP (prepare_change, explain_target, health)"
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
