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
use std::path::Path;

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
///
/// Defecto real de un dogfood: antes `linkage` era un simple alias de la
/// consistencia de revisión de Git (árbol sucio o HEAD movido →
/// automáticamente `stale`), sin decir NADA sobre si los bindings del
/// Record de verdad resolvían contra algo. Un Record con
/// `binding_declarations: []` podía leerse como `stale` — que a un lector
/// le suena a "el enlace está roto", cuando el veredicto honesto es que no
/// hay ningún enlace que verificar en absoluto. Ahora `linkage` se deriva
/// exclusivamente de si los bindings declarados resuelven contra el
/// working tree real (`BindingResolution`); `revision_consistency` sigue
/// siendo Git puro (ADR-0006 intacto en ese punto) y vive en un campo
/// separado del `Assessment`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Linkage {
    /// Todos los bindings con `path_hint` existen en el working tree.
    Current,
    /// Al menos un binding con `path_hint` ya no existe ahí.
    Stale,
    /// Cero bindings, o ninguno tiene `path_hint` verificable (solo
    /// `structural_id` sin proveedor que lo confirme).
    Unresolved,
}

/// Resolución de un binding individual — la evidencia detrás de `linkage`,
/// nunca solo el veredicto agregado. Expuesto en el `Assessment` para que
/// un humano (o `rationale doctor`, Fase 1.5) vea EXACTAMENTE cuál binding
/// falló, no solo que "algo" está `stale`.
#[derive(Debug, Clone, Serialize)]
pub struct BindingResolution {
    pub binding_id: String,
    pub resolved: bool,
    pub detail: String,
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
    /// La evidencia detrás de `state.linkage` — un binding por entrada.
    pub binding_resolution: Vec<BindingResolution>,
}

fn authority_status(record: &Record) -> AuthorityStatus {
    if crate::storage::is_revoked(record) {
        AuthorityStatus::Revoked
    } else if has_approved_authority(record) {
        AuthorityStatus::Approved
    } else {
        AuthorityStatus::Unreviewed
    }
}

/// Deriva `applicability` a partir de la consistencia de revisión
/// (ADR-0006) — nunca del proveedor estructural. Un Record `superseded`
/// explícito (Fase F, `applicability_policy.superseded_by`) no está
/// modelado todavía más allá de este campo; por ahora, todo Record vigente
/// es `active` salvo que la revisión no pueda resolverse en absoluto.
fn applicability(record: &Record, consistency: &Consistency) -> Applicability {
    if crate::storage::superseded_by(record).is_some() {
        return Applicability::Superseded;
    }
    match consistency {
        Consistency::Unresolved => Applicability::Unknown,
        Consistency::Exact | Consistency::WorkingTreeAhead | Consistency::StructuralIndexBehind => {
            Applicability::Active
        }
    }
}

