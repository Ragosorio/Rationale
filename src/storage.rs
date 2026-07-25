//! Canonical Store — lee Records desde `.rationale/records/` y valida los
//! campos mínimos exigidos (Arquitectura §11.6). La derivada/índice local
//! (SQLite, FTS) pertenece a Fase E — esta vertical slice lee YAML
//! directamente, sin capa derivada todavía.

use serde::Deserialize;
use std::path::Path;

// Campos como `id`, `kind`, `provider`, `path_hint` (BindingDeclaration),
// `actor`/`authority` (Approval) y `Record.kind` reflejan el schema completo
// de Rationale_v0.5.md §5.2-5.3 para que la deserialización sea fiel al
// formato real, aunque esta vertical slice mínima (Fase D) todavía no
// bifurca lógica sobre ellos — sí lo harán el Subject Resolver y el Trust
// Evaluator en Fase E/F.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BindingDeclaration {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub provider: Option<String>,
    pub structural_id: Option<String>,
    pub path_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Approval {
    pub actor: String,
    pub authority: String,
    pub status: String,
}

/// Evidence — Rationale_v0.5.md §5.4. Describe evidencia verificable o
/// referenciada; nunca el contenido íntegro (v0.5 §4.11, minimización).
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Evidence {
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub path: Option<String>,
    pub revision: Option<String>,
    #[serde(default)]
    pub verified: bool,
    pub content_hash: Option<String>,
    pub visibility: Option<String>,
}

/// Estado epistemológico de una afirmación (Rationale_v0.5.md §10, §12.1).
/// Un Record nuevo sin este campo se asume `Stated` (afirmación humana
/// explícita) — es el caso más común de captura manual; nunca se asume
/// `Observed` por defecto, porque eso implicaría verificación mecánica que
/// no ocurrió.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EpistemicStatus {
    Observed,
    #[default]
    Stated,
    Corroborated,
    Inferred,
    Disputed,
    Unknown,
}

impl std::fmt::Display for EpistemicStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EpistemicStatus::Observed => "observed",
            EpistemicStatus::Stated => "stated",
            EpistemicStatus::Corroborated => "corroborated",
            EpistemicStatus::Inferred => "inferred",
            EpistemicStatus::Disputed => "disputed",
            EpistemicStatus::Unknown => "unknown",
        };
        write!(f, "{s}")
    }
}

/// Referencia embebida al Subject dentro del Record (Rationale_v0.5.md
/// §27: el Record incluye una copia de `id`/`type`/`title` por comodidad y
/// portabilidad). `subjects::resolve_by_id_or_alias` valida esta referencia
/// contra el canon real en `.rationale/subjects/`.
#[derive(Debug, Deserialize, Clone)]
pub struct RecordSubjectRef {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Record {
    pub id: String,
    #[allow(dead_code)]
    pub kind: String,
    pub severity: String,
    pub statement: String,
    #[serde(default)]
    pub epistemic_status: EpistemicStatus,
    #[serde(default)]
    pub approvals: Vec<Approval>,
    #[serde(default)]
    pub binding_declarations: Vec<BindingDeclaration>,
    #[serde(default)]
    #[allow(dead_code)] // consumido por Trust Evaluator en Fase F (minimización, v0.5 §4.11)
    pub evidence: Vec<Evidence>,
    pub bound_revision: Option<String>,
    pub subject: Option<RecordSubjectRef>,
}

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Parse(String),
    MissingRequiredField(&'static str),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "error de I/O leyendo Record: {e}"),
            StorageError::Parse(e) => write!(f, "Record YAML inválido: {e}"),
            StorageError::MissingRequiredField(field) => {
                write!(f, "Record inválido: falta campo obligatorio '{field}'")
            }
        }
    }
}

/// Lee y valida un Record. La validación es mínima y determinista — no
/// depende de un LLM ni de heurísticas (policy.no-inferred-blocks).
pub fn read_record(path: &Path) -> Result<Record, StorageError> {
    let content = std::fs::read_to_string(path).map_err(StorageError::Io)?;
    let record: Record =
        yaml_serde::from_str(&content).map_err(|e| StorageError::Parse(e.to_string()))?;

    if record.id.is_empty() {
        return Err(StorageError::MissingRequiredField("id"));
    }
    if record.statement.is_empty() {
        return Err(StorageError::MissingRequiredField("statement"));
    }
    if record.severity.is_empty() {
        return Err(StorageError::MissingRequiredField("severity"));
    }

    Ok(record)
}

/// Un Record solo puede bloquear si cumple TODAS las condiciones de
/// `Rationale_v0.5.md §10.7` / `.rationale/subjects/policy.no-inferred-blocks.yaml`.
/// Esta vertical slice no implementa bloqueo todavía (Fase F), pero calcula
/// la señal de autoridad aprobada que ese futuro predicado necesitará.
pub fn has_approved_authority(record: &Record) -> bool {
    record.approvals.iter().any(|a| a.status == "approved")
}

/// Lista todos los Records en un directorio `.rationale/records/`.
pub fn list_records(records_dir: &Path) -> Result<Vec<Record>, StorageError> {
    let mut records = Vec::new();
    if !records_dir.is_dir() {
        return Ok(records);
    }
    let entries = std::fs::read_dir(records_dir).map_err(StorageError::Io)?;
    for entry in entries {
        let entry = entry.map_err(StorageError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            records.push(read_record(&path)?);
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_fixture_record_with_approval_and_binding() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "fixtures/vertical-slice/.rationale/records/constraint.no-global-admin-for-staff.yaml",
        );
        let record = read_record(&path).unwrap();
        assert_eq!(record.id, "constraint.no-global-admin-for-staff");
        assert_eq!(record.severity, "critical");
        assert!(has_approved_authority(&record));
        assert_eq!(record.binding_declarations.len(), 1);
        assert_eq!(
            record.binding_declarations[0].structural_id.as_deref(),
            Some("function:typescript:auth.resolveEntityRole")
        );
    }

    #[test]
    fn reads_real_project_record_without_approval() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".rationale/records/constraint.no-provider-internal-access.yaml");
        let record = read_record(&path).unwrap();
        assert_eq!(record.id, "constraint.no-provider-internal-access");
        assert!(
            !has_approved_authority(&record),
            "no debe tener autoridad aprobada todavía"
        );
        assert_eq!(
            record.binding_declarations.len(),
            1,
            "D3-D4 añadió el binding real hacia providers::codebase_memory"
        );
    }

    #[test]
    fn rejects_record_missing_statement() {
        let dir =
            std::env::temp_dir().join(format!("rationale-storage-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bad_path = dir.join("bad.yaml");
        std::fs::write(
            &bad_path,
            "id: constraint.bad\nkind: constraint\nseverity: critical\nstatement: \"\"\n",
        )
        .unwrap();
        let result = read_record(&bad_path);
        assert!(matches!(
            result,
            Err(StorageError::MissingRequiredField("statement"))
        ));
        std::fs::remove_dir_all(&dir).ok();
    }
}
