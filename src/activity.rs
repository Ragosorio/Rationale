//! Actividad local — lo que Rationale hace, observable (vNext).
//!
//! Cada proceso (`rationale serve`, una invocación de la CLI) es una sesión
//! que agrega eventos a `.rationale-local/activity/<session-id>.ndjson`. Un
//! archivo por sesión: Claude Code, Codex y Cursor corriendo a la vez nunca
//! compiten por el mismo archivo, y la UI combina las sesiones.
//!
//! Minimización (ADR-0017): los eventos llevan identificadores, conteos,
//! estados, latencias y tamaños. Solo dos textos libres, acotados: el spec del
//! target y la intención declarada. Nunca código, diffs, rationale ni
//! evidencia — el contenido de un Record viaja por referencia (su id) y la UI
//! lo lee del canon. `RATIONALE_ACTIVITY=off` desactiva el flujo.
//!
//! Registrar actividad nunca hace fallar una herramienta: un error de
//! escritura se advierte una vez por stderr y el trabajo continúa.

use crate::canon::{self, ActorContext};
use crate::evaluation;
use crate::operations::{Operation, OperationTarget};
use crate::relationships::{RelationshipAssessment, RelationshipState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

pub const SCHEMA_VERSION: &str = "rationale/activity/1";
/// Techo de la intención declarada: suficiente para reconocer la tarea en la
/// UI, insuficiente para convertir el log en un archivo de prompts.
pub const MAX_INTENT_CHARS: usize = 280;
/// Techo de cualquier otro texto: specs, rutas, nombres calificados, ids.
pub const MAX_IDENTIFIER_CHARS: usize = 200;
pub const MAX_LIST_ITEMS: usize = 64;
const RETENTION: Duration = Duration::from_secs(14 * 24 * 60 * 60);
const RETAINED_SESSIONS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    /// `agent` para un cliente MCP, `cli` para la línea de comandos.
    #[serde(rename = "type")]
    pub actor_type: String,
    pub client: String,
    pub client_source: String,
}

