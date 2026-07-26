//! Canonical Store — lee y escribe Records en `.rationale/records/`,
//! validando los campos mínimos exigidos (Arquitectura §11.6).
//!
//! Fidelidad de round-trip (Fase F1): el Record canónico real tiene ~24
//! campos de nivel superior (`schema_version`, `project_id`, `title`,
//! `scope`, `constraint_expression`, `problem`, `intent`, `provenance`,
//! `applicability_policy`, `binding_policy`, `context_policy`,
//! `sensitivity`...) y varios campos anidados no modelados (`scope` en cada
//! binding, `type`/`title` en la referencia a Subject, `domain`/`approved_at`
//! en cada Approval). Antes de F1 esto no importaba porque el módulo solo
//! leía; en cuanto se escribe, un round-trip ingenuo (leer → modificar →
//! escribir) borraría esos campos en silencio — exactamente lo que
//! `Arquitectura §11.7` prohíbe para la base derivada, y con más razón para
//! el canon mismo. Cada struct de este archivo captura lo no modelado en un
//! `extra: yaml_serde::Mapping` vía `#[serde(flatten)]`: los campos que la
//! lógica de Rationale sí necesita siguen siendo campos Rust tipados; el
//! resto viaja intacto. Verificado empíricamente (round-trip semántico
//! completo, cero pérdida) antes de adoptar este diseño — ver
//! `storage::tests::real_record_roundtrip_loses_no_data`.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Roles válidos para una Approval. La configuración del proyecto declara
/// qué actor tiene cada rol; un actor no declarado queda como contributor.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityRole {
    Contributor,
    DomainMaintainer,
    DomainOwner,
    SecurityOwner,
    ProductOwner,
    ArchitectureOwner,
    RepositoryPolicy,
}

impl AuthorityRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Contributor => "contributor",
            Self::DomainMaintainer => "domain-maintainer",
            Self::DomainOwner => "domain-owner",
            Self::SecurityOwner => "security-owner",
            Self::ProductOwner => "product-owner",
            Self::ArchitectureOwner => "architecture-owner",
            Self::RepositoryPolicy => "repository-policy",
        }
    }
}

impl std::fmt::Display for AuthorityRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn is_valid_authority(value: &str) -> bool {
    matches!(
        value,
        "contributor"
            | "domain-maintainer"
            | "domain-owner"
            | "security-owner"
            | "product-owner"
            | "architecture-owner"
            | "repository-policy"
    )
}

// Campos como `id`, `kind`, `provider`, `path_hint` (BindingDeclaration),
// `actor`/`authority` (Approval) y `Record.kind` reflejan el schema completo
// de Rationale_v0.5.md §5.2-5.3 para que la deserialización sea fiel al
// formato real, aunque esta vertical slice mínima (Fase D) todavía no
// bifurca lógica sobre ellos — sí lo harán el Subject Resolver y el Trust
// Evaluator en Fase E/F.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct BindingDeclaration {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub provider: Option<String>,
    pub structural_id: Option<String>,
    pub path_hint: Option<String>,
    /// Campos reales no modelados (p. ej. `scope`) — ver doc del módulo.
    #[serde(flatten)]
    pub extra: yaml_serde::Mapping,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct Approval {
    pub actor: String,
    pub authority: String,
    pub status: String,
    /// Campos reales no modelados (p. ej. `domain`, `approved_at`).
    #[serde(flatten)]
    pub extra: yaml_serde::Mapping,
}

/// Evidence — Rationale_v0.5.md §5.4. Describe evidencia verificable o
/// referenciada; nunca el contenido íntegro (v0.5 §4.11, minimización).
#[derive(Debug, Deserialize, Serialize, Clone)]
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
    #[serde(flatten)]
    pub extra: yaml_serde::Mapping,
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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecordSubjectRef {
    pub id: String,
    /// Campos reales no modelados (`type`, `title`) — ver doc del módulo.
    #[serde(flatten)]
    pub extra: yaml_serde::Mapping,
}

