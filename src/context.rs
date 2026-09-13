//! Contexto estructural de una operación (vNext).
//!
//! El Context Compiler no vuelca el grafo: parte del target resuelto, pide
//! al proveedor un vecindario acotado, superpone la memoria causal (Records
//! atados a nodos y relaciones explicadas con su estado derivado) y
//! selecciona el subgrafo mínimo que el agente necesita. Lo considerado y lo
//! seleccionado se conservan por separado: la UI enseña ambos y la
//! actividad los cuenta.

use crate::binding_match::{self, TargetKey};
use crate::operations::{GraphEdge, GraphNode, OperationGraph};
use crate::providers::{
    CodeIntelligenceProvider, CodeSnippet, Direction, NeighborQuery, NodeBinding, ProviderHandle,
    ProviderStatus, RelationKind, StructuralNode, StructuralRelationship,
};
use crate::relationships::{self, RelationshipAssessment, RelationshipState};
use crate::storage::Record;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct StructuralBudget {
    pub max_nodes: usize,
    pub max_relationships: usize,
    pub snippet_chars: usize,
}

impl Default for StructuralBudget {
    fn default() -> Self {
        StructuralBudget {
            max_nodes: 12,
            max_relationships: 16,
            snippet_chars: 1200,
        }
    }
}

/// Rol de un nodo respecto del target — ordena la selección y la UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Target,
    /// Extremo de una relación que el canon explica.
    Explained,
    Caller,
    Callee,
    Dependency,
    Dependent,
    Test,
    Context,
}