impl Actor {
    fn from_context(actor: &ActorContext) -> Self {
        let actor_type = if actor.client_source == "cli" {
            "cli"
        } else {
            "agent"
        };
        Actor {
            actor_type: actor_type.to_string(),
            client: ident(&actor.client),
            client_source: ident(&actor.client_source),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityEvent {
    pub schema_version: String,
    pub session_id: String,
    /// Orden dentro de la sesión (desde 1). Entre sesiones ordena `timestamp`.
    pub seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    /// RFC3339 UTC con milisegundos.
    pub timestamp: String,
    pub actor: Actor,
    pub project: String,
    pub kind: String,
    #[serde(default)]
    pub payload: Value,
}

/// Dónde y en nombre de quién se registra: el proyecto (su
/// `.rationale-local`), el actor, la traza de la llamada y la operación.
#[derive(Debug, Clone)]
pub struct Scope {
    pub local_dir: PathBuf,
    pub project_root: PathBuf,
    pub project_id: String,
    pub actor: ActorContext,
    /// Una traza por llamada: agrupa los eventos de un mismo `prepare_change`
    /// o `finalize_change`.
    pub trace_id: String,
    pub operation_id: Option<String>,
}

impl Scope {
    pub fn new(
        local_dir: &Path,
        project_root: &Path,
        project_id: &str,
        actor: &ActorContext,
    ) -> Self {
        Scope {
            local_dir: local_dir.to_path_buf(),
            project_root: project_root.to_path_buf(),
            project_id: project_id.to_string(),
            actor: actor.clone(),
            trace_id: canon::generate_id("trace"),
            operation_id: actor.operation_id.clone(),
        }
    }

    /// El proyecto Rationale que contiene `start`, si existe.
    pub fn for_project(start: &Path, actor: &ActorContext) -> Option<Self> {
        let config = crate::configuration::load(start).ok()?;
        let local_dir = crate::configuration::find_rationale_local(&config.project_root);
        Some(Scope::new(
            &local_dir,
            &config.project_root,
            &config.project_id,
            actor,
        ))
    }

    pub fn with_operation(mut self, operation_id: &str) -> Self {
        self.operation_id = Some(operation_id.to_string());
        self
    }
}

struct OpenedProject {
    project_root: PathBuf,
    project_id: String,
}

/// La sesión de actividad de un proceso.
pub struct Recorder {
    session_id: String,
    enabled: bool,
    seq: AtomicU64,
    /// Proyectos donde la sesión ya escribió (y ya garantizó la exclusión).
    opened: Mutex<HashMap<PathBuf, OpenedProject>>,
    /// `agent.connected` pendiente de encabezar cada proyecto nuevo.
    connected: Mutex<Option<Value>>,
    warned: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Recorder {
    /// Activo salvo `RATIONALE_ACTIVITY=off` (también `0`, `false`, `no`).
    pub fn new(session_id: Option<&str>) -> Self {
        let disabled = std::env::var("RATIONALE_ACTIVITY").is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "off" | "0" | "false" | "no"
            )
        });
        Self::with_enabled(session_id, !disabled)
    }

    pub fn with_enabled(session_id: Option<&str>, enabled: bool) -> Self {
        let session_id = session_id
            .filter(|id| crate::storage::validate_safe_id(id).is_ok())
            .map(str::to_string)
            .unwrap_or_else(|| canon::generate_id("session"));
        Recorder {
            session_id,
            enabled,
            seq: AtomicU64::new(0),
            opened: Mutex::new(HashMap::new()),
            connected: Mutex::new(None),
            warned: AtomicBool::new(false),
        }
    }

    pub fn emit(&self, scope: &Scope, kind: &str, payload: Value) {
        if !self.enabled {
            return;
        }
        if let Err(error) = self.try_emit(scope, kind, payload) {
            if !self.warned.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "advertencia: la actividad local no se pudo registrar ({error}); el trabajo \
                     continúa"
                );
            }
        }
    }

    /// El cliente MCP se identificó. Se registra en `scope` (el proyecto por
    /// defecto del servidor, si existe) y encabeza cualquier otro proyecto
    /// que la sesión toque después.
    pub fn connected(&self, scope: Option<&Scope>, payload: Value) {
        *lock(&self.connected) = Some(payload.clone());
        if let Some(scope) = scope {
            self.emit(scope, "agent.connected", payload);
        }
    }

    /// `session.ended` en cada proyecto donde la sesión escribió.
    pub fn end(&self, actor: &ActorContext) {
        let opened: Vec<(PathBuf, PathBuf, String)> = lock(&self.opened)
            .iter()
            .map(|(dir, project)| {
                (
                    dir.clone(),
                    project.project_root.clone(),
                    project.project_id.clone(),
                )
            })
            .collect();
        for (local_dir, project_root, project_id) in opened {
            let scope = Scope::new(&local_dir, &project_root, &project_id, actor);
            self.emit(&scope, "session.ended", json!({}));
        }
    }

    fn try_emit(&self, scope: &Scope, kind: &str, payload: Value) -> Result<(), String> {
        if !lock(&self.opened).contains_key(&scope.local_dir) {
            ensure_excluded(&scope.project_root)?;
            prune(&scope.local_dir);
            if kind != "session.started" {
                self.write(scope, "session.started", payload::session_started())?;
            }
            let connected = lock(&self.connected).clone();
            if let (Some(connected), true) = (connected, kind != "agent.connected") {
                self.write(scope, "agent.connected", connected)?;
            }
            lock(&self.opened).insert(
                scope.local_dir.clone(),
                OpenedProject {
                    project_root: scope.project_root.clone(),
                    project_id: scope.project_id.clone(),
                },
            );
        }
        self.write(scope, kind, payload)
    }

    fn write(&self, scope: &Scope, kind: &str, payload: Value) -> Result<(), String> {
        let event = ActivityEvent {
            schema_version: SCHEMA_VERSION.to_string(),
            session_id: self.session_id.clone(),
            seq: self.seq.fetch_add(1, Ordering::Relaxed) + 1,
            trace_id: Some(scope.trace_id.clone()),
            operation_id: scope.operation_id.clone(),
            timestamp: evaluation::now_rfc3339_millis(),
            actor: Actor::from_context(&scope.actor),
            project: ident(&scope.project_id),
            kind: kind.to_string(),
            payload,
        };
        let dir = activity_dir(&scope.local_dir);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(format!("{}.ndjson", self.session_id));
        let mut line = serde_json::to_vec(&event).map_err(|e| e.to_string())?;
        line.push(b'\n');
        // Una sola escritura por línea en modo append: un lector concurrente
        // ve líneas completas o un final parcial que `Tail` sabe esperar.
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(&line))
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}

