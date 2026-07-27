//! binding_match — el único comparador entre un target resuelto y los
//! bindings que un Record declara. `prepare` y `explain` deben usar
//! exactamente esta función: antes de este módulo cada uno implementaba su
//! propia comparación — una con un fallback arbitrario (`records.first()`
//! cuando nada matcheaba), la otra sin ninguno — y podían dar respuestas
//! distintas para el MISMO target. Confirmado en un dogfood real: `prepare`
//! devolvía un assessment seguro sobre un Record no relacionado mientras
//! `explain_target` devolvía vacío para la misma consulta.
//!
//! `structural_id` no tiene una gramática única en el canon real: conviven
//! `"function:typescript:auth.resolveEntityRole"` (kind:lang:qualified.name,
//! ver `.rationale/schemas/binding.schema.json`) y
//! `"rust:src/providers/codebase_memory.rs::CodebaseMemoryClient"`
//! (lang:path::symbol — el binding real de este mismo proyecto, en
//! `.rationale/records/constraint.no-provider-internal-access.yaml`). Por
//! eso este matcher NUNCA intenta extraer un path de `structural_id` — solo
//! compara el símbolo contra el final de la cadena, respetando un límite de
//! token (nunca `Role` matchea `resolveEntityRole`). Cuando el binding trae
//! `path_hint` además de `structural_id`, ese path SÍ debe coincidir con el
//! target — es la única forma honesta de acotar el símbolo a un archivo sin
//! inventar una gramática que el canon no garantiza.

use crate::project::Target;
use crate::storage::{has_approved_authority, BindingDeclaration, Record};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct TargetKey {
    pub rel_path: Option<String>,
    pub symbol: Option<String>,
}

/// Orden = precedencia: el último variant es el más específico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchKind {
    FileExact,
    FileContainsSymbol,
    Structural,
}

impl MatchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MatchKind::FileExact => "file-exact",
            MatchKind::FileContainsSymbol => "file-contains-symbol",
            MatchKind::Structural => "structural",
        }
    }
}

pub struct GovernanceMatch<'a> {
    pub record: &'a Record,
    #[allow(dead_code)] // usado por assessment::linkage (1.4) para diagnosticar resolución
    pub binding: &'a BindingDeclaration,
    pub kind: MatchKind,
}

/// Repo-relativo, separadores `/`, sin `./` inicial. `path_hint` nunca se
/// exige que exista en disco para poder compararlo — un binding hacia un
/// archivo borrado o renombrado debe poder seguir matcheando por texto; que
/// ya no exista es un problema de `linkage` (Fase 1.4), no de matching.
fn clean_rel(raw: &str) -> String {
    let normalized = raw.replace('\\', "/");
    normalized
        .strip_prefix("./")
        .unwrap_or(&normalized)
        .to_string()
}

/// El target de `project::resolve_target` ya viene canonicalizado y
/// absoluto; se re-canonicaliza aquí igual (idempotente para ese caso) para
/// que esta misma función sirva también con un path crudo devuelto por el
/// proveedor estructural (`finalize`, Fase 1.3) — si esa canonicalización
/// falla (el archivo ya no existe), se usa el path tal cual en vez de
/// fallar por completo, para no perder un `path_hint` legítimo.
pub fn target_rel_path(repo_root: &Path, target: &Target) -> Option<String> {
    let root = repo_root.canonicalize().ok()?;
    let candidate = target
        .path
        .canonicalize()
        .unwrap_or_else(|_| target.path.clone());
    let rel = candidate.strip_prefix(&root).ok()?;
    Some(clean_rel(&rel.to_string_lossy()))
}

pub fn target_key(repo_root: &Path, target: &Target) -> TargetKey {
    TargetKey {
        rel_path: target_rel_path(repo_root, target),
        symbol: target.symbol.clone(),
    }
}

/// Límite de token: el carácter justo antes del sufijo no puede ser
/// alfanumérico ni `_` — evita que `Role` matchee `resolveEntityRole`, un
/// falso positivo real del `ends_with` crudo que este módulo reemplaza.
fn ends_with_symbol_at_boundary(structural_id: &str, symbol: &str) -> bool {
    if symbol.is_empty() {
        return false;
    }
    if structural_id == symbol {
        return true;
    }
    match structural_id.strip_suffix(symbol) {
        Some(rest) => !matches!(rest.chars().last(), Some(c) if c.is_alphanumeric() || c == '_'),
        None => false,
    }
}

