//! `rationale review` — confirmación humana de propuestas (Fase F6).
//!
//! Un servidor MCP no puede hacer preguntas interactivas; por eso la
//! confirmación vive aquí, en la CLI, donde hay un humano de verdad leyendo
//! una terminal. `finalize_change` (Fase F5) solo escribe en
//! `.rationale/proposals/` — este módulo es la ÚNICA vía que promueve una
//! propuesta a `.rationale/records/`, y solo lo hace con una `Approval` real
//! añadida explícitamente aquí.
//!
//! Rationale_v0.5.md §15.5 (prevención de "aceptar todo"), cada regla
//! verificable en el código, no solo en la documentación:
//! - Una afirmación por pantalla, nunca el YAML entero (`describe_effect`).
//! - Muestra el efecto práctico de aprobar (`describe_effect`).
//! - Permite corregir el texto antes de aprobar (`approve` acepta
//!   `corrected_statement`).
//! - Nunca preselecciona aprobación para restricciones críticas —
//!   `required_confirmation_word` exige una palabra más larga y distinta
//!   para severidad `critical`, nunca la misma fricción que el resto.
//! - Registra cuánto tiempo pasó entre propuesta y confirmación
//!   (`log_decision`), sin asumir mala fe.

use crate::storage;
use crate::storage::{Approval, Evidence, Record};
use std::path::{Path, PathBuf};

pub struct PendingProposal {
    pub path: PathBuf,
    pub record: Record,
    /// Momento en que se escribió la propuesta — mtime del archivo, la
    /// mejor aproximación disponible sin un campo `proposed_at` explícito
    /// en el schema.
    pub proposed_at: std::time::SystemTime,
    /// Contenido crudo del archivo en el momento en que se listó — usado
    /// por `approve()`/`reject()` para detectar si la propuesta cambió en
    /// disco durante la ventana de revisión humana (revisión adversarial
    /// de Fase F, hallazgo 2: TOCTOU real — una segunda `finalize_change`
    /// sobre el mismo `record_id` mientras un humano decide se perdía en
    /// silencio, sin quedar en `.rejected/` ni en ningún log).
    original_content: String,
}

pub struct PendingProposalList {
    pub pending: Vec<PendingProposal>,
    pub skipped: Vec<(PathBuf, String)>,
}

/// Lista propuestas pendientes en `.rationale/proposals/`, ordenadas por
/// path (determinista). Una propuesta ilegible se conserva en `skipped` para
/// que la única interfaz humana de revisión nunca oculte corrupción.
pub fn list_pending_detailed(rationale_dir: &Path) -> PendingProposalList {
    let proposals_dir = rationale_dir.join("proposals");
    let mut pending = Vec::new();
    let mut skipped = Vec::new();
    let Ok(entries) = std::fs::read_dir(&proposals_dir) else {
        return PendingProposalList { pending, skipped };
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let original_content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                skipped.push((path, format!("error leyendo YAML: {e}")));
                continue;
            }
        };
        match storage::read_record(&path) {
            Ok(record) => {
                let proposed_at = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or_else(|_| std::time::SystemTime::now());
                pending.push(PendingProposal {
                    path,
                    record,
                    proposed_at,
                    original_content,
                });
            }
            Err(e) => skipped.push((path, e.to_string())),
        }
    }
    pending.sort_by(|a, b| a.path.cmp(&b.path));
    PendingProposalList { pending, skipped }
}

/// Compatibilidad para callers que solo necesitan las propuestas válidas.
#[allow(dead_code)]
pub fn list_pending(rationale_dir: &Path) -> Vec<PendingProposal> {
    list_pending_detailed(rationale_dir).pending
}

fn claim_proposal(rationale_dir: &Path, proposal: &PendingProposal) -> std::io::Result<PathBuf> {
    static CLAIM_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let in_review = rationale_dir.join("proposals").join(".in-review");
    std::fs::create_dir_all(&in_review)?;
    let sequence = CLAIM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!(
        "{}.yaml.{}-{sequence}",
        proposal.record.id,
        std::process::id()
    );
    let claimed = in_review.join(name);
    // rename es el claim CAS: exactamente un proceso puede mover el path
    // original. El perdedor recibe NotFound y no puede aprobar/rechazar una
    // copia obsoleta.
    std::fs::rename(&proposal.path, &claimed)?;
    Ok(claimed)
}

fn read_claimed_content(
    claimed: &Path,
    proposal: &PendingProposal,
) -> Result<String, storage::StorageError> {
    let current = std::fs::read_to_string(claimed).map_err(storage::StorageError::Io)?;
    if current != proposal.original_content {
        return Err(storage::StorageError::Parse(format!(
            "la propuesta '{}' cambió mientras se reclamaba — quedó recuperable en {}",
            proposal.record.id,
            claimed.display()
        )));
    }
    Ok(current)
}

