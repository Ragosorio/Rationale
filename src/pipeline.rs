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
use crate::storage::Record;
use crate::{
    assessment, binding_match, cache, canon, capture, configuration, project, retrieval, revision,
    signals, storage, subjects,
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
    /// `None` es válido para un proyecto recién inicializado sin Records:
    /// el packet puede informar salud/target/proveedor sin inventar una
    /// decisión ni panicar por un canon vacío.
    pub assessment: Option<assessment::Assessment>,
    /// Lo que antes eran `eprintln!` sueltos — cada caller decide destino.
    pub diagnostics: Vec<String>,
    pub latency_ms: u128,
}

/// Compila el packet completo para un target — el cuerpo de `prepare_change`.
/// Movido tal cual desde `cmd_prepare` (Fase D/E4): misma lógica, mismo
/// orden de pasos, solo que los `eprintln!` se acumulan en `diagnostics` en
/// vez de escribirse directamente.
pub fn prepare(
    req: &PrepareRequest,
    provider: &mut ProviderHandle,
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

    // 3. Consultar Codebase Memory — sesión MCP persistente real (ADR-0002).
    // La sesión (`provider`) ya fue spawneada por el caller: la CLI una vez
    // por invocación, el servidor MCP una sola vez para toda su vida.
    let (provider_status, provider_coverage, resolved_target, provider_warnings) = match provider {
        ProviderHandle::Live(client) => {
            let sym = symbol.clone().unwrap_or_default();
            let file = target
                .as_ref()
                .ok()
                .and_then(|target| target.path.strip_prefix(&req.repo_path).ok())
                .and_then(|path| path.to_str())
                .unwrap_or("");
            let result = client.resolve_target(req.repo_path.to_str().unwrap_or(""), file, &sym);
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

    // 5. + 6. Compilar el packet compacto — niveles de prioridad y budget
    // reales (Fase E4), no una sola constraint fija. `governing_by_kind`
    // le dice a retrieval qué Records gobiernan el target (y con qué
    // especificidad) para que nunca los trunque ni los oculte por
    // severidad — el mismo conjunto que ya calculamos arriba para elegir
    // `record`/`assessment`.
    let governing_by_kind: std::collections::HashMap<String, binding_match::MatchKind> =
        governing_matches
            .iter()
            .map(|m| (m.record.id.clone(), m.kind))
            .collect();
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
        &governing_by_kind,
    );

    Ok(PrepareOutcome {
        packet,
        assessment: computed_assessment,
        diagnostics,
        latency_ms: t0.elapsed().as_millis(),
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
    let base_revision = req
        .base_revision
        .clone()
        .or_else(|| snap.head.clone())
        .unwrap_or_default();

    // Excluir `.rationale/`, `.rationale-local/` y los archivos que
    // `install-agent` administra: ni el canon que esta llamada escribe ni el
    // bookkeeping de agentes son parte del cambio del usuario (defecto real
    // de dogfood: Records atados a `AGENTS.md` y `.mcp.json`).
    let mut exclude_prefixes: Vec<&str> = vec![".rationale/", ".rationale-local/"];
    exclude_prefixes.extend(crate::agents::managed_paths());
    let mechanical = capture::capture(&req.repo_path, &base_revision, &exclude_prefixes, provider);

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
    let local_dir = configuration::find_rationale_local(&config.project_root);
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
    let mut outcome = canon::capture_candidates(&ctx, req.candidates, provider);
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
