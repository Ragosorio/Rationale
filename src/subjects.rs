//! Subject — Rationale_v0.5.md §5.1, Arquitectura §11.8 (Subject Resolver,
//! versión mínima de lectura; la resolución determinista completa con
//! novelty_reason pertenece a Fase F).
//!
//! El Subject es la identidad conceptual estable de un comportamiento
//! gobernado. Los 7 Subjects fundacionales de `.rationale/subjects/` ya
//! existen desde Fase A pero ningún código los leía hasta ahora.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Subject {
    pub id: String,
    #[serde(rename = "type")]
    pub subject_type: String,
    pub title: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub applies_to: Vec<String>,
}

#[derive(Debug)]
pub enum SubjectError {
    Io(std::io::Error),
    Parse(String),
    MissingRequiredField(&'static str),
}

impl std::fmt::Display for SubjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubjectError::Io(e) => write!(f, "error de I/O leyendo Subject: {e}"),
            SubjectError::Parse(e) => write!(f, "Subject YAML inválido: {e}"),
            SubjectError::MissingRequiredField(field) => {
                write!(f, "Subject inválido: falta campo obligatorio '{field}'")
            }
        }
    }
}

pub fn read_subject(path: &Path) -> Result<Subject, SubjectError> {
    let content = std::fs::read_to_string(path).map_err(SubjectError::Io)?;
    let subject: Subject =
        yaml_serde::from_str(&content).map_err(|e| SubjectError::Parse(e.to_string()))?;

    if subject.id.is_empty() {
        return Err(SubjectError::MissingRequiredField("id"));
    }
    if subject.title.is_empty() {
        return Err(SubjectError::MissingRequiredField("title"));
    }
    Ok(subject)
}

/// Lista todos los Subjects en `.rationale/subjects/`.
pub fn list_subjects(subjects_dir: &Path) -> Result<Vec<Subject>, SubjectError> {
    let mut subjects = Vec::new();
    if !subjects_dir.is_dir() {
        return Ok(subjects);
    }
    let entries = std::fs::read_dir(subjects_dir).map_err(SubjectError::Io)?;
    for entry in entries {
        let entry = entry.map_err(SubjectError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            subjects.push(read_subject(&path)?);
        }
    }
    Ok(subjects)
}

/// Resuelve un Subject por ID exacto o alias — orden 1 y 2 de
/// `Rationale_v0.5.md §9.1` ("Subject Resolver"). Los demás pasos (overlap
/// de bindings, FTS, similitud semántica) pertenecen a Fase F.
pub fn resolve_by_id_or_alias<'a>(
    subjects: &'a [Subject],
    id_or_alias: &str,
) -> Option<&'a Subject> {
    subjects
        .iter()
        .find(|s| s.id == id_or_alias || s.aliases.iter().any(|a| a == id_or_alias))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_real_subject() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".rationale/subjects/architecture.provider-boundary.yaml");
        let subject = read_subject(&path).unwrap();
        assert_eq!(subject.id, "architecture.provider-boundary");
        assert_eq!(subject.subject_type, "system-behavior");
    }

    #[test]
    fn lists_all_seven_foundational_subjects() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rationale/subjects");
        let subjects = list_subjects(&dir).unwrap();
        assert_eq!(
            subjects.len(),
            7,
            "deben existir los 7 Subjects fundacionales de Fase A"
        );
    }

    #[test]
    fn resolves_by_alias() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rationale/subjects");
        let subjects = list_subjects(&dir).unwrap();
        let found = resolve_by_id_or_alias(&subjects, "architecture.codebase-memory-boundary");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "architecture.provider-boundary");
    }

    #[test]
    fn resolve_returns_none_for_unknown() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".rationale/subjects");
        let subjects = list_subjects(&dir).unwrap();
        assert!(resolve_by_id_or_alias(&subjects, "no.existe").is_none());
    }
}
