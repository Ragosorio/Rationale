//! `rationale doctor` — Fase 1.5: detecta defectos de integridad en el
//! canon que el productor roto (Fase 1.1-1.4 los corrigió en el código,
//! pero no en los datos ya escritos) dejó en `.rationale/`. Read-only por
//! defecto — reparar exige `--repair` y confirmación humana por hallazgo,
//! nunca automática (Proceso §21, mismo principio que `rationale review`).
//!
//! Los cuatro repos que vieron el productor roto (este, Monorepo,
//! BoostAPI, y el proyecto del dogfood real) ya tienen canon escrito con
//! severidades fuera de enum, Records sin bindings, y Subjects colgantes
//! — `doctor` es la herramienta para encontrarlos, no una migración de
//! versión (eso es `.rationale/migrations/`).

use crate::{review, storage, subjects};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Finding {
    /// Defecto real: el MCP defaulteaba `severity` a `"normal"`, fuera del
    /// enum del schema — invisible en retrieval sin ningún error.
    InvalidSeverity {
        record_id: String,
        path: PathBuf,
        current: String,
    },
    /// Un Record sin ningún binding no puede gobernar ningún target —
    /// `linkage: unresolved` por diseño (Fase 1.4). Solo se reporta: un
    /// binding inventado sería peor que ninguno.
    RecordWithoutBindings { record_id: String, path: PathBuf },
    /// Un `path_hint` que ya no existe en el working tree — `linkage:
    /// stale` para ese Record.
    BindingPathMissing {
        record_id: String,
        path: PathBuf,
        binding_id: String,
        path_hint: String,
    },
    /// `record.subject.id` no resuelve contra ningún Subject real en
    /// `.rationale/subjects/` — el defecto original del dogfood, previo a
    /// que `finalize_change` materializara el Subject (Fase 1.3).
    DanglingSubject {
        record_id: String,
        path: PathBuf,
        subject_id: String,
    },
    /// Un Record ya en `records/` (canon aprobado) sin ninguna
    /// `Approval` — solo alcanzable escribiendo el archivo a mano, nunca
    /// vía `rationale review`. Nunca se repara automáticamente: podría
    /// ser evidencia de una violación de Proceso §21 que alguien necesita
    /// investigar, no solo "arreglar".
    RecordInCanonWithoutApproval { record_id: String, path: PathBuf },
    /// `.rationale/bindings/` ya no lo crea `cmd_init` (nada lo lee ni
    /// escribe) — si existe y está vacío, es seguro borrarlo a mano.
    EmptyBindingsDirectory { path: PathBuf },
}

impl Finding {
    pub fn is_repairable(&self) -> bool {
        matches!(
            self,
            Finding::InvalidSeverity { .. } | Finding::DanglingSubject { .. }
        )
    }

