//! canon — mantenimiento autónomo del canon (vNext).
//!
//! Antes de vNext, `finalize_change` escribía una propuesta pendiente y solo
//! `rationale review` la convertía en Record. Esa cola hacía que el trabajo
//! normal de un agente produjera trabajo humano: decenas de propuestas
//! esperando aprobación, la mayoría ruido mecánico.
//!
//! vNext invierte la regla: Rationale se mantiene solo y el humano fija
//! (`pinned`) o corrige. El flujo es mecánico y auditable:
//!
//! ```text
//! candidato → gate → descartado | duplicado | Record canónico | conflicto
//! ```
//!
//! Solo el conflicto interrumpe a una persona: un candidato que intenta
//! reemplazar un Record `pinned` activo. Todo lo demás es determinista y
//! siempre se reporta — un descarte nunca es silencioso.
//!
//! Lo que este módulo NO hace: inferir contradicciones semánticas (un
//! solapamiento léxico "opuesto" con un Record fijado es una advertencia,
//! nunca un bloqueo), ordenar SHAs de Git, ni otorgar `pinned` por cuenta de
//! un agente.

use crate::providers::ProviderHandle;
use crate::storage::{
    self, BindingDeclaration, EpistemicStatus, Evidence, Provenance, ProvenanceKind, Record,
    RecordAuthority, RecordSubjectRef, Risk,
};
use crate::{binding_match, evaluation, project, retrieval, subjects};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const VALID_KINDS: [&str; 4] = ["constraint", "decision", "risk", "exception"];
const CONFLICT_SCHEMA: &str = "rationale/conflict/1";
const MIN_TEXT_CHARS: usize = 16;
const MIN_TEXT_WORDS: usize = 3;

// ---------------------------------------------------------------------------
// Contrato de entrada
// ---------------------------------------------------------------------------

/// Un binding declarado por el agente: `"src/x.rs::symbol"` o
/// `{ "path": "src/x.rs", "symbol": "symbol" }`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CandidateBinding {
    Spec(String),
    Structured {
        path: String,
        #[serde(default)]
        symbol: Option<String>,
    },
}