/// Deriva `linkage` a partir de si los bindings declarados por el Record
/// resuelven contra el working tree real — nunca de Git (ese es
/// `revision_consistency`, un campo separado). `path_hint` es lo único que
/// se verifica: `structural_id` no tiene una gramática única en el canon
/// real (ver `binding_match`), así que un binding solo-símbolo sin
/// `path_hint` es honestamente `Unresolved`, no `Current` por descarte.
fn linkage(record: &Record, repo_root: &Path) -> (Linkage, Vec<BindingResolution>) {
    if record.binding_declarations.is_empty() {
        return (Linkage::Unresolved, vec![]);
    }

    let mut resolutions = Vec::with_capacity(record.binding_declarations.len());
    let mut any_verifiable = false;
    let mut all_resolved = true;
    for binding in &record.binding_declarations {
        let (resolved, detail) = match &binding.path_hint {
            Some(hint) => {
                any_verifiable = true;
                if repo_root.join(hint).exists() {
                    (true, "el path existe en el working tree".to_string())
                } else {
                    (false, format!("'{hint}' no existe en el working tree"))
                }
            }
            None => (
                false,
                "sin path_hint — no verificable sin el proveedor estructural".to_string(),
            ),
        };
        if !resolved {
            all_resolved = false;
        }
        resolutions.push(BindingResolution {
            binding_id: binding.id.clone(),
            resolved,
            detail,
        });
    }

    let linkage = if !any_verifiable {
        Linkage::Unresolved
    } else if all_resolved {
        Linkage::Current
    } else {
        Linkage::Stale
    };
    (linkage, resolutions)
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
/// `repo_root` es contra qué se verifica `path_hint` de cada binding
/// (Fase 1.4) — nunca el proveedor estructural, que puede estar
/// deshabilitado o no instalado sin que eso deba impedir verificar algo
/// tan básico como "¿existe este archivo?".
pub fn compute(
    record: &Record,
    snapshot: &GitSnapshot,
    provider_status: ProviderStatus,
    provider_coverage: Coverage,
    repo_root: &Path,
) -> Assessment {
    let bound_revision = record.bound_revision.clone().unwrap_or_default();
    let consistency = crate::revision::check_consistency(snapshot, &bound_revision);
    let applicability = applicability(record, &consistency);
    let (linkage, binding_resolution) = linkage(record, repo_root);
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
        binding_resolution,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Approval, BindingDeclaration};

    fn unique_suffix() -> String {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )
    }

    /// Repo desechable — `linkage` ahora verifica `path_hint` contra un
    /// directorio real, no contra Git, así que los tests necesitan un
    /// working tree de verdad, no solo un `GitSnapshot` sintético.
    fn temp_repo() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rationale-assessment-test-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn record_with(approved: bool, bound_revision: &str) -> Record {
        record_with_bindings(approved, bound_revision, vec![])
    }

    fn record_with_bindings(
        approved: bool,
        bound_revision: &str,
        binding_declarations: Vec<BindingDeclaration>,
    ) -> Record {
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
                    extra: yaml_serde::Mapping::new(),
                }]
            } else {
                vec![]
            },
            binding_declarations,
            bound_revision: Some(bound_revision.to_string()),
            subject: None,
            extra: yaml_serde::Mapping::new(),
        }
    }

    fn file_binding(path_hint: &str) -> BindingDeclaration {
        BindingDeclaration {
            id: "binding.test".to_string(),
            kind: "file".to_string(),
            provider: None,
            structural_id: None,
            path_hint: Some(path_hint.to_string()),
            provisional: false,
            extra: yaml_serde::Mapping::new(),
        }
    }

    #[test]
    fn active_and_current_when_binding_path_exists() {
        let dir = temp_repo();
        std::fs::write(dir.join("a.rs"), "// ok\n").unwrap();
        let record = record_with_bindings(true, "abc123", vec![file_binding("a.rs")]);
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
            &dir,
        );
        assert_eq!(assessment.revision_consistency, Consistency::Exact);
        assert_eq!(assessment.state.applicability, Applicability::Active);
        assert_eq!(assessment.state.linkage, Linkage::Current);
        assert_eq!(assessment.state.authority, AuthorityStatus::Approved);
        assert!(assessment.binding_resolution[0].resolved);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unreviewed_authority_never_becomes_approved() {
        let dir = temp_repo();
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
            &dir,
        );
        assert_eq!(assessment.state.authority, AuthorityStatus::Unreviewed);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn revoked_authority_overrides_historical_approval() {
        let dir = temp_repo();
        let mut record = record_with(true, "abc123");
        let mut lifecycle = yaml_serde::Mapping::new();
        lifecycle.insert(
            yaml_serde::Value::String("status".to_string()),
            yaml_serde::Value::String("revoked".to_string()),
        );
        record.extra.insert(
            yaml_serde::Value::String("lifecycle".to_string()),
            yaml_serde::Value::Mapping(lifecycle),
        );
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
            &dir,
        );
        assert_eq!(assessment.state.authority, AuthorityStatus::Revoked);
        assert_eq!(crate::storage::authority_label(&record), "revoked");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn superseded_record_is_not_active_when_revision_is_exact() {
        let dir = temp_repo();
        let mut record = record_with(true, "abc123");
        let mut policy = yaml_serde::Mapping::new();
        policy.insert(
            yaml_serde::Value::String("superseded_by".to_string()),
            yaml_serde::Value::String("constraint.replacement".to_string()),
        );
        record.extra.insert(
            yaml_serde::Value::String("applicability_policy".to_string()),
            yaml_serde::Value::Mapping(policy),
        );
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
            &dir,
        );
        assert_eq!(assessment.state.applicability, Applicability::Superseded);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn head_moved_keeps_applicability_active_but_is_not_exact() {
        let dir = temp_repo();
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
            &dir,
        );
        assert_eq!(
            assessment.revision_consistency,
            Consistency::StructuralIndexBehind
        );
        // Sigue "active": un HEAD distinto no implica que la decisión dejó
        // de aplicar (Rationale_v0.5.md §4.2) — solo que requiere revalidación.
        assert_eq!(assessment.state.applicability, Applicability::Active);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_applicability_when_revision_unresolved() {
        let dir = temp_repo();
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
            &dir,
        );
        assert_eq!(assessment.state.applicability, Applicability::Unknown);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// El defecto real: un Record con `binding_declarations: []` (el caso
    /// del dogfood — `finalize_change` con `changed_files: []`) debe
    /// reportar `Unresolved`, un veredicto honesto ("no hay nada que
    /// verificar"), no `stale` ("algo está roto").
    #[test]
    fn linkage_unresolved_when_record_has_no_bindings() {
        let dir = temp_repo();
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
            &dir,
        );
        assert_eq!(assessment.state.linkage, Linkage::Unresolved);
        assert!(assessment.binding_resolution.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn linkage_stale_when_bound_path_no_longer_exists() {
        let dir = temp_repo();
        // Nunca se crea a.rs — el path_hint apunta a algo que ya no está.
        let record = record_with_bindings(true, "abc123", vec![file_binding("a.rs")]);
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
            &dir,
        );
        assert_eq!(assessment.state.linkage, Linkage::Stale);
        assert!(!assessment.binding_resolution[0].resolved);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// El cambio de diseño real de esta fase: un working tree sucio (o
    /// HEAD movido) ya NO fuerza `stale` — antes `linkage` era un alias de
    /// `revision_consistency`. Ahora son independientes: los bindings
    /// resuelven contra el disco, no contra Git.
    #[test]
    fn dirty_tree_alone_does_not_make_linkage_stale() {
        let dir = temp_repo();
        std::fs::write(dir.join("a.rs"), "// ok\n").unwrap();
        let record = record_with_bindings(true, "abc123", vec![file_binding("a.rs")]);
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: true,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
            &dir,
        );
        assert_eq!(
            assessment.revision_consistency,
            Consistency::WorkingTreeAhead
        );
        assert_eq!(
            assessment.state.linkage,
            Linkage::Current,
            "el árbol sucio afecta revision_consistency, nunca linkage"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Un binding solo-símbolo (sin `path_hint`) no es verificable sin el
    /// proveedor — nunca se asume `Current` por descarte.
    #[test]
    fn symbol_only_binding_without_path_hint_is_unresolved() {
        let dir = temp_repo();
        let symbol_binding = BindingDeclaration {
            id: "binding.symbol".to_string(),
            kind: "symbol".to_string(),
            provider: Some("codebase-memory".to_string()),
            structural_id: Some("function:typescript:foo".to_string()),
            path_hint: None,
            provisional: false,
            extra: yaml_serde::Mapping::new(),
        };
        let record = record_with_bindings(true, "abc123", vec![symbol_binding]);
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
            &dir,
        );
        assert_eq!(assessment.state.linkage, Linkage::Unresolved);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn linkage_current_requires_every_binding_to_resolve() {
        let dir = temp_repo();
        std::fs::write(dir.join("a.rs"), "// ok\n").unwrap();
        // b.rs nunca se crea.
        let record = record_with_bindings(
            true,
            "abc123",
            vec![file_binding("a.rs"), file_binding("b.rs")],
        );
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        let assessment = compute(
            &record,
            &snap,
            ProviderStatus::Successful,
            Coverage::Complete,
            &dir,
        );
        assert_eq!(assessment.state.linkage, Linkage::Stale);
        assert_eq!(assessment.binding_resolution.len(), 2);
        assert!(assessment.binding_resolution.iter().any(|b| b.resolved));
        assert!(assessment.binding_resolution.iter().any(|b| !b.resolved));
        std::fs::remove_dir_all(&dir).ok();
    }
}
