//! Pipeline compartido entre la CLI y el servidor MCP (Fase E5).
//!
//! Antes de este módulo, `cmd_prepare`/`cmd_health` en `main.rs` mezclaban
//! la lógica con la impresión a stdout/stderr — un servidor MCP no puede
//! usar ninguna de las dos (`Arquitectura §11.1`: stdout es exclusivamente
//! del protocolo). Este módulo extrae el pipeline puro: recibe una
//! petición, hace el trabajo, y devuelve datos estructurados más una lista
//! de `diagnostics` — cada caller (CLI o servidor MCP) decide dónde
//! escribirlos.

use crate::providers::{Coverage, ProviderHandle, ProviderStatus};
use crate::storage::Record;
use crate::{
    activity, assessment, binding_match, cache, canon, capture, configuration, context, evaluation,
    operations, project, relationships, retrieval, revision, signals, storage, subjects,
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
    /// Techo del subgrafo estructural que entra al packet.
    pub structural_budget: context::StructuralBudget,
    /// Quién pide el contexto — queda registrado en la operación.
    pub actor: canon::ActorContext,
}

pub struct PrepareOutcome {
    pub packet: retrieval::ContextPacket,
    /// `None` es válido para un proyecto recién inicializado sin Records:
    /// el packet puede informar salud/target/proveedor sin inventar una
    /// decisión ni panicar por un canon vacío.
    pub assessment: Option<assessment::Assessment>,
    /// Lo que antes eran `eprintln!` sueltos — cada caller decide destino.
    pub diagnostics: Vec<String>,
    pub latency_ms: u128,
    /// La operación abierta (persistida en `.rationale-local/operations/` si
    /// se pudo): la actividad y la UI la leen; el agente recibe su id.
    pub operation: operations::Operation,
    /// Dónde registró la actividad esta llamada: el caller emite con él
    /// `packet.delivered` cuando el packet sale hacia el agente.
    pub activity: activity::Scope,
}

