//! Assessment — Rationale_v0.5.md §5.6, el ejemplo completo de §27.
//!
//! "Record = lo que el proyecto decidió. Assessment = lo que Rationale
//! puede afirmar hoy sobre su vigencia y enlace." Un Assessment nunca se
//! autoría a mano — siempre se computa a partir del Record, la revisión de
//! Git (ADR-0006) y el estado del proveedor estructural (ADR-0002).
//!
//! En Fase E2 esto vive en memoria, calculado bajo demanda. La persistencia
//! en la capa derivada (SQLite, invalidación por revisión) es Fase E3.

use crate::providers::{Coverage, ProviderStatus};
use crate::revision::{Consistency, GitSnapshot};
use crate::storage::{has_approved_authority, EpistemicStatus, Record};
use serde::Serialize;

/// Rationale_v0.5.md §10.6 / §12.2. `unreviewed` es el estado inicial de
/// todo Record nuevo — nunca se autoaprueba (Proceso §21).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthorityStatus {
    Unreviewed,
    Approved,
    // Policy (regla de repositorio aprobada) y Revoked solo se alcanzan vía
    // `review_record` (Fase F) — todavía no implementado. Se declaran aquí
    // para que el schema esté completo desde ahora, no para usarse ya.
    #[allow(dead_code)]
    Policy,
    #[allow(dead_code)]
    Revoked,
}

/// Rationale_v0.5.md §12.3. Si la decisión sigue gobernando el sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Applicability {
    Active,
    // Solo se alcanza vía `supersede_record` (Fase F) — no implementado todavía.
    #[allow(dead_code)]
    Superseded,
    Unknown,
}

/// Rationale_v0.5.md §12.4. Calidad del enlace con la implementación actual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Linkage {
    Current,
    Stale,
    Unresolved,
}

impl std::fmt::Display for AuthorityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuthorityStatus::Unreviewed => "unreviewed",
            AuthorityStatus::Approved => "approved",
            AuthorityStatus::Policy => "policy",
            AuthorityStatus::Revoked => "revoked",
        };
        write!(f, "{s}")
    }
}

impl std::fmt::Display for Applicability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Applicability::Active => "active",
            Applicability::Superseded => "superseded",
            Applicability::Unknown => "unknown",
        };
        write!(f, "{s}")
    }
}

impl std::fmt::Display for Linkage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Linkage::Current => "current",
            Linkage::Stale => "stale",
            Linkage::Unresolved => "unresolved",
        };
        write!(f, "{s}")
    }
}

/// Los cuatro estados juntos, tal como aparecen en el ejemplo `state:` de
/// Rationale_v0.5.md §27.
#[derive(Debug, Clone, Serialize)]
pub struct AssessmentState {
    pub epistemic: EpistemicStatus,
    pub authority: AuthorityStatus,
    pub applicability: Applicability,
    pub linkage: Linkage,
}

#[derive(Debug, Serialize)]
pub struct Assessment {
    pub record_id: String,
    pub assessed_revision: Option<String>,
    pub revision_consistency: Consistency,
    pub state: AssessmentState,
    pub assessment_reason: String,
}

fn authority_status(record: &Record) -> AuthorityStatus {
    if has_approved_authority(record) {
        AuthorityStatus::Approved
    } else {
        AuthorityStatus::Unreviewed
    }
}

/// Deriva `applicability` y `linkage` a partir de la consistencia de
/// revisión (ADR-0006) — nunca del proveedor estructural. Un Record
/// `superseded` explícito (Fase F, `applicability_policy.superseded_by`)
/// no está modelado todavía; por ahora, todo Record vigente es `active`
/// salvo que la revisión no pueda resolverse en absoluto.
fn applicability_and_linkage(consistency: &Consistency) -> (Applicability, Linkage) {
    match consistency {
        Consistency::Exact => (Applicability::Active, Linkage::Current),
        Consistency::WorkingTreeAhead => (Applicability::Active, Linkage::Stale),
        Consistency::StructuralIndexBehind => (Applicability::Active, Linkage::Stale),
        Consistency::Unresolved => (Applicability::Unknown, Linkage::Unresolved),
    }
}

