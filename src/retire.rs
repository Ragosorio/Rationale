//! `rationale retire` — reparar el canon cuando un alcance entero desaparece.
//!
//! El caso real: un módulo («Comunicaciones») se retiró y 116 Records siguieron
//! activos describiéndolo como vigente, algunos atados también a código actual.
//! Repararlos a mano uno por uno no ocurre; repararlos con una heurística
//! silenciosa borraría decisiones válidas. Este comando:
//!
//! 1. **Planifica** sin escribir: por Record vigente atado al alcance, `retire`
//!    (todo su binding estaba ahí), `unbind` (también gobierna código actual:
//!    se le quitan solo los bindings retirados), `rebind` (`--moved-to` y la
//!    ruta nueva existe) o `needs_human` (fijado por una persona: nunca se
//!    toca).
//! 2. **Aplica** solo con `--apply`, bajo el lock del canon, con un evento de
//!    lifecycle por Record y un lote local (`.rationale-local/repairs/`) con
//!    el texto anterior y posterior de cada archivo.
//! 3. **Deshace** el lote byte a byte (`--undo`), salvo los archivos que
//!    cambiaron después (esos se reportan, nunca se pisan).
//!
//! Una retirada se escribe como `lifecycle.status: revoked` con un evento
//! `retired`: un binario anterior a 1.2 la lee como revocación (inactiva).
//! Todo queda en Git: el diff es revisable en un PR y `git revert` también lo
//! deshace.