/// Riesgo conocido asociado a un Record (Rationale_v0.5.md §27 ejemplo
/// `risks:`). No es una entidad persistida independiente en v1 (§8.1) —
/// vive embebido en el Record hasta que un caso real justifique lo contrario.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Risk {
    // `id` y `epistemic_status` reflejan el schema completo (v0.5 §27);
    // retrieval.rs solo expone `statement` en el packet por ahora — el
    // Trust Evaluator de Fase F usará epistemic_status para decidir cómo
    // presentar un riesgo no corroborado frente a uno observado.
    #[allow(dead_code)]
    pub id: String,
    pub statement: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub epistemic_status: EpistemicStatus,
    #[serde(flatten)]
    pub extra: yaml_serde::Mapping,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Record {
    pub id: String,
    pub kind: String,
    pub severity: String,
    pub statement: String,
    /// El motivo, separado de la afirmación normativa misma (nivel 3 de
    /// v0.5 §18.1, "razón principal").
    pub rationale: Option<String>,
    #[serde(default)]
    pub epistemic_status: EpistemicStatus,
    #[serde(default)]
    pub approvals: Vec<Approval>,
    #[serde(default)]
    pub binding_declarations: Vec<BindingDeclaration>,
    #[serde(default)]
    #[allow(dead_code)] // consumido por Trust Evaluator en Fase F (minimización, v0.5 §4.11)
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub risks: Vec<Risk>,
    pub bound_revision: Option<String>,
    pub subject: Option<RecordSubjectRef>,
    /// Campos reales no modelados (`schema_version`, `project_id`, `title`,
    /// `scope`, `constraint_expression`, `problem`, `intent`, `provenance`,
    /// `applicability_policy`, `binding_policy`, `context_policy`,
    /// `sensitivity`...) — ver doc del módulo. Verificado: un Record real
    /// completo sobrevive un round-trip leer→escribir→leer sin perder
    /// ninguno de estos campos.
    #[serde(flatten)]
    pub extra: yaml_serde::Mapping,
}

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Parse(String),
    Serialize(String),
    MissingRequiredField(&'static str),
    /// Un `id` que no es seguro para usarse como nombre de archivo — nunca
    /// se convierte silenciosamente en un path (path traversal real,
    /// encontrado y corregido durante la verificación de fin de Fase F).
    UnsafeIdentifier(String),
    InvalidAuthority(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "error de I/O en Record: {e}"),
            StorageError::Parse(e) => write!(f, "Record YAML inválido: {e}"),
            StorageError::Serialize(e) => write!(f, "no se pudo serializar el Record: {e}"),
            StorageError::MissingRequiredField(field) => {
                write!(f, "Record inválido: falta campo obligatorio '{field}'")
            }
            StorageError::UnsafeIdentifier(id) => {
                write!(f, "id inseguro para usar como nombre de archivo: '{id}'")
            }
            StorageError::InvalidAuthority(authority) => {
                write!(f, "autoridad inválida: '{authority}'")
            }
        }
    }
}

/// Valida que un `id` sea seguro para convertirse en un nombre de archivo
/// real — nunca debe interpretarse como un path. Rechaza vacío, `.`/`..`,
/// separadores de path y NUL. Obligatorio en cualquier lugar donde un `id`
/// (que puede venir de contenido YAML no confiable — una propuesta escrita
/// por `finalize_change` a partir de argumentos de un cliente MCP) se
/// convierte en un nombre de archivo (`finalize_change`, `rationale
/// review`). Sin esto, un `record_id` como `"../../../etc/pwned"` escribe
/// fuera de `.rationale/` — vulnerabilidad real confirmada empíricamente
/// antes de este fix.
pub fn validate_safe_id(id: &str) -> Result<(), StorageError> {
    if id.is_empty() {
        return Err(StorageError::MissingRequiredField("id"));
    }
    let is_unsafe =
        id == "." || id == ".." || id.contains('/') || id.contains('\\') || id.contains('\0');
    if is_unsafe {
        return Err(StorageError::UnsafeIdentifier(id.to_string()));
    }
    Ok(())
}

/// Validación mínima y determinista — no depende de un LLM ni de
/// heurísticas (`policy.no-inferred-blocks`). Compartida entre lectura y
/// escritura: nunca se persiste un Record que no pasaría esta misma
/// validación al releerlo.
fn validate(record: &Record) -> Result<(), StorageError> {
    if record.id.is_empty() {
        return Err(StorageError::MissingRequiredField("id"));
    }
    if record.statement.is_empty() {
        return Err(StorageError::MissingRequiredField("statement"));
    }
    if record.severity.is_empty() {
        return Err(StorageError::MissingRequiredField("severity"));
    }
    for approval in &record.approvals {
        if !is_valid_authority(&approval.authority) {
            return Err(StorageError::InvalidAuthority(approval.authority.clone()));
        }
    }
    Ok(())
}

