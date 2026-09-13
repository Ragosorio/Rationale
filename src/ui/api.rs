//! Vistas de solo lectura para la UI. Leen el mismo estado local que la CLI y
//! el servidor MCP —canon, snapshots de operación, actividad— y nunca
//! escriben: la UI observa, no es una segunda autoridad.

use crate::operations::{GraphEdge, GraphNode, Operation};
use crate::{activity, canon, configuration, operations, relationships, revision, storage};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Techo de snapshots que recorren las vistas de detalle (el mismo que
/// retiene `operations`).
const DETAIL_OPERATIONS: usize = 200;

pub type ApiResult = Result<Value, (u16, String)>;

pub struct Project {
    pub root: PathBuf,
    pub rationale_dir: PathBuf,
    pub local_dir: PathBuf,
    pub id: String,
}

impl Project {
    pub fn load(start: &Path) -> Result<Self, String> {
        let config = configuration::load(start).map_err(|e| e.to_string())?;
        Ok(Project {
            local_dir: configuration::find_rationale_local(&config.project_root),
            root: config.project_root,
            rationale_dir: config.rationale_dir,
            id: config.project_id,
        })
    }

    fn records(&self) -> Vec<storage::Record> {
        storage::list_records(&self.rationale_dir.join("records")).unwrap_or_default()
    }
}

fn record_status(record: &storage::Record, superseded: &HashSet<String>) -> &'static str {
    if storage::is_revoked(record) {
        "revoked"
    } else if superseded.contains(&record.id) {
        "superseded"
    } else {
        "active"
    }
}

fn record_view(record: &storage::Record, superseded: &HashSet<String>, project_id: &str) -> Value {
    json!({
        "id": record.id,
        "kind": record.kind,
        "severity": record.severity,
        "statement": record.statement,
        "rationale": record.rationale,
        "authority": storage::authority_label(record),
        "provenance": storage::provenance_kind(record).as_str(),
        "status": record_status(record, superseded),
        "supersedes": record.supersedes,
        "superseded_by": storage::superseded_by(record),
        "bindings": record.binding_declarations.iter().map(|binding| json!({
            "kind": binding.kind,
            "path": binding.path_hint,
            "structural_id": binding.structural_id,
            "provisional": binding.provisional,
        })).collect::<Vec<_>>(),
        "relationships": record.relationship_bindings.iter().map(|binding| json!({
            "key": relationships::relationship_key(project_id, binding),
            "kind": binding.kind,
            "source": binding.source.qualified_name,
            "source_file": binding.source.file_path,
            "source_key": binding.source.key(project_id),
            "target": binding.target.qualified_name,
            "target_file": binding.target.file_path,
            "target_key": binding.target.key(project_id),
        })).collect::<Vec<_>>(),
    })
}

/// Records referenciados, por id, listos para superponer al grafo.
fn records_by_id(project: &Project, ids: &BTreeSet<String>) -> Map<String, Value> {
    if ids.is_empty() {
        return Map::new();
    }
    let records = project.records();
    let superseded = canon::superseded_ids(&records);
    records
        .iter()
        .filter(|record| ids.contains(&record.id))
        .map(|record| {
            (
                record.id.clone(),
                record_view(record, &superseded, &project.id),
            )
        })
        .collect()
}

fn operation_summary(operation: &Operation) -> Value {
    json!({
        "operation_id": operation.operation_id,
        "created_at": operation.created_at,
        "actor": operation.actor,
        "target": operation.target,
        "intent": operation.intent,
        "selection": operation.selection,
        "finalized": operation.finalized,
    })
}

/// Sesiones de actividad, la más reciente primero.
pub fn sessions(events: &[activity::ActivityEvent]) -> Vec<Value> {
    let mut by_session: BTreeMap<&str, Vec<&activity::ActivityEvent>> = BTreeMap::new();
    for event in events {
        by_session
            .entry(event.session_id.as_str())
            .or_default()
            .push(event);
    }
    let mut list: Vec<Value> = by_session
        .into_iter()
        .filter_map(|(session_id, events)| {
            let first = events.first()?;
            let last = events.last()?;
            let mut operations: Vec<&str> = Vec::new();
            for event in &events {
                if let Some(id) = event.operation_id.as_deref() {
                    if !operations.contains(&id) {
                        operations.push(id);
                    }
                }
            }
            Some(json!({
                "session_id": session_id,
                "actor": last.actor,
                "started_at": first.timestamp,
                "last_event_at": last.timestamp,
                "last_kind": last.kind,
                "ended": events.iter().any(|event| event.kind == "session.ended"),
                "events": events.len(),
                "operations": operations.len(),
                "last_operation_id": operations.last(),
            }))
        })
        .collect();
    // Timestamps RFC3339 del mismo formato: el orden lexicográfico es temporal.
    list.sort_by(|a, b| {
        b["last_event_at"]
            .as_str()
            .cmp(&a["last_event_at"].as_str())
    });
    list
}

