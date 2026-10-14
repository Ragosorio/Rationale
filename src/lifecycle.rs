//! Estado de vida derivado de cada Record — la única fuente de «¿sigue
//! vigente?» para retrieval, `explain_target`, doctor y la Control Room.
//!
//! Antes cada consumidor decidía por su cuenta: `explain_target` no filtraba
//! nada (servía Records reemplazados como si gobernaran), retrieval solo
//! miraba el lado viejo de la supersesión (`superseded_by`) y la UI y el canon
//! usaban `canon::superseded_ids`. Un mismo Record podía estar vigente para
//! una herramienta y reemplazado para otra. El estado se deriva aquí, una vez
//! por consulta, y nunca se persiste: la historia queda en el propio canon.
//!
//! Compatibilidad: una retirada masiva (`rationale retire`) escribe
//! `lifecycle.status: revoked` con un evento `retired`, así que un binario
//! anterior la trata como revocada (inactiva) aunque no conozca el estado
//! `retired`.

use crate::storage::{self, Record, RecordAuthority};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Active,
    Superseded,
    Revoked,
    /// Revocado por la retirada explícita de un alcance (módulo eliminado).
    Retired,
}

impl RecordStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Revoked => "revoked",
            Self::Retired => "retired",
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Vista de estado de un canon cargado. Se construye una vez por consulta.
pub struct StatusIndex {
    superseded: HashSet<String>,
    /// Reemplazo conocido de cada Record reemplazado (lado viejo o nuevo).
    replaced_by: HashMap<String, String>,
    status: HashMap<String, RecordStatus>,
}

fn last_event_type(record: &Record) -> Option<String> {
    let lifecycle = record
        .extra
        .get(yaml_serde::Value::String("lifecycle".to_string()))?;
    let events = lifecycle
        .as_mapping()?
        .get(yaml_serde::Value::String("events".to_string()))?
        .as_sequence()?;
    events
        .last()?
        .as_mapping()?
        .get(yaml_serde::Value::String("type".to_string()))?
        .as_str()
        .map(str::to_string)
}

/// `true` cuando el Record fue revocado por `rationale retire`.
pub fn is_retired(record: &Record) -> bool {
    storage::is_revoked(record) && last_event_type(record).as_deref() == Some("retired")
}

impl StatusIndex {
    /// Supersesión derivada de ambos lados: el `superseded_by`/estado escrito
    /// en el Record viejo y el `supersedes` del nuevo (que cubre una fusión de
    /// ramas donde el archivo viejo no se actualizó). Un Record `pinned` solo
    /// cuenta como reemplazado si él mismo lo declara: un `supersedes` escrito
    /// por otro nunca lo desplaza en silencio. Un Record revocado no reemplaza
    /// a nadie.
    pub fn build(records: &[Record]) -> Self {
        let by_id: HashMap<&str, &Record> = records.iter().map(|r| (r.id.as_str(), r)).collect();
        let mut superseded = HashSet::new();
        let mut replaced_by = HashMap::new();
        for record in records {
            let marked = storage::superseded_by(record);
            if marked.is_some() || storage::lifecycle_status(record) == Some("superseded") {
                superseded.insert(record.id.clone());
            }
            if let Some(replacement) = marked {
                replaced_by.insert(record.id.clone(), replacement.to_string());
            }
        }
        for record in records.iter().filter(|r| !storage::is_revoked(r)) {
            for target in &record.supersedes {
                let Some(existing) = by_id.get(target.as_str()) else {
                    continue;
                };
                let pinned = storage::record_authority(existing) == RecordAuthority::Pinned;
                if !pinned || superseded.contains(target) {
                    superseded.insert(target.clone());
                    replaced_by
                        .entry(target.clone())
                        .or_insert_with(|| record.id.clone());
                }
            }
        }
        let status = records
            .iter()
            .map(|record| {
                let status = if is_retired(record) {
                    RecordStatus::Retired
                } else if storage::is_revoked(record) {
                    RecordStatus::Revoked
                } else if superseded.contains(&record.id) {
                    RecordStatus::Superseded
                } else {
                    RecordStatus::Active
                };
                (record.id.clone(), status)
            })
            .collect();
        StatusIndex {
            superseded,
            replaced_by,
            status,
        }
    }