/// Lee y valida un Record.
pub fn read_record(path: &Path) -> Result<Record, StorageError> {
    let content = std::fs::read_to_string(path).map_err(StorageError::Io)?;
    let record: Record =
        yaml_serde::from_str(&content).map_err(|e| StorageError::Parse(e.to_string()))?;
    validate(&record)?;
    Ok(record)
}

/// Escribe un Record de forma atómica (`Arquitectura §11.6`/`§15.3`: "usar
/// archivos temporales y rename atómico"). Nunca escribe in-place: serializa
/// a un archivo temporal en el mismo directorio (garantiza mismo
/// filesystem, condición necesaria para que `rename` sea atómico en
/// POSIX), fuerza los bytes a disco, y solo entonces renombra sobre el
/// destino final. Si el proceso muere entre medio, el archivo original
/// queda intacto — nunca truncado ni a medio escribir.
pub fn write_record(path: &Path, record: &Record) -> Result<(), StorageError> {
    validate(record)?;

    let yaml = yaml_serde::to_string(record).map_err(|e| StorageError::Serialize(e.to_string()))?;

    let dir = path.parent().ok_or_else(|| {
        StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "el path del Record no tiene directorio padre",
        ))
    })?;
    // `.rationale/proposals/` puede no existir todavía (Fase F5 es quien
    // primero escribe ahí) — crearlo es responsabilidad de la escritura
    // atómica, no de cada caller.
    std::fs::create_dir_all(dir).map_err(StorageError::Io)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("record.yaml");
    // El nombre temporal debe ser único incluso entre dos hilos del MISMO
    // proceso escribiendo el mismo Record a la vez — el PID solo no basta
    // (dos hilos comparten PID y clobbearían el mismo .tmp antes del
    // rename). Un contador atómico por proceso lo resuelve sin necesitar
    // una dependencia externa de UUIDs.
    static WRITE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = WRITE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp_path = dir.join(format!(".{file_name}.tmp-{}-{unique}", std::process::id()));

    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp_path).map_err(StorageError::Io)?;
        file.write_all(yaml.as_bytes()).map_err(StorageError::Io)?;
        file.sync_all().map_err(StorageError::Io)?;
    }

    std::fs::rename(&tmp_path, path).map_err(StorageError::Io)?;
    Ok(())
}

/// Un Record solo puede bloquear si cumple TODAS las condiciones de
/// `Rationale_v0.5.md §10.7` / `.rationale/subjects/policy.no-inferred-blocks.yaml`.
/// Esta vertical slice no implementa bloqueo todavía (Fase F), pero calcula
/// la señal de autoridad aprobada que ese futuro predicado necesitará.
pub fn has_approved_authority(record: &Record) -> bool {
    !is_revoked(record) && record.approvals.iter().any(|a| a.status == "approved")
}

/// Etiqueta estable para packets y diagnostics. Revocado tiene prioridad
/// sobre aprobaciones históricas porque esas aprobaciones siguen en el canon
/// solo como evidencia de la decisión anterior.
pub fn authority_label(record: &Record) -> &'static str {
    if is_revoked(record) {
        "revoked"
    } else if has_approved_authority(record) {
        "approved"
    } else {
        "unreviewed"
    }
}

/// Estado de lifecycle persistido por `review_record`. Se conserva dentro
/// de `extra` para que Records completos de v0.5 sobrevivan sin pérdida de
/// campos no modelados.
pub fn lifecycle_status(record: &Record) -> Option<&str> {
    let lifecycle_key = yaml_serde::Value::String("lifecycle".to_string());
    let status_key = yaml_serde::Value::String("status".to_string());
    match record.extra.get(&lifecycle_key) {
        Some(yaml_serde::Value::Mapping(lifecycle)) => {
            lifecycle.get(&status_key).and_then(|value| value.as_str())
        }
        _ => None,
    }
}

