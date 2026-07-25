//! Context Compiler — versión mínima para la vertical slice (Fase D).
//! Rationale_v0.5.md §18, Arquitectura §11.11.
//!
//! Esta vertical slice compila exactamente UNA constraint compacta, no un
//! paquete completo con niveles de prioridad (eso es Fase E). Aun así ya
//! respeta el principio central: declarar snapshot de consistencia primero,
//! nunca presentar contexto plausible pero potencialmente incorrecto.

use crate::providers::{Coverage, ProviderStatus};
use crate::revision::Consistency;
use crate::storage::Record;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub git_revision: Option<String>,
    pub consistency: String,
    pub provider_status: String,
    pub provider_coverage: String,
}

#[derive(Debug, Serialize)]
pub struct CriticalConstraint {
    pub id: String,
    pub statement: String,
    pub authority: String,
}

#[derive(Debug, Serialize)]
pub struct ContextPacket {
    pub snapshot: Snapshot,
    pub critical_constraints: Vec<CriticalConstraint>,
    pub resolved_target: Option<String>,
    pub warnings: Vec<String>,
}

/// Compila el packet final. Aplica la regla de bloqueo/autoridad de
/// `Rationale_v0.5.md §10.7` a nivel de EXPOSICIÓN: una constraint sin
/// aprobación se marca explícitamente `authority: unreviewed`, nunca se
/// disfraza de aprobada.
pub fn compile_packet(
    git_head: Option<String>,
    consistency: Consistency,
    provider_status: ProviderStatus,
    provider_coverage: Coverage,
    record: &Record,
    resolved_target: Option<String>,
    provider_warnings: Vec<String>,
) -> ContextPacket {
    let authority = if crate::storage::has_approved_authority(record) {
        "approved"
    } else {
        "unreviewed"
    };

    let provider_status_str = match provider_status {
        ProviderStatus::Successful => "successful",
        ProviderStatus::Degraded => "degraded",
        ProviderStatus::Unavailable => "unavailable",
    };
    let coverage_str = match provider_coverage {
        Coverage::Complete => "complete",
        Coverage::Partial => "partial",
        Coverage::Unknown => "unknown",
    };

    ContextPacket {
        snapshot: Snapshot {
            git_revision: git_head,
            consistency: consistency.to_string(),
            provider_status: provider_status_str.to_string(),
            provider_coverage: coverage_str.to_string(),
        },
        critical_constraints: vec![CriticalConstraint {
            id: record.id.clone(),
            statement: record.statement.clone(),
            authority: authority.to_string(),
        }],
        resolved_target,
        warnings: provider_warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Approval, BindingDeclaration, Record};

    fn fixed_record(approved: bool) -> Record {
        Record {
            id: "constraint.golden-test".to_string(),
            kind: "constraint".to_string(),
            severity: "critical".to_string(),
            statement: "Golden packet statement.".to_string(),
            approvals: if approved {
                vec![Approval {
                    actor: "user:security-owner".to_string(),
                    authority: "security-owner".to_string(),
                    status: "approved".to_string(),
                }]
            } else {
                vec![]
            },
            binding_declarations: vec![BindingDeclaration {
                id: "binding.golden".to_string(),
                kind: "symbol".to_string(),
                provider: Some("codebase-memory".to_string()),
                structural_id: Some("function:typescript:golden".to_string()),
                path_hint: Some("src/golden.ts".to_string()),
            }],
            bound_revision: Some("abc123fixed".to_string()),
        }
    }

    /// D5 — "golden packet": mismos inputs fijos deben producir SIEMPRE el
    /// mismo JSON, byte a byte. Si este test empieza a fallar sin que nadie
    /// haya cambiado el formato deliberadamente, algo dejó de ser
    /// determinista (Arquitectura §19.4).
    #[test]
    fn golden_packet_is_byte_for_byte_deterministic() {
        let record = fixed_record(true);
        let packet = compile_packet(
            Some("abc123fixed".to_string()),
            Consistency::Exact,
            ProviderStatus::Successful,
            Coverage::Complete,
            &record,
            Some("golden.qualifiedName".to_string()),
            vec![],
        );

        let json = serde_json::to_string(&packet).unwrap();
        let expected = r#"{"snapshot":{"git_revision":"abc123fixed","consistency":"exact","provider_status":"successful","provider_coverage":"complete"},"critical_constraints":[{"id":"constraint.golden-test","statement":"Golden packet statement.","authority":"approved"}],"resolved_target":"golden.qualifiedName","warnings":[]}"#;
        assert_eq!(json, expected);
    }

    /// D5 — "token budget respetado": esta vertical slice compila
    /// exactamente UNA constraint por diseño (no un paquete de N
    /// prioridades, eso es Fase E) — verificar que ese invariante se
    /// mantiene sin importar el estado de aprobación del Record.
    #[test]
    fn budget_never_exceeds_one_constraint() {
        for approved in [true, false] {
            let record = fixed_record(approved);
            let packet = compile_packet(
                None,
                Consistency::Unresolved,
                ProviderStatus::Unavailable,
                Coverage::Unknown,
                &record,
                None,
                vec![],
            );
            assert_eq!(packet.critical_constraints.len(), 1);
        }
    }

    /// Una constraint sin approval nunca debe disfrazarse de aprobada
    /// (Rationale_v0.5.md §10.7) — verificado a nivel de packet expuesto,
    /// no solo de storage::has_approved_authority.
    #[test]
    fn unapproved_record_is_never_exposed_as_approved() {
        let record = fixed_record(false);
        let packet = compile_packet(
            None,
            Consistency::Unresolved,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
            &record,
            None,
            vec![],
        );
        assert_eq!(packet.critical_constraints[0].authority, "unreviewed");
    }
}
