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
use crate::storage::{Approval, Record};
use std::path::{Path, PathBuf};

pub struct PendingProposal {
    pub path: PathBuf,
    pub record: Record,
    /// Momento en que se escribió la propuesta — mtime del archivo, la
    /// mejor aproximación disponible sin un campo `proposed_at` explícito
    /// en el schema.
    pub proposed_at: std::time::SystemTime,
}

/// Lista propuestas pendientes en `.rationale/proposals/`, ordenadas por
/// path (determinista). Una propuesta que no parsea como Record válido se
/// omite silenciosamente aquí — `rationale review` no es el lugar para
/// reportar corrupción de datos; `read_record` ya deja evidencia si se
/// invoca directamente.
pub fn list_pending(rationale_dir: &Path) -> Vec<PendingProposal> {
    let proposals_dir = rationale_dir.join("proposals");
    let mut pending = Vec::new();
    let Ok(entries) = std::fs::read_dir(&proposals_dir) else {
        return pending;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        if let Ok(record) = storage::read_record(&path) {
            let proposed_at = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or_else(|_| std::time::SystemTime::now());
            pending.push(PendingProposal {
                path,
                record,
                proposed_at,
            });
        }
    }
    pending.sort_by(|a, b| a.path.cmp(&b.path));
    pending
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
/// la única función de todo el sistema que hace esto. Consume la propuesta
/// (nunca se vuelve a leer del directorio de propuestas después).
pub fn approve(
    rationale_dir: &Path,
    mut proposal: PendingProposal,
    corrected_statement: Option<String>,
    reviewer_actor: &str,
) -> Result<PathBuf, storage::StorageError> {
    if let Some(s) = corrected_statement {
        proposal.record.statement = s;
    }
    proposal.record.approvals.push(Approval {
        actor: reviewer_actor.to_string(),
        authority: "reviewer".to_string(),
        status: "approved".to_string(),
        extra: yaml_serde::Mapping::new(),
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
    // La propuesta ya vive (con su Approval) en records/ — el archivo de
    // proposals/ quedaría duplicado y confuso si sobreviviera.
    let _ = std::fs::remove_file(&proposal.path);
    Ok(dest)
}

/// Rechaza una propuesta — se mueve a `proposals/.rejected/`, nunca se
/// borra en silencio (preserva evidencia de qué se propuso y se descartó).
pub fn reject(rationale_dir: &Path, proposal: &PendingProposal) -> std::io::Result<PathBuf> {
    let rejected_dir = rationale_dir.join("proposals").join(".rejected");
    std::fs::create_dir_all(&rejected_dir)?;
    let dest = rejected_dir.join(proposal.path.file_name().unwrap());
    std::fs::rename(&proposal.path, &dest)?;
    Ok(dest)
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
    use crate::storage::{BindingDeclaration, EpistemicStatus, RecordSubjectRef, Risk};

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
        let normal = proposal_record("constraint.normal-test", "normal");
        assert_eq!(required_confirmation_word(&critical), "approve-critical");
        assert_eq!(required_confirmation_word(&normal), "approve");
        assert_ne!(
            required_confirmation_word(&critical),
            required_confirmation_word(&normal)
        );
    }

    #[test]
    fn describe_effect_shows_single_statement_not_whole_yaml() {
        let record = proposal_record("constraint.test", "normal");
        let effect = describe_effect(&record);
        assert!(effect.contains("original statement"));
        assert!(effect.contains("src/foo.rs"));
        assert!(effect.contains("something could go wrong"));
        // No debe volcar el YAML crudo (ej. la clave "schema_version").
        assert!(!effect.contains("schema_version"));
    }

    #[test]
    fn approve_moves_proposal_to_records_with_approval() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.approve-test", "normal");
        let proposal_path = dir.join("proposals/constraint.approve-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();

        let proposal = PendingProposal {
            path: proposal_path.clone(),
            record,
            proposed_at: std::time::SystemTime::now(),
        };

        let dest = approve(&dir, proposal, None, "user:test-reviewer").unwrap();
        assert!(dest.starts_with(dir.join("records")));
        assert!(!proposal_path.exists(), "la propuesta debe consumirse");

        let approved = storage::read_record(&dest).unwrap();
        assert_eq!(approved.approvals.len(), 1);
        assert_eq!(approved.approvals[0].status, "approved");
        assert_eq!(approved.approvals[0].actor, "user:test-reviewer");
        assert!(storage::has_approved_authority(&approved));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// El Record promovido nunca debe seguir diciendo `status: pending` —
    /// contradiría la Approval real que se le acaba de añadir.
    #[test]
    fn approve_updates_stale_pending_status_to_approved() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.status-test", "normal");
        let proposal_path = dir.join("proposals/constraint.status-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();

        let proposal = PendingProposal {
            path: proposal_path,
            record,
            proposed_at: std::time::SystemTime::now(),
        };

        let dest = approve(&dir, proposal, None, "user:test-reviewer").unwrap();
        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("status: approved"));
        assert!(!content.contains("status: pending"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn approve_applies_corrected_statement() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.correct-test", "normal");
        let proposal_path = dir.join("proposals/constraint.correct-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();

        let proposal = PendingProposal {
            path: proposal_path,
            record,
            proposed_at: std::time::SystemTime::now(),
        };

        let dest = approve(
            &dir,
            proposal,
            Some("corrected statement text".to_string()),
            "user:test-reviewer",
        )
        .unwrap();

        let approved = storage::read_record(&dest).unwrap();
        assert_eq!(approved.statement, "corrected statement text");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reject_moves_to_rejected_subdir_never_deletes() {
        let dir = temp_rationale_dir();
        let record = proposal_record("constraint.reject-test", "normal");
        let proposal_path = dir.join("proposals/constraint.reject-test.yaml");
        storage::write_record(&proposal_path, &record).unwrap();

        let proposal = PendingProposal {
            path: proposal_path.clone(),
            record,
            proposed_at: std::time::SystemTime::now(),
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

    #[test]
    fn list_pending_ignores_non_yaml_and_sorts_deterministically() {
        let dir = temp_rationale_dir();
        std::fs::write(dir.join("proposals/README.md"), "not a proposal").unwrap();
        storage::write_record(
            &dir.join("proposals/constraint.b.yaml"),
            &proposal_record("constraint.b", "normal"),
        )
        .unwrap();
        storage::write_record(
            &dir.join("proposals/constraint.a.yaml"),
            &proposal_record("constraint.a", "normal"),
        )
        .unwrap();

        let pending = list_pending(&dir);
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].record.id, "constraint.a");
        assert_eq!(pending[1].record.id, "constraint.b");

        std::fs::remove_dir_all(&dir).ok();
    }
}