/// Un Record revocado nunca vuelve a ser una constraint autorizada aunque
/// conserve sus aprobaciones históricas como evidencia auditable.
pub fn is_revoked(record: &Record) -> bool {
    lifecycle_status(record) == Some("revoked")
}

/// Devuelve el Record que supersede al actual, si existe una transición
/// explícita de lifecycle.
pub fn superseded_by(record: &Record) -> Option<&str> {
    let policy_key = yaml_serde::Value::String("applicability_policy".to_string());
    let superseded_key = yaml_serde::Value::String("superseded_by".to_string());
    match record.extra.get(&policy_key) {
        Some(yaml_serde::Value::Mapping(policy)) => policy
            .get(&superseded_key)
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty()),
        _ => None,
    }
}

/// Resultado detallado de listar Records: los que sí parsearon, y los que
/// no (con el path y el error) — nunca se descartan en silencio. Ver
/// `list_records_detailed`.
pub struct RecordListResult {
    pub records: Vec<Record>,
    pub skipped: Vec<(std::path::PathBuf, StorageError)>,
}

/// Lista todos los Records en un directorio, reportando qué archivos no
/// pudieron leerse (revisión adversarial de Fase F, hallazgo 1: la versión
/// anterior abortaba TODA la lectura ante un solo archivo corrupto,
/// apagando el Subject Resolver para el canon entero en silencio para
/// cualquier caller que usara `.unwrap_or_default()`). Un archivo inválido
/// se salta — nunca descarta los demás Records ya leídos.
pub fn list_records_detailed(records_dir: &Path) -> Result<RecordListResult, StorageError> {
    let mut records = Vec::new();
    let mut skipped = Vec::new();
    if !records_dir.is_dir() {
        return Ok(RecordListResult { records, skipped });
    }
    let entries = std::fs::read_dir(records_dir).map_err(StorageError::Io)?;
    for entry in entries {
        let entry = entry.map_err(StorageError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            match read_record(&path) {
                Ok(record) => records.push(record),
                Err(e) => skipped.push((path, e)),
            }
        }
    }
    Ok(RecordListResult { records, skipped })
}

