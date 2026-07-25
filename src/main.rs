//! Rationale — vertical slice (Fase D).
//!
//! Arquitectura_Conceptual_v0.1.md §23, Fase D: exactamente
//!   init -> leer Record canónico -> resolver target -> consultar
//!   Codebase Memory -> verificar revisión -> devolver una constraint
//!   compacta.
//! No más que esto — el resto (FTS, budget multi-constraint, capture,
//! assessments persistidos) pertenece a Fase E/F.

mod configuration;
mod evaluation;
mod project;
mod providers;
mod retrieval;
mod revision;
mod storage;

use providers::{CodeIntelligenceProvider, ProviderStatus};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");

    match command {
        "init" => cmd_init(),
        "health" => cmd_health(&args[2..]),
        "prepare" => cmd_prepare(&args[2..]),
        _ => {
            eprintln!("Uso: rationale <init|health|prepare> [opciones]");
            eprintln!("  rationale init");
            eprintln!("  rationale health [--project-root <path>]");
            eprintln!(
                "  rationale prepare <target-spec> [--project-root <path>] [--repo-path <path>]"
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

    let config = configuration::load(&project_root).expect("cargar configuración");
    let snap = revision::snapshot(&config.project_root);

    let provider_line = match providers::codebase_memory::CodebaseMemoryClient::spawn() {
        Ok(mut client) => {
            let result = client.health(config.project_root.to_str().unwrap_or(""));
            format!(
                "\"provider_status\":\"{}\",\"provider_coverage\":\"{:?}\"",
                match result.status {
                    ProviderStatus::Successful => "successful",
                    ProviderStatus::Degraded => "degraded",
                    ProviderStatus::Unavailable => "unavailable",
                },
                result.coverage
            )
        }
        Err(e) => format!("\"provider_status\":\"unreachable\",\"provider_error\":\"{e}\""),
    };

    println!(
        "{{\"project_id\":\"{}\",\"project_root\":\"{}\",\"git_revision\":{},\"working_tree_dirty\":{},{}}}",
        config.project_id,
        config.project_root.display(),
        snap.head.map(|h| format!("\"{h}\"")).unwrap_or_else(|| "null".to_string()),
        snap.working_tree_dirty,
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

    let t0 = Instant::now();

    let config = configuration::load(&project_root).expect("cargar configuración");

    // 1. Leer el Record canónico (por ahora: el primero disponible que
    // mencione el symbol solicitado; Fase E añade FTS/scope real).
    let records_dir = config.rationale_dir.join("records");
    let records = storage::list_records(&records_dir).expect("leer records");

    // 2. Resolver el target dentro del proyecto (protección path traversal,
    // Arquitectura §15.3). El Target resuelto se reutiliza como fuente única
    // del símbolo, evitando parsear el spec dos veces.
    let target = project::resolve_target(&repo_path, &target_spec);
    let symbol = target.as_ref().ok().and_then(|t| t.symbol.clone());

    let record = records
        .iter()
        .find(|r| {
            symbol
                .as_ref()
                .map(|s| {
                    r.binding_declarations.iter().any(|b| {
                        b.structural_id
                            .as_ref()
                            .map(|sid| sid.ends_with(s.as_str()))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(true)
        })
        .or_else(|| records.first())
        .expect("no hay Records en .rationale/records/");

    // 3. Consultar Codebase Memory — sesión MCP persistente real (ADR-0002).
    let (provider_status, provider_coverage, resolved_target, provider_warnings) =
        match providers::codebase_memory::CodebaseMemoryClient::spawn() {
            Ok(mut client) => {
                let sym = symbol.clone().unwrap_or_default();
                let result = client.resolve_target(repo_path.to_str().unwrap_or(""), &sym);
                (
                    result.status,
                    result.coverage,
                    result.data.map(|t| t.qualified_name),
                    result.warnings,
                )
            }
            Err(e) => (
                ProviderStatus::Unavailable,
                providers::Coverage::Unknown,
                None,
                vec![format!("no se pudo iniciar Codebase Memory: {e}")],
            ),
        };

    match &target {
        Ok(t) => eprintln!("target resuelto: {}", t.path.display()),
        Err(e) => eprintln!("advertencia: {e}"),
    }

    // 4. Verificar revisión — SIEMPRE desde Git, nunca desde el proveedor (ADR-0006).
    let snap = revision::snapshot(&repo_path);
    let bound_revision = record.bound_revision.clone().unwrap_or_default();
    let consistency = revision::check_consistency(&snap, &bound_revision);

    // 5. + 6. Compilar y emitir el packet compacto.
    let packet = retrieval::compile_packet(
        snap.head.clone(),
        consistency.clone(),
        provider_status,
        provider_coverage,
        record,
        resolved_target,
        provider_warnings,
    );

    let packet_json = serde_json::to_string(&packet).expect("serialize packet");
    println!("{packet_json}");

    // Instrumentación desde el día uno (Arquitectura §20).
    let rationale_local = find_rationale_local(&project_root);
    let _ = evaluation::record_run(
        &rationale_local,
        &evaluation::RunLog {
            event: "prepare".to_string(),
            timestamp: evaluation::now_iso8601(),
            latency_ms: t0.elapsed().as_millis(),
            git_revision: snap.head,
            consistency: consistency.to_string(),
            provider_status: packet.snapshot.provider_status.clone(),
            provider_coverage: packet.snapshot.provider_coverage.clone(),
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