impl CandidateBinding {
    fn spec(&self) -> String {
        match self {
            Self::Spec(spec) => spec.clone(),
            Self::Structured {
                path,
                symbol: Some(symbol),
            } => format!("{path}::{symbol}"),
            Self::Structured { path, symbol: None } => path.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CandidateEvidence {
    #[serde(rename = "type", default = "default_evidence_type")]
    pub evidence_type: String,
    pub path: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_evidence_type() -> String {
    "reference".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CandidateSubject {
    pub id: String,
    pub title: String,
    #[serde(rename = "type", default)]
    pub subject_type: Option<String>,
    #[serde(default)]
    pub novelty_reason: Option<subjects::NoveltyReason>,
}

/// Conocimiento que el agente cree durable, estructurado por él mismo: el
/// agente sabe lo que acaba de hacer, así que el gate valida forma y
/// verificabilidad en vez de adivinar intención a partir del diff.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Candidate {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub statement: String,
    #[serde(default)]
    pub rationale: String,
    /// `durable` o `transient`. Ausente no se asume: sin declaración
    /// explícita no hay memoria durable.
    #[serde(default)]
    pub durability: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub bindings: Vec<CandidateBinding>,
    #[serde(default)]
    pub supersedes: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<CandidateEvidence>,
    #[serde(default)]
    pub subject: Option<CandidateSubject>,
}

/// Quién está actuando. `client` nunca se inventa: `unknown` cuando ni el
/// flag `--client` ni el propio cliente MCP lo declararon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActorContext {
    pub client: String,
    /// `flag`, `mcp-client-info`, `cli` o `unknown` — de dónde salió `client`.
    pub client_source: String,
    pub session_id: Option<String>,
    pub operation_id: Option<String>,
}

impl ActorContext {
    fn agent_label(&self) -> String {
        format!("agent:{}", self.client)
    }
}

/// Todo lo que el canon necesita saber del entorno, sin depender de cómo se
/// obtuvo (CLI, servidor MCP o test).
pub struct CanonContext<'a> {
    pub rationale_dir: &'a Path,
    pub project_id: &'a str,
    pub repo_path: &'a Path,
    /// `.rationale-local/` — locks y conflictos pendientes (nunca versionado).
    pub local_dir: &'a Path,
    pub head_revision: Option<String>,
    /// Rutas repo-relativas con cambios sin commitear: sus bindings nacen
    /// `provisional` porque un tercero con el mismo repo no puede verificarlos.
    pub uncommitted_paths: HashSet<String>,
    pub actor: ActorContext,
}

// ---------------------------------------------------------------------------
// Contrato de salida
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscardReason {
    MalformedCandidate,
    InvalidKind,
    DurabilityNotDeclared,
    Transient,
    StatementNotMeaningful,
    MissingRationale,
    RationaleRestatesStatement,
    MechanicalNoise,
    InvalidSeverity,
    NoMeaningfulBinding,
    InvalidId,
    IdKindMismatch,
    IdAlreadyExists,
    Duplicate,
    LegacyContract,
    WriteFailed,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscardedCandidate {
    pub index: usize,
    pub reason: DiscardReason,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommittedRecord {
    pub index: usize,
    pub id: String,
    pub kind: String,
    pub statement: String,
    pub authority: String,
    pub path: String,
    pub bindings: Vec<String>,
    pub superseded: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictOption {
    pub decision: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSummary {
    pub conflict_id: String,
    pub pinned_record_id: String,
    pub pinned_statement: String,
    pub candidate_statement: String,
    pub question: String,
    pub options: Vec<ConflictOption>,
}

#[derive(Debug, Default, Serialize)]
pub struct CaptureOutcome {
    pub committed: Vec<CommittedRecord>,
    pub discarded: Vec<DiscardedCandidate>,
    pub conflicts: Vec<ConflictSummary>,
    pub superseded: Vec<SupersededRecord>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupersededRecord {
    pub id: String,
    pub superseded_by: String,
}

// ---------------------------------------------------------------------------
// Texto: sanitización, normalización y ruido mecánico
// ---------------------------------------------------------------------------

/// Elimina bytes de control (incluido ESC) de texto de un cliente MCP no
/// confiable que se persiste y luego se muestra en terminales y en la UI.
/// Preserva `\n`/`\t`. Revisión adversarial de Fase F, hallazgo 3: sin esto
/// un statement con secuencias ANSI podía pintar un banner falso.
pub fn sanitize_control_chars(text: &str) -> String {
    text.chars()
        .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
        .collect()
}

/// Forma canónica para comparar afirmaciones: minúsculas, solo alfanuméricos
/// y espacios simples. Dos statements que solo difieren en puntuación o
/// mayúsculas son la misma afirmación.
pub fn normalize_statement(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_meaningful_text(text: &str) -> bool {
    text.trim().chars().count() >= MIN_TEXT_CHARS
        && text.split_whitespace().count() >= MIN_TEXT_WORDS
}

/// Descripciones de un cambio, no conocimiento sobre por qué el código es
/// como es. Solo cuentan como ruido si ni el statement ni el rationale
/// cargan un marcador causal o normativo (`CAUSAL_MARKERS`): el clasificador
/// prefiere dejar pasar ruido a descartar conocimiento real.
const MECHANICAL_PREFIXES: &[&str] = &[
    "added",
    "add",
    "adds",
    "changed",
    "change",
    "changes",
    "updated",
    "update",
    "updates",
    "removed",
    "remove",
    "deleted",
    "delete",
    "renamed",
    "rename",
    "moved",
    "move",
    "reformatted",
    "reformat",
    "formatted",
    "format",
    "bumped",
    "bump",
    "refactored",
    "refactor",
    "cleaned",
    "cleanup",
    "clean",
    "tidy",
    "lint",
    "fixed typo",
    "fix typo",
    "typo",
    "wip",
    "chore",
    "misc",
    "se agregó",
    "se agrego",
    "agregado",
    "agregar",
    "añadido",
    "añadir",
    "cambiado",
    "cambiar",
    "actualizado",
    "actualizar",
    "eliminado",
    "eliminar",
    "renombrado",
    "renombrar",
    "movido",
    "mover",
    "formateado",
    "formatear",
    "refactorizado",
    "limpieza",
];

const CAUSAL_MARKERS: &[&str] = &[
    "because",
    "so that",
    "in order to",
    "to avoid",
    "to prevent",
    "otherwise",
    "must",
    "never",
    "always",
    "should",
    "only",
    "required",
    "requires",
    "invariant",
    "guarantee",
    "guarantees",
    "ensure",
    "ensures",
    "since",
    "due to",
    "without",
    "porque",
    "para que",
    "para evitar",
    "evitar",
    "de lo contrario",
    "debe",
    "deben",
    "nunca",
    "siempre",
    "solo",
    "sólo",
    "requiere",
    "garantiza",
    "invariante",
    "ya que",
    "debido a",
    "sin esto",
    "sin",
];

fn contains_marker(normalized: &str, marker: &str) -> bool {
    let marker = normalize_statement(marker);
    format!(" {normalized} ").contains(&format!(" {marker} "))
}

fn starts_with_mechanical_prefix(normalized: &str) -> bool {
    MECHANICAL_PREFIXES.iter().any(|prefix| {
        let prefix = normalize_statement(prefix);
        normalized == prefix || normalized.starts_with(&format!("{prefix} "))
    })
}

/// `true` cuando el candidato describe un cambio mecánico sin ningún porqué.
pub fn is_mechanical_noise(statement: &str, rationale: &str) -> bool {
    let statement = normalize_statement(statement);
    let rationale = normalize_statement(rationale);
    if !starts_with_mechanical_prefix(&statement) {
        return false;
    }
    !CAUSAL_MARKERS
        .iter()
        .any(|marker| contains_marker(&statement, marker) || contains_marker(&rationale, marker))
}

fn discard(
    index: usize,
    reason: DiscardReason,
    detail: impl Into<String>,
    statement: &str,
) -> DiscardedCandidate {
    DiscardedCandidate {
        index,
        reason,
        detail: detail.into(),
        duplicate_of: None,
        statement: statement.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Gate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PreparedBinding {
    kind: String,
    path_hint: String,
    structural_id: Option<String>,
    provider: Option<String>,
    provisional: bool,
}

/// Un candidato que ya pasó las verificaciones que no dependen del estado
/// del canon (forma, texto, bindings). Serializable porque un conflicto lo
/// conserva hasta que el humano decide.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PreparedCandidate {
    index: usize,
    requested_id: Option<String>,
    kind: String,
    severity: String,
    statement: String,
    rationale: String,
    bindings: Vec<PreparedBinding>,
    supersedes: Vec<String>,
    risks: Vec<String>,
    evidence: Vec<CandidateEvidence>,
    subject: Option<CandidateSubject>,
}

/// Verificaciones puras de forma y texto — sin disco ni proveedor.
fn validate_shape(index: usize, candidate: &Candidate) -> Result<(), DiscardedCandidate> {
    let statement = sanitize_control_chars(candidate.statement.trim());
    let rationale = sanitize_control_chars(candidate.rationale.trim());

    if !VALID_KINDS.contains(&candidate.kind.as_str()) {
        return Err(discard(
            index,
            DiscardReason::InvalidKind,
            format!(
                "kind '{}' inválido — valores válidos: {}",
                candidate.kind,
                VALID_KINDS.join(", ")
            ),
            &statement,
        ));
    }
    match candidate.durability.as_deref() {
        Some("durable") => {}
        Some("transient") => {
            return Err(discard(
                index,
                DiscardReason::Transient,
                "el agente declaró el candidato como transitorio — no se escribe memoria durable",
                &statement,
            ))
        }
        Some(other) => {
            return Err(discard(
                index,
                DiscardReason::DurabilityNotDeclared,
                format!("durability '{other}' inválida — usa 'durable' o 'transient'"),
                &statement,
            ))
        }
        None => {
            return Err(discard(
                index,
                DiscardReason::DurabilityNotDeclared,
                "sin durability explícita no hay memoria durable — declara 'durable' solo si \
                 seguirá siendo cierto después de este cambio",
                &statement,
            ))
        }
    }
    if !is_meaningful_text(&statement) {
        return Err(discard(
            index,
            DiscardReason::StatementNotMeaningful,
            format!(
                "el statement debe afirmar algo verificable (≥{MIN_TEXT_CHARS} caracteres, \
                 ≥{MIN_TEXT_WORDS} palabras)"
            ),
            &statement,
        ));
    }
    if !is_meaningful_text(&rationale) {
        return Err(discard(
            index,
            DiscardReason::MissingRationale,
            "sin un porqué causal no es memoria de Rationale: el rationale explica por qué \
             el statement debe seguir siendo cierto",
            &statement,
        ));
    }
    let normalized_statement = normalize_statement(&statement);
    let normalized_rationale = normalize_statement(&rationale);
    if normalized_rationale == normalized_statement
        || normalized_statement.contains(&normalized_rationale)
    {
        return Err(discard(
            index,
            DiscardReason::RationaleRestatesStatement,
            "el rationale repite el statement en vez de explicar por qué",
            &statement,
        ));
    }
    if is_mechanical_noise(&statement, &rationale) {
        return Err(discard(
            index,
            DiscardReason::MechanicalNoise,
            "describe un cambio mecánico sin ningún porqué — el diff ya lo registra en Git",
            &statement,
        ));
    }
    if let Some(severity) = &candidate.severity {
        if storage::Severity::parse(severity).is_none() {
            return Err(discard(
                index,
                DiscardReason::InvalidSeverity,
                format!(
                    "severity '{severity}' inválida — valores válidos: {}",
                    storage::Severity::ALL.join(", ")
                ),
                &statement,
            ));
        }
    }
    if let Some(id) = &candidate.id {
        if storage::validate_safe_id(id).is_err() || !is_schema_id(id) {
            return Err(discard(
                index,
                DiscardReason::InvalidId,
                format!("id '{id}' inválido — usa '<kind>.<slug-en-minúsculas>'"),
                &statement,
            ));
        }
        if storage::infer_kind_from_id(id) != Some(candidate.kind.as_str()) {
            return Err(discard(
                index,
                DiscardReason::IdKindMismatch,
                format!(
                    "el prefijo de id '{id}' no coincide con kind '{}'",
                    candidate.kind
                ),
                &statement,
            ));
        }
    }
    Ok(())
}

/// `.rationale/schemas/record.schema.json`: `^[a-z][a-z0-9-]*\.[a-z][a-z0-9.-]*$`.
fn is_schema_id(id: &str) -> bool {
    let Some((prefix, rest)) = id.split_once('.') else {
        return false;
    };
    let head_ok = |s: &str| s.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    head_ok(prefix)
        && head_ok(rest)
        && prefix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && rest
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
}

/// Resuelve los bindings contra el árbol de trabajo real y, cuando hay
/// símbolo, contra el proveedor estructural. Un símbolo que el proveedor no
/// confirma nunca se sintetiza (`policy.no-inferred-blocks`): queda solo el
/// binding de archivo y una advertencia.
fn resolve_bindings(
    ctx: &CanonContext,
    bindings: &[CandidateBinding],
    provider: &mut ProviderHandle,
    warnings: &mut Vec<String>,
) -> Vec<PreparedBinding> {
    let mut resolved: Vec<PreparedBinding> = Vec::new();
    for binding in bindings {
        let spec = binding.spec();
        let target = match project::resolve_target(ctx.repo_path, &spec) {
            Ok(target) if target.path.is_file() => target,
            Ok(_) => {
                warnings.push(format!(
                    "binding '{spec}' no apunta a un archivo — ignorado"
                ));
                continue;
            }
            Err(error) => {
                warnings.push(format!("binding '{spec}' ignorado: {error}"));
                continue;
            }
        };
        let Some(rel_path) = binding_match::target_rel_path(ctx.repo_path, &target) else {
            warnings.push(format!(
                "binding '{spec}' no es relativo al repositorio — ignorado"
            ));
            continue;
        };
        // El canon y los datos locales no son código: un Record atado a
        // `.rationale/` se gobernaría a sí mismo.
        if rel_path.starts_with(".rationale/") || rel_path.starts_with(".rationale-local/") {
            warnings.push(format!(
                "binding '{spec}' apunta a datos de Rationale, no a código — ignorado"
            ));
            continue;
        }
        let provisional = ctx.uncommitted_paths.contains(&rel_path);

        if !resolved
            .iter()
            .any(|b| b.kind == "file" && b.path_hint == rel_path)
        {
            resolved.push(PreparedBinding {
                kind: "file".to_string(),
                path_hint: rel_path.clone(),
                structural_id: None,
                provider: None,
                provisional,
            });
        }

        let Some(symbol) = target.symbol.as_deref().filter(|s| !s.is_empty()) else {
            continue;
        };
        let confirmed = provider.as_provider().and_then(|client| {
            client
                .resolve_target(ctx.repo_path.to_str().unwrap_or(""), &rel_path, symbol)
                .data
        });
        match confirmed {
            Some(node) if node_matches_symbol(&node.qualified_name, symbol) => {
                if !resolved
                    .iter()
                    .any(|b| b.structural_id.as_deref() == Some(node.qualified_name.as_str()))
                {
                    resolved.push(PreparedBinding {
                        kind: "symbol".to_string(),
                        path_hint: rel_path.clone(),
                        structural_id: Some(node.qualified_name),
                        provider: Some("codebase-memory".to_string()),
                        provisional,
                    });
                }
            }
            _ => warnings.push(format!(
                "el proveedor no confirmó el símbolo '{symbol}' en '{rel_path}' — solo se enlaza \
                 el archivo"
            )),
        }
    }
    resolved
}

/// El proveedor puede devolver su mejor coincidencia aunque no sea el
/// símbolo pedido; un binding de símbolo solo nace si el nombre coincide en
/// un límite de token.
fn node_matches_symbol(qualified_name: &str, symbol: &str) -> bool {
    let tail = symbol.rsplit([':', '.']).next().unwrap_or(symbol);
    qualified_name == tail
        || qualified_name
            .strip_suffix(tail)
            .is_some_and(|rest| rest.ends_with(['.', ':', '/']))
}

fn prepare_candidate(
    ctx: &CanonContext,
    index: usize,
    candidate: Candidate,
    provider: &mut ProviderHandle,
    warnings: &mut Vec<String>,
) -> Result<PreparedCandidate, DiscardedCandidate> {
    validate_shape(index, &candidate)?;
    let statement = sanitize_control_chars(candidate.statement.trim());
    let bindings = resolve_bindings(ctx, &candidate.bindings, provider, warnings);
    if bindings.is_empty() {
        return Err(discard(
            index,
            DiscardReason::NoMeaningfulBinding,
            "ningún binding resolvió a un archivo real del repositorio — un Record que no se \
             puede anclar a código no puede recuperarse cuando importa",
            &statement,
        ));
    }
    let severity = match candidate.severity {
        Some(severity) => severity,
        None => {
            warnings.push(format!(
                "candidato {index}: sin severity declarada — se escribe 'medium' (ordena, nunca \
                 decide visibilidad)"
            ));
            "medium".to_string()
        }
    };
    Ok(PreparedCandidate {
        index,
        requested_id: candidate.id,
        kind: candidate.kind,
        severity,
        statement,
        rationale: sanitize_control_chars(candidate.rationale.trim()),
        bindings,
        supersedes: candidate
            .supersedes
            .into_iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect(),
        risks: candidate
            .risks
            .iter()
            .map(|risk| sanitize_control_chars(risk.trim()))
            .filter(|risk| !risk.is_empty())
            .collect(),
        evidence: candidate.evidence,
        subject: candidate.subject,
    })
}

// ---------------------------------------------------------------------------
// Lock cooperativo entre procesos
// ---------------------------------------------------------------------------

/// Serializa commits al canon entre procesos (varios agentes pueden correr
/// `rationale serve` a la vez). Vive en `.rationale-local/` para no ensuciar
/// el canon versionado. Un lock más viejo que `STALE_AFTER` pertenece a un
/// proceso muerto: un commit tarda milisegundos.
pub struct CanonLock {
    path: PathBuf,
}

const LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const STALE_AFTER: Duration = Duration::from_secs(30);

impl CanonLock {
    pub fn acquire(local_dir: &Path) -> Result<Self, String> {
        let path = local_dir.join("locks").join("canon.lock");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("no se pudo crear {}: {e}", parent.display()))?;
        }
        let started = Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    use std::io::Write;
                    let _ = writeln!(
                        file,
                        "pid={} at={}",
                        std::process::id(),
                        evaluation::now_iso8601()
                    );
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let stale = std::fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|modified| modified.elapsed().ok())
                        .is_some_and(|age| age > STALE_AFTER);
                    if stale {
                        let _ = std::fs::remove_file(&path);
                        continue;
                    }
                    if started.elapsed() > LOCK_TIMEOUT {
                        return Err(format!(
                            "otro proceso mantiene {} desde hace más de {}s",
                            path.display(),
                            LOCK_TIMEOUT.as_secs()
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => {
                    return Err(format!("no se pudo crear {}: {error}", path.display()));
                }
            }
        }
    }
}

impl Drop for CanonLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Estado derivado del canon
// ---------------------------------------------------------------------------

/// Records reemplazados, derivado de ambos lados de la relación: el
/// `superseded_by` escrito en el Record viejo y el `supersedes` del nuevo.
/// El segundo cubre la ventana entre las dos escrituras si un proceso muere
/// en medio. Un Record `pinned` solo cuenta como reemplazado si él mismo lo
/// declara — un `supersedes` escrito a mano nunca lo desplaza en silencio.
pub fn superseded_ids(records: &[Record]) -> HashSet<String> {
    let by_id: HashMap<&str, &Record> = records.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut superseded: HashSet<String> = records
        .iter()
        .filter(|r| {
            storage::superseded_by(r).is_some()
                || storage::lifecycle_status(r) == Some("superseded")
        })
        .map(|r| r.id.clone())
        .collect();
    for record in records.iter().filter(|r| !storage::is_revoked(r)) {
        for target in &record.supersedes {
            if let Some(existing) = by_id.get(target.as_str()) {
                if storage::record_authority(existing) == RecordAuthority::Normal {
                    superseded.insert(target.clone());
                }
            }
        }
    }
    superseded
}

pub fn is_active(record: &Record, superseded: &HashSet<String>) -> bool {
    !storage::is_revoked(record) && !superseded.contains(&record.id)
}

// ---------------------------------------------------------------------------
// Commit
// ---------------------------------------------------------------------------

fn records_dir(ctx: &CanonContext) -> PathBuf {
    ctx.rationale_dir.join("records")
}

fn record_path(ctx: &CanonContext, id: &str) -> PathBuf {
    records_dir(ctx).join(format!("{id}.yaml"))
}

fn fold_ascii(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        other => other,
    }
}

/// `kind.slug` legible y estable a partir del statement: las primeras
/// palabras significativas, ASCII, acotado. Colisiones se resuelven con
/// sufijo numérico bajo el lock.
fn slug_for(kind: &str, statement: &str) -> String {
    let words: Vec<String> = statement
        .to_lowercase()
        .chars()
        .map(fold_ascii)
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .take(7)
        .map(str::to_string)
        .collect();
    let mut slug = words.join("-");
    while slug.len() > 56 {
        match slug.rfind('-') {
            Some(cut) => slug.truncate(cut),
            None => slug.truncate(56),
        }
    }
    let slug = slug.trim_start_matches(|c: char| !c.is_ascii_lowercase());
    if slug.is_empty() {
        format!("{kind}.record")
    } else {
        format!("{kind}.{slug}")
    }
}

fn unique_id(ctx: &CanonContext, base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) && !record_path(ctx, base).exists() {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken.contains(id) && !record_path(ctx, id).exists())
        .expect("siempre existe un sufijo libre")
}

fn string_value(text: impl Into<String>) -> yaml_serde::Value {
    yaml_serde::Value::String(text.into())
}

fn lifecycle_event(fields: &[(&str, String)]) -> yaml_serde::Value {
    let mut event = yaml_serde::Mapping::new();
    for (key, value) in fields {
        event.insert(string_value(*key), string_value(value.clone()));
    }
    yaml_serde::Value::Mapping(event)
}

fn push_lifecycle_event(record: &mut Record, status: Option<&str>, event: yaml_serde::Value) {
    let lifecycle_key = string_value("lifecycle");
    if !matches!(
        record.extra.get(&lifecycle_key),
        Some(yaml_serde::Value::Mapping(_))
    ) {
        record.extra.insert(
            lifecycle_key.clone(),
            yaml_serde::Value::Mapping(Default::default()),
        );
    }
    let Some(yaml_serde::Value::Mapping(lifecycle)) = record.extra.get_mut(&lifecycle_key) else {
        unreachable!("lifecycle siempre es un mapping aquí");
    };
    if let Some(status) = status {
        lifecycle.insert(string_value("status"), string_value(status));
    }
    let events_key = string_value("events");
    if !matches!(
        lifecycle.get(&events_key),
        Some(yaml_serde::Value::Sequence(_))
    ) {
        lifecycle.insert(events_key.clone(), yaml_serde::Value::Sequence(Vec::new()));
    }
    if let Some(yaml_serde::Value::Sequence(events)) = lifecycle.get_mut(&events_key) {
        events.push(event);
    }
}

fn provenance_for(ctx: &CanonContext, kind: ProvenanceKind) -> Provenance {
    let mut extra = yaml_serde::Mapping::new();
    extra.insert(
        string_value("client"),
        string_value(ctx.actor.client.clone()),
    );
    extra.insert(
        string_value("client_source"),
        string_value(ctx.actor.client_source.clone()),
    );
    if let Some(operation_id) = &ctx.actor.operation_id {
        extra.insert(
            string_value("operation_id"),
            string_value(operation_id.clone()),
        );
    }
    if let Some(session_id) = &ctx.actor.session_id {
        extra.insert(string_value("session_id"), string_value(session_id.clone()));
    }
    extra.insert(
        string_value("asserted_at"),
        string_value(evaluation::now_iso8601()),
    );
    Provenance {
        kind: Some(kind.as_str().to_string()),
        extra,
    }
}

fn build_record(
    ctx: &CanonContext,
    prepared: &PreparedCandidate,
    id: &str,
    authority: RecordAuthority,
    subject_id: Option<String>,
    superseded: &[String],
) -> Record {
    let binding_declarations = prepared
        .bindings
        .iter()
        .enumerate()
        .map(|(i, binding)| BindingDeclaration {
            id: if binding.kind == "symbol" {
                format!("binding.{id}.symbol.{i}")
            } else {
                format!("binding.{id}.{i}")
            },
            kind: binding.kind.clone(),
            provider: binding.provider.clone(),
            structural_id: binding.structural_id.clone(),
            path_hint: Some(binding.path_hint.clone()),
            provisional: binding.provisional,
            extra: yaml_serde::Mapping::new(),
        })
        .collect();
    let risks = prepared
        .risks
        .iter()
        .enumerate()
        .map(|(i, statement)| Risk {
            id: format!("risk.{id}.{i}"),
            statement: statement.clone(),
            epistemic_status: EpistemicStatus::Stated,
            extra: yaml_serde::Mapping::new(),
        })
        .collect();
    let evidence = prepared
        .evidence
        .iter()
        .map(|item| {
            let mut extra = yaml_serde::Mapping::new();
            if let Some(note) = &item.note {
                extra.insert(
                    string_value("note"),
                    string_value(sanitize_control_chars(note)),
                );
            }
            Evidence {
                evidence_type: sanitize_control_chars(&item.evidence_type),
                path: Some(sanitize_control_chars(&item.path)),
                revision: ctx.head_revision.clone(),
                verified: false,
                content_hash: None,
                visibility: Some("repository".to_string()),
                extra,
            }
        })
        .collect();

    let mut extra = yaml_serde::Mapping::new();
    extra.insert(
        string_value("schema_version"),
        string_value("rationale/0.1"),
    );
    extra.insert(string_value("project_id"), string_value(ctx.project_id));
    let mut record = Record {
        id: id.to_string(),
        kind: prepared.kind.clone(),
        severity: prepared.severity.clone(),
        statement: prepared.statement.clone(),
        rationale: Some(prepared.rationale.clone()),
        epistemic_status: EpistemicStatus::Stated,
        authority: Some(authority.as_str().to_string()),
        provenance: Some(provenance_for(ctx, ProvenanceKind::AgentAsserted)),
        supersedes: superseded.to_vec(),
        approvals: vec![],
        binding_declarations,
        evidence,
        risks,
        bound_revision: ctx.head_revision.clone(),
        subject: subject_id.map(|id| RecordSubjectRef {
            id,
            extra: yaml_serde::Mapping::new(),
        }),
        extra,
    };
    let mut event = vec![
        ("type", "asserted".to_string()),
        ("actor", ctx.actor.agent_label()),
        ("timestamp", evaluation::now_iso8601()),
    ];
    if let Some(operation_id) = &ctx.actor.operation_id {
        event.push(("operation_id", operation_id.clone()));
    }
    push_lifecycle_event(&mut record, Some("active"), lifecycle_event(&event));
    record
}

/// Marca un Record como reemplazado. Relee el archivo inmediatamente antes de
/// escribir y aborta si cambió (mismo TOCTOU que `review::mutate_record`).
fn mark_superseded(
    ctx: &CanonContext,
    target_id: &str,
    replacement_id: &str,
    actor_label: &str,
    reason: &str,
) -> Result<(), String> {
    let path = record_path(ctx, target_id);
    let original = std::fs::read_to_string(&path)
        .map_err(|e| format!("no se pudo leer '{target_id}': {e}"))?;
    let mut record = storage::read_record(&path).map_err(|e| e.to_string())?;
    let policy_key = string_value("applicability_policy");
    if !matches!(
        record.extra.get(&policy_key),
        Some(yaml_serde::Value::Mapping(_))
    ) {
        record.extra.insert(
            policy_key.clone(),
            yaml_serde::Value::Mapping(Default::default()),
        );
    }
    if let Some(yaml_serde::Value::Mapping(policy)) = record.extra.get_mut(&policy_key) {
        policy.insert(string_value("superseded_by"), string_value(replacement_id));
    }
    push_lifecycle_event(
        &mut record,
        Some("superseded"),
        lifecycle_event(&[
            ("type", "superseded".to_string()),
            ("actor", actor_label.to_string()),
            ("reason", reason.to_string()),
            ("replacement_id", replacement_id.to_string()),
            ("timestamp", evaluation::now_iso8601()),
        ]),
    );
    let current = std::fs::read_to_string(&path)
        .map_err(|e| format!("no se pudo releer '{target_id}': {e}"))?;
    if current != original {
        return Err(format!(
            "'{target_id}' cambió durante la supersesión; no se sobrescribió"
        ));
    }
    storage::write_record(&path, &record).map_err(|e| e.to_string())
}

fn resolve_subject(
    ctx: &CanonContext,
    prepared: &PreparedCandidate,
    records: &[Record],
    warnings: &mut Vec<String>,
) -> Option<String> {
    let subject = prepared.subject.as_ref()?;
    if storage::validate_safe_id(&subject.id).is_err() {
        warnings.push(format!("subject '{}' inseguro — ignorado", subject.id));
        return None;
    }
    let subjects_dir = ctx.rationale_dir.join("subjects");
    let listing = subjects::list_subjects_detailed(&subjects_dir).unwrap_or_else(|_| {
        subjects::SubjectListResult {
            subjects: vec![],
            skipped: vec![],
        }
    });
    for (path, error) in &listing.skipped {
        warnings.push(format!(
            "Subject ilegible, el resolver no lo considera: {} ({error})",
            path.display()
        ));
    }
    let existing = listing.subjects;
    let mut existing_bindings: HashMap<String, Vec<String>> = HashMap::new();
    for record in records {
        if let Some(subject_ref) = &record.subject {
            existing_bindings
                .entry(subject_ref.id.clone())
                .or_default()
                .extend(
                    record
                        .binding_declarations
                        .iter()
                        .filter_map(|b| b.path_hint.clone()),
                );
        }
    }
    let proposed: Vec<String> = prepared
        .bindings
        .iter()
        .map(|b| b.path_hint.clone())
        .collect();
    let resolution = subjects::resolve(
        &existing,
        &subject.id,
        &subject.title,
        "project",
        &proposed,
        &existing_bindings,
    );
    let novelty_accepted = subject.novelty_reason.as_ref().is_some_and(|reason| {
        subjects::validate_novelty_reason(reason, &resolution.candidates).is_ok()
    });
    // Autónomo: un candidato fuerte se reutiliza en vez de bloquear la
    // captura esperando un `novelty_reason` (v0.5 §9.2 lo exigía para crear
    // un Subject nuevo, no para reutilizar uno existente).
    let selected = match resolution.action {
        subjects::ResolutionAction::Reuse => resolution.selected_subject.clone(),
        subjects::ResolutionAction::Alias | subjects::ResolutionAction::MergeCandidate
            if !novelty_accepted =>
        {
            let reused = resolution.candidates.first().map(|c| c.id.clone());
            if let Some(reused) = &reused {
                warnings.push(format!(
                    "subject '{}' se parece a '{reused}' — se reutiliza el existente",
                    subject.id
                ));
            }
            reused
        }
        _ => None,
    };
    if let Some(selected) = selected {
        return Some(selected);
    }
    match subjects::materialize_proposed(
        &subjects_dir,
        &subject.id,
        &sanitize_control_chars(&subject.title),
        &sanitize_control_chars(subject.subject_type.as_deref().unwrap_or("")),
        "project",
        &proposed,
        &ctx.actor.client,
        &evaluation::now_iso8601(),
    ) {
        Ok(_) => Some(subject.id.clone()),
        Err(error) => {
            warnings.push(format!(
                "no se pudo materializar el subject '{}': {error}",
                subject.id
            ));
            None
        }
    }
}

fn conflict_dir(ctx: &CanonContext) -> PathBuf {
    ctx.local_dir.join("conflicts")
}

fn new_id(prefix: &str) -> String {
    use sha2::{Digest, Sha256};
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let digest = Sha256::digest(format!("{nanos}:{}:{sequence}", std::process::id()));
    let hex: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
    // Prefijo temporal en hex: los ids ordenan por creación sin parsearlos.
    format!("{prefix}_{:x}{hex}", nanos / 1_000_000)
}

pub fn generate_id(prefix: &str) -> String {
    new_id(prefix)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingConflict {
    schema_version: String,
    conflict_id: String,
    created_at: String,
    actor: ActorContext,
    pinned_record_id: String,
    pinned_statement: String,
    candidate: PreparedCandidate,
    summary: ConflictSummary,
}

fn conflict_options() -> Vec<ConflictOption> {
    vec![
        ConflictOption {
            decision: "keep_pinned".to_string(),
            meaning: "la regla fijada sigue gobernando; la nueva afirmación se descarta"
                .to_string(),
        },
        ConflictOption {
            decision: "adopt_new".to_string(),
            meaning: "la nueva afirmación gobierna, hereda la fijación y la regla anterior queda \
                      como historia (requiere autoridad declarada en .rationale/config.yaml)"
                .to_string(),
        },
    ]
}

fn persist_conflict(
    ctx: &CanonContext,
    prepared: &PreparedCandidate,
    pinned: &Record,
) -> Result<ConflictSummary, String> {
    let conflict_id = new_id("conflict");
    let summary = ConflictSummary {
        conflict_id: conflict_id.clone(),
        pinned_record_id: pinned.id.clone(),
        pinned_statement: pinned.statement.clone(),
        candidate_statement: prepared.statement.clone(),
        question: format!(
            "Existe una regla fijada que esta afirmación reemplazaría. ¿Cuál debe gobernar? \
             Fijada ({}): «{}». Nueva: «{}».",
            pinned.id, pinned.statement, prepared.statement
        ),
        options: conflict_options(),
    };
    let pending = PendingConflict {
        schema_version: CONFLICT_SCHEMA.to_string(),
        conflict_id: conflict_id.clone(),
        created_at: evaluation::now_iso8601(),
        actor: ctx.actor.clone(),
        pinned_record_id: pinned.id.clone(),
        pinned_statement: pinned.statement.clone(),
        candidate: prepared.clone(),
        summary: summary.clone(),
    };
    let path = conflict_dir(ctx).join(format!("{conflict_id}.json"));
    let mut bytes = serde_json::to_vec_pretty(&pending)
        .map_err(|e| format!("no se pudo serializar el conflicto: {e}"))?;
    bytes.push(b'\n');
    storage::atomic_write_bytes(&path, &bytes)
        .map_err(|e| format!("no se pudo guardar {}: {e}", path.display()))?;
    Ok(summary)
}

/// Advertencia (nunca bloqueo) cuando un candidato comparte un binding con
/// un Record fijado y la heurística léxica sugiere polaridad opuesta.
fn warn_possible_pinned_contradiction(
    prepared: &PreparedCandidate,
    active_pinned: &[&Record],
    warnings: &mut Vec<String>,
) {
    for pinned in active_pinned {
        if prepared.supersedes.contains(&pinned.id) {
            continue;
        }
        let shares_binding = pinned.binding_declarations.iter().any(|binding| {
            binding
                .path_hint
                .as_deref()
                .is_some_and(|path| prepared.bindings.iter().any(|b| b.path_hint == path))
        });
        if !shares_binding {
            continue;
        }
        let opposed = retrieval::polarity_of(&prepared.statement, &pinned.statement)
            == retrieval::Polarity::Opposed;
        if opposed && retrieval::shared_terms(&prepared.statement, &pinned.statement).len() >= 2 {
            warnings.push(format!(
                "posible contradicción no verificada con la regla fijada '{}' — si la nueva \
                 afirmación la reemplaza, decláralo en supersedes; si no, ambas coexisten y la \
                 fijada conserva precedencia",
                pinned.id
            ));
        }
    }
}

/// Punto de entrada del gate: prepara cada candidato fuera del lock (el
/// proveedor puede tardar) y decide descarte, duplicado, conflicto o commit
/// bajo el lock, contra el canon releído en ese momento.
pub fn capture_candidates(
    ctx: &CanonContext,
    candidates: Vec<Result<Candidate, String>>,
    provider: &mut ProviderHandle,
) -> CaptureOutcome {
    let mut outcome = CaptureOutcome::default();
    let mut prepared = Vec::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        match candidate {
            Err(error) => outcome.discarded.push(discard(
                index,
                DiscardReason::MalformedCandidate,
                format!("candidato mal formado: {error}"),
                "",
            )),
            Ok(candidate) => {
                match prepare_candidate(ctx, index, candidate, provider, &mut outcome.warnings) {
                    Ok(ready) => prepared.push(ready),
                    Err(discarded) => outcome.discarded.push(discarded),
                }
            }
        }
    }
    if prepared.is_empty() {
        return outcome;
    }

    let _lock = match CanonLock::acquire(ctx.local_dir) {
        Ok(lock) => lock,
        Err(error) => {
            for candidate in prepared {
                outcome.discarded.push(discard(
                    candidate.index,
                    DiscardReason::WriteFailed,
                    format!("no se pudo tomar el lock del canon: {error}"),
                    &candidate.statement,
                ));
            }
            return outcome;
        }
    };

    // Un Record ilegible no debe desactivar la deduplicación ni la detección
    // de conflictos en silencio (revisión adversarial de Fase F, hallazgo 1).
    let listing = storage::list_records_detailed(&records_dir(ctx)).unwrap_or_else(|_| {
        storage::RecordListResult {
            records: vec![],
            skipped: vec![],
        }
    });
    for (path, error) in &listing.skipped {
        outcome.warnings.push(format!(
            "Record ilegible, no participa en deduplicación ni conflictos: {} ({error})",
            path.display()
        ));
    }
    let mut records = listing.records;
    for candidate in prepared {
        commit_one(
            ctx,
            candidate,
            &mut records,
            &mut outcome,
            RecordAuthority::Normal,
            None,
        );
    }
    // Los descartes de forma ocurren antes del lock y los de estado del
    // canon después; el reporte se ordena por candidato para que el agente
    // pueda leerlo en el mismo orden en que los envió.
    outcome.discarded.sort_by_key(|d| d.index);
    outcome
}

/// Decide y escribe un candidato ya preparado. `records` es el canon vivo
/// bajo el lock y se actualiza con cada commit para que el siguiente
/// candidato del mismo lote vea lo recién escrito.
fn commit_one(
    ctx: &CanonContext,
    candidate: PreparedCandidate,
    records: &mut Vec<Record>,
    outcome: &mut CaptureOutcome,
    authority: RecordAuthority,
    human_override: Option<&HumanDecision>,
) -> Option<String> {
    let superseded_now = superseded_ids(records);
    let normalized = normalize_statement(&candidate.statement);
    if let Some(existing) = records
        .iter()
        .find(|r| is_active(r, &superseded_now) && normalize_statement(&r.statement) == normalized)
    {
        let mut discarded = discard(
            candidate.index,
            DiscardReason::Duplicate,
            format!("ya existe como '{}'", existing.id),
            &candidate.statement,
        );
        discarded.duplicate_of = Some(existing.id.clone());
        outcome.discarded.push(discarded);
        return None;
    }

    // Supersesión explícita: solo cuenta contra Records activos. Un id
    // inexistente o ya inactivo se ignora con advertencia — coexistir es el
    // default seguro, nunca reemplazar por aproximación.
    let mut effective_supersedes = Vec::new();
    for target in &candidate.supersedes {
        match records.iter().find(|r| &r.id == target) {
            None => outcome.warnings.push(format!(
                "supersedes '{target}' no existe en el canon — se ignora; ambas afirmaciones \
                 coexisten"
            )),
            Some(existing) if !is_active(existing, &superseded_now) => outcome.warnings.push(
                format!("supersedes '{target}' ya no está activo — se ignora"),
            ),
            Some(existing)
                if storage::record_authority(existing) == RecordAuthority::Pinned
                    && human_override.is_none() =>
            {
                match persist_conflict(ctx, &candidate, existing) {
                    Ok(summary) => outcome.conflicts.push(summary),
                    Err(error) => outcome.discarded.push(discard(
                        candidate.index,
                        DiscardReason::WriteFailed,
                        error,
                        &candidate.statement,
                    )),
                }
                return None;
            }
            Some(_) => effective_supersedes.push(target.clone()),
        }
    }

    let active_pinned: Vec<&Record> = records
        .iter()
        .filter(|r| {
            is_active(r, &superseded_now) && storage::record_authority(r) == RecordAuthority::Pinned
        })
        .collect();
    warn_possible_pinned_contradiction(&candidate, &active_pinned, &mut outcome.warnings);

    let taken: HashSet<String> = records.iter().map(|r| r.id.clone()).collect();
    let id = match &candidate.requested_id {
        Some(requested) if taken.contains(requested) || record_path(ctx, requested).exists() => {
            outcome.discarded.push(discard(
                candidate.index,
                DiscardReason::IdAlreadyExists,
                format!(
                    "'{requested}' ya existe con otra afirmación — usa un id nuevo y declara \
                     supersedes si la reemplaza"
                ),
                &candidate.statement,
            ));
            return None;
        }
        Some(requested) => requested.clone(),
        None => unique_id(
            ctx,
            &slug_for(&candidate.kind, &candidate.statement),
            &taken,
        ),
    };

    let subject_id = resolve_subject(ctx, &candidate, records, &mut outcome.warnings);
    let mut record = build_record(
        ctx,
        &candidate,
        &id,
        authority,
        subject_id.clone(),
        &effective_supersedes,
    );
    if let Some(decision) = human_override {
        push_lifecycle_event(
            &mut record,
            None,
            lifecycle_event(&[
                ("type", "pinned-by-conflict-resolution".to_string()),
                ("actor", decision.actor.clone()),
                (
                    "relayed_by",
                    decision.relayed_by.clone().unwrap_or_default(),
                ),
                (
                    "human_answer",
                    decision.human_answer.clone().unwrap_or_default(),
                ),
                ("timestamp", evaluation::now_iso8601()),
            ]),
        );
    }
    let path = record_path(ctx, &id);
    if let Err(error) = storage::write_record(&path, &record) {
        outcome.discarded.push(discard(
            candidate.index,
            DiscardReason::WriteFailed,
            error.to_string(),
            &candidate.statement,
        ));
        return None;
    }

    let actor_label = human_override
        .map(|decision| decision.actor.clone())
        .unwrap_or_else(|| ctx.actor.agent_label());
    let mut superseded = Vec::new();
    for target in &effective_supersedes {
        match mark_superseded(ctx, target, &id, &actor_label, &candidate.rationale) {
            Ok(()) => {
                superseded.push(target.clone());
                outcome.superseded.push(SupersededRecord {
                    id: target.clone(),
                    superseded_by: id.clone(),
                });
            }
            // El Record nuevo ya declara `supersedes`: `superseded_ids` lo
            // deriva igual, así que el canon queda consistente aunque esta
            // segunda escritura falle.
            Err(error) => outcome.warnings.push(format!(
                "'{id}' reemplaza a '{target}', pero no se pudo marcar el anterior: {error}"
            )),
        }
    }

    outcome.committed.push(CommittedRecord {
        index: candidate.index,
        id: id.clone(),
        kind: record.kind.clone(),
        statement: record.statement.clone(),
        authority: authority.as_str().to_string(),
        path: path.display().to_string(),
        bindings: candidate
            .bindings
            .iter()
            .map(|b| match &b.structural_id {
                Some(symbol) => format!("{}::{symbol}", b.path_hint),
                None => b.path_hint.clone(),
            })
            .collect(),
        superseded,
        subject_id,
    });
    if let Ok(reread) = storage::read_record(&path) {
        records.retain(|r| r.id != id);
        records.push(reread);
    }
    for target in &effective_supersedes {
        if let Ok(reread) = storage::read_record(&record_path(ctx, target)) {
            records.retain(|r| &r.id != target);
            records.push(reread);
        }
    }
    Some(id)
}

// ---------------------------------------------------------------------------
// Conflictos: continuación después de preguntar al humano
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictDecision {
    KeepPinned,
    AdoptNew,
}

impl ConflictDecision {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "keep_pinned" | "keep-pinned" => Some(Self::KeepPinned),
            "adopt_new" | "adopt-new" => Some(Self::AdoptNew),
            _ => None,
        }
    }
}

/// La decisión humana tal como llega: quién es (identidad Git local), si el
/// proyecto lo declara con autoridad, y si la retransmitió un agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanDecision {
    pub actor: String,
    pub declared: bool,
    pub relayed_by: Option<String>,
    pub human_answer: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub decision: ConflictDecision,
    pub pinned_record_id: String,
    pub outcome: CaptureOutcome,
}

pub fn list_pending_conflicts(local_dir: &Path) -> Vec<ConflictSummary> {
    let dir = local_dir.join("conflicts");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut conflicts: Vec<ConflictSummary> = entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|content| serde_json::from_str::<PendingConflict>(&content).ok())
        .map(|pending| pending.summary)
        .collect();
    conflicts.sort_by(|a, b| a.conflict_id.cmp(&b.conflict_id));
    conflicts
}

pub fn resolve_conflict(
    ctx: &CanonContext,
    conflict_id: &str,
    decision: ConflictDecision,
    human: HumanDecision,
) -> Result<ConflictResolution, String> {
    storage::validate_safe_id(conflict_id).map_err(|e| e.to_string())?;
    let path = conflict_dir(ctx).join(format!("{conflict_id}.json"));
    let content = std::fs::read_to_string(&path)
        .map_err(|_| format!("no existe un conflicto pendiente '{conflict_id}'"))?;
    let pending: PendingConflict = serde_json::from_str(&content)
        .map_err(|e| format!("conflicto '{conflict_id}' ilegible: {e}"))?;

    // f8-project-authority: mantener la regla fijada no exige nada; reemplazarla
    // es un acto de autoridad y exige un actor declarado por el proyecto.
    if decision == ConflictDecision::AdoptNew && !human.declared {
        return Err(format!(
            "'{}' no está declarado en .rationale/config.yaml — reemplazar una regla fijada \
             exige autoridad declarada. La regla fijada sigue gobernando; el conflicto queda \
             pendiente.",
            human.actor
        ));
    }

    let mut outcome = CaptureOutcome::default();
    let _lock = CanonLock::acquire(ctx.local_dir)?;
    let mut records = storage::list_records(&records_dir(ctx)).unwrap_or_default();

    if decision == ConflictDecision::AdoptNew {
        let superseded_now = superseded_ids(&records);
        let pinned_still_active = records.iter().any(|r| {
            r.id == pending.pinned_record_id
                && is_active(r, &superseded_now)
                && storage::record_authority(r) == RecordAuthority::Pinned
        });
        let ctx_with_actor = CanonContext {
            rationale_dir: ctx.rationale_dir,
            project_id: ctx.project_id,
            repo_path: ctx.repo_path,
            local_dir: ctx.local_dir,
            head_revision: ctx.head_revision.clone(),
            uncommitted_paths: ctx.uncommitted_paths.clone(),
            actor: ActorContext {
                operation_id: pending.actor.operation_id.clone(),
                ..ctx.actor.clone()
            },
        };
        if pinned_still_active {
            commit_one(
                &ctx_with_actor,
                pending.candidate.clone(),
                &mut records,
                &mut outcome,
                RecordAuthority::Pinned,
                Some(&human),
            );
        } else {
            outcome.warnings.push(format!(
                "'{}' ya no es una regla fijada activa — la afirmación se registra sin herencia \
                 de fijación",
                pending.pinned_record_id
            ));
            commit_one(
                &ctx_with_actor,
                pending.candidate.clone(),
                &mut records,
                &mut outcome,
                RecordAuthority::Normal,
                None,
            );
        }
        if !outcome.conflicts.is_empty() || outcome.committed.is_empty() {
            return Err(format!(
                "no se pudo adoptar la nueva afirmación: {:?}",
                outcome
                    .discarded
                    .iter()
                    .map(|d| &d.detail)
                    .collect::<Vec<_>>()
            ));
        }
    }

    let resolved_dir = conflict_dir(ctx).join("resolved");
    std::fs::create_dir_all(&resolved_dir)
        .map_err(|e| format!("no se pudo crear {}: {e}", resolved_dir.display()))?;
    let mut resolved = serde_json::to_value(&pending).map_err(|e| e.to_string())?;
    resolved["resolution"] = serde_json::json!({
        "decision": decision,
        "decided_at": evaluation::now_iso8601(),
        "decided_by": human.actor,
        "declared_authority": human.declared,
        "relayed_by": human.relayed_by,
        "human_answer": human.human_answer,
        "committed": outcome.committed.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
    });
    let mut bytes = serde_json::to_vec_pretty(&resolved).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    storage::atomic_write_bytes(&resolved_dir.join(format!("{conflict_id}.json")), &bytes)
        .map_err(|e| format!("no se pudo archivar el conflicto: {e}"))?;
    std::fs::remove_file(&path).map_err(|e| format!("no se pudo cerrar el conflicto: {e}"))?;

    Ok(ConflictResolution {
        conflict_id: conflict_id.to_string(),
        decision,
        pinned_record_id: pending.pinned_record_id,
        outcome,
    })
}

// ---------------------------------------------------------------------------
// Migración de propuestas pendientes (pre-vNext)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct MigratedProposal {
    pub proposal_id: String,
    pub record_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchivedProposal {
    pub proposal_id: String,
    pub reason: DiscardReason,
    pub detail: String,
    pub archived_at: String,
}

#[derive(Debug, Default, Serialize)]
pub struct MigrationReport {
    pub dry_run: bool,
    pub canonicalized: Vec<MigratedProposal>,
    pub archived: Vec<ArchivedProposal>,
    pub unreadable: Vec<String>,
    pub failed: Vec<String>,
}

/// Una propuesta pendiente pasa por el mismo gate que un candidato nuevo.
/// Válida → Record canónico con procedencia `migrated`. Inválida o ruidosa →
/// archivada con su motivo en `.rationale/archive/proposals/`. Nunca se
/// borra información: el claim atómico (`constraint.f8-atomic-proposal-claim`)
/// garantiza que dos procesos no migren la misma propuesta.
pub fn migrate_pending_proposals(ctx: &CanonContext, dry_run: bool) -> MigrationReport {
    let mut report = MigrationReport {
        dry_run,
        ..Default::default()
    };
    let listing = crate::review::list_pending_detailed(ctx.rationale_dir);
    report.unreadable = listing
        .skipped
        .iter()
        .map(|(path, error)| format!("{}: {error}", path.display()))
        .collect();
    if listing.pending.is_empty() {
        return report;
    }

    let _lock = if dry_run {
        None
    } else {
        match CanonLock::acquire(ctx.local_dir) {
            Ok(lock) => Some(lock),
            Err(error) => {
                report.failed.push(error);
                return report;
            }
        }
    };
    let mut records = storage::list_records(&records_dir(ctx)).unwrap_or_default();

    for proposal in listing.pending {
        let proposal_id = proposal.record.id.clone();
        let verdict = migration_verdict(ctx, &proposal.record, &records);
        if dry_run {
            match verdict {
                Ok(()) => report.canonicalized.push(MigratedProposal {
                    proposal_id: proposal_id.clone(),
                    record_id: proposal_id,
                }),
                Err((reason, detail)) => report.archived.push(ArchivedProposal {
                    proposal_id,
                    reason,
                    detail,
                    archived_at: String::new(),
                }),
            }
            continue;
        }

        let claimed = match crate::review::claim_verified(ctx.rationale_dir, &proposal) {
            Ok(claimed) => claimed,
            Err(error) => {
                report.failed.push(format!("{proposal_id}: {error}"));
                continue;
            }
        };
        match verdict {
            Ok(()) => {
                let mut record = proposal.record;
                record.extra.remove(string_value("status"));
                record.authority = Some(RecordAuthority::Normal.as_str().to_string());
                let mut provenance = record.provenance.take().unwrap_or_default();
                provenance.kind = Some(ProvenanceKind::Migrated.as_str().to_string());
                provenance.extra.insert(
                    string_value("migrated_at"),
                    string_value(evaluation::now_iso8601()),
                );
                provenance.extra.insert(
                    string_value("migrated_from"),
                    string_value("pending-proposal"),
                );
                record.provenance = Some(provenance);
                push_lifecycle_event(
                    &mut record,
                    Some("active"),
                    lifecycle_event(&[
                        ("type", "migrated-from-proposal".to_string()),
                        ("actor", "rationale:migration".to_string()),
                        ("timestamp", evaluation::now_iso8601()),
                    ]),
                );
                let path = record_path(ctx, &record.id);
                match storage::write_record(&path, &record) {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&claimed);
                        report.canonicalized.push(MigratedProposal {
                            proposal_id: proposal_id.clone(),
                            record_id: record.id.clone(),
                        });
                        records.push(record);
                    }
                    Err(error) => report.failed.push(format!(
                        "{proposal_id}: {error} — la propuesta queda recuperable en {}",
                        claimed.display()
                    )),
                }
            }
            Err((reason, detail)) => {
                let archive_dir = ctx.rationale_dir.join("archive").join("proposals");
                let archived_at = evaluation::now_iso8601();
                let result = std::fs::create_dir_all(&archive_dir)
                    .and_then(|()| {
                        std::fs::rename(&claimed, archive_dir.join(format!("{proposal_id}.yaml")))
                    })
                    .and_then(|()| {
                        let note = format!(
                            "proposal_id: {proposal_id}\nreason: {}\ndetail: {}\narchived_at: {archived_at}\n",
                            serde_json::to_string(&reason).unwrap_or_default().trim_matches('"'),
                            serde_json::to_string(&detail).unwrap_or_default(),
                        );
                        storage::atomic_write_bytes(
                            &archive_dir.join(format!("{proposal_id}.migration.yaml")),
                            note.as_bytes(),
                        )
                    });
                match result {
                    Ok(()) => report.archived.push(ArchivedProposal {
                        proposal_id,
                        reason,
                        detail,
                        archived_at,
                    }),
                    Err(error) => report.failed.push(format!(
                        "{proposal_id}: no se pudo archivar ({error}) — queda en {}",
                        claimed.display()
                    )),
                }
            }
        }
    }
    report
}