fn assessment_reason(
    consistency: &Consistency,
    provider_status: &ProviderStatus,
    provider_coverage: &Coverage,
) -> String {
    match (consistency, provider_status, provider_coverage) {
        (Consistency::Exact, ProviderStatus::Successful, Coverage::Complete) => {
            "binding resuelto y revisión exacta".to_string()
        }
        (Consistency::Unresolved, _, _) => {
            "no se pudo determinar la revisión de Git — nunca se afirma vigencia sin esto"
                .to_string()
        }
        (Consistency::StructuralIndexBehind, _, _) => {
            "HEAD avanzó desde bound_revision — requiere revalidación, no descartar la decisión"
                .to_string()
        }
        (Consistency::WorkingTreeAhead, _, _) => {
            "working tree tiene cambios no confirmados".to_string()
        }
        (_, ProviderStatus::Unavailable, _) => {
            "proveedor estructural no disponible — assessment basado únicamente en Git".to_string()
        }
        (_, ProviderStatus::Degraded, _) => {
            "proveedor estructural degradado — cobertura no confirmada".to_string()
        }
        _ => "assessment calculado sin condición específica destacable".to_string(),
    }
}

/// Computa un Assessment. Nunca es un dato leído tal cual del canon — se
/// recalcula en cada consulta a partir de fuentes verificables.
pub fn compute(
    record: &Record,
    snapshot: &GitSnapshot,
    provider_status: ProviderStatus,
    provider_coverage: Coverage,
) -> Assessment {
    let bound_revision = record.bound_revision.clone().unwrap_or_default();
    let consistency = crate::revision::check_consistency(snapshot, &bound_revision);
    let (applicability, linkage) = applicability_and_linkage(&consistency);
    let reason = assessment_reason(&consistency, &provider_status, &provider_coverage);

    Assessment {
        record_id: record.id.clone(),
        assessed_revision: snapshot.head.clone(),
        revision_consistency: consistency,
        state: AssessmentState {
            epistemic: record.epistemic_status.clone(),
            authority: authority_status(record),
            applicability,
            linkage,
        },
        assessment_reason: reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Approval, BindingDeclaration};

    fn record_with(approved: bool, bound_revision: &str) -> Record {
        Record {
            id: "constraint.test".to_string(),
            kind: "constraint".to_string(),
            severity: "critical".to_string(),
            statement: "test".to_string(),
            rationale: None,
            epistemic_status: EpistemicStatus::Stated,
            evidence: vec![],
            risks: vec![],
            approvals: if approved {
                vec![Approval {
                    actor: "user:x".to_string(),
                    authority: "security-owner".to_string(),
                    status: "approved".to_string(),
                }]
            } else {
                vec![]
            },
            binding_declarations: Vec::<BindingDeclaration>::new(),
            bound_revision: Some(bound_revision.to_string()),
            subject: None,
        }
    }

    #[test]
    fn active_and_current_when_exact() {
        let record = record_with(true, "abc123");
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
        );
        assert_eq!(assessment.revision_consistency, Consistency::Exact);
        assert_eq!(assessment.state.applicability, Applicability::Active);
        assert_eq!(assessment.state.linkage, Linkage::Current);
        assert_eq!(assessment.state.authority, AuthorityStatus::Approved);
    }

    #[test]
    fn unreviewed_authority_never_becomes_approved() {
        let record = record_with(false, "abc123");
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
        );
        assert_eq!(assessment.state.authority, AuthorityStatus::Unreviewed);
    }

    #[test]
    fn stale_when_head_moved_never_exact() {
        let record = record_with(true, "abc123");
        let snap = GitSnapshot {
            head: Some("def456".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
        );
        assert_eq!(
            assessment.revision_consistency,
            Consistency::StructuralIndexBehind
        );
        assert_eq!(assessment.state.linkage, Linkage::Stale);
        // Sigue "active": un HEAD distinto no implica que la decisión dejó
        // de aplicar (Rationale_v0.5.md §4.2) — solo que requiere revalidación.
        assert_eq!(assessment.state.applicability, Applicability::Active);
    }

    #[test]
    fn unknown_applicability_when_revision_unresolved() {
        let record = record_with(true, "abc123");
        let snap = GitSnapshot {
            head: None,
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Unavailable,
            Coverage::Unknown,
        );
        assert_eq!(assessment.state.applicability, Applicability::Unknown);
        assert_eq!(assessment.state.linkage, Linkage::Unresolved);
    }
}