use crate::binding_match::is_under;
use crate::canon::{generate_id, lifecycle_event, push_lifecycle_event};
use crate::lifecycle::StatusIndex;
use crate::storage::{self, Record, RecordAuthority};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const BATCH_SCHEMA: &str = "rationale/retire-batch/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetireAction {
    Retire,
    Unbind,
    Rebind,
    NeedsHuman,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedAction {
    pub record_id: String,
    pub action: RetireAction,
    pub kind: String,
    pub severity: String,
    pub authority: String,
    /// Rutas de sus bindings dentro del alcance.
    pub bindings_in_scope: Vec<String>,
    /// Para `rebind`: la ruta nueva de cada una.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rebound_to: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetirePlan {
    pub scope: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moved_to: Option<String>,
    /// El alcance todavía existe en el árbol de trabajo.
    pub path_exists: bool,
    pub actions: Vec<PlannedAction>,
    /// Excepciones vigentes que quedarán sin regla si se aplica.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exceptions_left_without_rule: Vec<String>,
    pub summary: PlanSummary,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PlanSummary {
    pub retire: usize,
    pub unbind: usize,
    pub rebind: usize,
    pub needs_human: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFile {
    pub record_id: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub schema_version: String,
    pub batch_id: String,
    pub created_at: String,
    pub actor: String,
    pub scope: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moved_to: Option<String>,
    pub files: Vec<BatchFile>,
}

#[derive(Debug, Serialize)]
pub struct UndoReport {
    pub batch_id: String,
    pub restored: Vec<String>,
    /// Cambiaron después de aplicar el lote: no se pisaron.
    pub changed_since: Vec<String>,
}

fn clean(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    normalized
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn in_scope(path: &str, scope: &str) -> bool {
    let path = clean(path);
    path == scope || is_under(&path, scope)
}

fn moved(path: &str, scope: &str, moved_to: &str) -> String {
    let path = clean(path);
    format!("{moved_to}{}", &path[scope.len()..])
}

/// Valida el alcance pedido: relativo al repositorio y nunca el canon.
pub fn normalize_scope(raw: &str) -> Result<String, String> {
    let scope = clean(raw);
    if scope.is_empty() || scope == "." {
        return Err("retire needs a path inside the repository, not its root".to_string());
    }
    if scope.starts_with('/') || scope.split('/').any(|part| part == "..") {
        return Err(format!(
            "'{raw}' must be a path relative to the repository root"
        ));
    }
    if scope == ".rationale"
        || is_under(&scope, ".rationale")
        || scope.starts_with(".rationale-local")
    {
        return Err("Rationale's own data cannot be retired".to_string());
    }
    Ok(scope)
}

pub fn plan(
    rationale_dir: &Path,
    project_root: &Path,
    scope: &str,
    reason: &str,
    moved_to: Option<&str>,
) -> Result<RetirePlan, String> {
    let scope = normalize_scope(scope)?;
    let moved_to = moved_to.map(normalize_scope).transpose()?;
    if reason.trim().is_empty() {
        return Err("retire needs --reason: why the scope is gone".to_string());
    }
    let records =
        storage::list_records(&rationale_dir.join("records")).map_err(|e| e.to_string())?;
    let status = StatusIndex::build(&records);
    let mut actions = Vec::new();
    for record in records.iter().filter(|r| status.is_active(r)) {
        let paths: Vec<String> = record
            .binding_declarations
            .iter()
            .filter_map(|b| b.path_hint.clone())
            .chain(
                record
                    .relationship_bindings
                    .iter()
                    .flat_map(|b| [b.source.file_path.clone(), b.target.file_path.clone()]),
            )
            .collect();
        let inside: Vec<String> = paths
            .iter()
            .filter(|p| in_scope(p, &scope))
            .cloned()
            .collect();
        if inside.is_empty() {
            continue;
        }
        let outside = paths.iter().any(|p| !in_scope(p, &scope));
        let rebound: Vec<String> = match &moved_to {
            Some(target) => inside
                .iter()
                .map(|p| moved(p, &scope, target))
                .filter(|p| project_root.join(p).exists())
                .collect(),
            None => Vec::new(),
        };
        let pinned = storage::record_authority(record) == RecordAuthority::Pinned;
        let (action, note) = if pinned {
            (
                RetireAction::NeedsHuman,
                "pinned by a person: retire never changes it; unpin it, rebind it, or keep it"
                    .to_string(),
            )
        } else if !rebound.is_empty() && rebound.len() == inside.len() {
            (
                RetireAction::Rebind,
                "every binding in the scope exists under the new path".to_string(),
            )
        } else if outside {
            (
                RetireAction::Unbind,
                "also bound to current code: only its bindings in the scope are removed"
                    .to_string(),
            )
        } else {
            (
                RetireAction::Retire,
                "every binding was in the retired scope".to_string(),
            )
        };
        actions.push(PlannedAction {
            record_id: record.id.clone(),
            action,
            kind: record.kind.clone(),
            severity: record.severity.clone(),
            authority: storage::authority_label(record).to_string(),
            bindings_in_scope: inside,
            rebound_to: if action == RetireAction::Rebind {
                rebound
            } else {
                Vec::new()
            },
            note,
        });
    }
    actions.sort_by(|a, b| a.record_id.cmp(&b.record_id));
    let retiring: Vec<&str> = actions
        .iter()
        .filter(|a| a.action == RetireAction::Retire)
        .map(|a| a.record_id.as_str())
        .collect();
    let exceptions_left_without_rule = records
        .iter()
        .filter(|r| r.kind == "exception" && status.is_active(r))
        .filter(|r| !retiring.contains(&r.id.as_str()))
        .filter(|r| {
            let (targets, _) = crate::lifecycle::exception_targets(r);
            !targets.is_empty()
                && targets.iter().all(|t| {
                    retiring.contains(&t.as_str())
                        || status.status_of(t).is_none_or(|s| !s.is_active())
                })
                && targets.iter().any(|t| retiring.contains(&t.as_str()))
        })
        .map(|r| r.id.clone())
        .collect();
    let mut summary = PlanSummary::default();
    for action in &actions {
        match action.action {
            RetireAction::Retire => summary.retire += 1,
            RetireAction::Unbind => summary.unbind += 1,
            RetireAction::Rebind => summary.rebind += 1,
            RetireAction::NeedsHuman => summary.needs_human += 1,
        }
    }
    Ok(RetirePlan {
        path_exists: project_root.join(&scope).exists(),
        scope,
        reason: reason.trim().to_string(),
        moved_to,
        actions,
        exceptions_left_without_rule,
        summary,
    })
}

fn batches_dir(local_dir: &Path) -> PathBuf {
    local_dir.join("repairs")
}

fn apply_one(
    record: &mut Record,
    action: &PlannedAction,
    plan: &RetirePlan,
    batch_id: &str,
    actor: &str,
) {
    let base = |kind: &str| {
        vec![
            ("type", kind.to_string()),
            ("scope", plan.scope.clone()),
            ("reason", plan.reason.clone()),
            ("batch_id", batch_id.to_string()),
            ("actor", actor.to_string()),
            ("timestamp", crate::evaluation::now_iso8601()),
        ]
    };
    match action.action {
        RetireAction::Retire => {
            push_lifecycle_event(record, Some("revoked"), lifecycle_event(&base("retired")));
        }
        RetireAction::Unbind => {
            record.binding_declarations.retain(|b| {
                !b.path_hint
                    .as_deref()
                    .is_some_and(|p| in_scope(p, &plan.scope))
            });
            record.relationship_bindings.retain(|b| {
                !in_scope(&b.source.file_path, &plan.scope)
                    && !in_scope(&b.target.file_path, &plan.scope)
            });
            let mut fields = base("unbound");
            fields.push(("removed_bindings", action.bindings_in_scope.join(", ")));
            push_lifecycle_event(record, None, lifecycle_event(&fields));
        }
        RetireAction::Rebind => {
            let target = plan.moved_to.clone().unwrap_or_default();
            for binding in &mut record.binding_declarations {
                if let Some(path) = binding
                    .path_hint
                    .clone()
                    .filter(|p| in_scope(p, &plan.scope))
                {
                    binding.path_hint = Some(moved(&path, &plan.scope, &target));
                }
            }
            for relationship in &mut record.relationship_bindings {
                for endpoint in [&mut relationship.source, &mut relationship.target] {
                    if in_scope(&endpoint.file_path, &plan.scope) {
                        endpoint.file_path = moved(&endpoint.file_path, &plan.scope, &target);
                    }
                }
            }
            let mut fields = base("rebound");
            fields.push(("moved_to", target));
            push_lifecycle_event(record, None, lifecycle_event(&fields));
        }
        RetireAction::NeedsHuman => {}
    }
}

/// Aplica el plan. Rehace el plan bajo el lock para no escribir sobre un
/// canon que cambió desde que la persona lo leyó.
pub fn apply(
    rationale_dir: &Path,
    project_root: &Path,
    local_dir: &Path,
    plan_in: &RetirePlan,
    actor: &str,
) -> Result<(RetirePlan, Batch), String> {
    let _lock = crate::canon::CanonLock::acquire(local_dir)?;
    let plan = plan(
        rationale_dir,
        project_root,
        &plan_in.scope,
        &plan_in.reason,
        plan_in.moved_to.as_deref(),
    )?;
    let planned: Vec<(&str, RetireAction)> = plan_in
        .actions
        .iter()
        .map(|a| (a.record_id.as_str(), a.action))
        .collect();
    let current: Vec<(&str, RetireAction)> = plan
        .actions
        .iter()
        .map(|a| (a.record_id.as_str(), a.action))
        .collect();
    if planned != current {
        return Err("the canon changed since the plan was made; review the new plan".to_string());
    }
    let batch_id = generate_id("retire");
    let records_dir = rationale_dir.join("records");
    let mut files = Vec::new();
    for action in plan
        .actions
        .iter()
        .filter(|a| a.action != RetireAction::NeedsHuman)
    {
        storage::validate_safe_id(&action.record_id).map_err(|e| e.to_string())?;
        let path = records_dir.join(format!("{}.yaml", action.record_id));
        let before =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut record = storage::read_record(&path).map_err(|e| e.to_string())?;
        apply_one(&mut record, action, &plan, &batch_id, actor);
        storage::write_record(&path, &record).map_err(|e| e.to_string())?;
        let after = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        files.push(BatchFile {
            record_id: action.record_id.clone(),
            before,
            after,
        });
    }
    let batch = Batch {
        schema_version: BATCH_SCHEMA.to_string(),
        batch_id: batch_id.clone(),
        created_at: crate::evaluation::now_rfc3339_millis(),
        actor: actor.to_string(),
        scope: plan.scope.clone(),
        reason: plan.reason.clone(),
        moved_to: plan.moved_to.clone(),
        files,
    };
    let mut bytes = serde_json::to_vec_pretty(&batch).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    let path = batches_dir(local_dir).join(format!("{batch_id}.json"));
    storage::atomic_write_bytes(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((plan, batch))
}

/// Restaura cada archivo del lote si sigue tal como lo dejó el lote.
pub fn undo(
    rationale_dir: &Path,
    local_dir: &Path,
    batch_id: &str,
    force: bool,
) -> Result<UndoReport, String> {
    storage::validate_safe_id(batch_id).map_err(|e| e.to_string())?;
    if !batch_id.starts_with("retire_") {
        return Err(format!("'{batch_id}' is not a retire batch id"));
    }
    let _lock = crate::canon::CanonLock::acquire(local_dir)?;
    let path = batches_dir(local_dir).join(format!("{batch_id}.json"));
    let text = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "unknown batch '{batch_id}' (batches live in {})",
            batches_dir(local_dir).display()
        )
    })?;
    let batch: Batch = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let records_dir = rationale_dir.join("records");
    let mut restored = Vec::new();
    let mut changed_since = Vec::new();
    for file in &batch.files {
        storage::validate_safe_id(&file.record_id).map_err(|e| e.to_string())?;
        let record_path = records_dir.join(format!("{}.yaml", file.record_id));
        let current = std::fs::read_to_string(&record_path).unwrap_or_default();
        if current != file.after && !force {
            changed_since.push(file.record_id.clone());
            continue;
        }
        storage::atomic_write_bytes(&record_path, file.before.as_bytes())
            .map_err(|e| format!("{}: {e}", record_path.display()))?;
        restored.push(file.record_id.clone());
    }
    Ok(UndoReport {
        batch_id: batch.batch_id,
        restored,
        changed_since,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-retire-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join(".rationale/records")).unwrap();
        std::fs::create_dir_all(dir.join("src/new")).unwrap();
        std::fs::write(dir.join("src/new/email.ts"), "x\n").unwrap();
        dir
    }

    fn record(dir: &Path, id: &str, paths: &[&str], extra: &str) {
        let bindings: String = paths
            .iter()
            .enumerate()
            .map(|(i, p)| format!("  - id: b.{i}\n    type: file\n    path_hint: {p}\n"))
            .collect();
        std::fs::write(
            dir.join(format!(".rationale/records/{id}.yaml")),
            format!("id: {id}\nkind: decision\nseverity: medium\nstatement: \"s {id}\"\nrationale: \"r\"\nbinding_declarations:\n{bindings}{extra}"),
        )
        .unwrap();
    }

    #[test]
    fn plan_apply_and_undo_are_reviewable_and_exact() {
        let dir = project();
        record(&dir, "decision.a", &["src/old/a.ts"], "");
        record(
            &dir,
            "decision.b",
            &["src/old/b.ts", "src/new/email.ts"],
            "",
        );
        record(&dir, "decision.c", &["src/old/c.ts"], "authority: pinned\n");
        record(&dir, "decision.d", &["src/new/email.ts"], "");
        let rationale_dir = dir.join(".rationale");
        let local = dir.join(".rationale-local");
        let before: Vec<String> = ["a", "b", "c", "d"]
            .iter()
            .map(|id| {
                std::fs::read_to_string(dir.join(format!(".rationale/records/decision.{id}.yaml")))
                    .unwrap()
            })
            .collect();

        let plan = plan(&rationale_dir, &dir, "src/old/", "module removed", None).unwrap();
        assert_eq!(plan.scope, "src/old");
        assert_eq!(
            (
                plan.summary.retire,
                plan.summary.unbind,
                plan.summary.needs_human
            ),
            (1, 1, 1)
        );
        assert!(plan.actions.iter().all(|a| a.record_id != "decision.d"));

        let (_, batch) = apply(&rationale_dir, &dir, &local, &plan, "user:owner").unwrap();
        let records = storage::list_records(&rationale_dir.join("records")).unwrap();
        let status = StatusIndex::build(&records);
        let get = |id: &str| records.iter().find(|r| r.id == id).unwrap();
        assert_eq!(
            status.status(get("decision.a")),
            crate::lifecycle::RecordStatus::Retired
        );
        assert!(status.is_active(get("decision.b")));
        assert_eq!(get("decision.b").binding_declarations.len(), 1);
        assert!(
            status.is_active(get("decision.c")),
            "pinned is never touched"
        );
        assert_eq!(batch.files.len(), 2);

        let report = undo(&rationale_dir, &local, &batch.batch_id, false).unwrap();
        assert_eq!(report.restored.len(), 2);
        let after: Vec<String> = ["a", "b", "c", "d"]
            .iter()
            .map(|id| {
                std::fs::read_to_string(dir.join(format!(".rationale/records/decision.{id}.yaml")))
                    .unwrap()
            })
            .collect();
        assert_eq!(before, after, "undo restores every byte");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn undo_never_overwrites_a_record_changed_after_the_batch() {
        let dir = project();
        record(&dir, "decision.a", &["src/old/a.ts"], "");
        let rationale_dir = dir.join(".rationale");
        let local = dir.join(".rationale-local");
        let plan = plan(&rationale_dir, &dir, "src/old", "gone", None).unwrap();
        let (_, batch) = apply(&rationale_dir, &dir, &local, &plan, "user:owner").unwrap();
        let path = dir.join(".rationale/records/decision.a.yaml");
        std::fs::write(
            &path,
            std::fs::read_to_string(&path).unwrap() + "# edited\n",
        )
        .unwrap();
        let report = undo(&rationale_dir, &local, &batch.batch_id, false).unwrap();
        assert_eq!(report.changed_since, vec!["decision.a".to_string()]);
        assert!(std::fs::read_to_string(&path).unwrap().contains("# edited"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn moved_scopes_rebind_and_bad_scopes_are_refused() {
        let dir = project();
        std::fs::create_dir_all(dir.join("src/next")).unwrap();
        std::fs::write(dir.join("src/next/a.ts"), "x\n").unwrap();
        record(&dir, "decision.a", &["src/old/a.ts"], "");
        let rationale_dir = dir.join(".rationale");
        let plan = plan(&rationale_dir, &dir, "src/old", "renamed", Some("src/next")).unwrap();
        assert_eq!(plan.actions[0].action, RetireAction::Rebind);
        assert_eq!(
            plan.actions[0].rebound_to,
            vec!["src/next/a.ts".to_string()]
        );
        for bad in ["", ".", "/etc", "../x", ".rationale/records"] {
            assert!(normalize_scope(bad).is_err(), "{bad}");
        }
        std::fs::remove_dir_all(dir).ok();
    }
}