/// Variante simple para callers que no necesitan saber qué se saltó (la
/// mayoría). Nunca fallar por UN archivo corrupto es la garantía real —
/// solo falla si el directorio mismo no pudo leerse (I/O real).
pub fn list_records(records_dir: &Path) -> Result<Vec<Record>, StorageError> {
    Ok(list_records_detailed(records_dir)?.records)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vulnerabilidad real encontrada y corregida durante la verificación de
    /// fin de Fase F: un `record_id` con `../` escribía fuera de
    /// `.rationale/` (confirmado escribiendo un archivo real fuera del
    /// directorio del proyecto antes de este fix).
    #[test]
    fn validate_safe_id_rejects_path_traversal_attempts() {
        for malicious in [
            "../../../../etc/pwned",
            "..",
            ".",
            "sub/dir",
            "back\\slash",
            "nul\0byte",
        ] {
            assert!(
                validate_safe_id(malicious).is_err(),
                "'{malicious}' debe rechazarse como id inseguro"
            );
        }
    }

    #[test]
    fn validate_safe_id_accepts_normal_record_ids() {
        for safe in [
            "constraint.no-provider-internal-access",
            "constraint.no-double-charge",
            "decision.foo_bar-123",
        ] {
            assert!(validate_safe_id(safe).is_ok(), "'{safe}' debe aceptarse");
        }
    }

    #[test]
    fn approval_authority_must_match_declared_schema_enum() {
        let mut record = Record {
            id: "constraint.authority-test".to_string(),
            kind: "constraint".to_string(),
            severity: "high".to_string(),
            statement: "valid statement".to_string(),
            rationale: None,
            epistemic_status: EpistemicStatus::Stated,
            approvals: vec![Approval {
                actor: "user:test".to_string(),
                authority: "reviewer".to_string(),
                status: "approved".to_string(),
                extra: yaml_serde::Mapping::new(),
            }],
            binding_declarations: vec![],
            evidence: vec![],
            risks: vec![],
            bound_revision: None,
            subject: None,
            extra: yaml_serde::Mapping::new(),
        };
        assert!(matches!(
            validate(&record),
            Err(StorageError::InvalidAuthority(authority)) if authority == "reviewer"
        ));
        record.approvals[0].authority = "contributor".to_string();
        assert!(validate(&record).is_ok());
    }

    /// PID + nanos + contador atómico: bajo carga extrema (muchos tests en
    /// paralelo), la resolución real del reloj puede no ser tan fina como
    /// promete `as_nanos()` — dos hilos pueden generar el mismo timestamp y
    /// colisionar en el mismo directorio temporal (confirmado empíricamente:
    /// esto causaba fallos intermitentes de `git init` en tests de otro
    /// módulo bajo stress). El contador lo hace imposible dentro del mismo
    /// proceso.
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

    /// Verdadero si todo par clave/valor de `original` está presente y es
    /// equivalente en `rewritten` (recursivamente). `rewritten` puede tener
    /// claves adicionales — eso es agregar información (defaults
    /// explícitos), no perderla. Secuencias sí comparan orden y longitud
    /// exacta (listas como `binding_declarations` deben preservar orden).
    fn is_semantic_subset(original: &yaml_serde::Value, rewritten: &yaml_serde::Value) -> bool {
        use yaml_serde::Value;
        match (original, rewritten) {
            (Value::Mapping(orig_map), Value::Mapping(new_map)) => orig_map.iter().all(|(k, v)| {
                new_map
                    .get(k)
                    .is_some_and(|new_v| is_semantic_subset(v, new_v))
            }),
            (Value::Sequence(orig_seq), Value::Sequence(new_seq)) => {
                orig_seq.len() == new_seq.len()
                    && orig_seq
                        .iter()
                        .zip(new_seq.iter())
                        .all(|(a, b)| is_semantic_subset(a, b))
            }
            (a, b) => a == b,
        }
    }

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

    /// F1 — la garantía central de este módulo: el Record canónico real
    /// (~24 campos de nivel superior, varios anidados no modelados) debe
    /// sobrevivir leer → escribir → releer sin perder ningún dato, aunque
    /// el struct Rust solo tipe un subconjunto. La comparación es de
    /// **subconjunto**, no de igualdad estricta: la reescritura puede
    /// *agregar* defaults explícitos que el original dejaba implícitos
    /// (`epistemic_status: stated`, `bound_revision: null` cuando el campo
    /// no existía) — eso no es pérdida de datos, es lo opuesto. Lo que nunca
    /// debe pasar es que una clave/valor del original desaparezca o cambie.
    #[test]
    fn real_record_roundtrip_loses_no_data() {
        let original_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".rationale/records/constraint.no-provider-internal-access.yaml");
        let original_content = std::fs::read_to_string(&original_path).unwrap();

        let record = read_record(&original_path).unwrap();

        let dir = std::env::temp_dir().join(format!(
            "rationale-storage-roundtrip-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let written_path = dir.join("roundtrip.yaml");

        write_record(&written_path, &record).unwrap();
        let rewritten_content = std::fs::read_to_string(&written_path).unwrap();

        let original_value: yaml_serde::Value = yaml_serde::from_str(&original_content).unwrap();
        let rewritten_value: yaml_serde::Value = yaml_serde::from_str(&rewritten_content).unwrap();
        assert!(
            is_semantic_subset(&original_value, &rewritten_value),
            "cada clave/valor del Record original debe sobrevivir en la reescritura — \
             original: {original_value:?}\nreescrito: {rewritten_value:?}"
        );

        // También debe seguir siendo un Record válido y releíble.
        let reread = read_record(&written_path).unwrap();
        assert_eq!(reread.id, record.id);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// F1 — escritura atómica: nunca queda un archivo `.tmp-*` huérfano
    /// tras una escritura exitosa, y sobrescribir un Record existente lo
    /// reemplaza por completo (nunca fusiona contenido viejo y nuevo).
    #[test]
    fn write_record_leaves_no_tmp_file_and_fully_replaces_existing() {
        let dir = std::env::temp_dir().join(format!(
            "rationale-storage-atomic-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("atomic.yaml");

        let mut extra = yaml_serde::Mapping::new();
        extra.insert(
            yaml_serde::Value::String("title".to_string()),
            yaml_serde::Value::String("first version".to_string()),
        );
        let first = Record {
            id: "constraint.atomic-test".to_string(),
            kind: "constraint".to_string(),
            severity: "critical".to_string(),
            statement: "first statement".to_string(),
            rationale: None,
            epistemic_status: EpistemicStatus::Stated,
            approvals: vec![],
            binding_declarations: vec![],
            evidence: vec![],
            risks: vec![],
            bound_revision: None,
            subject: None,
            extra,
        };
        write_record(&path, &first).unwrap();

        let second = Record {
            statement: "second statement, completely different".to_string(),
            extra: yaml_serde::Mapping::new(),
            ..read_record(&path).unwrap()
        };
        write_record(&path, &second).unwrap();

        let reread = read_record(&path).unwrap();
        assert_eq!(reread.statement, "second statement, completely different");
        assert!(
            !reread
                .extra
                .contains_key(yaml_serde::Value::String("title".to_string())),
            "la segunda escritura debe reemplazar por completo, no fusionar"
        );

        let leftover_tmp_files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(
            leftover_tmp_files.is_empty(),
            "no debe quedar ningún archivo temporal tras una escritura exitosa: {leftover_tmp_files:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// F1 / ADR-0008 — evidencia de concurrencia intra-proceso: N hilos
    /// escribiendo el MISMO Record al mismo tiempo nunca deben producir un
    /// archivo corrupto o a medio escribir. El resultado final debe ser
    /// exactamente uno de los N candidatos válidos — "last write wins" sin
    /// fusión ni truncamiento, incluso bajo contención real.
    #[test]
    fn concurrent_writes_to_same_record_never_corrupt_the_file() {
        let dir = std::env::temp_dir().join(format!(
            "rationale-storage-concurrent-write-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = std::sync::Arc::new(dir.join("concurrent.yaml"));

        let candidate_statements: Vec<String> = (0..8)
            .map(|i| format!("statement from writer {i}"))
            .collect();

        let handles: Vec<_> = candidate_statements
            .iter()
            .cloned()
            .map(|statement| {
                let path = std::sync::Arc::clone(&path);
                std::thread::spawn(move || {
                    let record = Record {
                        id: "constraint.concurrent-test".to_string(),
                        kind: "constraint".to_string(),
                        severity: "critical".to_string(),
                        statement,
                        rationale: None,
                        epistemic_status: EpistemicStatus::Stated,
                        approvals: vec![],
                        binding_declarations: vec![],
                        evidence: vec![],
                        risks: vec![],
                        bound_revision: None,
                        subject: None,
                        extra: yaml_serde::Mapping::new(),
                    };
                    write_record(&path, &record)
                })
            })
            .collect();

        for handle in handles {
            handle
                .join()
                .expect("el hilo escritor no debe panicar")
                .expect("cada escritura individual debe completarse sin error");
        }

        // El archivo final debe ser un Record válido, completo, y su
        // statement debe ser EXACTAMENTE uno de los candidatos — nunca una
        // mezcla de dos escrituras ni un archivo truncado/corrupto.
        let final_record = read_record(&path)
            .expect("el archivo final debe seguir siendo un Record válido, no corrupto");
        assert!(
            candidate_statements.contains(&final_record.statement),
            "el resultado final debe ser exactamente uno de los candidatos, no una fusión: {}",
            final_record.statement
        );

        let leftover_tmp_files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(
            leftover_tmp_files.is_empty(),
            "ninguna escritura concurrente debe dejar temporales huérfanos: {leftover_tmp_files:?}"
        );

        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    /// F1 — nunca se persiste un Record inválido, ni siquiera parcialmente.
    #[test]
    fn write_record_rejects_invalid_record_before_touching_disk() {
        let dir = std::env::temp_dir().join(format!(
            "rationale-storage-invalid-write-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("invalid.yaml");

        let invalid = Record {
            id: "constraint.invalid".to_string(),
            kind: "constraint".to_string(),
            severity: "critical".to_string(),
            statement: String::new(),
            rationale: None,
            epistemic_status: EpistemicStatus::Stated,
            approvals: vec![],
            binding_declarations: vec![],
            evidence: vec![],
            risks: vec![],
            bound_revision: None,
            subject: None,
            extra: yaml_serde::Mapping::new(),
        };

        let result = write_record(&path, &invalid);
        assert!(matches!(
            result,
            Err(StorageError::MissingRequiredField("statement"))
        ));
        assert!(
            !path.exists(),
            "no debe crearse ningún archivo si el Record es inválido"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
