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
use crate::storage::{has_human_endorsement, BindingDeclaration, Record, RelationshipBinding};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct TargetKey {
    pub rel_path: Option<String>,
    pub symbol: Option<String>,
    /// El target es una carpeta: la consulta abarca lo que hay debajo.
    pub is_dir: bool,
}

/// Orden = precedencia: el último variant es el más específico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchKind {
    /// Consulta de carpeta: el Record está atado a código dentro de ella.
    DirectoryMember,
    /// El Record declara un alcance de carpeta que contiene el target.
    DirectoryScope,
    FileExact,
    /// Un Record de archivo (sin símbolo propio en ese archivo) gobierna
    /// todos sus símbolos.
    FileContainsSymbol,
    /// El target es un extremo de una relación que el Record explica
    /// (`relationship_bindings`, vNext): cambiar ese nodo puede romper el
    /// porqué de la relación.
    RelationshipEndpoint,
    /// El símbolo coincide con un binding que el proveedor todavía no
    /// confirmó (índice atrasado): verificado solo léxicamente.
    UnconfirmedSymbol,
    Structural,
}

impl MatchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MatchKind::DirectoryMember => "directory-member",
            MatchKind::DirectoryScope => "directory-scope",
            MatchKind::FileExact => "file-exact",
            MatchKind::FileContainsSymbol => "file-contains-symbol",
            MatchKind::RelationshipEndpoint => "relationship-endpoint",
            MatchKind::UnconfirmedSymbol => "unconfirmed-symbol",
            MatchKind::Structural => "structural",
        }
    }

    /// Por qué el Record entra en la respuesta, en una frase para el agente.
    pub fn reason(self) -> &'static str {
        match self {
            MatchKind::DirectoryMember => "bound to code inside the queried folder",
            MatchKind::DirectoryScope => "its folder scope contains the target",
            MatchKind::FileExact => "bound to this file",
            MatchKind::FileContainsSymbol => "file-level rule: applies to every symbol in the file",
            MatchKind::RelationshipEndpoint => {
                "the target is an endpoint of a relationship it explains"
            }
            MatchKind::UnconfirmedSymbol => {
                "bound to this symbol (lexically verified; the index does not know it yet)"
            }
            MatchKind::Structural => "bound to this symbol",
        }
    }
}

pub struct GovernanceMatch<'a> {
    pub record: &'a Record,
    pub kind: MatchKind,
}