/// El "efecto práctico" de aprobar (v0.5 §15.5) — nunca el YAML completo.
/// Una sola afirmación, su razón, a qué Subject y estructura afecta, y sus
/// riesgos declarados.
pub fn describe_effect(record: &Record) -> String {
    let mut lines = vec![format!("Afirmación propuesta: {}", record.statement)];
    if let Some(r) = &record.rationale {
        lines.push(format!("Razón: {r}"));
    }
    if let Some(subject) = &record.subject {
        lines.push(format!("Subject: {}", subject.id));
    }
    let paths: Vec<String> = record
        .binding_declarations
        .iter()
        .filter_map(|b| b.path_hint.clone())
        .collect();
    if !paths.is_empty() {
        lines.push(format!("Gobernará: {}", paths.join(", ")));
    }
    if !record.risks.is_empty() {
        let risks: Vec<String> = record.risks.iter().map(|r| r.statement.clone()).collect();
        lines.push(format!("Riesgos declarados: {}", risks.join(" | ")));
    }
    lines.push(format!("Severidad: {}", record.severity));
    lines.join("\n")
}

/// La palabra exacta que el humano debe escribir para aprobar. Nunca la
/// misma para `critical` que para el resto — v0.5 §15.5: "no debe
/// preseleccionar aprobación para restricciones críticas". Una palabra más
/// larga y específica no es "más segura" criptográficamente, pero sí exige
/// una acción deliberada distinta, no un reflejo de teclear la misma tecla
/// para todo.
pub fn required_confirmation_word(record: &Record) -> &'static str {
    if record.severity == "critical" {
        "approve-critical"
    } else {
        "approve"
    }
}

/// Promueve una propuesta a `.rationale/records/` con una `Approval` real —
/// la única función de todo el sistema que hace esto. Primero reclama el
/// archivo con rename atómico; el estado intermedio vive en `.in-review/` y
/// queda recuperable si el proceso muere antes de completar la promoción.
pub fn approve(
    rationale_dir: &Path,
    mut proposal: PendingProposal,
    corrected_statement: Option<String>,
    reviewer_actor: &str,
    reviewer_role: storage::AuthorityRole,
    reviewer_domain: Option<&str>,
) -> Result<PathBuf, storage::StorageError> {
    // `proposal.record.id` viene del contenido YAML de la propuesta — no
    // necesariamente del `record_id` ya validado en `finalize_change` (una
    // propuesta pudo editarse a mano, o llegar por otra vía en el futuro).
    // Nunca confiar en que ya fue validado río arriba (path traversal real,
    // ver `storage::validate_safe_id`).
    storage::validate_safe_id(&proposal.record.id)?;

    let claimed = claim_proposal(rationale_dir, &proposal).map_err(storage::StorageError::Io)?;
    read_claimed_content(&claimed, &proposal)?;

    if let Some(s) = corrected_statement {
        proposal.record.statement = s;
    }
    // `approval.schema.json` declara `approved_at`, pero nada lo escribía:
    // de la aprobación se guardaba el "quién" y nunca el "cuándo". Sin
    // esto, auditar cuándo se aprobó una decisión exige leer el mtime del
    // archivo (que un `git checkout`/rebase puede alterar) en vez de un
    // campo propio del dato.
    let mut extra = yaml_serde::Mapping::new();
    extra.insert(
        yaml_serde::Value::String("approved_at".to_string()),
        yaml_serde::Value::String(crate::evaluation::now_iso8601()),
    );
    if let Some(domain) = reviewer_domain {
        extra.insert(
            yaml_serde::Value::String("domain".to_string()),
            yaml_serde::Value::String(domain.to_string()),
        );
    }
    proposal.record.approvals.push(Approval {
        actor: reviewer_actor.to_string(),
        authority: reviewer_role.to_string(),
        status: "approved".to_string(),
        extra,
    });
    // El `status: pending` que `finalize_change` escribió ya no es cierto —
    // el Record se está promoviendo con una Approval real. Dejarlo diría
    // dos cosas contradictorias en el mismo archivo.
    proposal.record.extra.insert(
        yaml_serde::Value::String("status".to_string()),
        yaml_serde::Value::String("approved".to_string()),
    );

    let records_dir = rationale_dir.join("records");
    let dest = records_dir.join(format!("{}.yaml", proposal.record.id));
    storage::write_record(&dest, &proposal.record)?;
    // La propuesta ya vive (con su Approval) en records/. Si el proceso
    // muere antes de este punto, `.in-review/` conserva la evidencia para
    // recuperación manual; nunca se pierde silenciosamente.
    std::fs::remove_file(&claimed).map_err(storage::StorageError::Io)?;
    Ok(dest)
}

/// Rechaza una propuesta — se mueve a `proposals/.rejected/`, nunca se
/// borra en silencio (preserva evidencia de qué se propuso y se descartó).
///
/// Usa el mismo claim atómico que `approve()`: si el contenido cambió desde
/// que se listó, el archivo queda en `.in-review/` para recuperación y no se
/// mueve en silencio una versión distinta a la que se mostró.
pub fn reject(rationale_dir: &Path, proposal: &PendingProposal) -> std::io::Result<PathBuf> {
    let claimed = claim_proposal(rationale_dir, proposal)?;
    read_claimed_content(&claimed, proposal).map_err(|e| std::io::Error::other(e.to_string()))?;
    let rejected_dir = rationale_dir.join("proposals").join(".rejected");
    std::fs::create_dir_all(&rejected_dir)?;
    let dest = rejected_dir.join(proposal.path.file_name().unwrap());
    std::fs::rename(&claimed, &dest)?;
    Ok(dest)
}