pub fn meta(project: &Project) -> Value {
    let records = project.records();
    let superseded = canon::superseded_ids(&records);
    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    for record in &records {
        *by_status
            .entry(record_status(record, &superseded))
            .or_default() += 1;
    }
    let pinned = records
        .iter()
        .filter(|record| {
            storage::record_authority(record) == storage::RecordAuthority::Pinned
                && record_status(record, &superseded) == "active"
        })
        .count();
    let snapshot = revision::snapshot(&project.root);
    let events = activity::read_recent(&project.local_dir, 5000);
    json!({
        "name": "rationale",
        "version": env!("RATIONALE_BUILD_VERSION"),
        "project": {"id": project.id, "root": project.root.display().to_string()},
        "git": {"revision": snapshot.head, "dirty": snapshot.working_tree_dirty},
        "canon": {
            "records": records.len(),
            "active": by_status.get("active").copied().unwrap_or(0),
            "superseded": by_status.get("superseded").copied().unwrap_or(0),
            "revoked": by_status.get("revoked").copied().unwrap_or(0),
            "pinned": pinned,
        },
        "conflicts_pending": canon::list_pending_conflicts(&project.local_dir).len(),
        "latest_operation": operations::list_recent(&project.local_dir, 1)
            .first()
            .map(operation_summary),
        "sessions": sessions(&events),
    })
}

pub fn activity(project: &Project, limit: usize) -> Value {
    json!(activity::read_recent(
        &project.local_dir,
        limit.clamp(1, 5000)
    ))
}

pub fn operations(project: &Project, limit: usize) -> Value {
    json!(
        operations::list_recent(&project.local_dir, limit.clamp(1, 200))
            .iter()
            .map(operation_summary)
            .collect::<Vec<_>>()
    )
}

pub fn operation(project: &Project, operation_id: &str) -> ApiResult {
    operations::load(&project.local_dir, operation_id)
        .map(|operation| json!(operation))
        .ok_or_else(|| (404, format!("operación '{operation_id}' desconocida")))
}

pub fn records(project: &Project) -> Value {
    let records = project.records();
    let superseded = canon::superseded_ids(&records);
    json!(records
        .iter()
        .map(|record| record_view(record, &superseded, &project.id))
        .collect::<Vec<_>>())
}

pub fn conflicts(project: &Project) -> Value {
    json!(canon::list_pending_conflicts(&project.local_dir))
}

struct NodeUnion {
    node: GraphNode,
    selected: bool,
    operations: Vec<String>,
    last_seen: String,
}

struct EdgeUnion {
    edge: GraphEdge,
    selected: bool,
    operations: Vec<String>,
}

