//! Proveedor estructural de fixtures — un grafo estático en memoria.
//!
//! Existe para que el núcleo (context compiler, estado derivado de
//! relaciones, UI) se pruebe de forma determinista sin Codebase Memory, y
//! para demos offline (`RATIONALE_PROVIDER=fixture:<ruta.json>`). Implementa
//! el mismo contrato que el adaptador real y se identifica siempre como
//! `fixture`: nunca se hace pasar por un índice del repositorio.
//!
//! Formato JSON:
//!
//! ```json
//! {
//!   "nodes": [{"file_path": "src/pay.rs", "qualified_name": "src.pay.create_link",
//!              "label": "function", "start_line": 1, "end_line": 3,
//!              "source": "fn create_link() {}"}],
//!   "relationships": [{"source": "src.pay.create_link", "kind": "calls",
//!                      "target": "src.config.expiration"}]
//! }
//! ```

use super::{
    ArchitectureCluster, ArchitectureSummary, CodeIntelligenceProvider, CodeSnippet, Coverage,
    Direction, IndexStatus, NeighborQuery, NodeBinding, ProviderCapabilities, ProviderResult,
    RelationKind, ResolvedTarget, StructuralNode, StructuralPath, StructuralRelationship,
    StructuralSubgraph,
};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

const NAME: &str = "fixture";
const PROJECT_KEY: &str = "fixture";

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureNode {
    pub file_path: String,
    pub qualified_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "default_label")]
    pub label: String,
    #[serde(default)]
    pub start_line: Option<u32>,
    #[serde(default)]
    pub end_line: Option<u32>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub is_test: bool,
    #[serde(default)]
    pub source: Option<String>,
}