/// Compila el packet completo para un target — el cuerpo de `prepare_change`.
/// Movido tal cual desde `cmd_prepare` (Fase D/E4): misma lógica, mismo
/// orden de pasos, solo que los `eprintln!` se acumulan en `diagnostics` en
/// vez de escribirse directamente.
pub fn prepare(
    req: &PrepareRequest,
    provider: &mut ProviderHandle,
    recorder: &activity::Recorder,
) -> Result<PrepareOutcome, String> {
    let t0 = Instant::now();
    let mut diagnostics = Vec::new();

    let config = configuration::load(&req.project_root).map_err(|e| e.to_string())?;

    // 1. Leer el Record canónico.
    let records_dir = config.rationale_dir.join("records");
    let records = storage::list_records(&records_dir)
        .map_err(|e| format!("no se pudieron leer Records: {e}"))?;

    // 2. Resolver el target dentro del proyecto (protección path traversal,
    // Arquitectura §15.3). El Target resuelto se reutiliza como fuente única
    // del símbolo, evitando parsear el spec dos veces.
    let target = project::resolve_target(&req.repo_path, &req.target_spec);
    let symbol = target.as_ref().ok().and_then(|t| t.symbol.clone());

    // Único matcher compartido con `explain` (Fase 1.2): antes, cuando
    // nada matcheaba, este caller caía a `records.first()` (un Record
    // arbitrario sin relación real con el target) mientras `explain`
    // devolvía vacío para la misma consulta — los dos tools se
    // contradecían. Un vacío honesto reemplaza ese fallback.
    let target_key = target
        .as_ref()
        .ok()
        .map(|t| binding_match::target_key(&req.repo_path, t))
        .unwrap_or_default();
    let governing_matches = binding_match::governing(&target_key, &records);
    let record = governing_matches.first().map(|m| m.record);

    if record.is_none() {
        if records.is_empty() {
            diagnostics.push(
                "no hay Records en .rationale/records/; se devuelve un packet sin restricciones y sin assessment"
                    .to_string(),
            );
        } else {
            diagnostics.push(format!(
                "ningún Record de los {} existentes gobierna este target por binding; se devuelve un packet sin assessment de gobernanza directa",
                records.len()
            ));
        }
    }

    // Resolver el Subject referenciado por el Record contra el canon real
    // (Rationale_v0.5.md §9.1, orden 1-2: ID exacto y alias). Una
    // referencia colgante (Subject inexistente) se reporta, no se oculta.
    if let Some(subject_ref) = record.and_then(|record| record.subject.as_ref()) {
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

    // Ruta repo-relativa del target, calculada una sola vez con el mismo
    // helper que el matcher: antes un `strip_prefix` literal fallaba con un
    // `repo_path` relativo (`--project-root .`) y el proveedor recibía un
    // archivo vacío, que la búsqueda por patrón ocultaba devolviendo
    // cualquier nodo.
    let rel_file = target
        .as_ref()
        .ok()
        .and_then(|t| binding_match::target_rel_path(&req.repo_path, t));

    // La operación nace aquí (ADR-0017): todo evento de esta llamada, también
    // los del proveedor, la lleva. Su HEAD es la base honesta del cambio.
    let snap = revision::snapshot(&req.repo_path);
    let mut operation = operations::Operation::new(
        &req.actor,
        operations::OperationTarget {
            spec: req.target_spec.clone(),
            file_path: rel_file.clone(),
            symbol: symbol.clone(),
            node_key: None,
            qualified_name: None,
        },
        req.intent.clone(),
        snap.head.clone(),
    );
    let local_dir = configuration::find_rationale_local(&config.project_root);
    let scope = activity::Scope::new(
        &local_dir,
        &config.project_root,
        &config.project_id,
        &req.actor,
    )
    .with_operation(&operation.operation_id);
    recorder.emit(
        &scope,
        "context.requested",
        activity::payload::context_requested(&req.target_spec, req.intent.as_deref()),
    );
    let provider_name = provider
        .as_provider()
        .map(|client| client.capabilities().name);
    recorder.emit(
        &scope,
        "provider.started",
        activity::payload::provider_started(provider_name.as_deref()),
    );
    let provider_clock = Instant::now();

    // 3. Consultar Codebase Memory — sesión MCP persistente real (ADR-0002).
    // La sesión (`provider`) ya fue spawneada por el caller: la CLI una vez
    // por invocación, el servidor MCP una sola vez para toda su vida.
    let unavailable_reason = provider.unavailable_reason().map(str::to_string);
    let (provider_status, provider_coverage, resolved_target, provider_warnings) =
        match provider.as_provider() {
            Some(client) => {
                let repo = req.repo_path.to_str().unwrap_or("");
                match (
                    rel_file.as_deref(),
                    symbol.as_deref().filter(|s| !s.is_empty()),
                ) {
                    (Some(file), Some(sym)) => {
                        let result = client.resolve_target(repo, file, sym);
                        (
                            result.status,
                            result.coverage,
                            result.data.map(|t| t.qualified_name),
                            result.warnings,
                        )
                    }
                    // Sin símbolo o sin archivo del proyecto no hay nada que
                    // resolver: el snapshot informa la salud del proveedor, nunca
                    // un nodo arbitrario de una búsqueda por patrón.
                    _ => {
                        let result = client.health(repo);
                        (result.status, result.coverage, None, result.warnings)
                    }
                }
            }
            None => (
                ProviderStatus::Unavailable,
                Coverage::Unknown,
                None,
                vec![format!(
                    "no se pudo iniciar el proveedor estructural: {}",
                    unavailable_reason.unwrap_or_default()
                )],
            ),
        };
    let mut provider_elapsed = provider_clock.elapsed();

    match &target {
        Ok(t) => diagnostics.push(format!("target resuelto: {}", t.path.display())),
        Err(e) => diagnostics.push(format!("advertencia: {e}")),
    }

    // 4. Verificar revisión — SIEMPRE desde Git, nunca desde el proveedor (ADR-0006).
    // `snap` se tomó al abrir la operación.
    let bound_revision = record
        .and_then(|record| record.bound_revision.clone())
        .unwrap_or_default();
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

            if let Some(record) = record {
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
        }
        Err(e) => diagnostics.push(format!(
            "advertencia: capa derivada no disponible ({e}) — se continúa sin cache"
        )),
    }

    // Assessment (Rationale_v0.5.md §5.6): lo que Rationale puede afirmar
    // HOY sobre la vigencia del Record, separado del Record mismo. Nunca
    // se autoaprueba autoridad ni se sirve una revisión no verificada.
    let computed_assessment = record.map(|record| {
        let assessment = assessment::compute(
            record,
            &snap,
            provider_status.clone(),
            provider_coverage.clone(),
            &req.repo_path,
        );
        diagnostics.push(format!(
            "assessment: applicability={} linkage={} authority={} — {}",
            assessment.state.applicability,
            assessment.state.linkage,
            assessment.state.authority,
            assessment.assessment_reason
        ));
        assessment
    });

    if let Some(computed_assessment) = &computed_assessment {
        if let Ok(cache_dir) = cache::cache_root(&req.project_root) {
            if let Ok(conn) = cache::open(&cache_dir) {
                if let Err(e) = cache::cache_assessment(&conn, computed_assessment) {
                    diagnostics.push(format!(
                        "advertencia: no se pudo guardar el assessment en cache: {e}"
                    ));
                }
            }
        }
    }

    // 5. Context Compiler vNext: la operación enlaza prepare → trabajo →
    // finalize → UI; el vecindario estructural y las relaciones explicadas
    // se superponen a la memoria causal; el presupuesto es un techo.
    let governing_by_kind: std::collections::HashMap<String, binding_match::MatchKind> =
        governing_matches
            .iter()
            .map(|m| (m.record.id.clone(), m.kind))
            .collect();
    let superseded = canon::superseded_ids(&records);
    let active: Vec<&Record> = records
        .iter()
        .filter(|record| canon::is_active(record, &superseded))
        .collect();
    let gather_clock = Instant::now();
    let mut structural = context::gather(
        provider,
        context::GatherInput {
            repo_path: req.repo_path.to_str().unwrap_or(""),
            project_key: &config.project_id,
            target_file: rel_file.as_deref(),
            target_symbol: symbol.as_deref(),
            records: &active,
            budget: &req.structural_budget,
        },
    );
    provider_elapsed += gather_clock.elapsed();
    if let Some(node) = structural.target_node.as_ref() {
        operation.target.node_key = Some(node.key.clone());
        operation.target.qualified_name = Some(node.binding.qualified_name.clone());
    }
    recorder.emit(
        &scope,
        "target.resolved",
        activity::payload::target_resolved(&operation.target),
    );
    for assessment in &structural.relationships {
        if let Some(kind) = activity::relationship_event_kind(assessment.state) {
            recorder.emit(
                &scope,
                kind,
                activity::payload::relationship_state(assessment),
            );
        }
    }
    recorder.emit(
        &scope,
        "provider.finished",
        activity::payload::provider_finished(
            provider_name.as_deref(),
            provider_status.as_str(),
            provider_coverage.as_str(),
            structural.index_state.as_deref(),
            provider_elapsed.as_millis(),
        ),
    );
    let packet = retrieval::compile(
        retrieval::PacketInput {
            git_head: snap.head.clone(),
            consistency,
            provider_status,
            provider_coverage,
            records: &records,
            intent: req.intent.as_deref(),
            resolved_target,
            provider_warnings,
            budget: &req.budget,
            governing: &governing_by_kind,
            operation_id: Some(operation.operation_id.clone()),
            target: Some(retrieval::PacketTarget {
                spec: req.target_spec.clone(),
                file_path: rel_file,
                symbol,
                qualified_name: operation.target.qualified_name.clone(),
            }),
        },
        Some(&mut structural),
    );

    let mut selected_records: Vec<String> = packet
        .critical_constraints
        .iter()
        .map(|c| c.id.clone())
        .chain(packet.decisions.iter().map(|d| d.id.clone()))
        .chain(packet.relationships.iter().map(|r| r.record_id.clone()))
        .collect();
    selected_records.sort();
    selected_records.dedup();
    operation.graph = structural.to_graph();
    operation.selection = operations::Selection {
        considered_nodes: structural.nodes.len(),
        selected_nodes: structural.selected_nodes.len(),
        considered_relationships: structural.edges.len(),
        selected_relationships: structural.selected_edges.len(),
        considered_records: active.len(),
        selected_records,
        packet_bytes: serde_json::to_vec(&packet).map_or(0, |bytes| bytes.len()),
        estimated_tokens: packet.token_estimate,
    };
    // El snapshot es observación local, como la actividad, y contiene la
    // intención: `RATIONALE_ACTIVITY=off` también lo omite (ADR-0017).
    if recorder.is_enabled() {
        // ADR-0014 §Decision 3: la exclusión de Git antes del snapshot.
        if let Err(e) = activity::ensure_excluded(&config.project_root) {
            diagnostics.push(format!(
                "advertencia: no se pudo excluir .rationale-local/ de Git: {e}"
            ));
        }
        if let Err(e) = operations::save(&local_dir, &operation) {
            diagnostics.push(format!(
                "advertencia: no se pudo guardar el snapshot de la operación: {e}"
            ));
        }
    }
    recorder.emit(
        &scope,
        "packet.compiled",
        activity::payload::packet_compiled(&operation, packet.budget_overflow.as_deref()),
    );

    Ok(PrepareOutcome {
        packet,
        assessment: computed_assessment,
        diagnostics,
        latency_ms: t0.elapsed().as_millis(),
        operation,
        activity: scope,
    })
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

pub fn health(project_root: &Path, provider: &mut ProviderHandle) -> Result<HealthOutcome, String> {
    let config = configuration::load(project_root).map_err(|e| e.to_string())?;
    let snap = revision::snapshot(&config.project_root);

    let unavailable_reason = provider.unavailable_reason().map(str::to_string);
    let (provider_status, provider_coverage, provider_error) = match provider.as_provider() {
        Some(client) => {
            let result = client.health(config.project_root.to_str().unwrap_or(""));
            (result.status, result.coverage, None)
        }
        None => (
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            unavailable_reason,
        ),
    };

    Ok(HealthOutcome {
        project_id: config.project_id,
        project_root: config.project_root,
        git_revision: snap.head,
        working_tree_dirty: snap.working_tree_dirty,
        provider_status,
        provider_coverage,
        provider_error,
    })
}

/// Un Record que gobierna el target, tal como lo expone `explain_target`.
#[derive(Debug, Serialize)]
pub struct GoverningRecord {
    pub id: String,
    pub statement: String,
    pub rationale: Option<String>,
    pub authority: String,
    /// `agent_asserted | human_stated | migrated` — quién afirmó esto.
    pub provenance: String,
    pub epistemic_status: String,
    /// Cómo se determinó que este Record gobierna el target
    /// (`binding_match::MatchKind`) — expuesto para que `prepare` y
    /// `explain` sean auditablemente consistentes entre sí, nunca solo
    /// "confía en que coinciden".
    pub match_kind: String,
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
pub fn explain(
    target_spec: &str,
    project_root: &Path,
    repo_path: &Path,
) -> Result<ExplainOutcome, String> {
    let mut diagnostics = Vec::new();
    let mut known = Vec::new();
    let mut unknown = Vec::new();

    let config = configuration::load(project_root).map_err(|e| e.to_string())?;
    let records_dir = config.rationale_dir.join("records");
    let records = storage::list_records(&records_dir)
        .map_err(|e| format!("no se pudieron leer Records: {e}"))?;

    let target = project::resolve_target(repo_path, target_spec);
    let resolved_target = target.as_ref().ok().map(|t| t.path.display().to_string());
    if let Err(e) = &target {
        diagnostics.push(format!("advertencia: {e}"));
    }

    // Único matcher compartido con `prepare` (Fase 1.2) — antes cada uno
    // tenía su propia comparación y podían discrepar sobre el mismo
    // target. Ya no requiere symbol: un binding de archivo también
    // gobierna una consulta sobre el archivo entero.
    let key = target
        .as_ref()
        .ok()
        .map(|t| binding_match::target_key(repo_path, t))
        .unwrap_or_default();
    let governing_matches = binding_match::governing(&key, &records);
    let governing: Vec<&Record> = governing_matches.iter().map(|m| m.record).collect();

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

    let governing_records: Vec<GoverningRecord> = governing_matches
        .iter()
        .map(|m| GoverningRecord {
            id: m.record.id.clone(),
            statement: m.record.statement.clone(),
            rationale: m.record.rationale.clone(),
            authority: storage::authority_label(m.record).to_string(),
            provenance: storage::provenance_kind(m.record).as_str().to_string(),
            epistemic_status: m.record.epistemic_status.to_string(),
            match_kind: m.kind.as_str().to_string(),
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

    Ok(ExplainOutcome {
        resolved_target,
        governing_records,
        subject,
        known,
        unknown,
        diagnostics,
    })
}

/// Entrada de `finalize_change` vNext. El agente declara qué conocimiento
/// cree durable (`candidates`); Rationale solo verifica, deduplica y decide
/// — nunca redacta contenido normativo por su cuenta ni lo deriva del diff.
pub struct FinalizeRequest {
    pub project_root: PathBuf,
    pub repo_path: PathBuf,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    /// Target principal declarado — solo diagnóstico.
    pub target_spec: Option<String>,
    /// Revisión desde la que se captura el diff mecánico. Sin ella (ni
    /// operación que la aporte) se usa HEAD: el diff cubre solo el árbol de
    /// trabajo, nunca se inventa una base.
    pub base_revision: Option<String>,
    /// Cada candidato ya deserializado, o el error de forma que lo invalidó
    /// — un candidato mal formado se reporta, no tumba la llamada entera.
    pub candidates: Vec<Result<canon::Candidate, String>>,
    pub actor: canon::ActorContext,
    /// `statement` del contrato pre-vNext cuando la llamada no trae
    /// `candidates`: se reporta como descarte explícito, nunca como propuesta.
    pub legacy_statement: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FinalizeSummary {
    pub committed: usize,
    pub discarded: usize,
    pub conflicts: usize,
}

#[derive(Debug, Serialize)]
pub struct FinalizeOutcome {
    pub operation_id: Option<String>,
    pub summary: FinalizeSummary,
    pub committed: Vec<canon::CommittedRecord>,
    pub discarded: Vec<canon::DiscardedCandidate>,
    /// No vacío solo cuando un candidato intentó reemplazar una regla
    /// fijada: el agente debe preguntar al humano y llamar `resolve_conflict`.
    pub conflicts: Vec<canon::ConflictSummary>,
    pub superseded: Vec<canon::SupersededRecord>,
    /// Estado estructural derivado de las relaciones que los Records recién
    /// escritos explican — informativo: una relación `orphaned` o `unknown`
    /// al capturar se reporta, nunca bloquea ni borra la explicación.
    pub relationships: Vec<relationships::RelationshipAssessment>,
    /// Hechos observados del cambio — informan, nunca escriben memoria.
    pub signals: Vec<signals::Signal>,
    pub capture: capture::MechanicalCapture,
    pub warnings: Vec<String>,
    pub diagnostics: Vec<String>,
}

/// El cuerpo de `finalize_change`: captura mecánica del cambio (hechos
/// verificables) + gate de candidatos del canon autónomo. Un diff por sí
/// solo nunca produce memoria durable: sin candidatos, la respuesta es un
/// no-op honesto con los hechos capturados.
pub fn finalize(
    req: FinalizeRequest,
    provider: &mut ProviderHandle,
    recorder: &activity::Recorder,
) -> Result<FinalizeOutcome, String> {
    let mut diagnostics = Vec::new();
    let config = configuration::load(&req.project_root).map_err(|e| e.to_string())?;

    if let Some(spec) = &req.target_spec {
        match project::resolve_target(&req.repo_path, spec) {
            Ok(target) => diagnostics.push(format!("target declarado: {}", target.path.display())),
            Err(e) => diagnostics.push(format!("advertencia: target declarado no resuelto: {e}")),
        }
    }

    let snap = revision::snapshot(&req.repo_path);
    let local_dir = configuration::find_rationale_local(&config.project_root);
    let mut scope = activity::Scope::new(
        &local_dir,
        &config.project_root,
        &config.project_id,
        &req.actor,
    );
    scope.operation_id = req.operation_id.clone();
    // Un id desconocido (otra máquina, snapshot podado) no invalida el
    // cierre: solo no se enlaza con la operación.
    let operation = req.operation_id.as_deref().and_then(|id| {
        let found = operations::load(&local_dir, id);
        if found.is_none() && recorder.is_enabled() {
            diagnostics.push(format!(
                "advertencia: operación '{id}' desconocida; el cierre no se enlaza con ella"
            ));
        }
        found
    });
    if req.operation_id.is_some() && !recorder.is_enabled() {
        diagnostics.push(
            "actividad local desactivada (RATIONALE_ACTIVITY=off): la operación no tiene \
             snapshot; la base del diff es la declarada o HEAD"
                .to_string(),
        );
    }
    // Base honesta del diff: la declarada, o el HEAD que vio prepare_change
    // (cubre commits del agente entre prepare y finalize), o HEAD.
    let base_revision = req
        .base_revision
        .clone()
        .or_else(|| operation.as_ref().and_then(|op| op.base_revision.clone()))
        .or_else(|| snap.head.clone())
        .unwrap_or_default();

    // Excluir `.rationale/`, `.rationale-local/` y los archivos que
    // `install-agent` administra: ni el canon que esta llamada escribe ni el
    // bookkeeping de agentes son parte del cambio del usuario (defecto real
    // de dogfood: Records atados a `AGENTS.md` y `.mcp.json`).
    let mut exclude_prefixes: Vec<&str> = vec![".rationale/", ".rationale-local/"];
    exclude_prefixes.extend(crate::agents::managed_paths());
    let mechanical = capture::capture(&req.repo_path, &base_revision, &exclude_prefixes, provider);
    recorder.emit(
        &scope,
        "change.finalized",
        activity::payload::change_finalized(req.candidates.len(), mechanical.changed_files.len()),
    );
    for (index, candidate) in req.candidates.iter().enumerate() {
        let kind = candidate
            .as_ref()
            .ok()
            .map(|candidate| candidate.kind.as_str());
        recorder.emit(
            &scope,
            "capture.candidate",
            activity::payload::capture_candidate(index, kind),
        );
    }

    let mut signal_set: std::collections::HashSet<signals::Signal> =
        signals::signals_from_paths(&mechanical.changed_files)
            .into_iter()
            .collect();
    if let Some(summary) = &req.summary {
        signal_set.extend(signals::signals_from_text(summary));
    }
    for candidate in req.candidates.iter().flatten() {
        signal_set.extend(signals::signals_from_text(&candidate.statement));
    }
    let mut signal_list: Vec<signals::Signal> = signal_set.into_iter().collect();
    signal_list.sort_by_key(|signal| format!("{signal:?}"));

    let uncommitted_paths = mechanical
        .changed_files
        .iter()
        .filter(|file| file.origin != capture::ChangeOrigin::Committed)
        .map(|file| file.path.clone())
        .collect();
    let ctx = canon::CanonContext {
        rationale_dir: &config.rationale_dir,
        project_id: &config.project_id,
        repo_path: &req.repo_path,
        local_dir: &local_dir,
        head_revision: snap.head.clone(),
        uncommitted_paths,
        actor: canon::ActorContext {
            operation_id: req.operation_id.clone(),
            ..req.actor.clone()
        },
    };

    if req.candidates.is_empty() {
        diagnostics.push(
            "sin candidatos: el cambio queda registrado solo como hechos mecánicos (Git ya \
             guarda el diff); ninguna memoria durable nace del diff por sí solo"
                .to_string(),
        );
    }
    if let Err(e) = activity::ensure_excluded(&config.project_root) {
        diagnostics.push(format!(
            "advertencia: no se pudo excluir .rationale-local/ de Git: {e}"
        ));
    }
    let mut outcome = canon::capture_candidates(&ctx, req.candidates, provider);
    let committed_records: Vec<Record> = outcome
        .committed
        .iter()
        .filter_map(|committed| storage::read_record(Path::new(&committed.path)).ok())
        .filter(|record| !record.relationship_bindings.is_empty())
        .collect();
    let relationship_states = relationships::assess_records(
        provider,
        req.repo_path.to_str().unwrap_or(""),
        &config.project_id,
        &committed_records.iter().collect::<Vec<_>>(),
    );
    if let Some(statement) = req.legacy_statement {
        outcome.discarded.push(canon::DiscardedCandidate {
            index: 0,
            reason: canon::DiscardReason::LegacyContract,
            detail: "finalize_change ya no crea propuestas pendientes: envía `candidates` con \
                     kind, statement, rationale, durability y bindings"
                .to_string(),
            duplicate_of: None,
            statement: canon::sanitize_control_chars(&statement),
        });
    }

    for discarded in &outcome.discarded {
        recorder.emit(
            &scope,
            "capture.discarded",
            activity::payload::capture_discarded(discarded),
        );
    }
    for committed in &outcome.committed {
        recorder.emit(
            &scope,
            "record.committed",
            activity::payload::record_committed(committed),
        );
    }
    for superseded in &outcome.superseded {
        recorder.emit(
            &scope,
            "record.superseded",
            activity::payload::record_superseded(superseded),
        );
    }
    for conflict in &outcome.conflicts {
        recorder.emit(
            &scope,
            "conflict.detected",
            activity::payload::conflict_detected(conflict),
        );
    }
    for assessment in &relationship_states {
        if let Some(kind) = activity::relationship_event_kind(assessment.state) {
            recorder.emit(
                &scope,
                kind,
                activity::payload::relationship_state(assessment),
            );
        }
    }

    if let Some(operation) = &operation {
        let summary = operations::FinalizeSummary {
            finalized_at: evaluation::now_rfc3339_millis(),
            committed: outcome.committed.iter().map(|c| c.id.clone()).collect(),
            discarded: outcome.discarded.len(),
            conflicts: outcome
                .conflicts
                .iter()
                .map(|c| c.conflict_id.clone())
                .collect(),
        };
        if let Err(e) = operations::record_finalize(&local_dir, &operation.operation_id, summary) {
            diagnostics.push(format!("advertencia: {e}"));
        }
    }

    Ok(FinalizeOutcome {
        operation_id: req.operation_id,
        summary: FinalizeSummary {
            committed: outcome.committed.len(),
            discarded: outcome.discarded.len(),
            conflicts: outcome.conflicts.len(),
        },
        committed: outcome.committed,
        discarded: outcome.discarded,
        conflicts: outcome.conflicts,
        superseded: outcome.superseded,
        relationships: relationship_states,
        signals: signal_list,
        capture: mechanical,
        warnings: outcome.warnings,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Revisión adversarial de Fase F, hallazgo 3: un texto con secuencias
    /// de escape ANSI puede pintar un banner falso o borrar texto en el
    /// terminal de quien lo lea. Quitar el byte ESC (y otros caracteres de
    /// control) neutraliza la secuencia completa.
    #[test]
    fn sanitize_control_chars_strips_ansi_escape_but_preserves_newlines_and_tabs() {
        let malicious =
            "Staff must never receive global super_admin.\x1b[2K\r\x1b[32mAUTO-APPROVED\x1b[0m";
        let sanitized = canon::sanitize_control_chars(malicious);
        assert!(
            !sanitized.contains('\x1b'),
            "el byte ESC nunca debe sobrevivir"
        );
        assert!(!sanitized.contains('\r'));
        assert!(sanitized.contains("Staff must never receive global super_admin."));
        assert!(
            sanitized.contains("AUTO-APPROVED"),
            "el texto en sí no se borra, solo los códigos de control"
        );

        let multiline = "line one\n\tindented line two";
        assert_eq!(
            canon::sanitize_control_chars(multiline),
            multiline,
            "\\n y \\t deben preservarse — no son el vector de ataque"
        );
    }
}
