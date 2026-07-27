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
    assessment, binding_match, cache, capture, configuration, evaluation, project, retrieval,
    revision, signals, storage, subjects,
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
    /// Clasificación libre del Subject (`.rationale/subjects/*.yaml`
    /// `type:`, sin enum — ver `subject.schema.json`). `None` se
    /// materializa como `"unclassified"`: un marcador explícito de "el
    /// proponente no lo clasificó", nunca una adivinanza.
    pub subject_type: Option<String>,
    /// Requerido por v0.5 §294 cuando el Subject Resolver sugiere un
    /// candidato fuerte (`Alias`/`MergeCandidate`) pero el caller insiste en
    /// que es un concepto nuevo. Sin esto, una propuesta contra un
    /// candidato fuerte se bloquea (ver `FinalizeOutcome::blocked_reason`).
    pub novelty_reason: Option<subjects::NoveltyReason>,
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

/// Elimina bytes de control (incluido ESC, 0x1b — la base de toda
/// secuencia de escape ANSI) de texto que viene de un cliente MCP no
/// confiable y que se va a persistir Y a mostrar más tarde en el terminal
/// del revisor humano (`rationale review`, Fase F6). Preserva `\n`/`\t`
/// para no romper texto multilínea legítimo. Revisión adversarial de
/// Fase F, hallazgo 3: sin esto, un `intent`/`statement`/riesgo con
/// secuencias ANSI puede pintar un banner falso ("AUTO-APPROVED") u
/// ocultar texto exactamente en el momento en que el humano decide
/// aprobar — el único control real contra autoaprobación en todo el
/// sistema. Quitar solo el byte ESC ya neutraliza la secuencia completa
/// (el resto queda como texto literal inofensivo).
fn sanitize_control_chars(text: &str) -> String {
    text.chars()
        .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
        .collect()
}

/// Rechazo temprano, antes de que exista ningún dato mecánico real que
/// mostrar (validación de `record_id`/`subject_id`, que ocurre antes de
/// tocar Git). Evita repetir el `MechanicalCapture` vacío en cada punto de
/// rechazo temprano.
fn rejected_before_capture(diagnostics: Vec<String>, blocked_reason: String) -> FinalizeOutcome {
    FinalizeOutcome {
        level: signals::CaptureLevel::GitOnly,
        signals: vec![],
        capture: capture::MechanicalCapture {
            base_revision: None,
            final_revision: None,
            working_tree_dirty: false,
            changed_files: vec![],
            committed_file_count: 0,
            uncommitted_file_count: 0,
            verifiability: capture::Verifiability::NoChanges,
            provider_status: "unavailable".to_string(),
            provider_coverage: "unknown".to_string(),
        },
        subject_resolution: None,
        proposal_written: false,
        proposal_path: None,
        proposal_id: None,
        blocked_reason: Some(blocked_reason),
        diagnostics,
    }
}