pub fn activity_dir(local_dir: &Path) -> PathBuf {
    local_dir.join("activity")
}

/// ADR-0014 §Decision 3 para los escritores de vNext (actividad, snapshots de
/// operación, conflictos): la exclusión de Git de `.rationale-local/` antes
/// de su primer contenido. Una vez por proceso y proyecto: es idempotente,
/// pero cuesta un `git rev-parse`.
pub fn ensure_excluded(project_root: &Path) -> Result<(), String> {
    static ENSURED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    let ensured = ENSURED.get_or_init(|| Mutex::new(HashSet::new()));
    if lock(ensured).contains(project_root) {
        return Ok(());
    }
    crate::agents::ensure_local_data_excluded(project_root, false)?;
    lock(ensured).insert(project_root.to_path_buf());
    Ok(())
}

fn session_files(local_dir: &Path) -> Vec<(PathBuf, SystemTime)> {
    let Ok(entries) = std::fs::read_dir(activity_dir(local_dir)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("ndjson") {
                return None;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some((path, modified))
        })
        .collect()
}

fn parse_lines(bytes: &[u8]) -> Vec<ActivityEvent> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}

fn order(events: &mut [ActivityEvent]) {
    events.sort_by(|a, b| {
        evaluation::parse_timestamp_millis(&a.timestamp)
            .cmp(&evaluation::parse_timestamp_millis(&b.timestamp))
            .then_with(|| a.session_id.cmp(&b.session_id))
            .then_with(|| a.seq.cmp(&b.seq))
    });
}

/// Los `limit` eventos más recientes de todas las sesiones, en orden temporal.
/// Una línea a medio escribir se ignora: no es JSON válido todavía.
pub fn read_recent(local_dir: &Path, limit: usize) -> Vec<ActivityEvent> {
    let mut events: Vec<ActivityEvent> = session_files(local_dir)
        .into_iter()
        .filter_map(|(path, _)| std::fs::read(path).ok())
        .flat_map(|bytes| parse_lines(&bytes))
        .collect();
    order(&mut events);
    let skip = events.len().saturating_sub(limit);
    events.split_off(skip)
}

/// Sigue el crecimiento de todas las sesiones: la base del stream en vivo de
/// la UI. Solo entrega líneas completas; un final parcial espera al siguiente
/// `poll`.
pub struct Tail {
    local_dir: PathBuf,
    offsets: HashMap<PathBuf, u64>,
    pending: HashMap<PathBuf, Vec<u8>>,
}

impl Tail {
    /// Empieza después de lo ya escrito: el historial se pide a `read_recent`.
    pub fn from_end(local_dir: &Path) -> Self {
        let offsets = session_files(local_dir)
            .into_iter()
            .filter_map(|(path, _)| {
                std::fs::metadata(&path)
                    .ok()
                    .map(|metadata| (path, metadata.len()))
            })
            .collect();
        Tail {
            local_dir: local_dir.to_path_buf(),
            offsets,
            pending: HashMap::new(),
        }
    }