fn migration_verdict(
    ctx: &CanonContext,
    proposal: &Record,
    records: &[Record],
) -> Result<(), (DiscardReason, String)> {
    let candidate = Candidate {
        id: Some(proposal.id.clone()),
        kind: proposal.kind.clone(),
        statement: proposal.statement.clone(),
        rationale: proposal.rationale.clone().unwrap_or_default(),
        durability: Some("durable".to_string()),
        severity: Some(proposal.severity.clone()),
        bindings: vec![],
        supersedes: vec![],
        risks: vec![],
        evidence: vec![],
        subject: None,
    };
    validate_shape(0, &candidate).map_err(|d| (d.reason, d.detail))?;
    let any_binding_exists = proposal.binding_declarations.iter().any(|binding| {
        binding
            .path_hint
            .as_deref()
            .is_some_and(|hint| ctx.repo_path.join(hint).is_file())
    });
    if !any_binding_exists {
        return Err((
            DiscardReason::NoMeaningfulBinding,
            "ningún binding de la propuesta apunta a un archivo existente".to_string(),
        ));
    }
    let superseded = superseded_ids(records);
    let normalized = normalize_statement(&proposal.statement);
    if let Some(existing) = records
        .iter()
        .find(|r| is_active(r, &superseded) && normalize_statement(&r.statement) == normalized)
    {
        return Err((
            DiscardReason::Duplicate,
            format!("ya existe como '{}'", existing.id),
        ));
    }
    if records.iter().any(|r| r.id == proposal.id) || record_path(ctx, &proposal.id).exists() {
        return Err((
            DiscardReason::IdAlreadyExists,
            format!("ya existe un Record '{}' con otra afirmación", proposal.id),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_suffix() -> String {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )
    }

    struct Fixture {
        root: PathBuf,
        rationale_dir: PathBuf,
        local_dir: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rationale-canon-test-{}-{}",
                std::process::id(),
                unique_suffix()
            ));
            std::fs::create_dir_all(root.join(".rationale/records")).unwrap();
            std::fs::create_dir_all(root.join(".rationale/subjects")).unwrap();
            std::fs::create_dir_all(root.join("src")).unwrap();
            std::fs::write(root.join("src/payments.rs"), "fn create_link() {}\n").unwrap();
            std::fs::write(root.join("src/config.rs"), "fn expiration() {}\n").unwrap();
            Fixture {
                rationale_dir: root.join(".rationale"),
                local_dir: root.join(".rationale-local"),
                root,
            }
        }

        fn ctx(&self) -> CanonContext<'_> {
            CanonContext {
                rationale_dir: &self.rationale_dir,
                project_id: "fixture",
                repo_path: &self.root,
                local_dir: &self.local_dir,
                head_revision: Some("abc123".to_string()),
                uncommitted_paths: HashSet::from(["src/payments.rs".to_string()]),
                actor: ActorContext {
                    client: "claude-code".to_string(),
                    client_source: "flag".to_string(),
                    session_id: Some("session_test".to_string()),
                    operation_id: Some("op_test".to_string()),
                },
            }
        }

        fn records(&self) -> Vec<Record> {
            storage::list_records(&self.root.join(".rationale/records")).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn durable(statement: &str, rationale: &str) -> Candidate {
        Candidate {
            id: None,
            kind: "constraint".to_string(),
            statement: statement.to_string(),
            rationale: rationale.to_string(),
            durability: Some("durable".to_string()),
            severity: Some("high".to_string()),
            bindings: vec![CandidateBinding::Spec("src/payments.rs".to_string())],
            supersedes: vec![],
            risks: vec![],
            evidence: vec![],
            subject: None,
        }
    }

    const EXPIRY: &str = "Payment links must expire after ten minutes.";
    const EXPIRY_WHY: &str =
        "Stale links let customers pay prices that already changed, which finance cannot reconcile.";

    fn capture(fixture: &Fixture, candidates: Vec<Candidate>) -> CaptureOutcome {
        let mut provider = ProviderHandle::Unavailable("test".to_string());
        capture_candidates(
            &fixture.ctx(),
            candidates.into_iter().map(Ok).collect(),
            &mut provider,
        )
    }

    #[test]
    fn durable_candidate_becomes_a_canonical_record_without_approval() {
        let fixture = Fixture::new();
        let outcome = capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);

        assert_eq!(outcome.committed.len(), 1, "{outcome:?}");
        assert!(outcome.discarded.is_empty());
        let records = fixture.records();
        let record = &records[0];
        assert_eq!(
            record.id,
            "constraint.payment-links-must-expire-after-ten-minutes"
        );
        assert!(record.approvals.is_empty());
        assert_eq!(storage::record_authority(record), RecordAuthority::Normal);
        assert_eq!(
            storage::provenance_kind(record),
            ProvenanceKind::AgentAsserted
        );
        assert_eq!(storage::lifecycle_status(record), Some("active"));
        assert_eq!(record.bound_revision.as_deref(), Some("abc123"));
        assert_eq!(record.binding_declarations.len(), 1);
        assert!(
            record.binding_declarations[0].provisional,
            "un archivo con cambios sin commitear no es verificable por un tercero"
        );
        let provenance = record.provenance.as_ref().unwrap();
        assert_eq!(
            provenance.extra.get(string_value("client")),
            Some(&string_value("claude-code"))
        );
        assert!(
            !fixture.root.join(".rationale/proposals").exists(),
            "vNext nunca crea trabajo pendiente"
        );
    }

    #[test]
    fn gate_discards_without_writing_anything() {
        let fixture = Fixture::new();
        let mut transient = durable(EXPIRY, EXPIRY_WHY);
        transient.durability = Some("transient".to_string());
        let mut undeclared = durable(EXPIRY, EXPIRY_WHY);
        undeclared.durability = None;
        let no_rationale = durable(EXPIRY, "");
        let restated = durable(EXPIRY, "payment links must expire after ten minutes");
        let noise = durable(
            "Renamed the helper function in payments module",
            "The old name was confusing to read",
        );
        let mut unbound = durable(EXPIRY, EXPIRY_WHY);
        unbound.bindings = vec![CandidateBinding::Spec("src/does-not-exist.rs".to_string())];
        let mut bad_kind = durable(EXPIRY, EXPIRY_WHY);
        bad_kind.kind = "rule".to_string();
        let short = durable("Expire links.", EXPIRY_WHY);

        let outcome = capture(
            &fixture,
            vec![
                transient,
                undeclared,
                no_rationale,
                restated,
                noise,
                unbound,
                bad_kind,
                short,
            ],
        );
        let reasons: Vec<DiscardReason> = outcome.discarded.iter().map(|d| d.reason).collect();
        assert_eq!(
            reasons,
            vec![
                DiscardReason::Transient,
                DiscardReason::DurabilityNotDeclared,
                DiscardReason::MissingRationale,
                DiscardReason::RationaleRestatesStatement,
                DiscardReason::MechanicalNoise,
                DiscardReason::NoMeaningfulBinding,
                DiscardReason::InvalidKind,
                DiscardReason::StatementNotMeaningful,
            ]
        );
        assert!(outcome.committed.is_empty());
        assert!(fixture.records().is_empty());
    }

    #[test]
    fn bindings_never_anchor_rationale_data() {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.root.join(".rationale/records/other.yaml"),
            "id: x\n",
        )
        .unwrap();
        let mut candidate = durable(EXPIRY, EXPIRY_WHY);
        candidate.bindings = vec![CandidateBinding::Structured {
            path: ".rationale/records/other.yaml".to_string(),
            symbol: None,
        }];
        let outcome = capture(&fixture, vec![candidate]);
        assert_eq!(
            outcome.discarded[0].reason,
            DiscardReason::NoMeaningfulBinding
        );
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.contains("datos de Rationale")));
    }

    #[test]
    fn mechanical_verbs_with_a_causal_reason_are_not_noise() {
        assert!(is_mechanical_noise(
            "Updated the retry helper",
            "it was hard to follow"
        ));
        assert!(!is_mechanical_noise(
            "Removed the cache for session tokens",
            "Tokens must never outlive a revoked session, and the cache kept them alive."
        ));
        assert!(!is_mechanical_noise(
            "Payment links must expire after ten minutes",
            "anything"
        ));
    }

    #[test]
    fn duplicates_are_discarded_against_canon_and_within_a_batch() {
        let fixture = Fixture::new();
        let first = capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);
        assert_eq!(first.committed.len(), 1);

        let again = capture(
            &fixture,
            vec![
                durable("payment LINKS must expire after ten minutes!", EXPIRY_WHY),
                durable(
                    "Refund links must expire after one hour.",
                    "Refunds need longer confirmation windows because banks batch them.",
                ),
                durable(
                    "Refund links must expire after one hour",
                    "Refunds need longer confirmation windows because banks batch them.",
                ),
            ],
        );
        assert_eq!(again.committed.len(), 1);
        assert_eq!(again.discarded.len(), 2);
        assert!(again
            .discarded
            .iter()
            .all(|d| d.reason == DiscardReason::Duplicate));
        assert_eq!(
            again.discarded[0].duplicate_of.as_deref(),
            Some("constraint.payment-links-must-expire-after-ten-minutes")
        );
        assert_eq!(fixture.records().len(), 2);
    }

    #[test]
    fn explicit_supersession_of_a_normal_record_is_automatic() {
        let fixture = Fixture::new();
        capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);
        let mut replacement = durable(
            "Payment links must expire after the tenant-configured window.",
            "Each tenant defines its own payment policy, so a global ten-minute rule was wrong.",
        );
        replacement.supersedes =
            vec!["constraint.payment-links-must-expire-after-ten-minutes".to_string()];
        let outcome = capture(&fixture, vec![replacement]);

        assert_eq!(outcome.committed.len(), 1, "{outcome:?}");
        assert!(outcome.conflicts.is_empty());
        assert_eq!(outcome.superseded.len(), 1);
        let records = fixture.records();
        let old = records
            .iter()
            .find(|r| r.id == "constraint.payment-links-must-expire-after-ten-minutes")
            .unwrap();
        assert_eq!(storage::lifecycle_status(old), Some("superseded"));
        assert_eq!(
            storage::superseded_by(old),
            Some(outcome.committed[0].id.as_str())
        );
        let superseded = superseded_ids(&records);
        assert!(superseded.contains(&old.id));
        assert_eq!(superseded.len(), 1);
    }

    #[test]
    fn records_without_supersedes_coexist() {
        let fixture = Fixture::new();
        capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);
        let other = durable(
            "Payment links should never expire for enterprise tenants.",
            "Enterprise invoices are negotiated offline and paid weeks later by design.",
        );
        let outcome = capture(&fixture, vec![other]);
        assert_eq!(outcome.committed.len(), 1);
        let records = fixture.records();
        assert!(
            superseded_ids(&records).is_empty(),
            "sin supersedes explícito ambas siguen activas"
        );
    }

    fn pin(fixture: &Fixture, id: &str) {
        let path = fixture.root.join(format!(".rationale/records/{id}.yaml"));
        let mut record = storage::read_record(&path).unwrap();
        record.authority = Some("pinned".to_string());
        storage::write_record(&path, &record).unwrap();
    }

    fn human(declared: bool) -> HumanDecision {
        HumanDecision {
            actor: "user:owner <owner@example.com>".to_string(),
            declared,
            relayed_by: Some("claude-code".to_string()),
            human_answer: Some("la nueva gobierna".to_string()),
        }
    }

    #[test]
    fn superseding_a_pinned_record_returns_a_conflict_and_writes_nothing() {
        let fixture = Fixture::new();
        capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);
        let pinned_id = "constraint.payment-links-must-expire-after-ten-minutes";
        pin(&fixture, pinned_id);

        let mut attempt = durable(
            "Payment links should not expire at all.",
            "Customers complained that links died while they were still paying.",
        );
        attempt.supersedes = vec![pinned_id.to_string()];
        let outcome = capture(&fixture, vec![attempt]);

        assert!(outcome.committed.is_empty());
        assert_eq!(outcome.conflicts.len(), 1);
        let conflict = &outcome.conflicts[0];
        assert_eq!(conflict.pinned_record_id, pinned_id);
        assert!(conflict.question.contains("¿Cuál debe gobernar?"));
        assert_eq!(fixture.records().len(), 1, "nada nuevo en el canon");
        let pinned = fixture.records().pop().unwrap();
        assert_eq!(storage::lifecycle_status(&pinned), Some("active"));
        assert_eq!(
            list_pending_conflicts(&fixture.root.join(".rationale-local")).len(),
            1
        );
    }

    #[test]
    fn keep_pinned_resolution_discards_the_candidate() {
        let fixture = Fixture::new();
        capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);
        let pinned_id = "constraint.payment-links-must-expire-after-ten-minutes";
        pin(&fixture, pinned_id);
        let mut attempt = durable(
            "Payment links should not expire at all.",
            "Customers complained that links died while they were still paying.",
        );
        attempt.supersedes = vec![pinned_id.to_string()];
        let conflict_id = capture(&fixture, vec![attempt]).conflicts[0]
            .conflict_id
            .clone();

        let resolution = resolve_conflict(
            &fixture.ctx(),
            &conflict_id,
            ConflictDecision::KeepPinned,
            human(false),
        )
        .unwrap();
        assert!(resolution.outcome.committed.is_empty());
        assert_eq!(fixture.records().len(), 1);
        assert!(list_pending_conflicts(&fixture.root.join(".rationale-local")).is_empty());
        assert!(fixture
            .root
            .join(format!(
                ".rationale-local/conflicts/resolved/{conflict_id}.json"
            ))
            .is_file());
    }

    #[test]
    fn adopt_new_requires_declared_authority_and_inherits_the_pin() {
        let fixture = Fixture::new();
        capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);
        let pinned_id = "constraint.payment-links-must-expire-after-ten-minutes";
        pin(&fixture, pinned_id);
        let mut attempt = durable(
            "Payment links must expire after the tenant-configured window.",
            "Each tenant defines its own payment policy, so a global ten-minute rule was wrong.",
        );
        attempt.supersedes = vec![pinned_id.to_string()];
        let conflict_id = capture(&fixture, vec![attempt]).conflicts[0]
            .conflict_id
            .clone();

        let refused = resolve_conflict(
            &fixture.ctx(),
            &conflict_id,
            ConflictDecision::AdoptNew,
            human(false),
        );
        assert!(
            refused.is_err(),
            "sin autoridad declarada no se reemplaza una regla fijada"
        );
        assert_eq!(fixture.records().len(), 1);
        assert_eq!(
            list_pending_conflicts(&fixture.root.join(".rationale-local")).len(),
            1
        );

        let adopted = resolve_conflict(
            &fixture.ctx(),
            &conflict_id,
            ConflictDecision::AdoptNew,
            human(true),
        )
        .unwrap();
        assert_eq!(adopted.outcome.committed.len(), 1);
        let new_id = adopted.outcome.committed[0].id.clone();
        let records = fixture.records();
        let new_record = records.iter().find(|r| r.id == new_id).unwrap();
        assert_eq!(
            storage::record_authority(new_record),
            RecordAuthority::Pinned
        );
        let old = records.iter().find(|r| r.id == pinned_id).unwrap();
        assert_eq!(storage::lifecycle_status(old), Some("superseded"));
        assert_eq!(storage::superseded_by(old), Some(new_id.as_str()));
    }

    #[test]
    fn possible_contradiction_with_a_pinned_rule_is_a_warning_not_a_block() {
        let fixture = Fixture::new();
        capture(&fixture, vec![durable(EXPIRY, EXPIRY_WHY)]);
        pin(
            &fixture,
            "constraint.payment-links-must-expire-after-ten-minutes",
        );
        let outcome = capture(
            &fixture,
            vec![durable(
                "Payment links must never expire for enterprise tenants.",
                "Enterprise invoices are negotiated offline and paid weeks later by design.",
            )],
        );
        assert_eq!(outcome.committed.len(), 1, "{outcome:?}");
        assert!(outcome.conflicts.is_empty());
        assert!(
            outcome
                .warnings
                .iter()
                .any(|w| w.contains("posible contradicción no verificada")),
            "{:?}",
            outcome.warnings
        );
    }

    #[test]
    fn unknown_or_inactive_supersedes_targets_are_ignored_with_a_warning() {
        let fixture = Fixture::new();
        let mut candidate = durable(EXPIRY, EXPIRY_WHY);
        candidate.supersedes = vec!["constraint.never-existed".to_string()];
        let outcome = capture(&fixture, vec![candidate]);
        assert_eq!(outcome.committed.len(), 1);
        assert!(outcome.committed[0].superseded.is_empty());
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.contains("constraint.never-existed")));
    }

    #[test]
    fn requested_ids_are_validated_and_never_overwrite() {
        let fixture = Fixture::new();
        let mut explicit = durable(EXPIRY, EXPIRY_WHY);
        explicit.id = Some("constraint.payment-expiry".to_string());
        assert_eq!(
            capture(&fixture, vec![explicit]).committed[0].id,
            "constraint.payment-expiry"
        );

        let mut collision = durable(
            "Payment links must expire after five minutes.",
            "Shorter windows reduce the chance of paying a stale amount after price changes.",
        );
        collision.id = Some("constraint.payment-expiry".to_string());
        let mut mismatch = durable(
            "Payment links must expire after five minutes.",
            "Shorter windows reduce the chance of paying a stale amount after price changes.",
        );
        mismatch.id = Some("decision.payment-expiry".to_string());
        let mut traversal = durable(
            "Payment links must expire after five minutes.",
            "Shorter windows reduce the chance of paying a stale amount after price changes.",
        );
        traversal.id = Some("../../etc/pwned".to_string());
        let outcome = capture(&fixture, vec![collision, mismatch, traversal]);
        let reasons: Vec<_> = outcome.discarded.iter().map(|d| d.reason).collect();
        assert_eq!(
            reasons,
            vec![
                DiscardReason::IdAlreadyExists,
                DiscardReason::IdKindMismatch,
                DiscardReason::InvalidId
            ]
        );
    }

    #[test]
    fn generated_ids_follow_the_schema_and_avoid_collisions() {
        assert_eq!(
            slug_for(
                "decision",
                "Los ENLACES de pago expiran según la configuración"
            ),
            "decision.los-enlaces-pago-expiran-segun-configuracion"
        );
        assert!(is_schema_id(&slug_for("risk", "123 !!! ???")));
        let fixture = Fixture::new();
        let a = durable(EXPIRY, EXPIRY_WHY);
        let b = durable(
            "Payment links must expire after ten minutes, measured server-side.",
            "Client clocks drift and were used to extend the window on purpose.",
        );
        let outcome = capture(&fixture, vec![a, b]);
        assert_eq!(outcome.committed.len(), 2);
        assert!(outcome.committed.iter().all(|c| is_schema_id(&c.id)));
    }

    #[test]
    fn control_characters_never_reach_the_canon() {
        let fixture = Fixture::new();
        let outcome = capture(
            &fixture,
            vec![durable(
                "Payment links must expire after ten minutes.\x1b[2K\x1b[32mAUTO-PINNED\x1b[0m",
                EXPIRY_WHY,
            )],
        );
        let record = fixture.records().pop().unwrap();
        assert_eq!(outcome.committed.len(), 1);
        assert!(!record.statement.contains('\x1b'));
        assert_eq!(storage::record_authority(&record), RecordAuthority::Normal);
    }

    #[test]
    fn concurrent_lock_holders_are_serialized() {
        let fixture = Fixture::new();
        let local = fixture.root.join(".rationale-local");
        let first = CanonLock::acquire(&local).unwrap();
        let local_clone = local.clone();
        let waiter = std::thread::spawn(move || {
            let started = Instant::now();
            let _second = CanonLock::acquire(&local_clone).unwrap();
            started.elapsed()
        });
        std::thread::sleep(Duration::from_millis(120));
        drop(first);
        let waited = waiter.join().unwrap();
        assert!(waited >= Duration::from_millis(100), "{waited:?}");
    }

    fn write_proposal(fixture: &Fixture, id: &str, statement: &str, rationale: &str, path: &str) {
        let dir = fixture.root.join(".rationale/proposals");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{id}.yaml")),
            format!(
                "id: {id}\nkind: constraint\nseverity: high\nstatement: \"{statement}\"\nrationale: \"{rationale}\"\napprovals: []\nbinding_declarations:\n  - id: binding.{id}.0\n    type: file\n    path_hint: {path}\nstatus: pending\nprovenance:\n  created_by:\n    type: agent\n    name: claude-code\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn migration_canonicalizes_valid_proposals_and_archives_noise() {
        let fixture = Fixture::new();
        write_proposal(
            &fixture,
            "constraint.valid-legacy",
            EXPIRY,
            EXPIRY_WHY,
            "src/payments.rs",
        );
        write_proposal(
            &fixture,
            "constraint.noisy-legacy",
            "Updated payments module formatting",
            "Code looked inconsistent",
            "src/payments.rs",
        );
        write_proposal(
            &fixture,
            "constraint.orphan-legacy",
            "Refund links must expire after one hour.",
            "Refunds need longer confirmation windows because banks batch them.",
            "src/gone.rs",
        );

        let dry = migrate_pending_proposals(&fixture.ctx(), true);
        assert_eq!(dry.canonicalized.len(), 1);
        assert_eq!(dry.archived.len(), 2);
        assert!(fixture.records().is_empty(), "dry-run no escribe");

        let report = migrate_pending_proposals(&fixture.ctx(), false);
        assert_eq!(report.canonicalized.len(), 1, "{report:?}");
        assert_eq!(report.archived.len(), 2);
        assert!(report.failed.is_empty());

        let records = fixture.records();
        assert_eq!(records.len(), 1);
        let migrated = &records[0];
        assert_eq!(storage::provenance_kind(migrated), ProvenanceKind::Migrated);
        assert_eq!(storage::record_authority(migrated), RecordAuthority::Normal);
        assert!(
            migrated
                .provenance
                .as_ref()
                .unwrap()
                .extra
                .contains_key(string_value("created_by")),
            "la procedencia legada se conserva"
        );
        assert!(!migrated.extra.contains_key(string_value("status")));

        let archive = fixture.root.join(".rationale/archive/proposals");
        assert!(archive.join("constraint.noisy-legacy.yaml").is_file());
        assert!(archive
            .join("constraint.noisy-legacy.migration.yaml")
            .is_file());
        assert!(archive.join("constraint.orphan-legacy.yaml").is_file());
        let pending_left = std::fs::read_dir(fixture.root.join(".rationale/proposals"))
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
            .count();
        assert_eq!(pending_left, 0, "nada queda pendiente y nada se borró");

        let again = migrate_pending_proposals(&fixture.ctx(), false);
        assert!(
            again.canonicalized.is_empty() && again.archived.is_empty(),
            "idempotente"
        );
    }
}