    pub fn describe(&self) -> String {
        match self {
            Finding::InvalidSeverity {
                record_id, current, ..
            } => format!(
                "'{record_id}' tiene severity '{current}', fuera del enum ({}) — nunca decide \
                 visibilidad, pero conviene corregirla para que el orden en retrieval sea real",
                storage::Severity::ALL.join(", ")
            ),
            Finding::RecordWithoutBindings { record_id, .. } => format!(
                "'{record_id}' no tiene ningún binding_declaration — no puede gobernar ningún \
                 target (linkage: unresolved). No se repara solo: inventar un binding sería \
                 peor que no tener ninguno."
            ),
            Finding::BindingPathMissing {
                record_id,
                binding_id,
                path_hint,
                ..
            } => format!(
                "'{record_id}': el binding '{binding_id}' apunta a '{path_hint}', que ya no \
                 existe en el working tree (linkage: stale)."
            ),
            Finding::DanglingSubject {
                record_id,
                subject_id,
                ..
            } => format!(
                "'{record_id}' referencia el Subject '{subject_id}', que no existe en \
                 .rationale/subjects/ — se puede materializar un stub con review.status: \
                 unreviewed."
            ),
            Finding::RecordInCanonWithoutApproval { record_id, .. } => format!(
                "'{record_id}' está en records/ (canon aprobado) pero approvals está vacío — \
                 esto es estructuralmente inalcanzable vía 'rationale review'; investigar antes \
                 de tocarlo, nunca reparar automáticamente."
            ),
            Finding::EmptyBindingsDirectory { path } => format!(
                "{} existe y está vacío — nada en el código lo lee ni lo escribe; seguro de \
                 borrar a mano.",
                path.display()
            ),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub findings: Vec<Finding>,
}

/// Solo lectura — nunca escribe nada. `project_root` es contra qué se
/// verifica la existencia de cada `path_hint` (mismo criterio que
/// `assessment::linkage`, Fase 1.4).
pub fn check(rationale_dir: &Path, project_root: &Path) -> DoctorReport {
    let mut findings = Vec::new();

    let records_dir = rationale_dir.join("records");
    let records_result = storage::list_records_detailed(&records_dir).unwrap_or_else(|_| {
        storage::RecordListResult {
            records: vec![],
            skipped: vec![],
        }
    });

    let subjects_dir = rationale_dir.join("subjects");
    let known_subjects = subjects::list_subjects(&subjects_dir).unwrap_or_default();

    for record in &records_result.records {
        let path = records_dir.join(format!("{}.yaml", record.id));

        if storage::severity_of(record).is_none() {
            findings.push(Finding::InvalidSeverity {
                record_id: record.id.clone(),
                path: path.clone(),
                current: record.severity.clone(),
            });
        }

        if record.binding_declarations.is_empty() {
            findings.push(Finding::RecordWithoutBindings {
                record_id: record.id.clone(),
                path: path.clone(),
            });
        } else {
            for binding in &record.binding_declarations {
                if let Some(hint) = &binding.path_hint {
                    if !project_root.join(hint).exists() {
                        findings.push(Finding::BindingPathMissing {
                            record_id: record.id.clone(),
                            path: path.clone(),
                            binding_id: binding.id.clone(),
                            path_hint: hint.clone(),
                        });
                    }
                }
            }
        }

        if let Some(subject_ref) = &record.subject {
            if subjects::resolve_by_id_or_alias(&known_subjects, &subject_ref.id).is_none() {
                findings.push(Finding::DanglingSubject {
                    record_id: record.id.clone(),
                    path: path.clone(),
                    subject_id: subject_ref.id.clone(),
                });
            }
        }

        if record.approvals.is_empty() {
            findings.push(Finding::RecordInCanonWithoutApproval {
                record_id: record.id.clone(),
                path,
            });
        }
    }

    let bindings_dir = rationale_dir.join("bindings");
    if bindings_dir.is_dir() {
        let is_empty = std::fs::read_dir(&bindings_dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if is_empty {
            findings.push(Finding::EmptyBindingsDirectory { path: bindings_dir });
        }
    }

    DoctorReport { findings }
}

/// Repara UN hallazgo — siempre con la palabra de confirmación ya validada
/// por el caller (misma disciplina que `rationale review`: nunca se
/// autoaprueba, nunca se repara en lote sin que un humano vea cada uno).
/// Las mutaciones de Records pasan por `review::mutate_record` — nunca un
/// side-channel que esquive el audit trail de lifecycle (`Record.extra.
/// lifecycle.events`).
pub fn repair(
    rationale_dir: &Path,
    finding: &Finding,
    reviewer_actor: &str,
) -> Result<PathBuf, String> {
    match finding {
        Finding::InvalidSeverity {
            record_id, current, ..
        } => Err(format!(
            "InvalidSeverity para '{record_id}' (actual: '{current}') no se repara sin que el \
             humano elija el nuevo valor — usar repair_severity()"
        )),
        Finding::DanglingSubject {
            record_id,
            subject_id,
            ..
        } => {
            let subjects_dir = rationale_dir.join("subjects");
            let records_dir = rationale_dir.join("records");
            let record_path = records_dir.join(format!("{record_id}.yaml"));
            let record = storage::read_record(&record_path).map_err(|e| e.to_string())?;
            let title = record
                .subject
                .as_ref()
                .and_then(|s| {
                    let title_key = yaml_serde::Value::String("title".to_string());
                    s.extra.get(&title_key).and_then(|v| v.as_str())
                })
                .unwrap_or(subject_id)
                .to_string();
            match subjects::materialize_proposed(
                &subjects_dir,
                subject_id,
                &title,
                "",
                "project",
                &[],
                reviewer_actor,
                &crate::evaluation::now_iso8601(),
            )
            .map_err(|e| e.to_string())?
            {
                subjects::MaterializeOutcome::Created(path) => Ok(path),
                subjects::MaterializeOutcome::AlreadyExists(path) => Ok(path),
            }
        }
        _ => Err("este tipo de hallazgo no es reparable automáticamente — ver describe()".into()),
    }
}

/// Repara `InvalidSeverity` con el valor que el humano eligió — separado
/// de `repair()` porque exige un input adicional (`new_severity`) que solo
/// tiene sentido pedido interactivamente, nunca inferido.
pub fn repair_severity(
    rationale_dir: &Path,
    record_id: &str,
    new_severity: &str,
    reason: &str,
    reviewer_actor: &str,
    reviewer_role: storage::AuthorityRole,
    reviewer_declared: bool,
) -> Result<PathBuf, String> {
    review::mutate_record(
        rationale_dir,
        record_id,
        review::RecordMutation::CorrectSeverity {
            new_severity: new_severity.to_string(),
            reason: reason.to_string(),
        },
        reviewer_actor,
        reviewer_role,
        reviewer_declared,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(label: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-doctor-test-{label}-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_record_yaml(records_dir: &Path, id: &str, content: &str) {
        std::fs::create_dir_all(records_dir).unwrap();
        std::fs::write(records_dir.join(format!("{id}.yaml")), content).unwrap();
    }

    #[test]
    fn detects_invalid_severity() {
        let project = unique_dir("severity");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.legacy",
            "id: constraint.legacy\nkind: constraint\nseverity: normal\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(
            |f| matches!(f, Finding::InvalidSeverity { record_id, current, .. } if record_id == "constraint.legacy" && current == "normal")
        ));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_record_without_bindings() {
        let project = unique_dir("no-bindings");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.no-bindings",
            "id: constraint.no-bindings\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::RecordWithoutBindings { record_id, .. } if record_id == "constraint.no-bindings")));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_missing_binding_path() {
        let project = unique_dir("missing-path");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.missing-path",
            "id: constraint.missing-path\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: does/not/exist.rs\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::BindingPathMissing { record_id, path_hint, .. }
                if record_id == "constraint.missing-path" && path_hint == "does/not/exist.rs"
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_dangling_subject() {
        let project = unique_dir("dangling-subject");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.dangling",
            "id: constraint.dangling\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\nsubject:\n  id: some.missing-subject\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::DanglingSubject { record_id, subject_id, .. }
                if record_id == "constraint.dangling" && subject_id == "some.missing-subject"
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_record_without_approval() {
        let project = unique_dir("no-approval");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.no-approval",
            "id: constraint.no-approval\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals: []\n",
        );

        let report = check(&rationale_dir, &project);
        assert!(report.findings.iter().any(|f| matches!(
            f,
            Finding::RecordInCanonWithoutApproval { record_id, .. }
                if record_id == "constraint.no-approval"
        )));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn detects_empty_bindings_directory() {
        let project = unique_dir("empty-bindings-dir");
        let rationale_dir = project.join(".rationale");
        std::fs::create_dir_all(rationale_dir.join("bindings")).unwrap();

        let report = check(&rationale_dir, &project);
        assert!(report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::EmptyBindingsDirectory { .. })));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn clean_project_has_no_findings() {
        let project = unique_dir("clean");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.clean",
            "id: constraint.clean\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();

        let report = check(&rationale_dir, &project);
        assert!(
            report.findings.is_empty(),
            "no debía haber hallazgos: {:?}",
            report.findings
        );

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn repair_severity_writes_through_review_mutate_record_with_lifecycle_event() {
        let project = unique_dir("repair-severity");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.legacy",
            "id: constraint.legacy\nkind: constraint\nseverity: normal\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\n",
        );

        let path = repair_severity(
            &rationale_dir,
            "constraint.legacy",
            "high",
            "corregido por rationale doctor",
            "user:doctor-test",
            storage::AuthorityRole::Contributor,
            true,
        )
        .unwrap();

        let record = storage::read_record(&path).unwrap();
        assert_eq!(record.severity, "high");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("severity-corrected"));

        std::fs::remove_dir_all(&project).ok();
    }

    #[test]
    fn repair_dangling_subject_materializes_stub() {
        let project = unique_dir("repair-subject");
        let rationale_dir = project.join(".rationale");
        write_record_yaml(
            &rationale_dir.join("records"),
            "constraint.dangling",
            "id: constraint.dangling\nkind: constraint\nseverity: high\nstatement: \"x\"\napprovals:\n  - actor: \"user:x\"\n    authority: contributor\n    status: approved\nbinding_declarations:\n  - id: binding.a\n    type: file\n    path_hint: README.md\nsubject:\n  id: some.missing-subject\n  title: Missing subject\n",
        );
        std::fs::write(project.join("README.md"), "x\n").unwrap();

        let report = check(&rationale_dir, &project);
        let finding = report
            .findings
            .iter()
            .find(|f| matches!(f, Finding::DanglingSubject { .. }))
            .unwrap();

        let path = repair(&rationale_dir, finding, "user:doctor-test").unwrap();
        assert!(path.exists());

        let report_after = check(&rationale_dir, &project);
        assert!(!report_after
            .findings
            .iter()
            .any(|f| matches!(f, Finding::DanglingSubject { .. })));

        std::fs::remove_dir_all(&project).ok();
    }
}