/// El cuerpo de `finalize_change`: captura mecánica → señales → nivel →
/// resolución de Subject → escritura de la propuesta (nunca del Record
/// aprobado — eso es `rationale review`, Fase F6). Sigue el flujo de
/// `Arquitectura §13.3` hasta "Write canonical files atomically", pero
/// escribe en `.rationale/proposals/`, no en `.rationale/records/`.
pub fn finalize(
    req: &FinalizeRequest,
    provider: &mut ProviderHandle,
) -> Result<FinalizeOutcome, String> {
    let mut diagnostics = Vec::new();

    // Validar ANTES de tocar disco o Git: `record_id` viene de un cliente
    // MCP no confiable y se convierte en un nombre de archivo real más
    // abajo — sin esta validación, un `record_id` como
    // "../../../../etc/pwned" escribe fuera de `.rationale/` (path
    // traversal real, confirmado empíricamente y corregido aquí).
    if let Err(e) = storage::validate_safe_id(&req.record_id) {
        diagnostics.push(format!("record_id rechazado: {e}"));
        return Ok(rejected_before_capture(
            diagnostics,
            format!("record_id inseguro: {e}"),
        ));
    }
    // Mismo agujero, mismo fix: `subject_id` se convierte en nombre de
    // archivo real en `.rationale/subjects/` en cuanto se materializa más
    // abajo (defecto real: antes solo `record_id` se validaba aquí).
    if let Err(e) = storage::validate_safe_id(&req.subject_id) {
        diagnostics.push(format!("subject_id rechazado: {e}"));
        return Ok(rejected_before_capture(
            diagnostics,
            format!("subject_id inseguro: {e}"),
        ));
    }

    let config = configuration::load(&req.project_root).map_err(|e| e.to_string())?;

    // Resolver el target principal es solo diagnóstico aquí — nunca decide
    // qué se captura (eso lo hace el diff mecánico completo); sirve para
    // que quien lea la propuesta vea contra qué target se declaró el cambio.
    let declared_target = project::resolve_target(&req.repo_path, &req.target_spec);
    match &declared_target {
        Ok(t) => diagnostics.push(format!("target declarado: {}", t.path.display())),
        Err(e) => diagnostics.push(format!("advertencia: target declarado no resuelto: {e}")),
    }
    let declared_symbol = declared_target.as_ref().ok().and_then(|t| t.symbol.clone());

    // Excluir `.rationale/` del escaneo de untracked/staged — sin esto,
    // las propuestas que esta misma llamada está a punto de escribir en
    // `.rationale/proposals/` se atarían a sí mismas como si fueran parte
    // del cambio que las originó.
    let mechanical = capture::capture(
        &req.repo_path,
        &req.base_revision,
        &[".rationale/"],
        provider,
    );

    // Guarda de "nada que capturar" — distinta de Nivel 0 (Git-only):
    // Nivel 0 significa "hay cambios, pero son mecánicos y no ameritan un
    // Record"; esto significa "no hay ningún cambio en absoluto". Antes de
    // capturar staged/unstaged/untracked (Fase 1.3), un diff vacío
    // producía `changed_files: []` pero `determine_level` solo trata
    // Nivel 0 cuando `!changed_files.is_empty()` — con la lista vacía, esa
    // condición es falsa y el flujo caía a `Intent`, escribiendo feliz una
    // propuesta con `binding_declarations: []`. Un `blocked_reason`
    // explícito aquí lo cierra en la raíz.
    if mechanical.changed_files.is_empty() {
        diagnostics.push(
            "no hay ningún cambio (commiteado, staged, unstaged ni untracked) desde \
             base_revision — no hay nada que enlazar"
                .to_string(),
        );
        return Ok(FinalizeOutcome {
            level: signals::CaptureLevel::GitOnly,
            signals: vec![],
            capture: mechanical,
            subject_resolution: None,
            proposal_written: false,
            proposal_path: None,
            proposal_id: None,
            blocked_reason: Some(
                "no hay ningún cambio desde base_revision — no hay nada que enlazar".to_string(),
            ),
            diagnostics,
        });
    }

    // Vínculo de símbolo: SOLO si el proveedor estructural confirmó que el
    // símbolo existe — nunca se sintetiza un `structural_id` concatenando
    // el spec del target a mano. Un `structural_id` hecho a mano es
    // indistinguible, para un lector futuro, de uno que el proveedor
    // confirmó de verdad — eso es exactamente el tipo de inferencia
    // disfrazada de hecho que `policy.no-inferred-blocks` prohíbe. Si el
    // proveedor no está disponible, degrada en silencio a "sin binding de
    // símbolo": el binding de archivo (abajo) sigue gobernando el target
    // igual, gracias a la propagación archivo→símbolo de `binding_match`.
    let resolved_symbol: Option<crate::providers::ResolvedTarget> =
        match (&declared_symbol, &mut *provider) {
            (Some(sym), ProviderHandle::Live(client)) => {
                let result = client.resolve_target(req.repo_path.to_str().unwrap_or(""), sym);
                if result.data.is_none() {
                    diagnostics.push(format!(
                        "el proveedor no confirmó el símbolo '{sym}' — solo se enlaza el archivo"
                    ));
                }
                result.data
            }
            (Some(_), ProviderHandle::Unavailable(_)) => {
                diagnostics.push(
                    "proveedor no disponible — solo se enlaza el archivo, sin binding de símbolo"
                        .to_string(),
                );
                None
            }
            (None, _) => None,
        };

    let mut all_signals: std::collections::HashSet<signals::Signal> =
        signals::signals_from_paths(&mechanical.changed_files)
            .into_iter()
            .collect();
    all_signals.extend(signals::signals_from_text(&req.intent));
    all_signals.extend(signals::signals_from_text(&req.statement));
    let signal_list: Vec<signals::Signal> = all_signals.into_iter().collect();

    let level = signals::determine_level(&mechanical.changed_files, &signal_list, &req.severity);

    if level == signals::CaptureLevel::GitOnly {
        diagnostics.push(
            "Nivel 0 (Git-only): cambios mecánicos sin señales de alto valor — no se crea \
             ninguna propuesta (v0.5 §16)."
                .to_string(),
        );
        return Ok(FinalizeOutcome {
            level,
            signals: signal_list,
            capture: mechanical,
            subject_resolution: None,
            proposal_written: false,
            proposal_path: None,
            proposal_id: None,
            blocked_reason: Some("nivel Git-only: nada que capturar".to_string()),
            diagnostics,
        });
    }

    // Resolver el Subject contra el canon real (Fase F4) antes de escribir
    // nada — nunca se crea un Subject nuevo a ciegas.
    //
    // Revisión adversarial de Fase F, hallazgo 1: un solo archivo corrupto
    // en subjects/ o records/ apagaba el Resolver completo en silencio
    // (`.unwrap_or_default()` sobre un `Err` que antes abortaba TODA la
    // lectura). Ahora `list_*_detailed` nunca aborta por un archivo — y
    // cualquier archivo saltado se reporta explícitamente aquí, igual que
    // ya hace `prepare` (Arquitectura §27: "no ocultar cobertura parcial").
    let subjects_dir = config.rationale_dir.join("subjects");
    let records_dir = config.rationale_dir.join("records");

    let subjects_result = subjects::list_subjects_detailed(&subjects_dir).unwrap_or_else(|e| {
        diagnostics.push(format!(
            "advertencia: no se pudo leer el directorio de Subjects: {e}"
        ));
        subjects::SubjectListResult {
            subjects: vec![],
            skipped: vec![],
        }
    });
    for (path, e) in &subjects_result.skipped {
        diagnostics.push(format!(
            "advertencia: Subject no leído, el Resolver no lo considerará como candidato: {} ({e})",
            path.display()
        ));
    }
    let existing_subjects = subjects_result.subjects;

    let records_result = storage::list_records_detailed(&records_dir).unwrap_or_else(|e| {
        diagnostics.push(format!(
            "advertencia: no se pudo leer el directorio de Records: {e}"
        ));
        storage::RecordListResult {
            records: vec![],
            skipped: vec![],
        }
    });
    for (path, e) in &records_result.skipped {
        diagnostics.push(format!(
            "advertencia: Record no leído, no contará para overlap de bindings: {} ({e})",
            path.display()
        ));
    }
    let existing_records = records_result.records;

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

    let mut resolution = subjects::resolve(
        &existing_subjects,
        &req.subject_id,
        &req.subject_title,
        "project",
        &proposed_bindings,
        &existing_bindings,
    );

    if let Some(reason) = &req.novelty_reason {
        if let Err(error) = subjects::validate_novelty_reason(reason, &resolution.candidates) {
            diagnostics.push(format!("novelty_reason rechazada: {error}"));
            return Ok(FinalizeOutcome {
                level,
                signals: signal_list,
                capture: mechanical,
                subject_resolution: Some(resolution),
                proposal_written: false,
                proposal_path: None,
                proposal_id: None,
                blocked_reason: Some(format!("novelty_reason inválida: {error}")),
                diagnostics,
            });
        }
    }

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
        return Ok(FinalizeOutcome {
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
        });
    }

    if let Some(reason) = &req.novelty_reason {
        resolution.novelty_reason = Some(reason.clone());
    }

    // Si el resolver sugiere reusar (o el caller no dio novelty_reason para
    // anular Alias/MergeCandidate — ya cubierto arriba), el Record propuesto
    // referencia el Subject EXISTENTE, no uno nuevo.
    let final_subject_id = match &resolution.selected_subject {
        Some(existing_id) if req.novelty_reason.is_none() => existing_id.clone(),
        _ => req.subject_id.clone(),
    };

    // Siempre un binding de archivo por cada archivo cambiado — verificable
    // sin el proveedor, por diseño. `provisional` refleja la procedencia
    // real (Fase 1.3): un archivo `Committed` es verificable por
    // cualquiera con el mismo repo; cualquier otra procedencia no lo es
    // todavía, y el Record lo declara en vez de fingir la misma certeza.
    let mut binding_declarations: Vec<BindingDeclaration> = mechanical
        .changed_files
        .iter()
        .enumerate()
        .map(|(i, f)| BindingDeclaration {
            id: format!("binding.{}.{i}", req.record_id),
            kind: "file".to_string(),
            provider: None,
            structural_id: None,
            path_hint: Some(f.path.clone()),
            provisional: f.origin != capture::ChangeOrigin::Committed,
            extra: yaml_serde::Mapping::new(),
        })
        .collect();

    // Binding de símbolo adicional — solo cuando el proveedor confirmó el
    // símbolo (arriba). `structural_id` es el `qualified_name` real que el
    // proveedor devolvió, nunca una concatenación hecha a mano. El
    // `path_hint` (repo-relativo) acompaña al símbolo para que
    // `binding_match` pueda exigir que ambos coincidan con el target
    // — sin esto, un símbolo homónimo en otro archivo podría matchear.
    if let Some(resolved) = &resolved_symbol {
        let rel_path = binding_match::target_rel_path(
            &req.repo_path,
            &project::Target {
                path: PathBuf::from(&resolved.file_path),
                symbol: declared_symbol.clone(),
            },
        );
        let provisional = rel_path
            .as_ref()
            .and_then(|p| mechanical.changed_files.iter().find(|f| &f.path == p))
            .map(|f| f.origin != capture::ChangeOrigin::Committed)
            .unwrap_or(false);
        binding_declarations.push(BindingDeclaration {
            id: format!("binding.{}.symbol", req.record_id),
            kind: "symbol".to_string(),
            provider: Some("codebase-memory".to_string()),
            structural_id: Some(resolved.qualified_name.clone()),
            path_hint: rel_path,
            provisional,
            extra: yaml_serde::Mapping::new(),
        });
    }

    // Materializar el Subject AHORA, no al aprobar: un Subject es
    // identidad conceptual, no una aprobación normativa (tiene su propio
    // `review.status: unreviewed`, ortogonal a `Record.approvals` —
    // `authority_label` nunca lo lee). Hacerlo en `approve` en vez de aquí
    // dejaría al Subject Resolver ciego ante Subjects recién propuestos en
    // la MISMA sesión, porque `existing_bindings` (arriba) solo mira
    // `records/`, no `proposals/` — dos agentes seguidos propondrían el
    // mismo concepto con ids distintos de forma fiable. Nunca sobrescribe
    // uno existente (`materialize_proposed`), así que llamarlo también
    // para el caso `Reuse` es un no-op seguro, no un riesgo.
    let subjects_dir = config.rationale_dir.join("subjects");
    let applies_to: Vec<String> = binding_declarations
        .iter()
        .filter_map(|b| b.path_hint.clone())
        .collect();
    match subjects::materialize_proposed(
        &subjects_dir,
        &final_subject_id,
        &req.subject_title,
        req.subject_type.as_deref().unwrap_or(""),
        "project",
        &applies_to,
        "mcp-client",
        &evaluation::now_iso8601(),
    ) {
        Ok(subjects::MaterializeOutcome::Created(path)) => {
            diagnostics.push(format!(
                "Subject materializado en {} (review.status=unreviewed)",
                path.display()
            ));
        }
        Ok(subjects::MaterializeOutcome::AlreadyExists(_)) => {}
        Err(e) => diagnostics.push(format!(
            "advertencia: no se pudo materializar el Subject '{final_subject_id}': {e}"
        )),
    }

    let risks: Vec<Risk> = req
        .risks
        .iter()
        .enumerate()
        .map(|(i, statement)| Risk {
            id: format!("risk.{}.{i}", req.record_id),
            statement: sanitize_control_chars(statement),
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
    if let Some(reason) = &req.novelty_reason {
        let mut novelty = yaml_serde::Mapping::new();
        novelty.insert(
            yaml_serde::Value::String("contrasted_subject".to_string()),
            yaml_serde::Value::String(reason.contrasted_subject.clone()),
        );
        novelty.insert(
            yaml_serde::Value::String("difference_kind".to_string()),
            yaml_serde::Value::String(
                serde_json::to_string(&reason.difference_kind)
                    .unwrap_or_else(|_| "invariant".to_string())
                    .trim_matches('"')
                    .to_string(),
            ),
        );
        novelty.insert(
            yaml_serde::Value::String("difference".to_string()),
            yaml_serde::Value::String(reason.difference.clone()),
        );
        novelty.insert(
            yaml_serde::Value::String("evidence".to_string()),
            yaml_serde::Value::String(reason.evidence.clone()),
        );
        extra.insert(
            yaml_serde::Value::String("novelty_reason".to_string()),
            yaml_serde::Value::Mapping(novelty),
        );
    }

    let proposal = Record {
        id: req.record_id.clone(),
        kind: "constraint".to_string(),
        severity: req.severity.clone(),
        statement: sanitize_control_chars(&req.statement),
        rationale: Some(sanitize_control_chars(&req.intent)),
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
            Ok(FinalizeOutcome {
                level,
                signals: signal_list,
                capture: mechanical,
                subject_resolution: Some(resolution),
                proposal_written: true,
                proposal_path: Some(proposal_path.display().to_string()),
                proposal_id: Some(req.record_id.clone()),
                blocked_reason: None,
                diagnostics,
            })
        }
        Err(e) => {
            diagnostics.push(format!("error escribiendo la propuesta: {e}"));
            Ok(FinalizeOutcome {
                level,
                signals: signal_list,
                capture: mechanical,
                subject_resolution: Some(resolution),
                proposal_written: false,
                proposal_path: None,
                proposal_id: None,
                blocked_reason: Some(format!("error de escritura: {e}")),
                diagnostics,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Revisión adversarial de Fase F, hallazgo 3: un `intent`/`statement`
    /// con secuencias de escape ANSI puede pintar un banner falso o borrar
    /// texto en el terminal del revisor humano. Quitar el byte ESC (y otros
    /// caracteres de control) neutraliza la secuencia completa.
    #[test]
    fn sanitize_control_chars_strips_ansi_escape_but_preserves_newlines_and_tabs() {
        let malicious =
            "Staff must never receive global super_admin.\x1b[2K\r\x1b[32mAUTO-APPROVED\x1b[0m";
        let sanitized = sanitize_control_chars(malicious);
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
            sanitize_control_chars(multiline),
            multiline,
            "\\n y \\t deben preservarse — no son el vector de ataque"
        );
    }
}