impl NodeRole {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeRole::Target => "target",
            NodeRole::Explained => "explained",
            NodeRole::Caller => "caller",
            NodeRole::Callee => "callee",
            NodeRole::Dependency => "dependency",
            NodeRole::Dependent => "dependent",
            NodeRole::Test => "test",
            NodeRole::Context => "context",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextNode {
    pub key: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
    pub qualified_name: String,
    pub role: NodeRole,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub record_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextEdge {
    pub key: String,
    pub source: String,
    pub kind: String,
    pub target: String,
    pub state: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub record_ids: Vec<String>,
}

#[derive(Debug, Default)]
pub struct StructuralContext {
    pub provider_name: Option<String>,
    pub index_state: Option<String>,
    pub target_node: Option<StructuralNode>,
    pub nodes: Vec<ContextNode>,
    pub edges: Vec<ContextEdge>,
    pub selected_nodes: Vec<String>,
    pub selected_edges: Vec<String>,
    pub relationships: Vec<RelationshipAssessment>,
    pub snippet: Option<CodeSnippet>,
    pub known_unknowns: Vec<String>,
    pub warnings: Vec<String>,
    pub truncated: bool,
}

impl StructuralContext {
    pub fn selected_nodes(&self) -> Vec<&ContextNode> {
        self.nodes
            .iter()
            .filter(|node| self.selected_nodes.contains(&node.key))
            .collect()
    }

    pub fn selected_edges(&self) -> Vec<&ContextEdge> {
        self.edges
            .iter()
            .filter(|edge| self.selected_edges.contains(&edge.key))
            .collect()
    }

    /// Recorta nodos seleccionados de menor prioridad (nunca el target ni los
    /// extremos explicados) y las aristas que quedan sin extremos. Devuelve
    /// si pudo recortar algo — lo usa el presupuesto de tokens.
    pub fn drop_lowest_priority_node(&mut self) -> bool {
        let candidate = self
            .nodes
            .iter()
            .filter(|node| self.selected_nodes.contains(&node.key))
            .filter(|node| node.role > NodeRole::Explained)
            .max_by_key(|node| node.role)
            .map(|node| node.key.clone());
        let Some(key) = candidate else {
            return false;
        };
        self.selected_nodes.retain(|selected| selected != &key);
        let remaining: HashSet<&String> = self.selected_nodes.iter().collect();
        let edges = &self.edges;
        self.selected_edges.retain(|edge_key| {
            edges
                .iter()
                .find(|edge| &edge.key == edge_key)
                .is_some_and(|edge| {
                    remaining.contains(&edge.source) && remaining.contains(&edge.target)
                })
        });
        true
    }

    pub fn to_graph(&self) -> OperationGraph {
        OperationGraph {
            nodes: self
                .nodes
                .iter()
                .map(|node| GraphNode {
                    key: node.key.clone(),
                    name: node.name.clone(),
                    label: node.label.clone(),
                    file_path: node.file_path.clone(),
                    qualified_name: node.qualified_name.clone(),
                    role: node.role.as_str().to_string(),
                    selected: self.selected_nodes.contains(&node.key),
                    record_ids: node.record_ids.clone(),
                    start_line: node.start_line,
                })
                .collect(),
            edges: self
                .edges
                .iter()
                .map(|edge| GraphEdge {
                    key: edge.key.clone(),
                    source: edge.source.clone(),
                    kind: edge.kind.clone(),
                    target: edge.target.clone(),
                    selected: self.selected_edges.contains(&edge.key),
                    state: edge.state.clone(),
                    record_ids: edge.record_ids.clone(),
                })
                .collect(),
            truncated: self.truncated,
        }
    }
}

fn role_for(relationship: &StructuralRelationship, target: &NodeBinding) -> NodeRole {
    let outbound = relationship.source.qualified_name == target.qualified_name;
    match (relationship.kind, outbound) {
        (RelationKind::Calls, false) => NodeRole::Caller,
        (RelationKind::Calls, true) => NodeRole::Callee,
        (RelationKind::Tests, false) => NodeRole::Test,
        (
            RelationKind::Uses
            | RelationKind::Writes
            | RelationKind::Imports
            | RelationKind::DependsOn
            | RelationKind::Configures
            | RelationKind::HttpCalls,
            true,
        ) => NodeRole::Dependency,
        (_, false) => NodeRole::Dependent,
        _ => NodeRole::Context,
    }
}

/// Las claves se derivan siempre de la identidad portable con la clave de
/// proyecto del núcleo: la que calculó el proveedor no se usa, porque cada
/// adaptador puede derivarla distinto y una arista quedaría sin sus nodos.
fn context_node(node: &StructuralNode, role: NodeRole, project_key: &str) -> ContextNode {
    ContextNode {
        key: node.binding.key(project_key),
        name: node.name.clone(),
        label: node.label.clone(),
        file_path: node.binding.file_path.clone(),
        qualified_name: node.binding.qualified_name.clone(),
        role,
        record_ids: vec![],
        start_line: node.start_line,
    }
}

fn node_from_binding(binding: &NodeBinding, project_key: &str, role: NodeRole) -> ContextNode {
    ContextNode {
        key: binding.key(project_key),
        name: binding.short_name().to_string(),
        label: binding
            .symbol_kind
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        file_path: binding.file_path.clone(),
        qualified_name: binding.qualified_name.clone(),
        role,
        record_ids: vec![],
        start_line: None,
    }
}

/// Records cuyos bindings de símbolo apuntan a un nodo concreto. Los
/// bindings de archivo no se proyectan sobre cada símbolo del archivo: el
/// grafo mostraría la misma regla en decenas de nodos sin decir nada nuevo.
fn records_for_node<'a>(node: &ContextNode, records: &[&'a Record]) -> Vec<&'a Record> {
    let key = TargetKey {
        rel_path: Some(node.file_path.clone()),
        symbol: Some(node.name.clone()),
    };
    records
        .iter()
        .copied()
        .filter(|record| {
            record.binding_declarations.iter().any(|binding| {
                binding
                    .structural_id
                    .as_deref()
                    .is_some_and(|structural_id| {
                        binding.path_hint.as_deref() == Some(node.file_path.as_str())
                            && (structural_id == node.qualified_name
                                || structural_id.ends_with(&format!(".{}", node.qualified_name))
                                || structural_id.ends_with(&format!("::{}", node.name)))
                    })
            }) || record.relationship_bindings.iter().any(|binding| {
                binding_match::match_relationship(&key, binding).is_some()
                    && [&binding.source, &binding.target]
                        .iter()
                        .any(|endpoint| endpoint.qualified_name == node.qualified_name)
            })
        })
        .collect()
}

pub struct GatherInput<'a> {
    pub repo_path: &'a str,
    pub project_key: &'a str,
    pub target_file: Option<&'a str>,
    pub target_symbol: Option<&'a str>,
    /// Records activos del canon.
    pub records: &'a [&'a Record],
    pub budget: &'a StructuralBudget,
}