/// Mutaciones explícitas sobre un Record ya aprobado. Todas se ejecutan por
/// la CLI interactiva; el servidor MCP no expone esta API. El evento queda
/// dentro del Record canónico para que la historia viaje con Git.
#[derive(Debug)]
pub enum RecordMutation {
    Correct {
        statement: String,
        reason: String,
    },
    Dispute {
        reason: String,
    },
    Revoke {
        reason: String,
    },
    Supersede {
        replacement_id: String,
        reason: String,
    },
    ChangeAuthority {
        new_actor: String,
        new_role: storage::AuthorityRole,
        new_actor_declared: bool,
        reason: String,
    },
    AddEvidence {
        evidence: Evidence,
        reason: String,
    },
    /// Usada por `rationale doctor --repair` (Fase 1.5) para corregir una
    /// severidad fuera del enum del schema (legado, de antes de que
    /// `finalize_change` la exigiera) — nunca automático, siempre con
    /// confirmación humana por Record.
    CorrectSeverity {
        new_severity: String,
        reason: String,
    },
    /// Usada por `rationale doctor --repair` para corregir un `kind` que no
    /// coincide con el prefijo de `id` — el defecto real que rompió CI: un
    /// Record `id: decision.*` se escribió con `kind: constraint` porque
    /// `finalize_change` defaulteaba siempre a `"constraint"` cuando el
    /// caller no declaraba `kind` (ver `storage::infer_kind_from_id`). Nunca
    /// automático, siempre con confirmación humana por Record.
    CorrectKind {
        new_kind: String,
        reason: String,
    },
    /// Fase C del plan beta — `doctor` detecta `RecordWithoutBindings` pero
    /// nunca lo repara solo ("inventar un binding sería peor que ninguno").
    /// Este es el camino de migración para el canon legado que quedó así:
    /// el HUMANO aporta la ruta (y opcionalmente el símbolo), nunca
    /// Rationale. `extra.declared_by: "human"` distingue este binding de
    /// uno que un proveedor estructural confirmó — nunca se presenta como
    /// la misma certeza.
    AddHumanConfirmedBinding {
        path_hint: String,
        symbol: Option<String>,
        reason: String,
    },
}

fn ensure_mapping<'a>(map: &'a mut yaml_serde::Mapping, key: &str) -> &'a mut yaml_serde::Mapping {
    let key_value = yaml_serde::Value::String(key.to_string());
    let replace = !matches!(map.get(&key_value), Some(yaml_serde::Value::Mapping(_)));
    if replace {
        map.insert(
            key_value.clone(),
            yaml_serde::Value::Mapping(yaml_serde::Mapping::new()),
        );
    }
    match map.get_mut(&key_value) {
        Some(yaml_serde::Value::Mapping(value)) => value,
        _ => unreachable!("ensure_mapping siempre deja un mapping"),
    }
}

fn add_lifecycle_event(
    record: &mut Record,
    status: Option<&str>,
    event_type: &str,
    actor: &str,
    authority: storage::AuthorityRole,
    reason: &str,
    extra: Vec<(&str, String)>,
) {
    let lifecycle = ensure_mapping(&mut record.extra, "lifecycle");
    if let Some(status) = status {
        lifecycle.insert(
            yaml_serde::Value::String("status".to_string()),
            yaml_serde::Value::String(status.to_string()),
        );
    }
    let mut event = yaml_serde::Mapping::new();
    for (key, value) in [
        ("type", event_type.to_string()),
        ("actor", actor.to_string()),
        ("authority", authority.to_string()),
        ("reason", reason.to_string()),
        ("timestamp", crate::evaluation::now_iso8601()),
    ] {
        event.insert(
            yaml_serde::Value::String(key.to_string()),
            yaml_serde::Value::String(value),
        );
    }
    for (key, value) in extra {
        event.insert(
            yaml_serde::Value::String(key.to_string()),
            yaml_serde::Value::String(value),
        );
    }
    let events_key = yaml_serde::Value::String("events".to_string());
    let events = match lifecycle.get_mut(&events_key) {
        Some(yaml_serde::Value::Sequence(events)) => events,
        _ => {
            lifecycle.insert(events_key.clone(), yaml_serde::Value::Sequence(Vec::new()));
            match lifecycle.get_mut(&events_key) {
                Some(yaml_serde::Value::Sequence(events)) => events,
                _ => unreachable!("events siempre es una secuencia"),
            }
        }
    };
    events.push(yaml_serde::Value::Mapping(event));
}

fn active_for_mutation(record: &Record) -> Result<(), storage::StorageError> {
    match storage::lifecycle_status(record) {
        Some("revoked") => Err(storage::StorageError::Parse(
            "un Record revocado no puede mutarse de nuevo".to_string(),
        )),
        Some("superseded") => Err(storage::StorageError::Parse(
            "un Record superseded no puede mutarse de nuevo".to_string(),
        )),
        _ => Ok(()),
    }
}