/// El subgrafo de trabajo: el de una operación, o la unión de las `recent`
/// más nuevas. De cada nodo o arista manda su aparición más reciente (rol,
/// estado); `selected` es verdadero si alguna operación lo entregó.
pub fn graph(project: &Project, operation_id: Option<&str>, recent: usize) -> ApiResult {
    let operations: Vec<Operation> = match operation_id {
        Some(id) => vec![operations::load(&project.local_dir, id)
            .ok_or_else(|| (404, format!("operación '{id}' desconocida")))?],
        None => operations::list_recent(&project.local_dir, recent.clamp(1, 50)),
    };

    let mut node_order: Vec<String> = Vec::new();
    let mut nodes: HashMap<String, NodeUnion> = HashMap::new();
    let mut edge_order: Vec<String> = Vec::new();
    let mut edges: HashMap<String, EdgeUnion> = HashMap::new();
    for operation in &operations {
        for node in &operation.graph.nodes {
            let entry = nodes.entry(node.key.clone()).or_insert_with(|| {
                node_order.push(node.key.clone());
                NodeUnion {
                    node: node.clone(),
                    selected: false,
                    operations: Vec::new(),
                    last_seen: operation.created_at.clone(),
                }
            });
            entry.selected |= node.selected;
            entry.operations.push(operation.operation_id.clone());
        }
        for edge in &operation.graph.edges {
            let entry = edges.entry(edge.key.clone()).or_insert_with(|| {
                edge_order.push(edge.key.clone());
                EdgeUnion {
                    edge: edge.clone(),
                    selected: false,
                    operations: Vec::new(),
                }
            });
            entry.selected |= edge.selected;
            entry.operations.push(operation.operation_id.clone());
        }
    }

    let record_ids: BTreeSet<String> = nodes
        .values()
        .flat_map(|union| union.node.record_ids.iter().cloned())
        .chain(
            edges
                .values()
                .flat_map(|union| union.edge.record_ids.iter().cloned()),
        )
        .collect();

    Ok(json!({
        "project": project.id,
        "focus": operations.first().map(|operation| json!({
            "operation_id": operation.operation_id,
            "node_key": operation.target.node_key,
            "created_at": operation.created_at,
        })),
        "operations": operations.iter().map(operation_summary).collect::<Vec<_>>(),
        "nodes": node_order.iter().map(|key| {
            let union = &nodes[key];
            let node = &union.node;
            json!({
                "key": node.key,
                "name": node.name,
                "label": node.label,
                "file_path": node.file_path,
                "qualified_name": node.qualified_name,
                "role": node.role,
                "start_line": node.start_line,
                "record_ids": node.record_ids,
                "selected": union.selected,
                "operations": union.operations,
                "last_seen": union.last_seen,
            })
        }).collect::<Vec<_>>(),
        "edges": edge_order.iter().map(|key| {
            let union = &edges[key];
            let edge = &union.edge;
            json!({
                "key": edge.key,
                "source": edge.source,
                "target": edge.target,
                "kind": edge.kind,
                "state": edge.state,
                "record_ids": edge.record_ids,
                "selected": union.selected,
                "operations": union.operations,
            })
        }).collect::<Vec<_>>(),
        "records": records_by_id(project, &record_ids),
    }))
}