fn default_label() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureRelationship {
    pub source: String,
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FixtureGraph {
    #[serde(default)]
    pub nodes: Vec<FixtureNode>,
    #[serde(default)]
    pub relationships: Vec<FixtureRelationship>,
}

pub struct FixtureProvider {
    graph: FixtureGraph,
}

impl FixtureProvider {
    pub fn new(graph: FixtureGraph) -> Self {
        FixtureProvider { graph }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let graph: FixtureGraph = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(Self::new(graph))
    }

    /// Simula un refactor en tests: la relación directa desaparece.
    #[cfg(test)]
    pub fn remove_relationship(&mut self, source: &str, kind: RelationKind, target: &str) {
        self.graph.relationships.retain(|r| {
            !(r.source == source
                && r.target == target
                && RelationKind::parse(&r.kind) == Some(kind))
        });
    }

    #[cfg(test)]
    pub fn add_relationship(&mut self, source: &str, kind: RelationKind, target: &str) {
        self.graph.relationships.push(FixtureRelationship {
            source: source.to_string(),
            kind: kind.as_str().to_string(),
            target: target.to_string(),
        });
    }

    fn find(&self, qualified_name: &str) -> Option<&FixtureNode> {
        self.graph
            .nodes
            .iter()
            .find(|node| node.qualified_name == qualified_name)
    }

    fn binding(node: &FixtureNode) -> NodeBinding {
        NodeBinding {
            file_path: node.file_path.clone(),
            qualified_name: node.qualified_name.clone(),
            symbol_kind: Some(node.label.to_ascii_lowercase()),
        }
    }

    fn structural(node: &FixtureNode) -> StructuralNode {
        let binding = Self::binding(node);
        StructuralNode {
            key: binding.key(PROJECT_KEY),
            name: node
                .name
                .clone()
                .unwrap_or_else(|| binding.short_name().to_string()),
            label: node.label.to_ascii_lowercase(),
            start_line: node.start_line,
            end_line: node.end_line,
            signature: node.signature.clone(),
            is_test: node.is_test,
            binding,
        }
    }

    fn edges(&self) -> Vec<(&FixtureNode, RelationKind, &FixtureNode)> {
        self.graph
            .relationships
            .iter()
            .filter_map(|r| {
                let kind = RelationKind::parse(&r.kind).unwrap_or(RelationKind::Other);
                Some((self.find(&r.source)?, kind, self.find(&r.target)?))
            })
            .collect()
    }

    fn relationship(
        source: &FixtureNode,
        kind: RelationKind,
        target: &FixtureNode,
    ) -> StructuralRelationship {
        let source = Self::binding(source);
        let target = Self::binding(target);
        StructuralRelationship {
            key: StructuralRelationship::relationship_key(PROJECT_KEY, &source, kind, &target),
            source,
            kind,
            target,
            provider_kind: kind.as_str().to_string(),
        }
    }
}

impl CodeIntelligenceProvider for FixtureProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            name: NAME.to_string(),
            neighbors: true,
            bounded_paths: true,
            max_path_hops: 3,
            code_snippets: true,
            file_outline: true,
            architecture: true,
            changed_nodes: false,
        }
    }

    fn health(&mut self, _repo_path: &str) -> ProviderResult<()> {
        ProviderResult::ok(NAME, (), Coverage::Complete)
    }

    fn index_status(&mut self, _repo_path: &str) -> ProviderResult<IndexStatus> {
        ProviderResult::ok(
            NAME,
            IndexStatus {
                project: Some(NAME.to_string()),
                state: "ready".to_string(),
                nodes: Some(self.graph.nodes.len() as u64),
                edges: Some(self.graph.relationships.len() as u64),
            },
            Coverage::Complete,
        )
    }

    fn resolve_target(
        &mut self,
        repo_path: &str,
        file_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<ResolvedTarget> {
        let result = self.resolve_node(repo_path, file_path, symbol_name);
        ProviderResult {
            data: result.data.map(|node| ResolvedTarget {
                qualified_name: node.binding.qualified_name,
                file_path: node.binding.file_path,
            }),
            provider_name: result.provider_name,
            status: result.status,
            coverage: result.coverage,
            warnings: result.warnings,
        }
    }

    fn resolve_node(
        &mut self,
        _repo_path: &str,
        file_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<StructuralNode> {
        let wanted = symbol_name.replace("::", ".");
        let found = self.graph.nodes.iter().find(|node| {
            node.file_path == file_path
                && (node.qualified_name == wanted
                    || node.qualified_name.ends_with(&format!(".{wanted}")))
        });
        match found {
            Some(node) => ProviderResult::ok(NAME, Self::structural(node), Coverage::Complete),
            None => ProviderResult::not_found(
                NAME,
                format!("'{symbol_name}' no está en el fixture para '{file_path}'"),
            ),
        }
    }

    fn get_node(&mut self, _repo_path: &str, node: &NodeBinding) -> ProviderResult<StructuralNode> {
        match self.find(&node.qualified_name) {
            Some(found) if found.file_path == node.file_path => {
                ProviderResult::ok(NAME, Self::structural(found), Coverage::Complete)
            }
            _ => ProviderResult::not_found(
                NAME,
                format!("'{}' no está en el fixture", node.qualified_name),
            ),
        }
    }

    fn get_neighbors(
        &mut self,
        _repo_path: &str,
        node: &NodeBinding,
        query: &NeighborQuery,
    ) -> ProviderResult<StructuralSubgraph> {
        let Some(seed) = self.find(&node.qualified_name) else {
            return ProviderResult::not_found(
                NAME,
                format!("'{}' no está en el fixture", node.qualified_name),
            );
        };
        let mut subgraph = StructuralSubgraph {
            nodes: vec![Self::structural(seed)],
            ..Default::default()
        };
        let mut matched = 0usize;
        for (source, kind, target) in self.edges() {
            if !query.accepts(kind) {
                continue;
            }
            let outbound = source.qualified_name == seed.qualified_name
                && matches!(query.direction, Direction::Outbound | Direction::Both);
            let inbound = target.qualified_name == seed.qualified_name
                && matches!(query.direction, Direction::Inbound | Direction::Both);
            if !outbound && !inbound {
                continue;
            }
            matched += 1;
            if matched > query.limit {
                subgraph.truncated = true;
                continue;
            }
            let other = if outbound { target } else { source };
            subgraph.merge(StructuralSubgraph {
                nodes: vec![Self::structural(other)],
                relationships: vec![Self::relationship(source, kind, target)],
                truncated: false,
            });
        }
        ProviderResult::ok(NAME, subgraph, Coverage::Complete)
    }

    fn get_relationships(
        &mut self,
        _repo_path: &str,
        source: &NodeBinding,
        target: &NodeBinding,
    ) -> ProviderResult<Vec<StructuralRelationship>> {
        let relationships = self
            .edges()
            .into_iter()
            .filter(|(s, _, t)| {
                (s.qualified_name == source.qualified_name
                    && t.qualified_name == target.qualified_name)
                    || (s.qualified_name == target.qualified_name
                        && t.qualified_name == source.qualified_name)
            })
            .map(|(s, kind, t)| Self::relationship(s, kind, t))
            .collect();
        ProviderResult::ok(NAME, relationships, Coverage::Complete)
    }

    fn find_paths(
        &mut self,
        _repo_path: &str,
        source: &NodeBinding,
        target: &NodeBinding,
        kind: RelationKind,
        max_hops: usize,
    ) -> ProviderResult<Vec<StructuralPath>> {
        if self.find(&source.qualified_name).is_none()
            || self.find(&target.qualified_name).is_none()
        {
            return ProviderResult::not_found(NAME, "un extremo no está en el fixture");
        }
        let edges = self.edges();
        let mut adjacency: HashMap<&str, Vec<(RelationKind, &FixtureNode)>> = HashMap::new();
        for (s, k, t) in &edges {
            adjacency
                .entry(s.qualified_name.as_str())
                .or_default()
                .push((*k, *t));
        }
        // BFS por caminos simples: saltos intermedios de delegación
        // (`calls`) y el último del tipo pedido. Devuelve los más cortos.
        let mut paths = Vec::new();
        let mut queue: VecDeque<StructuralPath> = VecDeque::new();
        queue.push_back(StructuralPath {
            nodes: vec![source.clone()],
            kinds: vec![],
        });
        while let Some(path) = queue.pop_front() {
            if !paths.is_empty() && path.hops() >= paths_min_hops(&paths) {
                continue;
            }
            if path.hops() >= max_hops {
                continue;
            }
            let tail = path.nodes.last().expect("un camino nunca está vacío");
            for (edge_kind, next) in adjacency
                .get(tail.qualified_name.as_str())
                .into_iter()
                .flatten()
            {
                if path
                    .nodes
                    .iter()
                    .any(|n| n.qualified_name == next.qualified_name)
                {
                    continue;
                }
                let mut extended = path.clone();
                extended.nodes.push(Self::binding(next));
                extended.kinds.push(*edge_kind);
                if next.qualified_name == target.qualified_name {
                    if *edge_kind == kind {
                        paths.push(extended);
                    }
                } else if *edge_kind == RelationKind::Calls {
                    queue.push_back(extended);
                }
            }
        }
        paths.truncate(5);
        ProviderResult::ok(NAME, paths, Coverage::Complete)
    }

    fn get_code_snippet(
        &mut self,
        _repo_path: &str,
        node: &NodeBinding,
    ) -> ProviderResult<CodeSnippet> {
        match self.find(&node.qualified_name) {
            Some(found) => match &found.source {
                Some(source) => ProviderResult::ok(
                    NAME,
                    CodeSnippet {
                        binding: Self::binding(found),
                        start_line: found.start_line,
                        end_line: found.end_line,
                        source: source.clone(),
                        truncated: false,
                    },
                    Coverage::Complete,
                ),
                None => ProviderResult::not_found(NAME, "el fixture no trae source para este nodo"),
            },
            None => ProviderResult::not_found(
                NAME,
                format!("'{}' no está en el fixture", node.qualified_name),
            ),
        }
    }

    fn get_file_outline(
        &mut self,
        _repo_path: &str,
        file_path: &str,
    ) -> ProviderResult<Vec<StructuralNode>> {
        let nodes = self
            .graph
            .nodes
            .iter()
            .filter(|node| node.file_path == file_path)
            .map(Self::structural)
            .collect();
        ProviderResult::ok(NAME, nodes, Coverage::Complete)
    }

    fn get_architecture(&mut self, _repo_path: &str) -> ProviderResult<ArchitectureSummary> {
        let mut by_file: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.graph.nodes {
            by_file
                .entry(node.file_path.as_str())
                .or_default()
                .push(node.qualified_name.as_str());
        }
        let mut clusters: Vec<ArchitectureCluster> = by_file
            .into_iter()
            .map(|(file, members)| ArchitectureCluster {
                label: file.to_string(),
                members: members.len() as u64,
                top_nodes: members.iter().take(5).map(|m| m.to_string()).collect(),
            })
            .collect();
        clusters.sort_by(|a, b| a.label.cmp(&b.label));
        ProviderResult::ok(
            NAME,
            ArchitectureSummary {
                total_nodes: Some(self.graph.nodes.len() as u64),
                total_edges: Some(self.graph.relationships.len() as u64),
                clusters,
            },
            Coverage::Complete,
        )
    }
}

fn paths_min_hops(paths: &[StructuralPath]) -> usize {
    paths
        .iter()
        .map(StructuralPath::hops)
        .min()
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
pub fn payments_fixture() -> FixtureProvider {
    let node = |file: &str, qn: &str, label: &str| FixtureNode {
        file_path: file.to_string(),
        qualified_name: qn.to_string(),
        name: None,
        label: label.to_string(),
        start_line: Some(1),
        end_line: Some(3),
        signature: None,
        is_test: false,
        source: Some(format!("fn {}() {{}}", qn.rsplit('.').next().unwrap())),
    };
    let rel = |source: &str, kind: &str, target: &str| FixtureRelationship {
        source: source.to_string(),
        kind: kind.to_string(),
        target: target.to_string(),
    };
    FixtureProvider::new(FixtureGraph {
        nodes: vec![
            node("src/payments.rs", "src.payments.create_link", "function"),
            node("src/payments.rs", "src.payments.sign_link", "function"),
            node(
                "src/config.rs",
                "src.config.EntityConfig.payment_expiration",
                "field",
            ),
            node("src/config.rs", "src.config.load", "function"),
            node("src/api.rs", "src.api.post_link", "function"),
            node(
                "tests/payments.rs",
                "tests.payments.links_expire",
                "function",
            ),
        ],
        relationships: vec![
            rel("src.api.post_link", "calls", "src.payments.create_link"),
            rel(
                "src.payments.create_link",
                "uses",
                "src.config.EntityConfig.payment_expiration",
            ),
            rel(
                "src.payments.create_link",
                "calls",
                "src.payments.sign_link",
            ),
            rel(
                "src.config.load",
                "writes",
                "src.config.EntityConfig.payment_expiration",
            ),
            rel(
                "tests.payments.links_expire",
                "tests",
                "src.payments.create_link",
            ),
            rel(
                "src.payments.create_link",
                "similar_to",
                "src.payments.sign_link",
            ),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(qn: &str, file: &str) -> NodeBinding {
        NodeBinding {
            file_path: file.to_string(),
            qualified_name: qn.to_string(),
            symbol_kind: None,
        }
    }

    #[test]
    fn resolves_symbols_within_the_declared_file_only() {
        let mut provider = payments_fixture();
        let found = provider.resolve_node("", "src/payments.rs", "create_link");
        assert_eq!(
            found.data.unwrap().binding.qualified_name,
            "src.payments.create_link"
        );
        let wrong_file = provider.resolve_node("", "src/api.rs", "create_link");
        assert!(wrong_file.data.is_none());
        assert!(
            matches!(wrong_file.coverage, Coverage::Unknown),
            "ausencia no es prueba de inexistencia"
        );
    }

    #[test]
    fn neighbors_respect_direction_kinds_and_exclude_inferred_edges() {
        let mut provider = payments_fixture();
        let seed = b("src.payments.create_link", "src/payments.rs");
        let both = provider
            .get_neighbors("", &seed, &NeighborQuery::default())
            .data
            .unwrap();
        let kinds: Vec<RelationKind> = both.relationships.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RelationKind::Calls));
        assert!(kinds.contains(&RelationKind::Uses));
        assert!(kinds.contains(&RelationKind::Tests));
        assert!(
            !kinds.contains(&RelationKind::SimilarTo),
            "una similitud no es estructura"
        );

        let inbound = provider
            .get_neighbors(
                "",
                &seed,
                &NeighborQuery {
                    direction: Direction::Inbound,
                    kinds: vec![],
                    limit: 1,
                },
            )
            .data
            .unwrap();
        assert_eq!(inbound.relationships.len(), 1);
        assert!(inbound.truncated, "el límite nunca oculta que había más");
    }

    #[test]
    fn bounded_paths_follow_delegation_then_the_requested_kind() {
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
        let source = b("src.payments.create_link", "src/payments.rs");
        let target = b(
            "src.config.EntityConfig.payment_expiration",
            "src/config.rs",
        );
        let paths = provider
            .find_paths("", &source, &target, RelationKind::Uses, 3)
            .data
            .unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(
            paths[0].kinds,
            vec![RelationKind::Calls, RelationKind::Uses]
        );

        let too_short = provider
            .find_paths("", &source, &target, RelationKind::Uses, 1)
            .data
            .unwrap();
        assert!(
            too_short.is_empty(),
            "max_hops es un techo, no una sugerencia"
        );
        let wrong_kind = provider
            .find_paths("", &source, &target, RelationKind::Writes, 3)
            .data
            .unwrap();
        assert!(
            wrong_kind.is_empty(),
            "un camino que termina en otro tipo no es compatible"
        );
    }
}