    pub fn poll(&mut self) -> Vec<ActivityEvent> {
        let mut events = Vec::new();
        for (path, _) in session_files(&self.local_dir) {
            let Ok(len) = std::fs::metadata(&path).map(|metadata| metadata.len()) else {
                continue;
            };
            let offset = self.offsets.entry(path.clone()).or_insert(0);
            if len < *offset {
                // Archivo reemplazado o truncado: se relee desde el principio.
                *offset = 0;
                self.pending.remove(&path);
            }
            if len == *offset {
                continue;
            }
            let Ok(mut file) = std::fs::File::open(&path) else {
                continue;
            };
            if file.seek(SeekFrom::Start(*offset)).is_err() {
                continue;
            }
            let mut chunk = Vec::new();
            if file.take(len - *offset).read_to_end(&mut chunk).is_err() {
                continue;
            }
            *offset += chunk.len() as u64;
            let buffer = self.pending.entry(path).or_default();
            buffer.extend_from_slice(&chunk);
            if let Some(last_newline) = buffer.iter().rposition(|byte| *byte == b'\n') {
                let complete: Vec<u8> = buffer.drain(..=last_newline).collect();
                events.extend(parse_lines(&complete));
            }
        }
        order(&mut events);
        events
    }
}

/// Retención: sesiones de más de 14 días fuera, y nunca más de 500 archivos.
/// Por antigüedad y no solo por conteo, para que una tanda de sesiones cortas
/// (una suite de tests, por ejemplo) no desaloje la historia real.
fn prune(local_dir: &Path) {
    let now = SystemTime::now();
    let mut files = session_files(local_dir);
    files.retain(|(path, modified)| {
        let stale = now
            .duration_since(*modified)
            .is_ok_and(|age| age > RETENTION);
        if stale {
            let _ = std::fs::remove_file(path);
        }
        !stale
    });
    if files.len() > RETAINED_SESSIONS {
        files.sort_by_key(|(_, modified)| *modified);
        let excess = files.len() - RETAINED_SESSIONS;
        for (path, _) in files.iter().take(excess) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Una sola línea, sin bytes de control, con techo de caracteres.
pub fn bounded(text: &str, max: usize) -> String {
    let single_line: String = canon::sanitize_control_chars(text)
        .chars()
        .map(|c| if c == '\n' || c == '\t' { ' ' } else { c })
        .collect();
    let trimmed = single_line.trim();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn ident(text: &str) -> String {
    bounded(text, MAX_IDENTIFIER_CHARS)
}

fn idents<'a>(items: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    items.into_iter().take(MAX_LIST_ITEMS).map(ident).collect()
}

/// El evento que merece una relación explicada que dejó de ser directa.
pub fn relationship_event_kind(state: RelationshipState) -> Option<&'static str> {
    match state {
        RelationshipState::Indirect => Some("relationship.indirect"),
        RelationshipState::Orphaned => Some("relationship.orphaned"),
        RelationshipState::Observed | RelationshipState::Unknown => None,
    }
}

/// Constructores de payload: la única vía para armar eventos, de modo que el
/// test guardián de minimización cubra todo lo que se escribe.
pub mod payload {
    use super::*;
    use crate::canon::{
        CommittedRecord, ConflictResolution, ConflictSummary, DiscardedCandidate, SupersededRecord,
    };

    pub fn session_started() -> Value {
        json!({"version": env!("RATIONALE_BUILD_VERSION"), "pid": std::process::id()})
    }

    pub fn agent_connected(actor: &ActorContext, protocol_version: Option<&str>) -> Value {
        json!({
            "client": ident(&actor.client),
            "client_source": ident(&actor.client_source),
            "protocol_version": protocol_version.map(ident),
        })
    }

    pub fn context_requested(target_spec: &str, intent: Option<&str>) -> Value {
        json!({
            "target": ident(target_spec),
            "intent": intent.map(|text| bounded(text, MAX_INTENT_CHARS)),
        })
    }

    pub fn provider_started(provider: Option<&str>) -> Value {
        json!({"provider": provider.map(ident)})
    }

    pub fn target_resolved(target: &OperationTarget) -> Value {
        json!({
            "resolved": target.node_key.is_some(),
            "file_path": target.file_path.as_deref().map(ident),
            "symbol": target.symbol.as_deref().map(ident),
            "qualified_name": target.qualified_name.as_deref().map(ident),
            "node_key": target.node_key.as_deref().map(ident),
        })
    }

    pub fn provider_finished(
        provider: Option<&str>,
        status: &str,
        coverage: &str,
        index_state: Option<&str>,
        latency_ms: u128,
    ) -> Value {
        json!({
            "provider": provider.map(ident),
            "status": ident(status),
            "coverage": ident(coverage),
            "index_state": index_state.map(ident),
            "latency_ms": latency_ms,
        })
    }

    /// Lo que la UI necesita para encender el grafo: claves consideradas y
    /// seleccionadas, Records servidos y tamaño del packet.
    pub fn packet_compiled(operation: &Operation, budget_overflow: Option<&str>) -> Value {
        let nodes = &operation.graph.nodes;
        let edges = &operation.graph.edges;
        json!({
            "target_node": operation.target.node_key.as_deref().map(ident),
            "considered_nodes": nodes.len(),
            "considered_node_keys": idents(nodes.iter().map(|n| n.key.as_str())),
            "selected_node_keys": idents(nodes.iter().filter(|n| n.selected).map(|n| n.key.as_str())),
            "considered_relationships": edges.len(),
            "considered_edge_keys": idents(edges.iter().map(|e| e.key.as_str())),
            "selected_edge_keys": idents(edges.iter().filter(|e| e.selected).map(|e| e.key.as_str())),
            "considered_records": operation.selection.considered_records,
            "selected_records": idents(operation.selection.selected_records.iter().map(String::as_str)),
            "packet_bytes": operation.selection.packet_bytes,
            "estimated_tokens": operation.selection.estimated_tokens,
            "budget_overflow": budget_overflow.map(ident),
        })
    }

    pub fn packet_delivered(latency_ms: u128, packet_bytes: usize) -> Value {
        json!({"latency_ms": latency_ms, "packet_bytes": packet_bytes})
    }

    pub fn change_finalized(candidates: usize, changed_files: usize) -> Value {
        json!({"candidates": candidates, "changed_files": changed_files})
    }

    pub fn capture_candidate(index: usize, kind: Option<&str>) -> Value {
        json!({"index": index, "kind": kind.map(ident), "well_formed": kind.is_some()})
    }

    pub fn capture_discarded(discarded: &DiscardedCandidate) -> Value {
        json!({
            "index": discarded.index,
            "reason": discarded.reason,
            "duplicate_of": discarded.duplicate_of.as_deref().map(ident),
        })
    }

    pub fn record_committed(committed: &CommittedRecord) -> Value {
        json!({
            "record_id": ident(&committed.id),
            "kind": ident(&committed.kind),
            "authority": ident(&committed.authority),
            "bindings": idents(committed.bindings.iter().map(String::as_str)),
            "relationships": idents(committed.relationships.iter().map(String::as_str)),
            "superseded": idents(committed.superseded.iter().map(String::as_str)),
        })
    }

    pub fn record_superseded(superseded: &SupersededRecord) -> Value {
        json!({
            "record_id": ident(&superseded.id),
            "superseded_by": ident(&superseded.superseded_by),
        })
    }

    pub fn relationship_state(assessment: &RelationshipAssessment) -> Value {
        json!({
            "record_id": ident(&assessment.record_id),
            "key": ident(&assessment.key),
            "kind": ident(&assessment.kind),
            "state": assessment.state.as_str(),
            "from": ident(&assessment.source.qualified_name),
            "to": ident(&assessment.target.qualified_name),
        })
    }

    pub fn conflict_detected(conflict: &ConflictSummary) -> Value {
        json!({
            "conflict_id": ident(&conflict.conflict_id),
            "pinned_record_id": ident(&conflict.pinned_record_id),
        })
    }

    pub fn conflict_resolved(resolution: &ConflictResolution) -> Value {
        json!({
            "conflict_id": ident(&resolution.conflict_id),
            "decision": resolution.decision,
            "pinned_record_id": ident(&resolution.pinned_record_id),
            "committed": idents(resolution.outcome.committed.iter().map(|c| c.id.as_str())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::{
        CaptureOutcome, CommittedRecord, ConflictDecision, ConflictResolution, ConflictSummary,
        DiscardReason, DiscardedCandidate, SupersededRecord,
    };
    use crate::operations::GraphNode;
    use crate::providers::{NodeBinding, RelationKind, StructuralPath};

    fn temp_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-activity-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn actor() -> ActorContext {
        ActorContext {
            client: "claude-code".to_string(),
            client_source: "flag".to_string(),
            session_id: None,
            operation_id: None,
        }
    }

    fn scope_in(dir: &Path) -> Scope {
        Scope::new(&dir.join(".rationale-local"), dir, "project-x", &actor())
    }

    fn session_events(dir: &Path, session: &str) -> Vec<ActivityEvent> {
        let path = dir
            .join(".rationale-local/activity")
            .join(format!("{session}.ndjson"));
        parse_lines(&std::fs::read(path).unwrap())
    }

    fn kinds(events: &[ActivityEvent]) -> Vec<&str> {
        events.iter().map(|e| e.kind.as_str()).collect()
    }

    #[test]
    fn events_are_append_only_per_session_with_monotonic_seq_and_rfc3339_time() {
        let dir = temp_dir();
        let recorder = Recorder::with_enabled(Some("session_one"), true);
        let scope = scope_in(&dir);
        recorder.emit(
            &scope,
            "context.requested",
            payload::context_requested("src/a.rs::f", Some("make f faster")),
        );
        recorder.emit(
            &scope.clone().with_operation("op_1"),
            "packet.compiled",
            json!({}),
        );

        let events = session_events(&dir, "session_one");
        assert_eq!(
            kinds(&events),
            vec!["session.started", "context.requested", "packet.compiled"]
        );
        assert_eq!(
            events.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        for event in &events {
            assert_eq!(event.schema_version, SCHEMA_VERSION);
            assert_eq!(event.project, "project-x");
            assert_eq!(event.actor.actor_type, "agent");
            assert!(evaluation::parse_timestamp_millis(&event.timestamp).is_some());
            assert!(event.timestamp.ends_with('Z'), "{}", event.timestamp);
        }
        assert_eq!(events[2].operation_id.as_deref(), Some("op_1"));
        assert_eq!(events[1].trace_id, events[2].trace_id);
        assert_eq!(events[1].payload["intent"], "make f faster");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_connection_heads_every_project_and_the_session_ends_everywhere() {
        let (first, second) = (temp_dir(), temp_dir());
        let recorder = Recorder::with_enabled(Some("session_two"), true);
        recorder.connected(None, payload::agent_connected(&actor(), Some("2024-11-05")));
        for dir in [&first, &second] {
            recorder.emit(
                &scope_in(dir),
                "context.requested",
                payload::context_requested("x", None),
            );
        }
        recorder.end(&actor());
        for dir in [&first, &second] {
            assert_eq!(
                kinds(&session_events(dir, "session_two")),
                vec![
                    "session.started",
                    "agent.connected",
                    "context.requested",
                    "session.ended"
                ]
            );
        }
        std::fs::remove_dir_all(first).ok();
        std::fs::remove_dir_all(second).ok();
    }

    #[test]
    fn sessions_merge_in_time_order_and_tail_delivers_only_new_complete_lines() {
        let dir = temp_dir();
        let local = dir.join(".rationale-local");
        let a = Recorder::with_enabled(Some("session_a"), true);
        let b = Recorder::with_enabled(Some("session_b"), true);
        let scope = scope_in(&dir);
        for (recorder, kind) in [(&a, "test.a1"), (&b, "test.b1"), (&a, "test.a2")] {
            recorder.emit(&scope, kind, json!({}));
            std::thread::sleep(Duration::from_millis(3));
        }
        let recent = read_recent(&local, 100);
        let own: Vec<&str> = kinds(&recent)
            .into_iter()
            .filter(|k| k.starts_with("test."))
            .collect();
        assert_eq!(own, vec!["test.a1", "test.b1", "test.a2"]);
        assert_eq!(kinds(&read_recent(&local, 2)), vec!["test.b1", "test.a2"]);

        let mut tail = Tail::from_end(&local);
        assert!(tail.poll().is_empty(), "el historial no se repite");
        a.emit(&scope, "test.a3", json!({}));
        assert_eq!(kinds(&tail.poll()), vec!["test.a3"]);

        let line = serde_json::to_string(&ActivityEvent {
            kind: "test.partial".to_string(),
            ..read_recent(&local, 1).remove(0)
        })
        .unwrap();
        let (head, rest) = line.split_at(line.len() / 2);
        let path = activity_dir(&local).join("session_b.ndjson");
        let append = |bytes: &[u8]| {
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap()
                .write_all(bytes)
                .unwrap();
        };
        append(head.as_bytes());
        assert!(tail.poll().is_empty(), "una línea a medias no es un evento");
        append(format!("{rest}\n").as_bytes());
        assert_eq!(kinds(&tail.poll()), vec!["test.partial"]);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn git_exclusion_is_installed_before_the_first_event() {
        let dir = temp_dir();
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["init", "-q"])
            .status()
            .unwrap();
        assert!(status.success());
        let recorder = Recorder::with_enabled(None, true);
        recorder.emit(&scope_in(&dir), "context.requested", json!({}));
        let exclude = std::fs::read_to_string(dir.join(".git/info/exclude")).unwrap();
        assert!(exclude.lines().any(|l| l.trim() == ".rationale-local/"));
        let porcelain = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["status", "--porcelain", "--untracked-files=all"])
            .output()
            .unwrap();
        assert!(
            !String::from_utf8_lossy(&porcelain.stdout).contains(".rationale-local"),
            "la actividad nunca aparece ante Git"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_disabled_recorder_writes_nothing() {
        let dir = temp_dir();
        let recorder = Recorder::with_enabled(Some("session_off"), false);
        recorder.emit(&scope_in(&dir), "context.requested", json!({}));
        recorder.end(&actor());
        assert!(!dir.join(".rationale-local").exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn retention_drops_stale_sessions_and_caps_the_count() {
        let dir = temp_dir();
        let local = dir.join(".rationale-local");
        let activity = activity_dir(&local);
        std::fs::create_dir_all(&activity).unwrap();
        let touch = |name: &str, age: Duration| {
            let path = activity.join(name);
            std::fs::write(&path, b"").unwrap();
            std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(SystemTime::now() - age)
                .unwrap();
        };
        touch("session_stale.ndjson", RETENTION + Duration::from_secs(60));
        for i in 0..(RETAINED_SESSIONS + 2) {
            touch(
                &format!("session_{i:04}.ndjson"),
                Duration::from_secs((RETAINED_SESSIONS + 2 - i) as u64 * 10),
            );
        }
        prune(&local);
        let remaining = session_files(&local);
        assert_eq!(remaining.len(), RETAINED_SESSIONS);
        let names: Vec<String> = remaining
            .iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(!names.contains(&"session_stale.ndjson".to_string()));
        assert!(
            !names.contains(&"session_0000.ndjson".to_string()),
            "sale la más antigua"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    /// Guardián de ADR-0017 (y de la promesa incumplida de ADR-0012 §Validation):
    /// ningún payload lleva texto libre sin techo, bytes de control, listas
    /// sin límite ni contenido de Records, código o respuestas humanas.
    #[test]
    fn payloads_are_bounded_and_carry_content_only_by_reference() {
        let huge = format!("{}\x1b[31m\n\t{}", "x".repeat(5000), "y".repeat(5000));
        let huge_actor = ActorContext {
            client: huge.clone(),
            client_source: huge.clone(),
            session_id: Some(huge.clone()),
            operation_id: Some(huge.clone()),
        };
        let target = OperationTarget {
            spec: huge.clone(),
            file_path: Some(huge.clone()),
            symbol: Some(huge.clone()),
            node_key: Some(huge.clone()),
            qualified_name: Some(huge.clone()),
        };
        let mut operation = Operation::new(&actor(), target.clone(), Some(huge.clone()), None);
        operation.graph.nodes = (0..200)
            .map(|_| GraphNode {
                key: huge.clone(),
                selected: true,
                ..Default::default()
            })
            .collect();
        operation.selection.selected_records = vec![huge.clone(); 200];
        let node = NodeBinding {
            file_path: huge.clone(),
            qualified_name: huge.clone(),
            symbol_kind: None,
        };
        let committed = CommittedRecord {
            index: 0,
            id: huge.clone(),
            kind: huge.clone(),
            statement: huge.clone(),
            authority: huge.clone(),
            path: huge.clone(),
            bindings: vec![huge.clone(); 200],
            relationships: vec![huge.clone(); 3],
            superseded: vec![huge.clone()],
            subject_id: Some(huge.clone()),
        };
        let payloads = vec![
            payload::session_started(),
            payload::agent_connected(&huge_actor, Some(&huge)),
            payload::context_requested(&huge, Some(&huge)),
            payload::provider_started(Some(&huge)),
            payload::target_resolved(&target),
            payload::provider_finished(Some(&huge), &huge, &huge, Some(&huge), 7),
            payload::packet_compiled(&operation, Some(&huge)),
            payload::packet_delivered(7, 8),
            payload::change_finalized(3, 4),
            payload::capture_candidate(0, Some(&huge)),
            payload::capture_discarded(&DiscardedCandidate {
                index: 0,
                reason: DiscardReason::MechanicalNoise,
                detail: huge.clone(),
                duplicate_of: Some(huge.clone()),
                statement: huge.clone(),
            }),
            payload::record_committed(&committed),
            payload::record_superseded(&SupersededRecord {
                id: huge.clone(),
                superseded_by: huge.clone(),
            }),
            payload::relationship_state(&RelationshipAssessment {
                record_id: huge.clone(),
                binding_id: huge.clone(),
                key: huge.clone(),
                source: node.clone(),
                kind: huge.clone(),
                target: node.clone(),
                state: RelationshipState::Orphaned,
                path: Some(StructuralPath {
                    nodes: vec![node.clone(), node.clone()],
                    kinds: vec![RelationKind::Calls],
                }),
                detail: huge.clone(),
            }),
            payload::conflict_detected(&ConflictSummary {
                conflict_id: huge.clone(),
                pinned_record_id: huge.clone(),
                pinned_statement: huge.clone(),
                candidate_statement: huge.clone(),
                question: huge.clone(),
                options: vec![],
            }),
            payload::conflict_resolved(&ConflictResolution {
                conflict_id: huge.clone(),
                decision: ConflictDecision::KeepPinned,
                pinned_record_id: huge.clone(),
                outcome: CaptureOutcome {
                    committed: vec![committed.clone()],
                    ..Default::default()
                },
            }),
        ];

        const FORBIDDEN_KEYS: [&str; 12] = [
            "statement",
            "rationale",
            "detail",
            "question",
            "summary",
            "source",
            "code",
            "diff",
            "evidence",
            "pinned_statement",
            "candidate_statement",
            "human_answer",
        ];
        fn walk(value: &Value, path: &str) {
            match value {
                Value::String(text) => {
                    assert!(
                        text.chars().count() <= MAX_INTENT_CHARS.max(MAX_IDENTIFIER_CHARS),
                        "{path}: {} caracteres",
                        text.chars().count()
                    );
                    assert!(!text.chars().any(char::is_control), "{path}: control");
                }
                Value::Array(items) => {
                    assert!(items.len() <= MAX_LIST_ITEMS, "{path}: {}", items.len());
                    for (i, item) in items.iter().enumerate() {
                        walk(item, &format!("{path}[{i}]"));
                    }
                }
                Value::Object(map) => {
                    for (key, item) in map {
                        assert!(
                            !FORBIDDEN_KEYS.contains(&key.as_str()),
                            "{path}.{key}: el contenido viaja por referencia"
                        );
                        walk(item, &format!("{path}.{key}"));
                    }
                }
                _ => {}
            }
        }
        for (i, value) in payloads.iter().enumerate() {
            walk(value, &format!("payload[{i}]"));
        }
        assert_eq!(
            payloads[2]["intent"].as_str().unwrap().chars().count(),
            MAX_INTENT_CHARS
        );
    }
}