fn valid_key(key: &str, prefix: &str) -> bool {
    key.strip_prefix(prefix)
        .is_some_and(|hex| hex.len() == 16 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn node_ref(operation: &Operation, key: &str) -> Value {
    match operation.graph.nodes.iter().find(|node| node.key == key) {
        Some(node) => json!({
            "key": node.key,
            "name": node.name,
            "qualified_name": node.qualified_name,
            "file_path": node.file_path,
            "label": node.label,
        }),
        None => json!({"key": key}),
    }
}

/// Detalle de un nodo: su aparición más reciente, relaciones vistas en las
/// operaciones, Records que lo explican y las operaciones donde apareció.
pub fn node(project: &Project, key: &str) -> ApiResult {
    if !valid_key(key, "n_") {
        return Err((400, "clave de nodo inválida".to_string()));
    }
    let operations = operations::list_recent(&project.local_dir, DETAIL_OPERATIONS);
    let mut latest: Option<GraphNode> = None;
    let mut appearances = Vec::new();
    let mut seen_edges: BTreeSet<String> = BTreeSet::new();
    let mut incoming = Vec::new();
    let mut outgoing = Vec::new();
    let mut record_ids: BTreeSet<String> = BTreeSet::new();
    for operation in &operations {
        let Some(node) = operation.graph.nodes.iter().find(|node| node.key == key) else {
            continue;
        };
        if latest.is_none() {
            latest = Some(node.clone());
        }
        record_ids.extend(node.record_ids.iter().cloned());
        appearances.push(json!({
            "operation_id": operation.operation_id,
            "created_at": operation.created_at,
            "intent": operation.intent,
            "role": node.role,
            "selected": node.selected,
        }));
        for edge in &operation.graph.edges {
            let outbound = edge.source == key;
            if !(outbound || edge.target == key) || !seen_edges.insert(edge.key.clone()) {
                continue;
            }
            record_ids.extend(edge.record_ids.iter().cloned());
            let other = if outbound { &edge.target } else { &edge.source };
            let view = json!({
                "key": edge.key,
                "kind": edge.kind,
                "state": edge.state,
                "record_ids": edge.record_ids,
                "node": node_ref(operation, other),
            });
            if outbound {
                outgoing.push(view);
            } else {
                incoming.push(view);
            }
        }
    }
    let Some(node) = latest else {
        return Err((
            404,
            "el nodo no aparece en ninguna operación reciente".to_string(),
        ));
    };
    Ok(json!({
        "node": node,
        "incoming": incoming,
        "outgoing": outgoing,
        "records": records_by_id(project, &record_ids),
        "appearances": appearances,
    }))
}

/// Detalle de una relación: extremos, estado actual, historia de estados en
/// orden cronológico y los Records que explican por qué existe.
pub fn edge(project: &Project, key: &str) -> ApiResult {
    if !valid_key(key, "e_") {
        return Err((400, "clave de relación inválida".to_string()));
    }
    let operations = operations::list_recent(&project.local_dir, DETAIL_OPERATIONS);
    let mut current: Option<(GraphEdge, Value, Value)> = None;
    let mut history = Vec::new();
    let mut record_ids: BTreeSet<String> = BTreeSet::new();
    for operation in &operations {
        let Some(edge) = operation.graph.edges.iter().find(|edge| edge.key == key) else {
            continue;
        };
        if current.is_none() {
            current = Some((
                edge.clone(),
                node_ref(operation, &edge.source),
                node_ref(operation, &edge.target),
            ));
        }
        record_ids.extend(edge.record_ids.iter().cloned());
        history.push(json!({
            "operation_id": operation.operation_id,
            "created_at": operation.created_at,
            "state": edge.state,
            "selected": edge.selected,
        }));
    }
    // Una relación explicada por el canon también se reconoce por su clave,
    // aunque ninguna operación reciente la haya tocado.
    let canon_records = project.records();
    for record in &canon_records {
        if record
            .relationship_bindings
            .iter()
            .any(|binding| relationships::relationship_key(&project.id, binding) == key)
        {
            record_ids.insert(record.id.clone());
        }
    }
    history.reverse();
    let edge_view = match current {
        Some((edge, source, target)) => json!({
            "key": edge.key,
            "kind": edge.kind,
            "state": edge.state,
            "source": source,
            "target": target,
        }),
        None if !record_ids.is_empty() => json!({"key": key, "state": "unknown"}),
        None => {
            return Err((
                404,
                "la relación no aparece en operaciones ni en el canon".to_string(),
            ))
        }
    };
    Ok(json!({
        "edge": edge_view,
        "history": history,
        "records": records_by_id(project, &record_ids),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(session: &str, seq: u64, timestamp: &str, kind: &str) -> activity::ActivityEvent {
        activity::ActivityEvent {
            schema_version: activity::SCHEMA_VERSION.to_string(),
            session_id: session.to_string(),
            seq,
            trace_id: None,
            operation_id: Some(format!("op_{session}")),
            timestamp: timestamp.to_string(),
            actor: activity::Actor {
                actor_type: "agent".to_string(),
                client: "claude-code".to_string(),
                client_source: "flag".to_string(),
            },
            project: "p".to_string(),
            kind: kind.to_string(),
            payload: json!({}),
        }
    }

    #[test]
    fn sessions_are_grouped_newest_first_and_know_when_they_ended() {
        let events = vec![
            event("a", 1, "2026-09-12T10:00:00.000Z", "session.started"),
            event("b", 1, "2026-09-12T10:00:01.000Z", "session.started"),
            event("a", 2, "2026-09-12T10:00:02.000Z", "session.ended"),
            event("b", 2, "2026-09-12T10:00:03.000Z", "packet.compiled"),
        ];
        let list = sessions(&events);
        assert_eq!(list[0]["session_id"], "b");
        assert_eq!(list[0]["ended"], false);
        assert_eq!(list[1]["session_id"], "a");
        assert_eq!(list[1]["ended"], true);
        assert_eq!(list[1]["operations"], 1);
    }

    #[test]
    fn keys_are_validated_before_touching_state() {
        assert!(valid_key("n_0123456789abcdef", "n_"));
        assert!(!valid_key("n_0123", "n_"));
        assert!(!valid_key("e_0123456789abcdef", "n_"));
        assert!(!valid_key("n_../../../etc/pas", "n_"));
    }
}