/// Aplica una mutación de lifecycle con una comprobación TOCTOU adicional:
/// si el YAML cambió entre la lectura interactiva y el write, se aborta y se
/// conserva el archivo intacto para que el humano vuelva a revisarlo.
pub fn mutate_record(
    rationale_dir: &Path,
    record_id: &str,
    mutation: RecordMutation,
    reviewer_actor: &str,
    reviewer_role: storage::AuthorityRole,
    reviewer_declared: bool,
) -> Result<PathBuf, storage::StorageError> {
    storage::validate_safe_id(record_id)?;
    if !reviewer_declared {
        return Err(storage::StorageError::Parse(
            "la mutación de lifecycle requiere un actor declarado en .rationale/config.yaml; "
                .to_string(),
        ));
    }
    let path = rationale_dir
        .join("records")
        .join(format!("{record_id}.yaml"));
    let original_content = std::fs::read_to_string(&path).map_err(storage::StorageError::Io)?;
    let mut record = storage::read_record(&path)?;
    active_for_mutation(&record)?;

    match mutation {
        RecordMutation::Correct { statement, reason } => {
            if statement.trim().is_empty() {
                return Err(storage::StorageError::MissingRequiredField("statement"));
            }
            record.statement = statement;
            add_lifecycle_event(
                &mut record,
                None,
                "corrected",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![],
            );
        }
        RecordMutation::Dispute { reason } => {
            record.epistemic_status = storage::EpistemicStatus::Disputed;
            add_lifecycle_event(
                &mut record,
                Some("disputed"),
                "disputed",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![],
            );
        }
        RecordMutation::Revoke { reason } => {
            add_lifecycle_event(
                &mut record,
                Some("revoked"),
                "revoked",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![],
            );
        }
        RecordMutation::Supersede {
            replacement_id,
            reason,
        } => {
            storage::validate_safe_id(&replacement_id)?;
            let replacement = rationale_dir
                .join("records")
                .join(format!("{replacement_id}.yaml"));
            if !replacement.is_file() {
                return Err(storage::StorageError::Parse(format!(
                    "el Record replacement '{}' no existe en records/",
                    replacement_id
                )));
            }
            let policy = ensure_mapping(&mut record.extra, "applicability_policy");
            policy.insert(
                yaml_serde::Value::String("superseded_by".to_string()),
                yaml_serde::Value::String(replacement_id.clone()),
            );
            add_lifecycle_event(
                &mut record,
                Some("superseded"),
                "superseded",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![("replacement_id", replacement_id)],
            );
        }
        RecordMutation::ChangeAuthority {
            new_actor,
            new_role,
            new_actor_declared,
            reason,
        } => {
            if !new_actor_declared {
                return Err(storage::StorageError::Parse(
                    "la nueva autoridad debe estar declarada por el proyecto; no se permite autoelevación"
                        .to_string(),
                ));
            }
            record.approvals.push(Approval {
                actor: new_actor.clone(),
                authority: new_role.to_string(),
                status: "approved".to_string(),
                extra: {
                    let mut extra = yaml_serde::Mapping::new();
                    extra.insert(
                        yaml_serde::Value::String("event".to_string()),
                        yaml_serde::Value::String("authority-changed".to_string()),
                    );
                    extra
                },
            });
            let lifecycle = ensure_mapping(&mut record.extra, "lifecycle");
            lifecycle.insert(
                yaml_serde::Value::String("authority_actor".to_string()),
                yaml_serde::Value::String(new_actor.clone()),
            );
            lifecycle.insert(
                yaml_serde::Value::String("authority_role".to_string()),
                yaml_serde::Value::String(new_role.to_string()),
            );
            add_lifecycle_event(
                &mut record,
                None,
                "authority-changed",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![("new_actor", new_actor), ("new_role", new_role.to_string())],
            );
        }
        RecordMutation::AddEvidence { evidence, reason } => {
            record.evidence.push(evidence);
            add_lifecycle_event(
                &mut record,
                None,
                "evidence-added",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![],
            );
        }
        RecordMutation::CorrectSeverity {
            new_severity,
            reason,
        } => {
            if storage::Severity::parse(&new_severity).is_none() {
                return Err(storage::StorageError::InvalidSeverity(new_severity));
            }
            let old_severity = record.severity.clone();
            record.severity = new_severity.clone();
            add_lifecycle_event(
                &mut record,
                None,
                "severity-corrected",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![
                    ("old_severity", old_severity),
                    ("new_severity", new_severity),
                ],
            );
        }
        RecordMutation::CorrectKind { new_kind, reason } => {
            const VALID_KINDS: [&str; 4] = ["constraint", "decision", "risk", "exception"];
            if !VALID_KINDS.contains(&new_kind.as_str()) {
                return Err(storage::StorageError::Parse(format!(
                    "kind '{new_kind}' inválido — valores válidos: {}",
                    VALID_KINDS.join(", ")
                )));
            }
            let old_kind = record.kind.clone();
            record.kind = new_kind.clone();
            add_lifecycle_event(
                &mut record,
                None,
                "kind-corrected",
                reviewer_actor,
                reviewer_role,
                &reason,
                vec![("old_kind", old_kind), ("new_kind", new_kind)],
            );
        }
        RecordMutation::AddHumanConfirmedBinding {
            path_hint,
            symbol,
            reason,
        } => {
            if path_hint.trim().is_empty() {
                return Err(storage::StorageError::MissingRequiredField("path_hint"));
            }
            let mut extra = yaml_serde::Mapping::new();
            // Nunca indistinguible de un binding que un proveedor
            // estructural confirmó — `provider` se queda en `None` (como
            // cualquier binding de archivo real) y este marcador declara
            // explícitamente que el dato lo aportó una persona, no un
            // proceso automático.
            extra.insert(
                yaml_serde::Value::String("declared_by".to_string()),
                yaml_serde::Value::String("human".to_string()),
            );
            let binding_id = format!(
                "binding.{record_id}.human-confirmed-{}",
                record.binding_declarations.len()
            );
            record
                .binding_declarations
                .push(storage::BindingDeclaration {
                    id: binding_id.clone(),
                    kind: if symbol.is_some() {
                        "symbol".to_string()
                    } else {
                        "file".to_string()
                    },
                    provider: None,
                    structural_id: symbol.clone(),
                    path_hint: Some(path_hint.clone()),
                    provisional: false,
                    extra,
                });
            let mut event_extra = vec![("path_hint", path_hint), ("binding_id", binding_id)];
            if let Some(symbol) = symbol {
                event_extra.push(("symbol", symbol));
            }
            add_lifecycle_event(
                &mut record,
                None,
                "binding-added-by-human",
                reviewer_actor,
                reviewer_role,
                &reason,
                event_extra,
            );
        }
    }

    let current_content = std::fs::read_to_string(&path).map_err(storage::StorageError::Io)?;
    if current_content != original_content {
        return Err(storage::StorageError::Parse(format!(
            "el Record '{}' cambió durante la revisión; no se sobrescribió",
            record_id
        )));
    }
    storage::write_record(&path, &record)?;
    Ok(path)
}

