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