/// Repo-relativo, separadores `/`, sin `./` inicial ni `/` final. `path_hint`
/// nunca se exige que exista en disco para poder compararlo — un binding
/// hacia un archivo borrado o renombrado debe poder seguir matcheando por
/// texto; que ya no exista es un problema de `linkage`, no de matching.
fn clean_rel(raw: &str) -> String {
    let normalized = raw.replace('\\', "/");
    let trimmed = normalized.strip_prefix("./").unwrap_or(&normalized);
    let trimmed = trimmed.trim_end_matches('/');
    if trimmed == "." {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// `path` está dentro de `dir` (la raíz `""` contiene todo).
pub fn is_under(path: &str, dir: &str) -> bool {
    dir.is_empty()
        || path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/'))
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
        is_dir: target.path.is_dir(),
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

/// Un binding de símbolo: confirmado por el proveedor (`structural_id`) o
/// verificado solo léxicamente (`symbol`, índice atrasado).
fn is_symbol_binding(binding: &BindingDeclaration) -> bool {
    binding.structural_id.is_some() || binding.symbol.is_some()
}

/// Un binding de archivo declarado explícitamente como alcance de archivo
/// completo (`scope: file`), aunque el Record también nombre un símbolo de
/// ese archivo.
fn is_explicit_file_scope(binding: &BindingDeclaration) -> bool {
    binding
        .extra
        .get(yaml_serde::Value::String("scope".to_string()))
        .and_then(|value| value.as_str())
        == Some("file")
}

fn has_symbol_binding_in(record: &Record, path: &str) -> bool {
    record.binding_declarations.iter().any(|binding| {
        is_symbol_binding(binding)
            && binding.path_hint.as_deref().map(clean_rel).as_deref() == Some(path)
    })
}

/// `symbol_scoped_file`: el Record tiene bindings de símbolo en el archivo de
/// este binding. Hasta v1.1 cada binding `archivo::símbolo` escribía además
/// un binding de archivo «acompañante»; tratarlo como alcance de archivo hacía
/// que un Record de `cancelShipment` gobernara `createShipment` y todo lo
/// demás del archivo.
fn match_binding(
    key: &TargetKey,
    binding: &BindingDeclaration,
    symbol_scoped_file: bool,
) -> Option<MatchKind> {
    let hint = binding.path_hint.as_deref().map(clean_rel);
    let target = key.rel_path.as_deref();

    if binding.kind == "directory" {
        let (dir, target) = (hint?, target?);
        if is_under(target, &dir) || target == dir {
            return Some(MatchKind::DirectoryScope);
        }
        return (key.is_dir && is_under(&dir, target)).then_some(MatchKind::DirectoryMember);
    }

    if key.is_dir {
        let (hint, target) = (hint?, target?);
        return is_under(&hint, target).then_some(MatchKind::DirectoryMember);
    }

    if is_symbol_binding(binding) {
        let path_ok = match (&hint, target) {
            (Some(h), Some(t)) => h == t,
            (None, _) => true,
            (Some(_), None) => false,
        };
        return match &key.symbol {
            Some(symbol) => {
                if let Some(structural_id) = &binding.structural_id {
                    (path_ok && ends_with_symbol_at_boundary(structural_id, symbol))
                        .then_some(MatchKind::Structural)
                } else {
                    let lexical = binding.symbol.as_deref().unwrap_or_default();
                    (path_ok && ends_with_symbol_at_boundary(lexical, short_symbol(symbol)))
                        .then_some(MatchKind::UnconfirmedSymbol)
                }
            }
            // Consulta de archivo: todo Record atado a un símbolo del archivo
            // pertenece al archivo.
            None => (hint.is_some() && hint.as_deref() == target).then_some(MatchKind::FileExact),
        };
    }

    let (hint, target) = (hint?, target?);
    if hint != target {
        return None;
    }
    match &key.symbol {
        None => Some(MatchKind::FileExact),
        Some(_) if symbol_scoped_file && !is_explicit_file_scope(binding) => None,
        Some(_) => Some(MatchKind::FileContainsSymbol),
    }
}

/// Último segmento de un símbolo calificado (`Type::method` → `method`).
fn short_symbol(symbol: &str) -> &str {
    symbol
        .rsplit([':', '.'])
        .find(|s| !s.is_empty())
        .unwrap_or(symbol)
}

/// Un extremo de relación gobierna el target cuando coincide el archivo y,
/// si la consulta trae símbolo, el nombre calificado termina en ese símbolo
/// en un límite de token (`Tipo::metodo` se compara como `Tipo.metodo`). En
/// una consulta de carpeta, cuando un extremo está dentro de ella.
pub fn match_relationship(key: &TargetKey, binding: &RelationshipBinding) -> Option<MatchKind> {
    let rel_path = key.rel_path.as_deref()?;
    if key.is_dir {
        return [&binding.source, &binding.target]
            .into_iter()
            .any(|endpoint| is_under(&clean_rel(&endpoint.file_path), rel_path))
            .then_some(MatchKind::DirectoryMember);
    }
    [&binding.source, &binding.target]
        .into_iter()
        .any(|endpoint| {
            clean_rel(&endpoint.file_path) == rel_path
                && match &key.symbol {
                    None => true,
                    Some(symbol) => ends_with_symbol_at_boundary(
                        &endpoint.qualified_name,
                        &symbol.replace("::", "."),
                    ),
                }
        })
        .then_some(MatchKind::RelationshipEndpoint)
}

/// Cómo gobierna un Record el target, o `None`.
pub fn match_record(key: &TargetKey, record: &Record) -> Option<MatchKind> {
    record
        .binding_declarations
        .iter()
        .filter_map(|binding| {
            let symbol_scoped = binding
                .path_hint
                .as_deref()
                .map(clean_rel)
                .is_some_and(|path| has_symbol_binding_in(record, &path));
            match_binding(key, binding, symbol_scoped)
        })
        .chain(
            record
                .relationship_bindings
                .iter()
                .filter_map(|binding| match_relationship(key, binding)),
        )
        .max()
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
    let mut matches: Vec<GovernanceMatch<'a>> = records
        .iter()
        .filter_map(|record| match_record(key, record).map(|kind| GovernanceMatch { record, kind }))
        .collect();
    matches.sort_by(|a, b| {
        b.kind
            .cmp(&a.kind)
            .then_with(|| has_human_endorsement(b.record).cmp(&has_human_endorsement(a.record)))
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
            authority: None,
            provenance: None,
            supersedes: vec![],
            approvals: vec![],
            binding_declarations: vec![binding],
            relationship_bindings: vec![],
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
            symbol: None,
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
            symbol: None,
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
            ..Default::default()
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
                ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };
        assert!(
            governing(&key, &records).is_empty(),
            "'Role' no debe matchear 'resolveEntityRole' — el límite de token debe rechazarlo"
        );

        let real_key = TargetKey {
            rel_path: None,
            symbol: Some("resolveEntityRole".to_string()),
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
    fn relationship_endpoints_govern_their_symbols_only() {
        let mut record =
            record_with_binding("decision.payment-expiration", file_binding("docs/x.md"));
        record.binding_declarations.clear();
        record.relationship_bindings.push(RelationshipBinding {
            id: "rel.0".to_string(),
            source: crate::providers::NodeBinding {
                file_path: "src/payments.rs".to_string(),
                qualified_name: "src.payments.PaymentService.create_link".to_string(),
                symbol_kind: None,
            },
            kind: "uses".to_string(),
            target: crate::providers::NodeBinding {
                file_path: "src/config.rs".to_string(),
                qualified_name: "src.config.EntityConfig.payment_expiration".to_string(),
                symbol_kind: None,
            },
            extra: yaml_serde::Mapping::new(),
        });
        let records = vec![record];
        let query = |path: &str, symbol: Option<&str>| TargetKey {
            rel_path: Some(path.to_string()),
            symbol: symbol.map(str::to_string),
            ..Default::default()
        };

        let source = governing(
            &query("src/payments.rs", Some("PaymentService::create_link")),
            &records,
        );
        assert_eq!(source.len(), 1);
        assert_eq!(source[0].kind, MatchKind::RelationshipEndpoint);
        assert_eq!(
            governing(
                &query("src/config.rs", Some("payment_expiration")),
                &records
            )
            .len(),
            1
        );
        assert_eq!(governing(&query("src/config.rs", None), &records).len(), 1);
        assert!(
            governing(&query("src/payments.rs", Some("sign_link")), &records).is_empty(),
            "otro símbolo del mismo archivo no es extremo de la relación"
        );
        assert!(governing(&query("src/other.rs", Some("create_link")), &records).is_empty());
    }

    #[test]
    fn clean_rel_normalizes_separators_and_leading_dot_slash() {
        assert_eq!(clean_rel("./src/foo.rs"), "src/foo.rs");
        assert_eq!(clean_rel("src\\foo.rs"), "src/foo.rs");
        assert_eq!(clean_rel("src/foo.rs"), "src/foo.rs");
    }

    fn key(path: &str, symbol: Option<&str>, is_dir: bool) -> TargetKey {
        TargetKey {
            rel_path: Some(path.to_string()),
            symbol: symbol.map(str::to_string),
            is_dir,
        }
    }

    /// Hasta v1.1 cada `archivo::símbolo` escribía también un binding de
    /// archivo acompañante: el Record de `cancelShipment` gobernaba
    /// `createShipment` (el «contexto de todo el archivo» del reporte).
    #[test]
    fn a_companion_file_binding_does_not_spread_a_symbol_record_over_the_file() {
        let mut record = record_with_binding("decision.cancel", file_binding("src/service.ts"));
        record.binding_declarations.push(structural_binding(
            "src.service.cancelShipment",
            Some("src/service.ts"),
        ));
        let records = vec![record];
        let own = governing(
            &key("src/service.ts", Some("cancelShipment"), false),
            &records,
        );
        assert_eq!(own[0].kind, MatchKind::Structural);
        assert!(governing(
            &key("src/service.ts", Some("createShipment"), false),
            &records
        )
        .is_empty());
        assert_eq!(
            governing(&key("src/service.ts", None, false), &records)[0].kind,
            MatchKind::FileExact,
            "a file query still sees every Record bound inside the file"
        );
    }

    #[test]
    fn an_explicit_file_scope_still_governs_every_symbol() {
        let mut file = file_binding("src/service.ts");
        file.extra.insert(
            yaml_serde::Value::String("scope".to_string()),
            yaml_serde::Value::String("file".to_string()),
        );
        let mut record = record_with_binding("constraint.file-wide", file);
        record.binding_declarations.push(structural_binding(
            "src.service.cancelShipment",
            Some("src/service.ts"),
        ));
        let records = vec![record];
        let other = governing(
            &key("src/service.ts", Some("createShipment"), false),
            &records,
        );
        assert_eq!(other[0].kind, MatchKind::FileContainsSymbol);
    }

    #[test]
    fn a_lexical_symbol_binding_governs_only_its_symbol() {
        let mut binding = file_binding("src/service.ts");
        binding.kind = "symbol".to_string();
        binding.symbol = Some("splitShipment".to_string());
        let records = vec![record_with_binding("decision.split", binding)];
        let own = governing(
            &key("src/service.ts", Some("splitShipment"), false),
            &records,
        );
        assert_eq!(own[0].kind, MatchKind::UnconfirmedSymbol);
        assert!(governing(
            &key("src/service.ts", Some("createShipment"), false),
            &records
        )
        .is_empty());
        assert_eq!(
            governing(&key("src/service.ts", None, false), &records).len(),
            1
        );
    }

    #[test]
    fn folder_queries_and_folder_scopes() {
        let mut scope = file_binding("src/billing");
        scope.kind = "directory".to_string();
        let records = vec![
            record_with_binding("constraint.inside", file_binding("src/shipping/rates.ts")),
            record_with_binding(
                "constraint.deeper",
                file_binding("src/shipping/carriers/dhl.ts"),
            ),
            record_with_binding("constraint.outside", file_binding("src/shippingx/a.ts")),
            record_with_binding("constraint.billing-scope", scope),
        ];
        let folder = governing(&key("src/shipping", None, true), &records);
        let ids: Vec<_> = folder.iter().map(|m| m.record.id.as_str()).collect();
        assert_eq!(ids, ["constraint.deeper", "constraint.inside"]);
        assert!(folder.iter().all(|m| m.kind == MatchKind::DirectoryMember));

        let in_scope = governing(
            &key("src/billing/invoice.ts", Some("issueInvoice"), false),
            &records,
        );
        assert_eq!(in_scope[0].record.id, "constraint.billing-scope");
        assert_eq!(in_scope[0].kind, MatchKind::DirectoryScope);
        assert!(governing(&key("src/billing-old/x.ts", None, false), &records).is_empty());
        assert_eq!(
            governing(&key("src", None, true), &records).len(),
            4,
            "an ancestor folder contains a folder scope"
        );
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
            ..Default::default()
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
