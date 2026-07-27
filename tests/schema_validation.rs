//! E6 — schema validation: los 7 schemas JSON de `.rationale/schemas/` deben
//! ser JSON válido, y sus campos `required` deben coincidir con los campos
//! no-`Option` de los structs Rust correspondientes.
//!
//! Rationale no valida JSON Schema en runtime con un crate externo
//! (`jsonschema` arrastra HTTP/TLS por resolución remota de `$ref`, contra
//! el principio local-first de `Arquitectura §4.1` — ver
//! `.rationale/schemas/README.md`). Este test es la verificación
//! determinista que sí corre: que el schema declarado como especificación
//! formal no haya divergido silenciosamente del struct real que dice
//! describir.

use serde_json::Value;
use std::path::Path;

fn read_schema(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".rationale/schemas")
        .join(name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("{} no es JSON válido: {e}", name))
}

fn required_fields(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Los 7 schemas deben existir y ser JSON sintácticamente válido — el
/// mínimo indispensable antes de comparar campos.
#[test]
fn all_seven_schemas_are_valid_json() {
    for name in [
        "record.schema.json",
        "subject.schema.json",
        "binding.schema.json",
        "evidence.schema.json",
        "approval.schema.json",
        "assessment.schema.json",
        "context-packet.schema.json",
    ] {
        let _ = read_schema(name);
    }
}

/// `record.schema.json` — Record (`src/storage.rs`) solo exige
/// id/kind/severity/statement; el resto (rationale, epistemic_status,
/// approvals, binding_declarations, evidence, risks, bound_revision,
/// subject) son `Option`/`#[serde(default)]`.
#[test]
fn record_schema_required_matches_struct() {
    let schema = read_schema("record.schema.json");
    let mut required = required_fields(&schema);
    required.sort();
    assert_eq!(required, vec!["id", "kind", "severity", "statement"]);
}

/// `subject.schema.json` — Subject (`src/subjects.rs`): id/type/title son
/// obligatorios; scope/aliases/applies_to tienen `#[serde(default)]`.
#[test]
fn subject_schema_required_matches_struct() {
    let schema = read_schema("subject.schema.json");
    let mut required = required_fields(&schema);
    required.sort();
    assert_eq!(required, vec!["id", "title", "type"]);
}

/// `binding.schema.json` — BindingDeclaration (`src/storage.rs`): solo
/// id/type son `String`; provider/structural_id/path_hint son `Option`.
#[test]
fn binding_schema_required_matches_struct() {
    let schema = read_schema("binding.schema.json");
    let mut required = required_fields(&schema);
    required.sort();
    assert_eq!(required, vec!["id", "type"]);
}

/// Defecto real: `pipeline::finalize` escribe `kind: "file"` desde su
/// primera versión, pero el schema nunca declaró `"file"` en el enum de
/// `type` — el productor emitía un valor fuera de su propio contrato sin
/// que ningún test lo detectara.
#[test]
fn binding_schema_type_enum_includes_file() {
    let schema = read_schema("binding.schema.json");
    let kinds: Vec<&str> = schema["properties"]["type"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"file"),
        "el productor real escribe bindings kind: \"file\" — el schema debe declararlo"
    );
}

#[test]
fn binding_schema_declares_provisional() {
    let schema = read_schema("binding.schema.json");
    assert_eq!(schema["properties"]["provisional"]["type"], "boolean");
}

