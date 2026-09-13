//! Estado estructural de las relaciones explicadas por el canon (vNext).
//!
//! Un Record puede explicar por qué `A --uses--> B` existe. Esa explicación
//! es memoria causal y vive en el canon; si la relación sigue existiendo es
//! un hecho estructural regenerable, así que se **deriva** en cada consulta
//! y nunca se persiste:
//!
//! - `observed`: la relación directa compatible existe hoy.
//! - `indirect`: ya no es directa, pero un camino acotado y compatible
//!   (delegación por `calls` + el mismo tipo al final) conecta los extremos.
//! - `orphaned`: el proveedor respondió y no puede localizarla — la
//!   explicación puede haber quedado obsoleta. Nunca se borra por esto.
//! - `unknown`: el proveedor no pudo responder. Un proveedor caído no es
//!   evidencia de que la relación desapareció.
//!
//! La alcanzabilidad arbitraria no reancla una explicación: un camino de 13
//! saltos no significa que el porqué siga aplicando. `MAX_HOPS` es el techo.

use crate::providers::{
    CodeIntelligenceProvider, NodeBinding, ProviderHandle, ProviderStatus, RelationKind,
    StructuralPath, StructuralRelationship,
};
use crate::storage::{Record, RelationshipBinding};
use serde::Serialize;

pub const MAX_HOPS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipState {
    Observed,
    Indirect,
    Orphaned,
    Unknown,
}