pub fn gather(provider: &mut ProviderHandle, input: GatherInput) -> StructuralContext {
    let mut context = StructuralContext::default();
    let mut nodes: HashMap<String, ContextNode> = HashMap::new();
    match (provider.as_provider(), input.target_file) {
        (None, _) => context.known_unknowns.push(
            "sin proveedor estructural: vecindario, relaciones y código relevante no \
             verificados"
                .to_string(),
        ),
        (Some(_), None) => context
            .known_unknowns
            .push("el target no resolvió a un archivo del proyecto".to_string()),
        (Some(client), Some(file)) => {
            context.provider_name = Some(client.capabilities().name);
            let status = client.index_status(input.repo_path);
            context.index_state = status.data.map(|s| s.state);
            context.warnings.extend(status.warnings);
            match input.target_symbol.filter(|s| !s.is_empty()) {
                // Target de archivo: el outline es la estructura honesta; no se
                // inventa un "nodo archivo" con relaciones que el proveedor no dio.
                None => {
                    let outline = client.get_file_outline(input.repo_path, file);
                    context.warnings.extend(outline.warnings);
                    for node in outline.data.unwrap_or_default() {
                        let node = context_node(&node, NodeRole::Context, input.project_key);
                        nodes.insert(node.key.clone(), node);
                    }
                    context.known_unknowns.push(
                        "target de archivo: el contexto estructural es su outline, sin \
                         vecindario por símbolo"
                            .to_string(),
                    );
                }
                Some(symbol) => {
                    gather_symbol(client, &input, file, symbol, &mut context, &mut nodes)
                }
            }
        }
    }
    // Las relaciones explicadas que tocan el target entran siempre — también
    // sin proveedor, con estado `unknown`: una explicación nunca desaparece
    // del contexto porque la estructura no se pudo verificar.
    overlay_explained_relationships(provider, &input, &mut context, &mut nodes);
    select(&input, nodes, &mut context);
    annotate_records(&mut context, input.records);
    context
}

fn gather_symbol(
    client: &mut dyn CodeIntelligenceProvider,
    input: &GatherInput,
    file: &str,
    symbol: &str,
    context: &mut StructuralContext,
    nodes: &mut HashMap<String, ContextNode>,
) {
    let project_key = input.project_key;
    let resolved = client.resolve_node(input.repo_path, file, symbol);
    context.warnings.extend(resolved.warnings.clone());
    let Some(mut target_node) = resolved.data else {
        context.known_unknowns.push(match resolved.status {
            ProviderStatus::Successful => format!(
                "'{symbol}' no está en el índice del proveedor para '{file}' — no implica que no exista"
            ),
            _ => "el proveedor no pudo resolver el target".to_string(),
        });
        return;
    };
    target_node.key = target_node.binding.key(project_key);
    nodes.insert(
        target_node.key.clone(),
        context_node(&target_node, NodeRole::Target, project_key),
    );

    let neighbors = client.get_neighbors(
        input.repo_path,
        &target_node.binding,
        &NeighborQuery {
            direction: Direction::Both,
            kinds: vec![],
            limit: input.budget.max_nodes * 4,
        },
    );
    context.warnings.extend(neighbors.warnings);
    let subgraph = neighbors.data.unwrap_or_default();
    context.truncated = subgraph.truncated;
    for relationship in &subgraph.relationships {
        let outbound = relationship.source.qualified_name == target_node.binding.qualified_name;
        let other = if outbound {
            &relationship.target
        } else {
            &relationship.source
        };
        let role = role_for(relationship, &target_node.binding);
        if let Some(found) = subgraph.nodes.iter().find(|n| n.binding == *other) {
            let entry = nodes
                .entry(found.binding.key(project_key))
                .or_insert_with(|| context_node(found, role, project_key));
            entry.role = entry.role.min(role);
        }
        context.edges.push(ContextEdge {
            key: StructuralRelationship::relationship_key(
                project_key,
                &relationship.source,
                relationship.kind,
                &relationship.target,
            ),
            source: relationship.source.key(project_key),
            kind: relationship.kind.as_str().to_string(),
            target: relationship.target.key(project_key),
            state: RelationshipState::Observed.as_str().to_string(),
            record_ids: vec![],
        });
    }

    let snippet = client.get_code_snippet(input.repo_path, &target_node.binding);
    if let Some(mut snippet) = snippet.data {
        if snippet.source.chars().count() > input.budget.snippet_chars {
            snippet.source = snippet
                .source
                .chars()
                .take(input.budget.snippet_chars)
                .collect();
            snippet.truncated = true;
        }
        context.snippet = Some(snippet);
    }
    context.target_node = Some(target_node);
}