    pub fn status(&self, record: &Record) -> RecordStatus {
        self.status
            .get(&record.id)
            .copied()
            .unwrap_or(RecordStatus::Active)
    }

    pub fn status_of(&self, id: &str) -> Option<RecordStatus> {
        self.status.get(id).copied()
    }

    pub fn is_active(&self, record: &Record) -> bool {
        self.status(record).is_active()
    }

    /// El Record que reemplazó a este, si se conoce.
    pub fn replaced_by(&self, id: &str) -> Option<&str> {
        self.replaced_by.get(id).map(String::as_str)
    }

    /// El Record vigente al final de la cadena de reemplazos (`a → b → c`
    /// devuelve `c`). Se detiene ante ciclos o Records desconocidos.
    pub fn current_replacement(&self, id: &str) -> Option<&str> {
        let mut seen = HashSet::new();
        let mut current = self.replaced_by(id)?;
        while seen.insert(current) {
            match self.replaced_by(current) {
                Some(next) => current = next,
                None => break,
            }
        }
        Some(current)
    }

    pub fn superseded_ids(&self) -> &HashSet<String> {
        &self.superseded
    }
}

/// Prefijos de id de Record según el schema (`<kind>.<slug>`).
const RECORD_KINDS: [&str; 4] = ["constraint.", "decision.", "risk.", "exception."];

/// Ids de Record citados en un texto (`constraint.rotate-secrets-90d`).
pub fn mentioned_ids(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for kind in RECORD_KINDS {
        let mut rest = text;
        while let Some(at) = rest.find(kind) {
            let boundary = rest[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.' || c == '-'));
            let tail = &rest[at + kind.len()..];
            let slug: String = tail
                .chars()
                .take_while(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '.'
                })
                .collect();
            let slug = slug.trim_end_matches(['.', '-']);
            if boundary && !slug.is_empty() {
                let id = format!("{kind}{slug}");
                if !found.contains(&id) {
                    found.push(id);
                }
            }
            rest = &rest[at + kind.len()..];
        }
    }
    found
}