impl RelationshipState {
    pub fn as_str(self) -> &'static str {
        match self {
            RelationshipState::Observed => "observed",
            RelationshipState::Indirect => "indirect",
            RelationshipState::Orphaned => "orphaned",
            RelationshipState::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationshipAssessment {
    pub record_id: String,
    pub binding_id: String,
    /// Clave estable de la relación agregada (`source`, `kind`, `target`).
    pub key: String,
    pub source: NodeBinding,
    pub kind: String,
    pub target: NodeBinding,
    pub state: RelationshipState,
    /// El camino que la mantiene cuando es `indirect`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<StructuralPath>,
    pub detail: String,
}

pub fn relationship_key(project_id: &str, binding: &RelationshipBinding) -> String {
    StructuralRelationship::relationship_key(
        project_id,
        &binding.source,
        binding.relation_kind().unwrap_or(RelationKind::Other),
        &binding.target,
    )
}

fn assessment(
    record: &Record,
    binding: &RelationshipBinding,
    project_id: &str,
    state: RelationshipState,
    path: Option<StructuralPath>,
    detail: impl Into<String>,
) -> RelationshipAssessment {
    RelationshipAssessment {
        record_id: record.id.clone(),
        binding_id: binding.id.clone(),
        key: relationship_key(project_id, binding),
        source: binding.source.clone(),
        kind: binding.kind.clone(),
        target: binding.target.clone(),
        state,
        path,
        detail: detail.into(),
    }
}

/// Deriva el estado de una relación explicada. Nunca muta el canon.
pub fn assess(
    provider: &mut ProviderHandle,
    repo_path: &str,
    project_id: &str,
    record: &Record,
    binding: &RelationshipBinding,
) -> RelationshipAssessment {
    let Some(kind) = binding.relation_kind() else {
        return assessment(
            record,
            binding,
            project_id,
            RelationshipState::Unknown,
            None,
            format!("tipo de relación '{}' desconocido", binding.kind),
        );
    };
    let Some(client) = provider.as_provider() else {
        return assessment(
            record,
            binding,
            project_id,
            RelationshipState::Unknown,
            None,
            "sin proveedor estructural — no se puede verificar; no implica que la relación \
             haya desaparecido",
        );
    };
    assess_with(client, repo_path, project_id, record, binding, kind)
}

fn provider_failed<T>(result: &crate::providers::ProviderResult<T>) -> bool {
    matches!(
        result.status,
        ProviderStatus::Unavailable | ProviderStatus::Degraded
    )
}

fn assess_with(
    client: &mut dyn CodeIntelligenceProvider,
    repo_path: &str,
    project_id: &str,
    record: &Record,
    binding: &RelationshipBinding,
    kind: RelationKind,
) -> RelationshipAssessment {
    for (label, endpoint) in [("origen", &binding.source), ("destino", &binding.target)] {
        let found = client.get_node(repo_path, endpoint);
        if provider_failed(&found) {
            return assessment(
                record,
                binding,
                project_id,
                RelationshipState::Unknown,
                None,
                format!(
                    "el proveedor no pudo consultar el {label}: {}",
                    found.warnings.join("; ")
                ),
            );
        }
        if found.data.is_none() {
            return assessment(
                record,
                binding,
                project_id,
                RelationshipState::Orphaned,
                None,
                format!(
                    "el {label} '{}' ya no se localiza en '{}' — la explicación puede haber \
                     quedado obsoleta (renombre, movimiento o eliminación)",
                    endpoint.qualified_name, endpoint.file_path
                ),
            );
        }
    }

    let direct = client.get_relationships(repo_path, &binding.source, &binding.target);
    if provider_failed(&direct) {
        return assessment(
            record,
            binding,
            project_id,
            RelationshipState::Unknown,
            None,
            format!(
                "el proveedor no pudo consultar relaciones: {}",
                direct.warnings.join("; ")
            ),
        );
    }
    let observed = direct
        .data
        .unwrap_or_default()
        .into_iter()
        .any(|relationship| {
            relationship.kind == kind
                && relationship.source.qualified_name == binding.source.qualified_name
                && relationship.target.qualified_name == binding.target.qualified_name
        });
    if observed {
        return assessment(
            record,
            binding,
            project_id,
            RelationshipState::Observed,
            None,
            "la relación directa existe en el índice actual",
        );
    }

    let paths = client.find_paths(repo_path, &binding.source, &binding.target, kind, MAX_HOPS);
    if provider_failed(&paths) {
        return assessment(
            record,
            binding,
            project_id,
            RelationshipState::Unknown,
            None,
            format!(
                "sin relación directa y el proveedor no pudo buscar caminos: {}",
                paths.warnings.join("; ")
            ),
        );
    }
    match paths
        .data
        .unwrap_or_default()
        .into_iter()
        .min_by_key(StructuralPath::hops)
    {
        Some(path) => {
            let hops = path.hops();
            assessment(
                record,
                binding,
                project_id,
                RelationshipState::Indirect,
                Some(path),
                format!(
                    "ya no es directa; un camino compatible de {hops} saltos conecta los extremos"
                ),
            )
        }
        None => assessment(
            record,
            binding,
            project_id,
            RelationshipState::Orphaned,
            None,
            format!(
                "ni relación directa ni camino compatible de hasta {MAX_HOPS} saltos — la \
                 explicación puede haber quedado obsoleta"
            ),
        ),
    }
}

/// Todos los bindings de relación de los Records dados, evaluados.
pub fn assess_records(
    provider: &mut ProviderHandle,
    repo_path: &str,
    project_id: &str,
    records: &[&Record],
) -> Vec<RelationshipAssessment> {
    records
        .iter()
        .flat_map(|record| {
            record
                .relationship_bindings
                .iter()
                .map(move |binding| (*record, binding))
        })
        .map(|(record, binding)| assess(provider, repo_path, project_id, record, binding))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::fixture::payments_fixture;

    fn node(file: &str, qn: &str) -> NodeBinding {
        NodeBinding {
            file_path: file.to_string(),
            qualified_name: qn.to_string(),
            symbol_kind: None,
        }
    }

    fn record_with(binding: RelationshipBinding) -> Record {
        Record {
            id: "decision.payment-expiration-per-entity".to_string(),
            kind: "decision".to_string(),
            severity: "high".to_string(),
            statement: "Payment expiration belongs to entity configuration.".to_string(),
            rationale: Some("Each tenant defines its own payment policy.".to_string()),
            relationship_bindings: vec![binding],
            ..Default::default()
        }
    }

    fn uses_binding() -> RelationshipBinding {
        RelationshipBinding {
            id: "rel.payment-expiration.0".to_string(),
            source: node("src/payments.rs", "src.payments.create_link"),
            kind: "uses".to_string(),
            target: node(
                "src/config.rs",
                "src.config.EntityConfig.payment_expiration",
            ),
            extra: yaml_serde::Mapping::new(),
        }
    }

    fn assess_fixture(
        provider: crate::providers::fixture::FixtureProvider,
    ) -> RelationshipAssessment {
        let binding = uses_binding();
        let record = record_with(binding.clone());
        let mut handle = ProviderHandle::Custom(Box::new(provider));
        assess(&mut handle, "", "fixture", &record, &binding)
    }

    #[test]
    fn a_direct_relationship_is_observed() {
        let result = assess_fixture(payments_fixture());
        assert_eq!(
            result.state,
            RelationshipState::Observed,
            "{}",
            result.detail
        );
        assert!(result.path.is_none());
    }

    #[test]
    fn a_relationship_moved_behind_a_delegation_is_indirect() {
        let mut provider = payments_fixture();
        provider.remove_relationship(
            "src.payments.create_link",
            RelationKind::Uses,
            "src.config.EntityConfig.payment_expiration",
        );
        provider.add_relationship(
            "src.payments.sign_link",
            RelationKind::Uses,
            "src.config.EntityConfig.payment_expiration",
        );
        let result = assess_fixture(provider);
        assert_eq!(
            result.state,
            RelationshipState::Indirect,
            "{}",
            result.detail
        );
        assert_eq!(result.path.unwrap().hops(), 2);
    }

    #[test]
    fn a_relationship_that_cannot_be_located_is_orphaned_never_deleted() {
        let mut provider = payments_fixture();
        provider.remove_relationship(
            "src.payments.create_link",
            RelationKind::Uses,
            "src.config.EntityConfig.payment_expiration",
        );
        let result = assess_fixture(provider);
        assert_eq!(
            result.state,
            RelationshipState::Orphaned,
            "{}",
            result.detail
        );

        let binding = RelationshipBinding {
            source: node("src/payments.rs", "src.payments.renamed_away"),
            ..uses_binding()
        };
        let record = record_with(binding.clone());
        let mut handle = ProviderHandle::Custom(Box::new(payments_fixture()));
        let missing_endpoint = assess(&mut handle, "", "fixture", &record, &binding);
        assert_eq!(missing_endpoint.state, RelationshipState::Orphaned);
        assert!(missing_endpoint.detail.contains("renamed_away"));
        assert_eq!(
            record.relationship_bindings.len(),
            1,
            "evaluar nunca muta el canon"
        );
    }

    #[test]
    fn long_reachability_is_not_a_valid_reanchor() {
        let mut provider = payments_fixture();
        provider.remove_relationship(
            "src.payments.create_link",
            RelationKind::Uses,
            "src.config.EntityConfig.payment_expiration",
        );
        // create_link → sign_link → api → config.load … nunca `uses` al final
        // dentro del techo de saltos: sigue huérfana.
        provider.add_relationship(
            "src.payments.sign_link",
            RelationKind::Calls,
            "src.config.load",
        );
        let result = assess_fixture(provider);
        assert_eq!(
            result.state,
            RelationshipState::Orphaned,
            "un camino que termina en `writes` no reancla una explicación de `uses`"
        );
    }

    #[test]
    fn without_a_provider_the_state_is_unknown_not_orphaned() {
        let binding = uses_binding();
        let record = record_with(binding.clone());
        let mut handle = ProviderHandle::Unavailable("test".to_string());
        let result = assess(&mut handle, "", "fixture", &record, &binding);
        assert_eq!(result.state, RelationshipState::Unknown);
    }

    #[test]
    fn relationship_bindings_serialize_as_stable_yaml() {
        let binding = uses_binding();
        let yaml = yaml_serde::to_string(&binding).unwrap();
        assert!(yaml.contains("kind: uses"), "{yaml}");
        assert!(
            yaml.contains("qualified_name: src.payments.create_link"),
            "{yaml}"
        );
        let back: RelationshipBinding = yaml_serde::from_str(&yaml).unwrap();
        assert_eq!(back, binding);
        assert_eq!(
            relationship_key("p", &binding),
            relationship_key("p", &back),
            "la clave de la relación es determinista"
        );
    }
}