fn overlay_explained_relationships(
    provider: &mut ProviderHandle,
    input: &GatherInput,
    context: &mut StructuralContext,
    nodes: &mut HashMap<String, ContextNode>,
) {
    let Some(file) = input.target_file else {
        return;
    };
    let project_key = input.project_key;
    let target_key = TargetKey {
        rel_path: Some(file.to_string()),
        symbol: input
            .target_symbol
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    };
    for record in input.records {
        for binding in &record.relationship_bindings {
            if binding_match::match_relationship(&target_key, binding).is_none() {
                continue;
            }
            let assessment =
                relationships::assess(provider, input.repo_path, project_key, record, binding);
            for endpoint in [&binding.source, &binding.target] {
                nodes
                    .entry(endpoint.key(project_key))
                    .and_modify(|node| node.role = node.role.min(NodeRole::Explained))
                    .or_insert_with(|| {
                        node_from_binding(endpoint, project_key, NodeRole::Explained)
                    });
            }
            match context
                .edges
                .iter_mut()
                .find(|edge| edge.key == assessment.key)
            {
                Some(edge) => {
                    if !edge.record_ids.contains(&record.id) {
                        edge.record_ids.push(record.id.clone());
                    }
                    edge.state = assessment.state.as_str().to_string();
                }
                None => context.edges.push(ContextEdge {
                    key: assessment.key.clone(),
                    source: binding.source.key(project_key),
                    kind: binding.kind.clone(),
                    target: binding.target.key(project_key),
                    state: assessment.state.as_str().to_string(),
                    record_ids: vec![record.id.clone()],
                }),
            }
            match assessment.state {
                RelationshipState::Indirect | RelationshipState::Orphaned => {
                    context.warnings.push(format!(
                        "relación explicada por '{}' está {}: {}",
                        record.id,
                        assessment.state.as_str(),
                        assessment.detail
                    ));
                }
                RelationshipState::Unknown => context.known_unknowns.push(format!(
                    "estado de la relación de '{}' desconocido: {}",
                    record.id, assessment.detail
                )),
                RelationshipState::Observed => {}
            }
            context.relationships.push(assessment);
        }
    }
}

/// Selección acotada: el target y los extremos explicados siempre; el resto
/// por rol hasta `max_nodes`. Las aristas explicadas siempre; el resto hasta
/// `max_relationships`. El techo nunca es una cuota.
fn select(
    input: &GatherInput,
    nodes: HashMap<String, ContextNode>,
    context: &mut StructuralContext,
) {
    let file = input.target_file.unwrap_or("");
    let mut ordered: Vec<ContextNode> = nodes.into_values().collect();
    ordered.sort_by(|a, b| {
        a.role
            .cmp(&b.role)
            .then_with(|| (a.file_path != file).cmp(&(b.file_path != file)))
            .then_with(|| a.file_path.cmp(&b.file_path))
            .then_with(|| a.start_line.cmp(&b.start_line))
            .then_with(|| a.qualified_name.cmp(&b.qualified_name))
    });
    for node in &ordered {
        let always = node.role <= NodeRole::Explained;
        if always || context.selected_nodes.len() < input.budget.max_nodes {
            context.selected_nodes.push(node.key.clone());
        }
    }
    context.nodes = ordered;

    let selected: HashSet<&String> = context.selected_nodes.iter().collect();
    let mut edges: Vec<&ContextEdge> = context
        .edges
        .iter()
        .filter(|edge| selected.contains(&edge.source) && selected.contains(&edge.target))
        .collect();
    edges.sort_by_key(|edge| (edge.record_ids.is_empty(), edge.key.clone()));
    let explained_count = edges.iter().filter(|e| !e.record_ids.is_empty()).count();
    let cap = input.budget.max_relationships.max(explained_count);
    let chosen: Vec<String> = edges
        .iter()
        .take(cap)
        .map(|edge| edge.key.clone())
        .collect();
    context.selected_edges = chosen;
}