/// Compara un solo binding contra el target. Pública también para
/// `rationale doctor` (Fase 1.5), que la usa para diagnosticar por qué un
/// binding concreto no resuelve.
pub fn match_one(key: &TargetKey, binding: &BindingDeclaration) -> Option<MatchKind> {
    let hint = binding.path_hint.as_deref().map(clean_rel);

    if let (Some(symbol), Some(structural_id)) = (&key.symbol, &binding.structural_id) {
        if ends_with_symbol_at_boundary(structural_id, symbol) {
            // Si el binding también declara path_hint, debe coincidir con
            // el target — nunca se acepta un símbolo de un archivo distinto
            // solo porque el nombre coincide.
            let path_ok = match (&hint, &key.rel_path) {
                (Some(h), Some(t)) => h == t,
                (None, _) => true,
                (Some(_), None) => false,
            };
            if path_ok {
                return Some(MatchKind::Structural);
            }
        }
    }

    if let (Some(hint), Some(rel_path)) = (&hint, &key.rel_path) {
        if hint == rel_path {
            return Some(if key.symbol.is_some() {
                MatchKind::FileContainsSymbol
            } else {
                MatchKind::FileExact
            });
        }
    }

    None
}

/// Todos los Records que gobiernan el target — uno por Record (el binding
/// de mejor `MatchKind` cuando varios matchean), ordenados por
/// especificidad, luego autoridad aprobada primero, luego `id` ascendente
/// para determinismo. Nunca trunca ni elige uno "al azar": vacío es una
/// respuesta honesta cuando nada matchea. Es exactamente el punto donde
/// `prepare` y `explain` discrepaban antes de este módulo.
pub fn governing<'a>(key: &TargetKey, records: &'a [Record]) -> Vec<GovernanceMatch<'a>> {
    if key.rel_path.is_none() && key.symbol.is_none() {
        return Vec::new();
    }

    let mut best_per_record: std::collections::HashMap<&str, GovernanceMatch<'a>> =
        std::collections::HashMap::new();
    for record in records {
        for binding in &record.binding_declarations {
            if let Some(kind) = match_one(key, binding) {
                match best_per_record.entry(record.id.as_str()) {
                    std::collections::hash_map::Entry::Occupied(mut existing) => {
                        if kind > existing.get().kind {
                            existing.insert(GovernanceMatch {
                                record,
                                binding,
                                kind,
                            });
                        }
                    }
                    std::collections::hash_map::Entry::Vacant(slot) => {
                        slot.insert(GovernanceMatch {
                            record,
                            binding,
                            kind,
                        });
                    }
                }
            }
        }
    }

    let mut matches: Vec<GovernanceMatch<'a>> = best_per_record.into_values().collect();
    matches.sort_by(|a, b| {
        b.kind
            .cmp(&a.kind)
            .then_with(|| has_approved_authority(b.record).cmp(&has_approved_authority(a.record)))
            .then_with(|| a.record.id.cmp(&b.record.id))
    });
    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::EpistemicStatus;

    fn record_with_binding(id: &str, binding: BindingDeclaration) -> Record {
        Record {
            id: id.to_string(),
            kind: "constraint".to_string(),
            severity: "high".to_string(),
            statement: format!("statement for {id}"),
            rationale: None,
            epistemic_status: EpistemicStatus::Stated,
            approvals: vec![],
            binding_declarations: vec![binding],
            evidence: vec![],
            risks: vec![],
            bound_revision: None,
            subject: None,
            extra: yaml_serde::Mapping::new(),
        }
    }

    fn file_binding(path_hint: &str) -> BindingDeclaration {
        BindingDeclaration {
            id: "binding.file".to_string(),
            kind: "file".to_string(),
            provider: None,
            structural_id: None,
            path_hint: Some(path_hint.to_string()),
            provisional: false,
            extra: yaml_serde::Mapping::new(),
        }
    }

    fn structural_binding(structural_id: &str, path_hint: Option<&str>) -> BindingDeclaration {
        BindingDeclaration {
            id: "binding.symbol".to_string(),
            kind: "symbol".to_string(),
            provider: Some("codebase-memory".to_string()),
            structural_id: Some(structural_id.to_string()),
            path_hint: path_hint.map(|p| p.to_string()),
            provisional: false,
            extra: yaml_serde::Mapping::new(),
        }
    }

    #[test]
    fn structural_beats_file_containment() {
        let records = vec![
            record_with_binding("constraint.file", file_binding("src/upload.rs")),
            record_with_binding(
                "constraint.symbol",
                structural_binding("rust:src/upload.rs::submit_file", Some("src/upload.rs")),
            ),
        ];
        let key = TargetKey {
            rel_path: Some("src/upload.rs".to_string()),
            symbol: Some("submit_file".to_string()),
        };
        let matches = governing(&key, &records);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].record.id, "constraint.symbol");
        assert_eq!(matches[0].kind, MatchKind::Structural);
        assert_eq!(matches[1].record.id, "constraint.file");
        assert_eq!(matches[1].kind, MatchKind::FileContainsSymbol);
    }

    /// La propagación archivo→símbolo (defecto 7 del dogfood): un binding
    /// de archivo debe gobernar cualquier símbolo consultado dentro de ese
    /// archivo, sin necesitar un binding específico por función.
    #[test]
    fn file_binding_governs_every_symbol_in_that_file() {
        let records = vec![record_with_binding(
            "constraint.upload-rules",
            file_binding("app/upload.tsx"),
        )];
        for symbol in ["submitFile", "cancelUpload", "retryUpload"] {
            let key = TargetKey {
                rel_path: Some("app/upload.tsx".to_string()),
                symbol: Some(symbol.to_string()),
            };
            let matches = governing(&key, &records);
            assert_eq!(
                matches.len(),
                1,
                "el binding de archivo debe gobernar el símbolo '{symbol}'"
            );
            assert_eq!(matches[0].kind, MatchKind::FileContainsSymbol);
        }
    }

    #[test]
    fn bare_file_query_without_symbol_matches_file_exact() {
        let records = vec![record_with_binding(
            "constraint.upload-rules",
            file_binding("app/upload.tsx"),
        )];
        let key = TargetKey {
            rel_path: Some("app/upload.tsx".to_string()),
            symbol: None,
        };
        let matches = governing(&key, &records);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, MatchKind::FileExact);
    }

    /// El falso positivo real que el `ends_with` crudo producía: un símbolo
    /// corto que es sufijo literal de otro nombre no debe matchear.
    #[test]
    fn suffix_match_respects_token_boundary() {
        let records = vec![record_with_binding(
            "constraint.roles",
            structural_binding("function:typescript:auth.resolveEntityRole", None),
        )];
        let key = TargetKey {
            rel_path: None,
            symbol: Some("Role".to_string()),
        };
        assert!(
            governing(&key, &records).is_empty(),
            "'Role' no debe matchear 'resolveEntityRole' — el límite de token debe rechazarlo"
        );

        let real_key = TargetKey {
            rel_path: None,
            symbol: Some("resolveEntityRole".to_string()),
        };
        assert_eq!(governing(&real_key, &records).len(), 1);
    }

    /// Un símbolo con el mismo nombre en un archivo distinto no debe
    /// matchear cuando el binding declara path_hint — evita que dos
    /// funciones homónimas en archivos distintos se confundan.
    #[test]
    fn symbol_suffix_does_not_match_a_different_file() {
        let records = vec![record_with_binding(
            "constraint.other-file",
            structural_binding("rust:src/other.rs::submit_file", Some("src/other.rs")),
        )];
        let key = TargetKey {
            rel_path: Some("src/upload.rs".to_string()),
            symbol: Some("submit_file".to_string()),
        };
        assert!(
            governing(&key, &records).is_empty(),
            "el path_hint del binding difiere del target — no debe matchear"
        );
    }

    /// Sin `structural_id` ni `path_hint` coincidente, no hay match — nunca
    /// un guess parcial ni un fallback a "el primero de la lista".
    #[test]
    fn unresolvable_binding_yields_no_match_never_a_guess() {
        let records = vec![
            record_with_binding("constraint.a", file_binding("src/unrelated-a.rs")),
            record_with_binding("constraint.b", file_binding("src/unrelated-b.rs")),
        ];
        let key = TargetKey {
            rel_path: Some("src/upload.rs".to_string()),
            symbol: Some("submit_file".to_string()),
        };
        assert!(governing(&key, &records).is_empty());
    }

    #[test]
    fn empty_target_key_matches_nothing() {
        let records = vec![record_with_binding(
            "constraint.a",
            file_binding("src/a.rs"),
        )];
        let key = TargetKey::default();
        assert!(governing(&key, &records).is_empty());
    }

    #[test]
    fn clean_rel_normalizes_separators_and_leading_dot_slash() {
        assert_eq!(clean_rel("./src/foo.rs"), "src/foo.rs");
        assert_eq!(clean_rel("src\\foo.rs"), "src/foo.rs");
        assert_eq!(clean_rel("src/foo.rs"), "src/foo.rs");
    }

    #[test]
    fn governing_deduplicates_multiple_bindings_on_the_same_record() {
        let mut record = record_with_binding("constraint.dual", file_binding("src/upload.rs"));
        record.binding_declarations.push(structural_binding(
            "rust:src/upload.rs::submit_file",
            Some("src/upload.rs"),
        ));
        let key = TargetKey {
            rel_path: Some("src/upload.rs".to_string()),
            symbol: Some("submit_file".to_string()),
        };
        let matches = governing(&key, std::slice::from_ref(&record));
        assert_eq!(
            matches.len(),
            1,
            "un solo Record con dos bindings que matchean debe aparecer una sola vez"
        );
        assert_eq!(matches[0].kind, MatchKind::Structural);
    }
}