#[derive(Debug, serde::Serialize)]
struct ReviewLog {
    event: String,
    timestamp: String,
    record_id: String,
    decision: String,
    /// Cuánto tiempo pasó entre que se propuso y se confirmó — señal de
    /// calidad (v0.5 §15.5), nunca usada para asumir mala fe.
    time_to_confirm_ms: u128,
}

/// Registra la decisión en `.rationale-local/runs/review-decisions.ndjson`
/// — local, nunca enviado a ningún servicio (mismo patrón que
/// `evaluation::record_run`).
pub fn log_decision(
    rationale_local_dir: &Path,
    record_id: &str,
    decision: &str,
    time_to_confirm_ms: u128,
) -> std::io::Result<()> {
    let runs_dir = rationale_local_dir.join("runs");
    std::fs::create_dir_all(&runs_dir)?;
    let log_path = runs_dir.join("review-decisions.ndjson");
    let log = ReviewLog {
        event: "review_decision".to_string(),
        timestamp: crate::evaluation::now_iso8601(),
        record_id: record_id.to_string(),
        decision: decision.to_string(),
        time_to_confirm_ms,
    };
    let line = serde_json::to_string(&log).expect("serialize review log");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{line}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        AuthorityRole, BindingDeclaration, EpistemicStatus, RecordSubjectRef, Risk,
    };

    fn contributor() -> (AuthorityRole, Option<&'static str>) {
        (AuthorityRole::Contributor, None)
    }

    fn proposal_record(id: &str, severity: &str) -> Record {
        let mut extra = yaml_serde::Mapping::new();
        extra.insert(
            yaml_serde::Value::String("status".to_string()),
            yaml_serde::Value::String("pending".to_string()),
        );
        Record {
            id: id.to_string(),
            kind: "constraint".to_string(),
            severity: severity.to_string(),
            statement: "original statement".to_string(),
            rationale: Some("because reasons".to_string()),
            epistemic_status: EpistemicStatus::Stated,
            approvals: vec![],
            binding_declarations: vec![BindingDeclaration {
                id: "binding.test".to_string(),
                kind: "file".to_string(),
                provider: None,
                structural_id: None,
                path_hint: Some("src/foo.rs".to_string()),
                provisional: false,
                extra: yaml_serde::Mapping::new(),
            }],
            evidence: vec![],
            risks: vec![Risk {
                id: "risk.test".to_string(),
                statement: "something could go wrong".to_string(),
                epistemic_status: EpistemicStatus::Stated,
                extra: yaml_serde::Mapping::new(),
            }],
            bound_revision: Some("abc123".to_string()),
            subject: Some(RecordSubjectRef {
                id: "some.subject".to_string(),
                extra: yaml_serde::Mapping::new(),
            }),
            extra,
        }
    }

    fn temp_rationale_dir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-review-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("proposals")).unwrap();
        std::fs::create_dir_all(dir.join("records")).unwrap();
        dir
    }

    #[test]
    fn required_confirmation_word_differs_for_critical() {
        let critical = proposal_record("constraint.critical-test", "critical");
        let normal = proposal_record("constraint.normal-test", "medium");
        assert_eq!(required_confirmation_word(&critical), "approve-critical");
        assert_eq!(required_confirmation_word(&normal), "approve");
        assert_ne!(
            required_confirmation_word(&critical),
            required_confirmation_word(&normal)
        );
    }

    #[test]
    fn describe_effect_shows_single_statement_not_whole_yaml() {
        let record = proposal_record("constraint.test", "medium");
        let effect = describe_effect(&record);
        assert!(effect.contains("original statement"));
        assert!(effect.contains("src/foo.rs"));
        assert!(effect.contains("something could go wrong"));
        // No debe volcar el YAML crudo (ej. la clave "schema_version").
        assert!(!effect.contains("schema_version"));
    }

    /// Vulnerabilidad real encontrada y corregida durante la verificación de
    /// fin de Fase F: un `record.id` con `../` habría escrito fuera de
    /// `.rationale/records/`. `approve()` debe rechazarlo antes de tocar
    /// disco, no solo confiar en que `finalize_change` ya lo validó.
    #[test]
    fn approve_rejects_path_traversal_in_record_id() {
        let dir = temp_rationale_dir();
        let mut record = proposal_record("placeholder", "medium");
        record.id = "../../../../tmp/pwned-by-rationale-test".to_string();

        let proposal = PendingProposal {
            path: dir.join("proposals/placeholder.yaml"),
            record,
            proposed_at: std::time::SystemTime::now(),
            // Nunca se llega a comparar — validate_safe_id falla antes del
            // chequeo TOCTOU.
            original_content: String::new(),
        };

        let (role, domain) = contributor();
        let result = approve(&dir, proposal, None, "user:test-reviewer", role, domain);
        assert!(
            matches!(result, Err(storage::StorageError::UnsafeIdentifier(_))),
            "debe rechazar el id inseguro, no escribir nada"
        );
        assert!(
            !std::path::Path::new("/tmp/pwned-by-rationale-test.yaml").exists(),
            "no debe haber escapado del directorio del proyecto"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn approve_moves_proposal_to_records_with_approval() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.approve-test", "medium");
        let proposal_path = dir.join("proposals/constraint.approve-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();
        let original_content = std::fs::read_to_string(&proposal_path).unwrap();

        let proposal = PendingProposal {
            path: proposal_path.clone(),
            record,
            proposed_at: std::time::SystemTime::now(),
            original_content,
        };

        let (role, domain) = contributor();
        let dest = approve(&dir, proposal, None, "user:test-reviewer", role, domain).unwrap();
        assert!(dest.starts_with(dir.join("records")));
        assert!(!proposal_path.exists(), "la propuesta debe consumirse");

        let approved = storage::read_record(&dest).unwrap();
        assert_eq!(approved.approvals.len(), 1);
        assert_eq!(approved.approvals[0].status, "approved");
        assert_eq!(approved.approvals[0].actor, "user:test-reviewer");
        assert!(storage::has_approved_authority(&approved));
        // `approval.schema.json` declara `approved_at`; antes de este fix
        // nada lo escribía — de la aprobación solo quedaba el "quién".
        let approved_at = approved.approvals[0]
            .extra
            .get(yaml_serde::Value::String("approved_at".to_string()))
            .and_then(|v| v.as_str())
            .expect("approved_at debe escribirse al aprobar");
        assert!(!approved_at.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    /// El Record promovido nunca debe seguir diciendo `status: pending` —
    /// contradiría la Approval real que se le acaba de añadir.
    #[test]
    fn approve_updates_stale_pending_status_to_approved() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.status-test", "medium");
        let proposal_path = dir.join("proposals/constraint.status-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();
        let original_content = std::fs::read_to_string(&proposal_path).unwrap();

        let proposal = PendingProposal {
            path: proposal_path,
            record,
            proposed_at: std::time::SystemTime::now(),
            original_content,
        };

        let (role, domain) = contributor();
        let dest = approve(&dir, proposal, None, "user:test-reviewer", role, domain).unwrap();
        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("status: approved"));
        assert!(!content.contains("status: pending"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn approve_applies_corrected_statement() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.correct-test", "medium");
        let proposal_path = dir.join("proposals/constraint.correct-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();
        let original_content = std::fs::read_to_string(&proposal_path).unwrap();

        let proposal = PendingProposal {
            path: proposal_path,
            record,
            proposed_at: std::time::SystemTime::now(),
            original_content,
        };

        let dest = approve(
            &dir,
            proposal,
            Some("corrected statement text".to_string()),
            "user:test-reviewer",
            contributor().0,
            contributor().1,
        )
        .unwrap();

        let approved = storage::read_record(&dest).unwrap();
        assert_eq!(approved.statement, "corrected statement text");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reject_moves_to_rejected_subdir_never_deletes() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.reject-test", "medium");
        let proposal_path = dir.join("proposals/constraint.reject-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();
        let original_content = std::fs::read_to_string(&proposal_path).unwrap();

        let proposal = PendingProposal {
            path: proposal_path.clone(),
            record,
            proposed_at: std::time::SystemTime::now(),
            original_content,
        };

        let dest = reject(&dir, &proposal).unwrap();
        assert!(!proposal_path.exists());
        assert!(
            dest.exists(),
            "el rechazo debe preservar evidencia, no borrar"
        );
        assert!(dest.starts_with(dir.join("proposals/.rejected")));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Reproduce el hallazgo 2a de la revisión adversarial de Fase F: una
    /// segunda escritura sobre la misma propuesta MIENTRAS un humano la
    /// tiene cargada en memoria ya no debe perderse en silencio — debe
    /// rechazar la aprobación explícitamente.
    #[test]
    fn approve_detects_proposal_overwritten_during_review_window() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.toctou-test", "medium");
        let proposal_path = dir.join("proposals/constraint.toctou-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();

        // Simula `list_pending()`: se carga en memoria ANTES del cambio.
        let original_content = std::fs::read_to_string(&proposal_path).unwrap();
        let proposal = PendingProposal {
            path: proposal_path.clone(),
            record,
            proposed_at: std::time::SystemTime::now(),
            original_content,
        };

        // Mientras el humano "piensa", otra finalize_change sobrescribe la
        // propuesta con contenido distinto.
        let mut overwritten = proposal_record("constraint.toctou-test", "medium");
        overwritten.statement = "SECOND VERSION written during the review window".to_string();
        storage::write_record(&proposal_path, &overwritten).unwrap();

        let (role, domain) = contributor();
        let result = approve(&dir, proposal, None, "user:test-reviewer", role, domain);
        assert!(
            result.is_err(),
            "debe rechazar promover una copia obsoleta cuando el archivo cambió"
        );
        // La versión nueva (real) queda intacta en `.in-review/` — nada se
        // perdió y la recuperación puede inspeccionarla manualmente.
        let claimed = std::fs::read_dir(dir.join("proposals/.in-review"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let still_there = storage::read_record(&claimed).unwrap();
        assert_eq!(
            still_there.statement,
            "SECOND VERSION written during the review window"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Reproduce el hallazgo 2b: una segunda aprobación sobre una propuesta
    /// que YA fue promovida (y por tanto borrada de `proposals/`) por otra
    /// sesión debe fallar explícitamente, no sobrescribir en silencio.
    #[test]
    fn approve_detects_proposal_already_promoted_by_another_session() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.double-approve-test", "medium");
        let proposal_path = dir.join("proposals/constraint.double-approve-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();
        let original_content = std::fs::read_to_string(&proposal_path).unwrap();

        let proposal_a = PendingProposal {
            path: proposal_path.clone(),
            record: storage::read_record(&proposal_path).unwrap(),
            proposed_at: std::time::SystemTime::now(),
            original_content: original_content.clone(),
        };
        let proposal_b = PendingProposal {
            path: proposal_path.clone(),
            record: storage::read_record(&proposal_path).unwrap(),
            proposed_at: std::time::SystemTime::now(),
            original_content,
        };

        // El revisor A aprueba primero — consume la propuesta.
        let (role, domain) = contributor();
        approve(&dir, proposal_a, None, "user:reviewer-a", role, domain).unwrap();
        assert!(!proposal_path.exists());

        // El revisor B, con la misma propuesta cargada ANTES de que A la
        // promoviera, intenta aprobar también.
        let (role, domain) = contributor();
        let result_b = approve(&dir, proposal_b, None, "user:reviewer-b", role, domain);
        assert!(
            result_b.is_err(),
            "debe rechazar una segunda aprobación sobre una propuesta ya promovida"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_pending_ignores_non_yaml_and_sorts_deterministically() {
        let dir = temp_rationale_dir();
        std::fs::write(dir.join("proposals/README.md"), "not a proposal").unwrap();
        storage::write_record(
            &dir.join("proposals/constraint.b.yaml"),
            &proposal_record("constraint.b", "medium"),
        )
        .unwrap();
        storage::write_record(
            &dir.join("proposals/constraint.a.yaml"),
            &proposal_record("constraint.a", "medium"),
        )
        .unwrap();

        let pending = list_pending(&dir);
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].record.id, "constraint.a");
        assert_eq!(pending[1].record.id, "constraint.b");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_pending_reports_corrupt_yaml_instead_of_hiding_it() {
        let dir = temp_rationale_dir();
        std::fs::write(
            dir.join("proposals/broken.yaml"),
            "id: constraint.broken\nstatement: [not valid record\n",
        )
        .unwrap();

        let result = list_pending_detailed(&dir);
        assert!(result.pending.is_empty());
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].0.ends_with("broken.yaml"));
        assert!(!result.skipped[0].1.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    fn canonical_record_dir(id: &str) -> (PathBuf, PathBuf) {
        let dir = temp_rationale_dir();
        let mut record = proposal_record(id, "high");
        record.approvals.push(Approval {
            actor: "user:declared".to_string(),
            authority: "architecture-owner".to_string(),
            status: "approved".to_string(),
            extra: yaml_serde::Mapping::new(),
        });
        let path = dir.join("records").join(format!("{id}.yaml"));
        storage::write_record(&path, &record).unwrap();
        (dir, path)
    }

    #[test]
    fn review_record_supports_dispute_revoke_and_evidence_with_history() {
        let (dir, path) = canonical_record_dir("constraint.lifecycle-test");
        let reviewer = "user:declared";

        mutate_record(
            &dir,
            "constraint.lifecycle-test",
            RecordMutation::Dispute {
                reason: "la evidencia necesita revisión independiente".to_string(),
            },
            reviewer,
            storage::AuthorityRole::ArchitectureOwner,
            true,
        )
        .unwrap();
        let disputed = storage::read_record(&path).unwrap();
        assert_eq!(
            disputed.epistemic_status,
            storage::EpistemicStatus::Disputed
        );
        assert_eq!(storage::lifecycle_status(&disputed), Some("disputed"));

        mutate_record(
            &dir,
            "constraint.lifecycle-test",
            RecordMutation::AddEvidence {
                evidence: Evidence {
                    evidence_type: "test".to_string(),
                    path: Some("docs/test.md".to_string()),
                    revision: Some("abc123".to_string()),
                    verified: true,
                    content_hash: None,
                    visibility: Some("repository".to_string()),
                    extra: yaml_serde::Mapping::new(),
                },
                reason: "prueba reproducible añadida".to_string(),
            },
            reviewer,
            storage::AuthorityRole::ArchitectureOwner,
            true,
        )
        .unwrap();
        mutate_record(
            &dir,
            "constraint.lifecycle-test",
            RecordMutation::Revoke {
                reason: "la disputa no se resolvió".to_string(),
            },
            reviewer,
            storage::AuthorityRole::ArchitectureOwner,
            true,
        )
        .unwrap();

        let revoked = storage::read_record(&path).unwrap();
        assert_eq!(storage::lifecycle_status(&revoked), Some("revoked"));
        assert!(!storage::has_approved_authority(&revoked));
        assert_eq!(revoked.evidence.len(), 1);
        let lifecycle = revoked
            .extra
            .get(yaml_serde::Value::String("lifecycle".to_string()))
            .unwrap();
        let events = match lifecycle {
            yaml_serde::Value::Mapping(map) => map
                .get(yaml_serde::Value::String("events".to_string()))
                .and_then(|value| value.as_sequence())
                .unwrap(),
            _ => panic!("lifecycle debe ser un mapping"),
        };
        assert_eq!(events.len(), 3);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Fase C del plan beta: el único camino para cerrar
    /// `RecordWithoutBindings` en canon legado es que un humano aporte la
    /// ruta. El binding resultante debe quedar marcado `declared_by: human`
    /// — nunca indistinguible de uno que un proveedor confirmó.
    #[test]
    fn add_human_confirmed_binding_marks_provenance_and_logs_lifecycle() {
        let dir = temp_rationale_dir();
        let mut record = proposal_record("constraint.human-binding-test", "high");
        record.binding_declarations.clear();
        record.approvals.push(Approval {
            actor: "user:declared".to_string(),
            authority: "architecture-owner".to_string(),
            status: "approved".to_string(),
            extra: yaml_serde::Mapping::new(),
        });
        let path = dir
            .join("records")
            .join("constraint.human-binding-test.yaml");
        storage::write_record(&path, &record).unwrap();
        let reviewer = "user:declared";

        mutate_record(
            &dir,
            "constraint.human-binding-test",
            RecordMutation::AddHumanConfirmedBinding {
                path_hint: "app/upload.ts".to_string(),
                symbol: Some("submit".to_string()),
                reason: "confirmado a mano tras revisar el código real".to_string(),
            },
            reviewer,
            storage::AuthorityRole::ArchitectureOwner,
            true,
        )
        .unwrap();

        let updated = storage::read_record(&path).unwrap();
        assert_eq!(updated.binding_declarations.len(), 1);
        let binding = &updated.binding_declarations[0];
        assert_eq!(binding.path_hint.as_deref(), Some("app/upload.ts"));
        assert_eq!(binding.structural_id.as_deref(), Some("submit"));
        assert!(binding.provider.is_none());
        assert_eq!(
            binding
                .extra
                .get(yaml_serde::Value::String("declared_by".to_string()))
                .and_then(|v| v.as_str()),
            Some("human")
        );

        let lifecycle = updated
            .extra
            .get(yaml_serde::Value::String("lifecycle".to_string()))
            .unwrap();
        let events = match lifecycle {
            yaml_serde::Value::Mapping(map) => map
                .get(yaml_serde::Value::String("events".to_string()))
                .and_then(|value| value.as_sequence())
                .unwrap(),
            _ => panic!("lifecycle debe ser un mapping"),
        };
        assert_eq!(events.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn review_record_rejects_undeclared_authority_and_auto_elevation() {
        let (dir, path) = canonical_record_dir("constraint.authority-lifecycle-test");
        let result = mutate_record(
            &dir,
            "constraint.authority-lifecycle-test",
            RecordMutation::ChangeAuthority {
                new_actor: "user:invented".to_string(),
                new_role: storage::AuthorityRole::ArchitectureOwner,
                new_actor_declared: false,
                reason: "attempt".to_string(),
            },
            "user:declared",
            storage::AuthorityRole::ArchitectureOwner,
            true,
        );
        assert!(result.is_err());
        assert_eq!(storage::read_record(&path).unwrap().approvals.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn review_record_supersede_requires_existing_replacement_and_marks_source() {
        let (dir, path) = canonical_record_dir("constraint.superseded-source");
        let replacement = proposal_record("constraint.superseded-replacement", "high");
        storage::write_record(
            &dir.join("records")
                .join("constraint.superseded-replacement.yaml"),
            &replacement,
        )
        .unwrap();
        mutate_record(
            &dir,
            "constraint.superseded-source",
            RecordMutation::Supersede {
                replacement_id: "constraint.superseded-replacement".to_string(),
                reason: "reemplazado por una formulación más precisa".to_string(),
            },
            "user:declared",
            storage::AuthorityRole::ArchitectureOwner,
            true,
        )
        .unwrap();
        let source = storage::read_record(&path).unwrap();
        assert_eq!(
            storage::superseded_by(&source),
            Some("constraint.superseded-replacement")
        );
        assert_eq!(storage::lifecycle_status(&source), Some("superseded"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
