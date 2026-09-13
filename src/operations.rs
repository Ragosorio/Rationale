//! Operaciones — el hilo que une un cambio de punta a punta (vNext).
//!
//! `prepare_change` abre una operación y devuelve su `operation_id`; ese id
//! enlaza el contexto compilado, el trabajo del agente, `finalize_change`, la
//! captura y la actividad que ve la UI. El snapshot de la operación vive en
//! `.rationale-local/operations/<id>.json`: es estado local derivado —
//! regenerable, nunca canon — y conserva exactamente el subgrafo que recibió
//! el agente para que la UI lo muestre sin volver a consultar al proveedor.

use crate::canon::ActorContext;
use crate::evaluation;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "rationale/operation/1";
/// Techo de snapshots conservados: la UI muestra lo reciente, y un proyecto
/// activo no debe acumular miles de archivos locales.
const RETAINED_OPERATIONS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationTarget {
    pub spec: String,
    pub file_path: Option<String>,
    pub symbol: Option<String>,
    pub node_key: Option<String>,
    pub qualified_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    pub key: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
    pub qualified_name: String,
    pub role: String,
    pub selected: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GraphEdge {
    pub key: String,
    pub source: String,
    pub kind: String,
    pub target: String,
    pub selected: bool,
    /// `observed` para aristas del proveedor; `indirect`, `orphaned` o
    /// `unknown` para relaciones que solo existen como explicación del canon.
    pub state: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub record_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OperationGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Selection {
    pub considered_nodes: usize,
    pub selected_nodes: usize,
    pub considered_relationships: usize,
    pub selected_relationships: usize,
    pub considered_records: usize,
    pub selected_records: Vec<String>,
    pub packet_bytes: usize,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinalizeSummary {
    pub finalized_at: String,
    pub committed: Vec<String>,
    pub discarded: usize,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Operation {
    pub schema_version: String,
    pub operation_id: String,
    pub created_at: String,
    pub actor: ActorContext,
    pub target: OperationTarget,
    pub intent: Option<String>,
    /// HEAD en el momento de `prepare_change`: la base honesta del diff
    /// cuando `finalize_change` no declara otra.
    pub base_revision: Option<String>,
    pub graph: OperationGraph,
    pub selection: Selection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finalized: Option<FinalizeSummary>,
}

impl Operation {
    pub fn new(
        actor: &ActorContext,
        target: OperationTarget,
        intent: Option<String>,
        base_revision: Option<String>,
    ) -> Self {
        let operation_id = crate::canon::generate_id("op");
        Operation {
            schema_version: SCHEMA.to_string(),
            actor: ActorContext {
                operation_id: Some(operation_id.clone()),
                ..actor.clone()
            },
            operation_id,
            created_at: evaluation::now_rfc3339_millis(),
            target,
            intent,
            base_revision,
            graph: OperationGraph::default(),
            selection: Selection::default(),
            finalized: None,
        }
    }
}

pub fn operations_dir(local_dir: &Path) -> PathBuf {
    local_dir.join("operations")
}

fn operation_path(local_dir: &Path, operation_id: &str) -> Option<PathBuf> {
    crate::storage::validate_safe_id(operation_id).ok()?;
    operation_id
        .starts_with("op_")
        .then(|| operations_dir(local_dir).join(format!("{operation_id}.json")))
}

pub fn save(local_dir: &Path, operation: &Operation) -> Result<PathBuf, String> {
    let path = operation_path(local_dir, &operation.operation_id)
        .ok_or_else(|| format!("operation_id inválido: {}", operation.operation_id))?;
    let mut bytes = serde_json::to_vec_pretty(operation)
        .map_err(|e| format!("no se pudo serializar la operación: {e}"))?;
    bytes.push(b'\n');
    crate::storage::atomic_write_bytes(&path, &bytes)
        .map_err(|e| format!("no se pudo guardar {}: {e}", path.display()))?;
    prune(local_dir);
    Ok(path)
}

pub fn load(local_dir: &Path, operation_id: &str) -> Option<Operation> {
    let path = operation_path(local_dir, operation_id)?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Operaciones más recientes primero (los ids ordenan por tiempo de creación).
pub fn list_recent(local_dir: &Path, limit: usize) -> Vec<Operation> {
    let Ok(entries) = std::fs::read_dir(operations_dir(local_dir)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("op_") && name.ends_with(".json"))
        .collect();
    names.sort();
    names.reverse();
    names
        .into_iter()
        .filter_map(|name| load(local_dir, name.trim_end_matches(".json")))
        .take(limit)
        .collect()
}

fn prune(local_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(operations_dir(local_dir)) else {
        return;
    };
    let mut names: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("op_") && n.ends_with(".json"))
        })
        .collect();
    if names.len() <= RETAINED_OPERATIONS {
        return;
    }
    names.sort();
    for stale in &names[..names.len() - RETAINED_OPERATIONS] {
        let _ = std::fs::remove_file(stale);
    }
}

/// Registra el cierre de la operación. Un id desconocido (otra máquina,
/// snapshot podado) no es un error de `finalize_change`: solo no se enlaza.
pub fn record_finalize(
    local_dir: &Path,
    operation_id: &str,
    summary: FinalizeSummary,
) -> Result<(), String> {
    let Some(mut operation) = load(local_dir, operation_id) else {
        return Err(format!("operación '{operation_id}' desconocida"));
    };
    operation.finalized = Some(summary);
    save(local_dir, &operation).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_dir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-operations-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn actor() -> ActorContext {
        ActorContext {
            client: "claude-code".to_string(),
            client_source: "flag".to_string(),
            session_id: Some("session_x".to_string()),
            operation_id: None,
        }
    }

    fn target() -> OperationTarget {
        OperationTarget {
            spec: "src/payments.rs::create_link".to_string(),
            file_path: Some("src/payments.rs".to_string()),
            symbol: Some("create_link".to_string()),
            node_key: None,
            qualified_name: None,
        }
    }

    #[test]
    fn operations_roundtrip_and_link_finalize() {
        let dir = local_dir();
        let operation = Operation::new(
            &actor(),
            target(),
            Some("configurable expiry".into()),
            Some("abc".into()),
        );
        assert!(operation.operation_id.starts_with("op_"));
        assert_eq!(
            operation.actor.operation_id.as_deref(),
            Some(operation.operation_id.as_str())
        );
        save(&dir, &operation).unwrap();

        let loaded = load(&dir, &operation.operation_id).unwrap();
        assert_eq!(loaded, operation);

        record_finalize(
            &dir,
            &operation.operation_id,
            FinalizeSummary {
                finalized_at: evaluation::now_iso8601(),
                committed: vec!["decision.x".to_string()],
                discarded: 1,
                conflicts: vec![],
            },
        )
        .unwrap();
        assert_eq!(
            load(&dir, &operation.operation_id)
                .unwrap()
                .finalized
                .unwrap()
                .committed,
            vec!["decision.x".to_string()]
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unsafe_or_foreign_ids_never_touch_the_filesystem() {
        let dir = local_dir();
        for id in ["../escape", "op_../../x", "conflict_abc", ""] {
            assert!(load(&dir, id).is_none(), "{id}");
            assert!(operation_path(&dir, id).is_none(), "{id}");
        }
        assert!(record_finalize(
            &dir,
            "op_missing",
            FinalizeSummary {
                finalized_at: String::new(),
                committed: vec![],
                discarded: 0,
                conflicts: vec![],
            }
        )
        .is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn recent_operations_are_newest_first_and_retention_is_bounded() {
        let dir = local_dir();
        let mut ids = Vec::new();
        for _ in 0..(RETAINED_OPERATIONS + 5) {
            let operation = Operation::new(&actor(), target(), None, None);
            ids.push(operation.operation_id.clone());
            save(&dir, &operation).unwrap();
        }
        let files = std::fs::read_dir(operations_dir(&dir)).unwrap().count();
        assert_eq!(files, RETAINED_OPERATIONS);
        let recent = list_recent(&dir, 3);
        assert_eq!(recent.len(), 3);
        assert!(recent[0].operation_id >= recent[1].operation_id);
        std::fs::remove_dir_all(dir).ok();
    }
}
