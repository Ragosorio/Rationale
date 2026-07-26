//! Pipeline compartido entre la CLI y el servidor MCP (Fase E5).
//!
//! Antes de este módulo, `cmd_prepare`/`cmd_health` en `main.rs` mezclaban
//! la lógica con la impresión a stdout/stderr — un servidor MCP no puede
//! usar ninguna de las dos (`Arquitectura §11.1`: stdout es exclusivamente
//! del protocolo). Este módulo extrae el pipeline puro: recibe una
//! petición, hace el trabajo, y devuelve datos estructurados más una lista
//! de `diagnostics` — cada caller (CLI o servidor MCP) decide dónde
//! escribirlos.

use crate::providers::{CodeIntelligenceProvider, Coverage, ProviderHandle, ProviderStatus};
use crate::storage::{
    Approval, BindingDeclaration, EpistemicStatus, Record, RecordSubjectRef, Risk,
};
use crate::{
    assessment, cache, capture, configuration, project, retrieval, revision, signals, storage,
    subjects,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Entrada de `prepare_change` (Rationale_v0.5.md §24): target, intención,
/// alcance y presupuesto. Revisión y modo se derivan de Git y de la
/// presencia de `intent`, no se piden explícitamente todavía (Fase F
/// extendería esto con revisión/workspace explícitos si hiciera falta).
pub struct PrepareRequest {
    pub target_spec: String,
    pub intent: Option<String>,
    pub project_root: PathBuf,
    pub repo_path: PathBuf,
    pub budget: retrieval::Budget,
}

pub struct PrepareOutcome {
    pub packet: retrieval::ContextPacket,
    pub assessment: assessment::Assessment,
    /// Lo que antes eran `eprintln!` sueltos — cada caller decide destino.
    pub diagnostics: Vec<String>,
    pub latency_ms: u128,
}

/// Compila el packet completo para un target — el cuerpo de `prepare_change`.
/// Movido tal cual desde `cmd_prepare` (Fase D/E4): misma lógica, mismo
/// orden de pasos, solo que los `eprintln!` se acumulan en `diagnostics` en
/// vez de escribirse directamente.
pub fn prepare(req: &PrepareRequest, provider: &mut ProviderHandle) -> PrepareOutcome {
    let t0 = Instant::now();
    let mut diagnostics = Vec::new();

    let config = configuration::load(&req.project_root).expect("cargar configuración");

    // 1. Leer el Record canónico (por ahora: el primero disponible que
    // mencione el symbol solicitado; Fase E añade FTS/scope real).
    let records_dir = config.rationale_dir.join("records");
    let records = storage::list_records(&records_dir).expect("leer records");

    // 2. Resolver el target dentro del proyecto (protección path traversal,
    // Arquitectura §15.3). El Target resuelto se reutiliza como fuente única
    // del símbolo, evitando parsear el spec dos veces.
    let target = project::resolve_target(&req.repo_path, &req.target_spec);
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

    // Resolver el Subject referenciado por el Record contra el canon real
    // (Rationale_v0.5.md §9.1, orden 1-2: ID exacto y alias). Una
    // referencia colgante (Subject inexistente) se reporta, no se oculta.
    if let Some(subject_ref) = &record.subject {
        let subjects_dir = config.rationale_dir.join("subjects");
        match subjects::list_subjects(&subjects_dir) {
            Ok(subjects_list) => {
                match subjects::resolve_by_id_or_alias(&subjects_list, &subject_ref.id) {
                    Some(subject) => diagnostics.push(format!(
                        "subject: {} [{}] (scope={}, applies_to={} entradas)",
                        subject.title,
                        subject.subject_type,
                        subject.scope,
                        subject.applies_to.len()
                    )),
                    None => diagnostics.push(format!(
                        "advertencia: el Record referencia el Subject '{}' pero no existe en {}",
                        subject_ref.id,
                        subjects_dir.display()
                    )),
                }
            }
            Err(e) => diagnostics.push(format!("advertencia: no se pudieron leer Subjects: {e}")),
        }
    }

    // 3. Consultar Codebase Memory — sesión MCP persistente real (ADR-0002).
    // La sesión (`provider`) ya fue spawneada por el caller: la CLI una vez
    // por invocación, el servidor MCP una sola vez para toda su vida.
    let (provider_status, provider_coverage, resolved_target, provider_warnings) = match provider {
        ProviderHandle::Live(client) => {
            let sym = symbol.clone().unwrap_or_default();
            let result = client.resolve_target(req.repo_path.to_str().unwrap_or(""), &sym);
            (
                result.status,
                result.coverage,
                result.data.map(|t| t.qualified_name),
                result.warnings,
            )
        }
        ProviderHandle::Unavailable(msg) => (
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            None,
            vec![format!("no se pudo iniciar Codebase Memory: {msg}")],
        ),
    };

    match &target {
        Ok(t) => diagnostics.push(format!("target resuelto: {}", t.path.display())),
        Err(e) => diagnostics.push(format!("advertencia: {e}")),
    }

    // 4. Verificar revisión — SIEMPRE desde Git, nunca desde el proveedor (ADR-0006).
    let snap = revision::snapshot(&req.repo_path);
    let bound_revision = record.bound_revision.clone().unwrap_or_default();
    let consistency = revision::check_consistency(&snap, &bound_revision);

    // Capa derivada (ADR-0004/0005): reconstruible por completo desde
    // .rationale/ — nunca la única copia de una decisión (Arquitectura §11.7).
    let current_revision = snap.head.clone().unwrap_or_default();
    match cache::cache_root(&req.project_root).and_then(|dir| cache::open(&dir).map(|c| (dir, c))) {
        Ok((_dir, conn)) => {
            // FTS se reconstruye en cada consulta a partir de los Records ya
            // cargados — barato a esta escala y demuestra regenerabilidad
            // total sin depender de que el cache sobreviva entre ejecuciones.
            if let Err(e) = cache::rebuild_fts(&conn, &records) {
                diagnostics.push(format!(
                    "advertencia: no se pudo reconstruir el índice FTS: {e}"
                ));
            }

            // Recuperación determinista antes que semántica (v0.5 §19.1):
            // FTS es un paso de candidatos diagnóstico aquí, no decide
            // selección todavía (eso lo hace compile_packet con el budget real).
            if let Some(sym) = &symbol {
                match cache::search_candidates(&conn, sym, 5) {
                    Ok(candidates) if !candidates.is_empty() => {
                        diagnostics.push(format!("candidatos FTS para '{sym}': {candidates:?}"));
                    }
                    Ok(_) => diagnostics.push(format!("candidatos FTS para '{sym}': ninguno")),
                    Err(e) => diagnostics.push(format!("advertencia: búsqueda FTS falló: {e}")),
                }
            }

            match cache::get_cached_assessment(&conn, &record.id, &current_revision) {
                Ok(Some(cached)) => {
                    diagnostics.push(format!(
                        "assessment: cache HIT — {}",
                        cached.assessment_reason
                    ));
                }
                Ok(None) => {
                    diagnostics.push(
                        "assessment: cache MISS — calculando y guardando para esta revisión"
                            .to_string(),
                    );
                }
                Err(e) => diagnostics.push(format!(
                    "advertencia: error leyendo cache de assessments: {e}"
                )),
            }
        }
        Err(e) => diagnostics.push(format!(
            "advertencia: capa derivada no disponible ({e}) — se continúa sin cache"
        )),
    }

    // Assessment (Rationale_v0.5.md §5.6): lo que Rationale puede afirmar
    // HOY sobre la vigencia del Record, separado del Record mismo. Nunca
    // se autoaprueba autoridad ni se sirve una revisión no verificada.
    let computed_assessment = assessment::compute(
        record,
        &snap,
        provider_status.clone(),
        provider_coverage.clone(),
    );
    diagnostics.push(format!(
        "assessment: applicability={} linkage={} authority={} — {}",
        computed_assessment.state.applicability,
        computed_assessment.state.linkage,
        computed_assessment.state.authority,
        computed_assessment.assessment_reason
    ));

    if let Ok(cache_dir) = cache::cache_root(&req.project_root) {
        if let Ok(conn) = cache::open(&cache_dir) {
            if let Err(e) = cache::cache_assessment(&conn, &computed_assessment) {
                diagnostics.push(format!(
                    "advertencia: no se pudo guardar el assessment en cache: {e}"
                ));
            }
        }
    }

    // 5. + 6. Compilar el packet compacto — niveles de prioridad y budget
    // reales (Fase E4), no una sola constraint fija.
    let packet = retrieval::compile_packet(
        snap.head.clone(),
        consistency,
        provider_status,
        provider_coverage,
        &records,
        req.intent.as_deref(),
        resolved_target,
        provider_warnings,
        &req.budget,
    );

    PrepareOutcome {
        packet,
        assessment: computed_assessment,
        diagnostics,
        latency_ms: t0.elapsed().as_millis(),
    }
}

/// Salida de `health` — Rationale_v0.5.md §24: revisión, working tree,
/// proveedor, y sus errores tal cual (sin normalizar a string todavía; cada
/// caller decide cómo presentarlos — la CLI conserva el formato histórico
/// `Debug`, el servidor MCP usa las mismas etiquetas de `retrieval.rs`).
pub struct HealthOutcome {
    pub project_id: String,
    pub project_root: PathBuf,
    pub git_revision: Option<String>,
    pub working_tree_dirty: bool,
    pub provider_status: ProviderStatus,
    pub provider_coverage: Coverage,
    pub provider_error: Option<String>,
}

pub fn health(project_root: &Path, provider: &mut ProviderHandle) -> HealthOutcome {
    let config = configuration::load(project_root).expect("cargar configuración");
    let snap = revision::snapshot(&config.project_root);

    let (provider_status, provider_coverage, provider_error) = match provider {
        ProviderHandle::Live(client) => {
            let result = client.health(config.project_root.to_str().unwrap_or(""));
            (result.status, result.coverage, None)
        }
        ProviderHandle::Unavailable(msg) => (
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            Some(msg.clone()),
        ),
    };

    HealthOutcome {
        project_id: config.project_id,
        project_root: config.project_root,
        git_revision: snap.head,
        working_tree_dirty: snap.working_tree_dirty,
        provider_status,
        provider_coverage,
        provider_error,
    }
}

/// Un Record que gobierna el target, tal como lo expone `explain_target`.
#[derive(Debug, Serialize)]
pub struct GoverningRecord {
    pub id: String,
    pub statement: String,
    pub rationale: Option<String>,
    pub authority: String,
    pub epistemic_status: String,
}

#[derive(Debug, Serialize)]
pub struct SubjectSummary {
    pub id: String,
    pub title: String,
    pub subject_type: String,
    pub scope: String,
}

pub struct ExplainOutcome {
    pub resolved_target: Option<String>,
    pub governing_records: Vec<GoverningRecord>,
    pub subject: Option<SubjectSummary>,
    /// Lo que Rationale puede afirmar con evidencia — nunca "no existe una
    /// decisión" cuando en realidad es "no se encontró dentro de la
    /// cobertura disponible" (v0.5 §19.2).
    pub known: Vec<String>,
    pub unknown: Vec<String>,
    pub diagnostics: Vec<String>,
}

/// El cuerpo de `explain_target` (v0.5 §24): por qué existe un target, qué
/// Records lo gobiernan por binding exacto, y qué parte es conocida vs
/// desconocida. Recuperación determinista, sin heurísticas ni FTS todavía
/// (eso es candidato de Fase F si el binding exacto resulta insuficiente).
pub fn explain(target_spec: &str, project_root: &Path, repo_path: &Path) -> ExplainOutcome {
    let mut diagnostics = Vec::new();
    let mut known = Vec::new();
    let mut unknown = Vec::new();

    let config = configuration::load(project_root).expect("cargar configuración");
    let records_dir = config.rationale_dir.join("records");
    let records = storage::list_records(&records_dir).expect("leer records");

    let target = project::resolve_target(repo_path, target_spec);
    let symbol = target.as_ref().ok().and_then(|t| t.symbol.clone());
    let resolved_target = target.as_ref().ok().map(|t| t.path.display().to_string());
    if let Err(e) = &target {
        diagnostics.push(format!("advertencia: {e}"));
    }

    let governing: Vec<&Record> = match &symbol {
        Some(sym) => records
            .iter()
            .filter(|r| {
                r.binding_declarations.iter().any(|b| {
                    b.structural_id
                        .as_ref()
                        .map(|sid| sid.ends_with(sym.as_str()))
                        .unwrap_or(false)
                })
            })
            .collect(),
        None => Vec::new(),
    };

    if governing.is_empty() {
        unknown.push(
            "no se encontró ningún Record con un binding exacto hacia este target dentro de la \
             cobertura disponible; no implica que ninguna decisión lo gobierne"
                .to_string(),
        );
    } else {
        known.push(format!(
            "{} Record(s) gobiernan este target por binding exacto",
            governing.len()
        ));
    }

    let governing_records: Vec<GoverningRecord> = governing
        .iter()
        .map(|r| GoverningRecord {
            id: r.id.clone(),
            statement: r.statement.clone(),
            rationale: r.rationale.clone(),
            authority: if storage::has_approved_authority(r) {
                "approved".to_string()
            } else {
                "unreviewed".to_string()
            },
            epistemic_status: r.epistemic_status.to_string(),
        })
        .collect();

    let mut subject = None;
    if let Some(primary) = governing.first() {
        if let Some(subject_ref) = &primary.subject {
            let subjects_dir = config.rationale_dir.join("subjects");
            match subjects::list_subjects(&subjects_dir) {
                Ok(subjects_list) => {
                    match subjects::resolve_by_id_or_alias(&subjects_list, &subject_ref.id) {
                        Some(s) => {
                            known.push(format!(
                                "Subject resuelto: {} [{}]",
                                s.title, s.subject_type
                            ));
                            subject = Some(SubjectSummary {
                                id: s.id.clone(),
                                title: s.title.clone(),
                                subject_type: s.subject_type.clone(),
                                scope: s.scope.clone(),
                            });
                        }
                        None => unknown.push(format!(
                            "el Record referencia el Subject '{}' pero no existe en el canon",
                            subject_ref.id
                        )),
                    }
                }
                Err(e) => {
                    diagnostics.push(format!("advertencia: no se pudieron leer Subjects: {e}"))
                }
            }
        }
    }

    ExplainOutcome {
        resolved_target,
        governing_records,
        subject,
        known,
        unknown,
        diagnostics,
    }
}

/// Entrada de `finalize_change` (Rationale_v0.5.md §24, flujo de
/// `Arquitectura §13.3`). Lo que NO puede derivarse mecánicamente (la razón,
/// la afirmación normativa propuesta, el Subject candidato) viene del
/// caller — Rationale nunca inventa contenido normativo, solo lo estructura
/// y lo contrasta contra el canon existente.
pub struct FinalizeRequest {
    pub target_spec: String,
    pub project_root: PathBuf,
    pub repo_path: PathBuf,
    /// Revisión desde la que se captura el diff — normalmente la revisión
    /// que un `prepare_change` anterior ya reportó como "preflight".
    pub base_revision: String,
    /// Por qué se hizo el cambio — alimenta la detección de señales
    /// (`signals::signals_from_text`) y se guarda como `rationale` del Record.
    pub intent: String,
    /// La afirmación normativa propuesta. Requerido por la API, pero solo
    /// se usa si el nivel de captura resultante supera Nivel 0 (Git-only).
    pub statement: String,
    pub severity: String,
    pub record_id: String,
    pub subject_id: String,
    pub subject_title: String,
    /// Requerido por v0.5 §294 cuando el Subject Resolver sugiere un
    /// candidato fuerte (`Alias`/`MergeCandidate`) pero el caller insiste en
    /// que es un concepto nuevo. Sin esto, una propuesta contra un
    /// candidato fuerte se bloquea (ver `FinalizeOutcome::blocked_reason`).
    pub novelty_reason: Option<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FinalizeOutcome {
    pub level: signals::CaptureLevel,
    pub signals: Vec<signals::Signal>,
    pub capture: capture::MechanicalCapture,
    pub subject_resolution: Option<subjects::Resolution>,
    pub proposal_written: bool,
    pub proposal_path: Option<String>,
    pub proposal_id: Option<String>,
    /// Por qué NO se escribió una propuesta, cuando aplica — Nivel 0 (nada
    /// que capturar) o un candidato de Subject fuerte sin `novelty_reason`
    /// (v0.5 §294). Nunca silencioso: siempre que `proposal_written` es
    /// `false`, este campo explica por qué.
    pub blocked_reason: Option<String>,
    pub diagnostics: Vec<String>,
}

/// El cuerpo de `finalize_change`: captura mecánica → señales → nivel →
/// resolución de Subject → escritura de la propuesta (nunca del Record
/// aprobado — eso es `rationale review`, Fase F6). Sigue el flujo de
/// `Arquitectura §13.3` hasta "Write canonical files atomically", pero
/// escribe en `.rationale/proposals/`, no en `.rationale/records/`.
pub fn finalize(req: &FinalizeRequest, provider: &mut ProviderHandle) -> FinalizeOutcome {
    let mut diagnostics = Vec::new();
    let config = configuration::load(&req.project_root).expect("cargar configuración");

    // Resolver el target principal es solo diagnóstico aquí — nunca decide
    // qué se captura (eso lo hace el diff mecánico completo); sirve para
    // que quien lea la propuesta vea contra qué target se declaró el cambio.
    match project::resolve_target(&req.repo_path, &req.target_spec) {
        Ok(t) => diagnostics.push(format!("target declarado: {}", t.path.display())),
        Err(e) => diagnostics.push(format!("advertencia: target declarado no resuelto: {e}")),
    }

    let mechanical = capture::capture(&req.repo_path, &req.base_revision, provider);

    let mut all_signals: std::collections::HashSet<signals::Signal> =
        signals::signals_from_paths(&mechanical.changed_files)
            .into_iter()
            .collect();
    all_signals.extend(signals::signals_from_text(&req.intent));
    all_signals.extend(signals::signals_from_text(&req.statement));
    let signal_list: Vec<signals::Signal> = all_signals.into_iter().collect();

    let level = signals::determine_level(&mechanical.changed_files, &signal_list);

    if level == signals::CaptureLevel::GitOnly {
        diagnostics.push(
            "Nivel 0 (Git-only): cambios mecánicos sin señales de alto valor — no se crea \
             ninguna propuesta (v0.5 §16)."
                .to_string(),
        );
        return FinalizeOutcome {
            level,
            signals: signal_list,
            capture: mechanical,
            subject_resolution: None,
            proposal_written: false,
            proposal_path: None,
            proposal_id: None,
            blocked_reason: Some("nivel Git-only: nada que capturar".to_string()),
            diagnostics,
        };
    }

    // Resolver el Subject contra el canon real (Fase F4) antes de escribir
    // nada — nunca se crea un Subject nuevo a ciegas.
    let subjects_dir = config.rationale_dir.join("subjects");
    let records_dir = config.rationale_dir.join("records");
    let existing_subjects = subjects::list_subjects(&subjects_dir).unwrap_or_default();
    let existing_records = storage::list_records(&records_dir).unwrap_or_default();

    let mut existing_bindings: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for record in &existing_records {
        if let Some(subject_ref) = &record.subject {
            let paths: Vec<String> = record
                .binding_declarations
                .iter()
                .filter_map(|b| b.path_hint.clone())
                .collect();
            existing_bindings
                .entry(subject_ref.id.clone())
                .or_default()
                .extend(paths);
        }
    }

    let proposed_bindings: Vec<String> = mechanical
        .changed_files
        .iter()
        .map(|f| f.path.clone())
        .collect();

    let resolution = subjects::resolve(
        &existing_subjects,
        &req.subject_id,
        &req.subject_title,
        "project",
        &proposed_bindings,
        &existing_bindings,
    );

    let needs_novelty_reason = matches!(
        resolution.action,
        subjects::ResolutionAction::Alias | subjects::ResolutionAction::MergeCandidate
    );
    if needs_novelty_reason && req.novelty_reason.is_none() {
        diagnostics.push(format!(
            "el Subject Resolver sugiere '{:?}' contra un candidato existente — se requiere \
             novelty_reason explícito para proponer un Subject nuevo de todas formas (v0.5 §294)",
            resolution.action
        ));
        return FinalizeOutcome {
            level,
            signals: signal_list,
            capture: mechanical,
            subject_resolution: Some(resolution),
            proposal_written: false,
            proposal_path: None,
            proposal_id: None,
            blocked_reason: Some(
                "candidato de Subject fuerte sin novelty_reason — ver subject_resolution.candidates"
                    .to_string(),
            ),
            diagnostics,
        };
    }

    // Si el resolver sugiere reusar (o el caller no dio novelty_reason para
    // anular Alias/MergeCandidate — ya cubierto arriba), el Record propuesto
    // referencia el Subject EXISTENTE, no uno nuevo.
    let final_subject_id = match &resolution.selected_subject {
        Some(existing_id) if req.novelty_reason.is_none() => existing_id.clone(),
        _ => req.subject_id.clone(),
    };

    let binding_declarations: Vec<BindingDeclaration> = mechanical
        .changed_files
        .iter()
        .enumerate()
        .map(|(i, f)| BindingDeclaration {
            id: format!("binding.{}.{i}", req.record_id),
            kind: "file".to_string(),
            provider: None,
            structural_id: None,
            path_hint: Some(f.path.clone()),
            extra: yaml_serde::Mapping::new(),
        })
        .collect();

    let risks: Vec<Risk> = req
        .risks
        .iter()
        .enumerate()
        .map(|(i, statement)| Risk {
            id: format!("risk.{}.{i}", req.record_id),
            statement: statement.clone(),
            epistemic_status: EpistemicStatus::Stated,
            extra: yaml_serde::Mapping::new(),
        })
        .collect();

    let mut extra = yaml_serde::Mapping::new();
    extra.insert(
        yaml_serde::Value::String("schema_version".to_string()),
        yaml_serde::Value::String("rationale/0.1".to_string()),
    );
    extra.insert(
        yaml_serde::Value::String("project_id".to_string()),
        yaml_serde::Value::String(config.project_id.clone()),
    );
    extra.insert(
        yaml_serde::Value::String("status".to_string()),
        yaml_serde::Value::String("pending".to_string()),
    );

    let proposal = Record {
        id: req.record_id.clone(),
        kind: "constraint".to_string(),
        severity: req.severity.clone(),
        statement: req.statement.clone(),
        rationale: Some(req.intent.clone()),
        epistemic_status: EpistemicStatus::Stated,
        // Nunca se autoaprueba — approvals vacío es la garantía estructural
        // de que ninguna propuesta nace aprobada (Proceso §21).
        approvals: Vec::<Approval>::new(),
        binding_declarations,
        evidence: vec![],
        risks,
        bound_revision: mechanical.final_revision.clone(),
        subject: Some(RecordSubjectRef {
            id: final_subject_id.clone(),
            extra: yaml_serde::Mapping::new(),
        }),
        extra,
    };

    let proposals_dir = config.rationale_dir.join("proposals");
    let proposal_path = proposals_dir.join(format!("{}.yaml", req.record_id));

    match storage::write_record(&proposal_path, &proposal) {
        Ok(()) => {
            diagnostics.push(format!(
                "propuesta escrita en {} (nivel={level:?}, subject={final_subject_id})",
                proposal_path.display()
            ));
            FinalizeOutcome {
                level,
                signals: signal_list,
                capture: mechanical,
                subject_resolution: Some(resolution),
                proposal_written: true,
                proposal_path: Some(proposal_path.display().to_string()),
                proposal_id: Some(req.record_id.clone()),
                blocked_reason: None,
                diagnostics,
            }
        }
        Err(e) => {
            diagnostics.push(format!("error escribiendo la propuesta: {e}"));
            FinalizeOutcome {
                level,
                signals: signal_list,
                capture: mechanical,
                subject_resolution: Some(resolution),
                proposal_written: false,
                proposal_path: None,
                proposal_id: None,
                blocked_reason: Some(format!("error de escritura: {e}")),
                diagnostics,
            }
        }
    }
}