fn annotate_records(context: &mut StructuralContext, records: &[&Record]) {
    for node in &mut context.nodes {
        node.record_ids = records_for_node(node, records)
            .into_iter()
            .map(|record| record.id.clone())
            .collect();
        node.record_ids.sort();
        node.record_ids.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::fixture::payments_fixture;
    use crate::storage::RelationshipBinding;

    fn uses_record() -> Record {
        Record {
            id: "decision.payment-expiration-per-entity".to_string(),
            kind: "decision".to_string(),
            severity: "high".to_string(),
            statement: "Payment expiration belongs to entity configuration.".to_string(),
            rationale: Some("Each tenant defines its own payment policy.".to_string()),
            relationship_bindings: vec![RelationshipBinding {
                id: "rel.0".to_string(),
                source: NodeBinding {
                    file_path: "src/payments.rs".to_string(),
                    qualified_name: "src.payments.create_link".to_string(),
                    symbol_kind: None,
                },
                kind: "uses".to_string(),
                target: NodeBinding {
                    file_path: "src/config.rs".to_string(),
                    qualified_name: "src.config.EntityConfig.payment_expiration".to_string(),
                    symbol_kind: None,
                },
                extra: yaml_serde::Mapping::new(),
            }],
            ..Default::default()
        }
    }

    fn gather_with(
        provider: crate::providers::fixture::FixtureProvider,
        records: &[&Record],
        budget: StructuralBudget,
    ) -> StructuralContext {
        let mut handle = ProviderHandle::Custom(Box::new(provider));
        gather(
            &mut handle,
            GatherInput {
                repo_path: "",
                project_key: "fixture",
                target_file: Some("src/payments.rs"),
                target_symbol: Some("create_link"),
                records,
                budget: &budget,
            },
        )
    }

    #[test]
    fn neighborhood_is_bounded_ranked_and_annotated_with_rationale() {
        let record = uses_record();
        let context = gather_with(payments_fixture(), &[&record], StructuralBudget::default());

        let target = context.target_node.as_ref().unwrap();
        assert_eq!(target.binding.qualified_name, "src.payments.create_link");
        let roles: HashMap<&str, NodeRole> = context
            .nodes
            .iter()
            .map(|n| (n.name.as_str(), n.role))
            .collect();
        assert_eq!(roles["create_link"], NodeRole::Target);
        assert_eq!(roles["post_link"], NodeRole::Caller);
        assert_eq!(roles["sign_link"], NodeRole::Callee);
        assert_eq!(roles["payment_expiration"], NodeRole::Explained);
        assert_eq!(roles["links_expire"], NodeRole::Test);

        let explained = context
            .edges
            .iter()
            .find(|e| e.kind == "uses")
            .expect("la relación explicada está en el contexto");
        assert_eq!(explained.state, "observed");
        assert_eq!(explained.record_ids, vec![record.id.clone()]);
        assert_eq!(context.relationships.len(), 1);
        assert!(context.snippet.is_some());
        assert!(
            !context.edges.iter().any(|e| e.kind == "similar_to"),
            "una similitud inferida nunca entra como estructura"
        );
    }

    #[test]
    fn a_tight_budget_keeps_the_target_and_explained_endpoints() {
        let record = uses_record();
        let context = gather_with(
            payments_fixture(),
            &[&record],
            StructuralBudget {
                max_nodes: 1,
                max_relationships: 0,
                snippet_chars: 5,
            },
        );
        let selected = context.selected_nodes();
        let names: Vec<&str> = selected.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"create_link"));
        assert!(
            names.contains(&"payment_expiration"),
            "un extremo explicado nunca se recorta"
        );
        assert!(!names.contains(&"post_link"));
        assert_eq!(
            context.selected_edges().len(),
            1,
            "la arista explicada sobrevive aunque el techo sea 0"
        );
        assert!(context.snippet.as_ref().unwrap().truncated);
    }

    #[test]
    fn a_moved_relationship_stays_visible_with_its_derived_state() {
        let record = uses_record();
        let mut provider = payments_fixture();
        provider.remove_relationship(
            "src.payments.create_link",
            RelationKind::Uses,
            "src.config.EntityConfig.payment_expiration",
        );
        let context = gather_with(provider, &[&record], StructuralBudget::default());
        let explained = context
            .edges
            .iter()
            .find(|e| e.kind == "uses")
            .expect("la explicación nunca desaparece del contexto");
        assert_eq!(explained.state, "orphaned");
        assert!(context.warnings.iter().any(|w| w.contains("orphaned")));
    }

    #[test]
    fn without_a_provider_structure_is_a_known_unknown() {
        let mut handle = ProviderHandle::Unavailable("test".to_string());
        let context = gather(
            &mut handle,
            GatherInput {
                repo_path: "",
                project_key: "fixture",
                target_file: Some("src/payments.rs"),
                target_symbol: Some("create_link"),
                records: &[],
                budget: &StructuralBudget::default(),
            },
        );
        assert!(context.nodes.is_empty());
        assert_eq!(context.known_unknowns.len(), 1);
    }

    #[test]
    fn dropping_nodes_never_removes_the_target_or_explained_endpoints() {
        let record = uses_record();
        let mut context = gather_with(payments_fixture(), &[&record], StructuralBudget::default());
        while context.drop_lowest_priority_node() {}
        let names: Vec<String> = context
            .selected_nodes()
            .iter()
            .map(|n| n.name.clone())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(context.selected_edges().iter().all(|e| e.kind == "uses"));
    }

    fn gather_for(
        handle: &mut ProviderHandle,
        project_key: &str,
        file: &str,
        symbol: Option<&str>,
        records: &[&Record],
    ) -> StructuralContext {
        gather(
            handle,
            GatherInput {
                repo_path: "",
                project_key,
                target_file: Some(file),
                target_symbol: symbol,
                records,
                budget: &StructuralBudget::default(),
            },
        )
    }

    /// Defecto real: el adaptador de CBM deriva sus claves con otra clave de
    /// proyecto; si el núcleo las mezclara, ninguna arista tendría sus nodos.
    #[test]
    fn keys_come_from_the_core_project_key_not_from_the_provider() {
        let record = uses_record();
        let mut handle = ProviderHandle::Custom(Box::new(payments_fixture()));
        let context = gather_for(
            &mut handle,
            "core-project",
            "src/payments.rs",
            Some("create_link"),
            &[&record],
        );
        let keys: HashSet<&str> = context
            .selected_nodes()
            .iter()
            .map(|n| n.key.as_str())
            .collect();
        assert!(context.selected_edges().len() > 1);
        for edge in context.selected_edges() {
            assert!(
                keys.contains(edge.source.as_str()) && keys.contains(edge.target.as_str()),
                "arista sin sus nodos: {edge:?}"
            );
        }
        let target = context.target_node.as_ref().unwrap();
        assert_eq!(target.key, target.binding.key("core-project"));
    }

    #[test]
    fn explained_relationships_survive_without_a_provider() {
        let record = uses_record();
        let mut handle = ProviderHandle::Unavailable("test".to_string());
        let context = gather_for(
            &mut handle,
            "fixture",
            "src/payments.rs",
            Some("create_link"),
            &[&record],
        );
        let edge = context
            .selected_edges()
            .into_iter()
            .find(|e| e.kind == "uses")
            .expect("la explicación sigue en el contexto");
        assert_eq!(edge.state, "unknown");
        assert_eq!(context.relationships.len(), 1);
        assert_eq!(context.selected_nodes().len(), 2);
    }

    #[test]
    fn a_file_target_also_carries_its_explained_relationships() {
        let record = uses_record();
        let mut handle = ProviderHandle::Custom(Box::new(payments_fixture()));
        let context = gather_for(&mut handle, "fixture", "src/config.rs", None, &[&record]);
        assert_eq!(context.relationships.len(), 1);
        let role = |name: &str| {
            context
                .nodes
                .iter()
                .find(|n| n.name == name)
                .map(|n| n.role)
        };
        assert_eq!(role("load"), Some(NodeRole::Context));
        assert_eq!(role("payment_expiration"), Some(NodeRole::Explained));
    }
}