/// La causa raíz del defecto de severidad: el schema y el retrieval deben
/// coincidir siempre en qué valores son válidos. Este test no puede
/// importar `storage::Severity` (el crate solo tiene binario, sin `[lib]`
/// — mismo motivo documentado en `tests/mcp_server.rs`), así que fija el
/// enum literal; si alguno de los dos lados cambia sin el otro, este test
/// lo detecta.
#[test]
fn severity_enum_matches_the_four_values_rationale_actually_uses() {
    let schema = read_schema("record.schema.json");
    let severities: Vec<&str> = schema["properties"]["severity"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(severities, vec!["critical", "high", "medium", "low"]);
}

/// `evidence.schema.json` — Evidence (`src/storage.rs`): solo `type` es
/// obligatorio; path/revision/content_hash/visibility son `Option` y
/// `verified` tiene `#[serde(default)]`.
#[test]
fn evidence_schema_required_matches_struct() {
    let schema = read_schema("evidence.schema.json");
    let required = required_fields(&schema);
    assert_eq!(required, vec!["type"]);
}

/// `approval.schema.json` — Approval (`src/storage.rs`): las tres son
/// `String` obligatorias.
#[test]
fn approval_schema_required_matches_struct() {
    let schema = read_schema("approval.schema.json");
    let mut required = required_fields(&schema);
    required.sort();
    assert_eq!(required, vec!["actor", "authority", "status"]);
}

#[test]
fn approval_schema_authority_enum_excludes_undeclared_reviewer_role() {
    let schema = read_schema("approval.schema.json");
    let roles: Vec<&str> = schema["properties"]["authority"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec![
            "contributor",
            "domain-maintainer",
            "domain-owner",
            "security-owner",
            "product-owner",
            "architecture-owner",
            "repository-policy"
        ]
    );
    assert!(!roles.contains(&"reviewer"));
}

#[test]
fn record_schema_declares_structured_novelty_reason() {
    let schema = read_schema("record.schema.json");
    let novelty = &schema["properties"]["novelty_reason"];
    let mut required: Vec<&str> = novelty["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    required.sort_unstable();
    assert_eq!(
        required,
        vec![
            "contrasted_subject",
            "difference",
            "difference_kind",
            "evidence"
        ]
    );
    assert_eq!(
        novelty["properties"]["difference_kind"]["enum"],
        serde_json::json!(["behavior", "scope", "lifecycle", "authority", "invariant"])
    );
}

/// `assessment.schema.json` — Assessment (`src/assessment.rs`):
/// `assessed_revision` es el único campo `Option`, correctamente excluido.
#[test]
fn assessment_schema_required_matches_struct() {
    let schema = read_schema("assessment.schema.json");
    let mut required = required_fields(&schema);
    required.sort();
    assert_eq!(
        required,
        vec![
            "assessment_reason",
            "record_id",
            "revision_consistency",
            "state"
        ]
    );
}

/// `context-packet.schema.json` — ContextPacket (`src/retrieval.rs`):
/// `snapshot` y `critical_constraints` son los campos que el schema declara
/// semánticamente indispensables (Nivel 0-1, nunca se recortan por
/// presupuesto — v0.5 §30.1.7). El resto del packet siempre está presente
/// en la serialización real (Vec/usize no son opcionales a nivel Rust),
/// pero el schema los trata como "siempre presentes, no necesariamente
/// significativos" — ej. `known_risks: []` es válido y no invalida el
/// packet. Este test fija esa decisión, no la reinterpreta.
#[test]
fn context_packet_schema_required_matches_struct() {
    let schema = read_schema("context-packet.schema.json");
    let mut required = required_fields(&schema);
    required.sort();
    assert_eq!(required, vec!["critical_constraints", "snapshot"]);
}

/// Fase 1.2 — `intent_conflicts` pasó de `Vec<String>` a objetos tipados
/// (`detection`/`polarity`/`shared_terms`...). El schema debe reflejar el
/// contrato real, no el de antes.
#[test]
fn context_packet_intent_conflicts_items_are_objects() {
    let schema = read_schema("context-packet.schema.json");
    let items = &schema["properties"]["intent_conflicts"]["items"];
    assert_eq!(items["type"], "object");
    let mut required: Vec<&str> = items["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    required.sort_unstable();
    assert_eq!(
        required,
        vec![
            "detection",
            "governs_target",
            "polarity",
            "record_id",
            "statement"
        ]
    );
    assert_eq!(
        items["properties"]["detection"]["enum"],
        serde_json::json!(["governs-target", "lexical-overlap"])
    );
}

#[test]
fn context_packet_critical_constraints_expose_governance_fields() {
    let schema = read_schema("context-packet.schema.json");
    let items = &schema["properties"]["critical_constraints"]["items"];
    let required: Vec<&str> = items["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(required.contains(&"governs_target"));
    assert!(required.contains(&"severity"));
}