/// Los Records a los que una excepción se aplica: `exception_to` explícito
/// o, en Records escritos antes de que existiera el campo, los ids que cita
/// su texto. `true` cuando el vínculo es explícito.
pub fn exception_targets(record: &Record) -> (Vec<String>, bool) {
    let explicit: Vec<String> = match record
        .extra
        .get(yaml_serde::Value::String("exception_to".to_string()))
    {
        Some(yaml_serde::Value::Sequence(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(yaml_serde::Value::String(id)) => vec![id.clone()],
        _ => Vec::new(),
    };
    if !explicit.is_empty() {
        return (explicit, true);
    }
    let mut text = record.statement.clone();
    if let Some(rationale) = &record.rationale {
        text.push(' ');
        text.push_str(rationale);
    }
    let cited = mentioned_ids(&text)
        .into_iter()
        .filter(|id| id != &record.id && !id.starts_with("exception."))
        .collect();
    (cited, false)
}

/// Una excepción vigente cuya regla ya no gobierna: todos los Records a los
/// que se aplica están reemplazados, revocados o no existen. Devuelve la nota
/// para el agente, nunca una conclusión: la persona decide si la excepción
/// sigue aplicando a la regla nueva.
pub fn orphaned_exception(record: &Record, status: &StatusIndex) -> Option<String> {
    if record.kind != "exception" || !status.is_active(record) {
        return None;
    }
    let (targets, _) = exception_targets(record);
    if targets.is_empty() {
        return None;
    }
    let mut described = Vec::new();
    for target in &targets {
        match status.status_of(target) {
            Some(state) if state.is_active() => return None,
            Some(state) => described.push(match status.current_replacement(target) {
                Some(replacement) => format!("{target} ({}, now {replacement})", state.as_str()),
                None => format!("{target} ({})", state.as_str()),
            }),
            None => described.push(format!("{target} (not in the canon)")),
        }
    }
    Some(format!(
        "orphaned exception: it excepts {} — confirm with a person whether it still applies \
         before relying on it",
        described.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(yaml: &str) -> Record {
        yaml_serde::from_str(yaml).unwrap()
    }

    fn base(id: &str, extra: &str) -> Record {
        record(&format!(
            "id: {id}\nkind: decision\nseverity: medium\nstatement: s {id}\nrationale: r\n{extra}"
        ))
    }

    #[test]
    fn supersession_is_derived_from_either_side() {
        let old_marked = base(
            "decision.a",
            "applicability_policy:\n  superseded_by: decision.b\nlifecycle:\n  status: superseded\n",
        );
        let new_marked = base("decision.b", "supersedes: [decision.a]\n");
        let old_unmarked = base("decision.c", "");
        let new_only = base("decision.d", "supersedes: [decision.c]\n");
        let records = vec![old_marked, new_marked, old_unmarked, new_only];
        let index = StatusIndex::build(&records);
        assert_eq!(index.status(&records[0]), RecordStatus::Superseded);
        assert_eq!(index.status(&records[2]), RecordStatus::Superseded);
        assert_eq!(index.replaced_by("decision.c"), Some("decision.d"));
        assert!(index.is_active(&records[1]));
        assert!(index.is_active(&records[3]));
    }

    #[test]
    fn a_pinned_record_is_not_displaced_by_another_records_supersedes() {
        let pinned = base("constraint.p", "authority: pinned\n");
        let challenger = base("constraint.q", "supersedes: [constraint.p]\n");
        let records = vec![pinned, challenger];
        let index = StatusIndex::build(&records);
        assert!(index.is_active(&records[0]));
    }

    #[test]
    fn a_revoked_record_replaces_nobody() {
        let old = base("decision.a", "");
        let revoked = base(
            "decision.b",
            "supersedes: [decision.a]\nlifecycle:\n  status: revoked\n",
        );
        let records = vec![old, revoked];
        let index = StatusIndex::build(&records);
        assert!(index.is_active(&records[0]));
        assert_eq!(index.status(&records[1]), RecordStatus::Revoked);
    }

    #[test]
    fn retirement_is_a_revocation_with_a_retired_event() {
        let retired = base(
            "decision.a",
            "lifecycle:\n  status: revoked\n  events:\n  - type: asserted\n  - type: retired\n    scope: src/old/\n",
        );
        let records = vec![retired];
        let index = StatusIndex::build(&records);
        assert_eq!(index.status(&records[0]), RecordStatus::Retired);
        assert!(
            storage::is_revoked(&records[0]),
            "older binaries see a revocation"
        );
    }

    #[test]
    fn an_exception_whose_rule_was_replaced_is_orphaned() {
        let old = base(
            "constraint.rotate-90d",
            "applicability_policy:\n  superseded_by: constraint.rotate-30d\nlifecycle:\n  status: superseded\n",
        );
        let new = base(
            "constraint.rotate-30d",
            "supersedes: [constraint.rotate-90d]\n",
        );
        let mut cited = base("exception.legacy", "");
        cited.kind = "exception".to_string();
        cited.rationale = Some("Exception to constraint.rotate-90d: legacy KMS.".to_string());
        let mut explicit = base(
            "exception.explicit",
            "exception_to: [constraint.rotate-30d]\n",
        );
        explicit.kind = "exception".to_string();
        let records = vec![old, new, cited, explicit];
        let index = StatusIndex::build(&records);
        let note = orphaned_exception(&records[2], &index).expect("orphaned");
        assert!(note.contains("constraint.rotate-90d (superseded, now constraint.rotate-30d)"));
        assert!(orphaned_exception(&records[3], &index).is_none());
        assert_eq!(
            exception_targets(&records[3]),
            (vec!["constraint.rotate-30d".to_string()], true)
        );
    }

    #[test]
    fn mentioned_ids_respect_word_boundaries() {
        assert_eq!(
            mentioned_ids("See constraint.money-in-cents. Not xconstraint.fake, but decision.a-b."),
            vec![
                "constraint.money-in-cents".to_string(),
                "decision.a-b".to_string()
            ]
        );
    }

    #[test]
    fn replacement_chains_resolve_to_the_current_record() {
        let a = base("decision.a", "");
        let b = base("decision.b", "supersedes: [decision.a]\n");
        let c = base("decision.c", "supersedes: [decision.b]\n");
        let records = vec![a, b, c];
        let index = StatusIndex::build(&records);
        assert_eq!(index.current_replacement("decision.a"), Some("decision.c"));
        assert_eq!(index.current_replacement("decision.c"), None);
    }
}
