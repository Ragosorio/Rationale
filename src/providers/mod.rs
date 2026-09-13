//! Structural Provider Adapter — Rationale_v0.5.md §21, Arquitectura §11.5.
//!
//! Frontera de responsabilidad (Arquitectura §7.2, confirmada con evidencia
//! en docs/research/codebase-memory/): nunca leer el almacenamiento interno
//! del proveedor, nunca inferir capacidades desde un string de versión,
//! nunca tratar un resultado vacío como "la relación no existe".
//!
//! vNext: Rationale es dueño de su modelo estructural normalizado
//! (`StructuralNode`, `StructuralRelationship`, `StructuralPath`,
//! `StructuralSubgraph`...). Cada proveedor traduce su grafo a estos tipos;
//! nada específico de Codebase Memory — ids internos, prefijos de proyecto,
//! nombres de etiquetas — cruza hacia el núcleo. Mañana un proveedor nativo
//! produce exactamente las mismas estructuras.

// Temporal: el Context Compiler (fase 5, commit siguiente) consume el modelo
// completo; hasta entonces parte del contrato no tiene caller.
#![allow(dead_code)]

pub mod codebase_memory;
pub mod fixture;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub enum Coverage {
    Complete,
    // Reservado: docs/research/codebase-memory/04-cli-contracts.md (B1.3)
    // encontró que versiones de Codebase Memory posteriores a 0.8.1 exponen
    // parse_partial/skipped/not_indexed vía MCP real. Esta vertical slice
    // usa el binario release instalado (0.8.1), que no los produce todavía
    // — la variante queda declarada para cuando el adaptador negocie esa
    // capability (Fase E).
    #[allow(dead_code)]
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ProviderStatus {
    Successful,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderResult<T> {
    pub data: Option<T>,
    pub provider_name: String,
    pub status: ProviderStatus,
    pub coverage: Coverage,
    pub warnings: Vec<String>,
}

impl<T> ProviderResult<T> {
    pub fn ok(provider_name: &str, data: T, coverage: Coverage) -> Self {
        ProviderResult {
            data: Some(data),
            provider_name: provider_name.to_string(),
            status: ProviderStatus::Successful,
            coverage,
            warnings: vec![],
        }
    }

    /// Respuesta sin datos que NO afirma ausencia: el proveedor respondió,
    /// pero lo pedido no está dentro de su cobertura conocida.
    pub fn not_found(provider_name: &str, warning: impl Into<String>) -> Self {
        ProviderResult {
            data: None,
            provider_name: provider_name.to_string(),
            status: ProviderStatus::Successful,
            coverage: Coverage::Unknown,
            warnings: vec![warning.into()],
        }
    }

    pub fn degraded(provider_name: &str, warning: impl Into<String>) -> Self {
        ProviderResult {
            data: None,
            provider_name: provider_name.to_string(),
            status: ProviderStatus::Degraded,
            coverage: Coverage::Unknown,
            warnings: vec![warning.into()],
        }
    }

    pub fn unavailable(provider_name: &str, warning: impl Into<String>) -> Self {
        ProviderResult {
            data: None,
            provider_name: provider_name.to_string(),
            status: ProviderStatus::Unavailable,
            coverage: Coverage::Unknown,
            warnings: vec![warning.into()],
        }
    }

    pub fn unsupported(provider_name: &str, capability: &str) -> Self {
        Self::degraded(
            provider_name,
            format!("el proveedor no soporta '{capability}'"),
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedTarget {
    pub qualified_name: String,
    pub file_path: String,
}

// ---------------------------------------------------------------------------
// Modelo estructural normalizado de Rationale
// ---------------------------------------------------------------------------

/// Identidad semántica durable de un nodo: archivo + nombre calificado
/// portable (+ tipo de símbolo opcional). Nunca un id interno del
/// proveedor, que puede cambiar entre reindexaciones, ni un nombre con el
/// prefijo del proyecto del proveedor, que codifica la ruta local de quien
/// indexó (`Users-roor.osorio-Desktop-Rationale.src...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeBinding {
    pub file_path: String,
    pub qualified_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<String>,
}

fn short_hash(parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(parts.join("\0"));
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

impl NodeBinding {
    /// Referencia práctica para API/UI: hash determinista de la identidad
    /// normalizada. No es la verdad semántica — la identidad es el propio
    /// `NodeBinding` — y no incluye `symbol_kind` para que un proveedor que
    /// no lo reporte produzca la misma clave.
    pub fn key(&self, project_id: &str) -> String {
        format!(
            "n_{}",
            short_hash(&[project_id, &self.file_path, &self.qualified_name])
        )
    }

    /// Nombre corto legible (último segmento del nombre calificado).
    pub fn short_name(&self) -> &str {
        self.qualified_name
            .rsplit(['.', ':'])
            .next()
            .unwrap_or(&self.qualified_name)
    }
}

/// Tipo de relación normalizado. Los inferidos por similitud
/// (`SemanticallyRelated`, `SimilarTo`) nunca son hechos estructurales: se
/// excluyen del contexto por defecto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Calls,
    Uses,
    Writes,
    Imports,
    Defines,
    Implements,
    Tests,
    Configures,
    DependsOn,
    HttpCalls,
    Contains,
    Decorates,
    SemanticallyRelated,
    SimilarTo,
    Other,
}

impl RelationKind {
    pub const STRUCTURAL: [RelationKind; 10] = [
        RelationKind::Calls,
        RelationKind::Uses,
        RelationKind::Writes,
        RelationKind::Imports,
        RelationKind::Defines,
        RelationKind::Implements,
        RelationKind::Tests,
        RelationKind::Configures,
        RelationKind::DependsOn,
        RelationKind::HttpCalls,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RelationKind::Calls => "calls",
            RelationKind::Uses => "uses",
            RelationKind::Writes => "writes",
            RelationKind::Imports => "imports",
            RelationKind::Defines => "defines",
            RelationKind::Implements => "implements",
            RelationKind::Tests => "tests",
            RelationKind::Configures => "configures",
            RelationKind::DependsOn => "depends_on",
            RelationKind::HttpCalls => "http_calls",
            RelationKind::Contains => "contains",
            RelationKind::Decorates => "decorates",
            RelationKind::SemanticallyRelated => "semantically_related",
            RelationKind::SimilarTo => "similar_to",
            RelationKind::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<RelationKind> {
        let normalized = value.trim().to_ascii_lowercase();
        [
            RelationKind::Calls,
            RelationKind::Uses,
            RelationKind::Writes,
            RelationKind::Imports,
            RelationKind::Defines,
            RelationKind::Implements,
            RelationKind::Tests,
            RelationKind::Configures,
            RelationKind::DependsOn,
            RelationKind::HttpCalls,
            RelationKind::Contains,
            RelationKind::Decorates,
            RelationKind::SemanticallyRelated,
            RelationKind::SimilarTo,
            RelationKind::Other,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == normalized)
    }

    pub fn is_inferred(self) -> bool {
        matches!(
            self,
            RelationKind::SemanticallyRelated | RelationKind::SimilarTo
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralNode {
    pub key: String,
    pub binding: NodeBinding,
    pub name: String,
    /// Etiqueta normalizada en minúsculas (`function`, `method`, `class`...).
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(default)]
    pub is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralRelationship {
    pub key: String,
    pub source: NodeBinding,
    pub kind: RelationKind,
    pub target: NodeBinding,
    /// Etiqueta cruda del proveedor, solo para diagnóstico.
    pub provider_kind: String,
}

impl StructuralRelationship {
    pub fn relationship_key(
        project_id: &str,
        source: &NodeBinding,
        kind: RelationKind,
        target: &NodeBinding,
    ) -> String {
        format!(
            "e_{}",
            short_hash(&[
                &source.key(project_id),
                kind.as_str(),
                &target.key(project_id)
            ])
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralPath {
    /// Nodos en orden, incluidos origen y destino.
    pub nodes: Vec<NodeBinding>,
    /// Relación de cada salto (`nodes.len() - 1` elementos).
    pub kinds: Vec<RelationKind>,
}

impl StructuralPath {
    pub fn hops(&self) -> usize {
        self.kinds.len()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralSubgraph {
    pub nodes: Vec<StructuralNode>,
    pub relationships: Vec<StructuralRelationship>,
    /// `true` cuando el proveedor tenía más de lo que el límite permitió
    /// devolver — nunca se oculta que el vecindario está incompleto.
    pub truncated: bool,
}

impl StructuralSubgraph {
    pub fn merge(&mut self, other: StructuralSubgraph) {
        for node in other.nodes {
            if !self.nodes.iter().any(|n| n.key == node.key) {
                self.nodes.push(node);
            }
        }
        for relationship in other.relationships {
            if !self.relationships.iter().any(|r| r.key == relationship.key) {
                self.relationships.push(relationship);
            }
        }
        self.truncated |= other.truncated;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeSnippet {
    pub binding: NodeBinding,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub source: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexStatus {
    pub project: Option<String>,
    /// `ready`, `indexing`, `missing` o `unknown` — nunca inventado.
    pub state: String,
    pub nodes: Option<u64>,
    pub edges: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchitectureCluster {
    pub label: String,
    pub members: u64,
    pub top_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchitectureSummary {
    pub total_nodes: Option<u64>,
    pub total_edges: Option<u64>,
    pub clusters: Vec<ArchitectureCluster>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Outbound,
    Inbound,
    Both,
}

#[derive(Debug, Clone)]
pub struct NeighborQuery {
    pub direction: Direction,
    /// Vacío = `RelationKind::STRUCTURAL`.
    pub kinds: Vec<RelationKind>,
    pub limit: usize,
}

impl Default for NeighborQuery {
    fn default() -> Self {
        NeighborQuery {
            direction: Direction::Both,
            kinds: vec![],
            limit: 40,
        }
    }
}

impl NeighborQuery {
    pub fn accepts(&self, kind: RelationKind) -> bool {
        if self.kinds.is_empty() {
            RelationKind::STRUCTURAL.contains(&kind)
        } else {
            self.kinds.contains(&kind)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCapabilities {
    pub name: String,
    pub neighbors: bool,
    pub bounded_paths: bool,
    pub max_path_hops: usize,
    pub code_snippets: bool,
    pub file_outline: bool,
    pub architecture: bool,
    pub changed_nodes: bool,
}

/// Contrato estructural de Rationale (v0.5 §21, vNext). No es un espejo de
/// las herramientas de ningún proveedor: cada método expresa una pregunta
/// que el Context Compiler necesita, y cada implementación la traduce.
///
/// Los métodos con implementación por defecto responden `Degraded` +
/// "no soporta": un proveedor parcial nunca finge una respuesta vacía.
pub trait CodeIntelligenceProvider {
    fn capabilities(&self) -> ProviderCapabilities;
    fn health(&mut self, repo_path: &str) -> ProviderResult<()>;

    fn index_status(&mut self, _repo_path: &str) -> ProviderResult<IndexStatus> {
        ProviderResult::unsupported(&self.capabilities().name, "index_status")
    }

    fn resolve_target(
        &mut self,
        repo_path: &str,
        file_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<ResolvedTarget>;

    /// El nodo real detrás de `path::symbol`, con identidad normalizada.
    fn resolve_node(
        &mut self,
        _repo_path: &str,
        _file_path: &str,
        _symbol_name: &str,
    ) -> ProviderResult<StructuralNode> {
        ProviderResult::unsupported(&self.capabilities().name, "resolve_node")
    }

    fn get_node(
        &mut self,
        _repo_path: &str,
        _node: &NodeBinding,
    ) -> ProviderResult<StructuralNode> {
        ProviderResult::unsupported(&self.capabilities().name, "get_node")
    }

    fn get_neighbors(
        &mut self,
        _repo_path: &str,
        _node: &NodeBinding,
        _query: &NeighborQuery,
    ) -> ProviderResult<StructuralSubgraph> {
        ProviderResult::unsupported(&self.capabilities().name, "get_neighbors")
    }

    /// Unión de vecindarios de varias semillas, deduplicada.
    fn get_subgraph(
        &mut self,
        repo_path: &str,
        seeds: &[NodeBinding],
        query: &NeighborQuery,
    ) -> ProviderResult<StructuralSubgraph> {
        let name = self.capabilities().name;
        let mut merged = StructuralSubgraph::default();
        let mut warnings = Vec::new();
        let mut any = false;
        for seed in seeds {
            let result = self.get_neighbors(repo_path, seed, query);
            warnings.extend(result.warnings);
            if let Some(subgraph) = result.data {
                any = true;
                merged.merge(subgraph);
            }
        }
        if !any && !seeds.is_empty() {
            let mut result = ProviderResult::degraded(&name, "ningún vecindario disponible");
            result.warnings.extend(warnings);
            return result;
        }
        let mut result = ProviderResult::ok(&name, merged, Coverage::Complete);
        result.warnings = warnings;
        result
    }

    /// Relaciones directas entre dos nodos, en cualquier dirección.
    fn get_relationships(
        &mut self,
        _repo_path: &str,
        _source: &NodeBinding,
        _target: &NodeBinding,
    ) -> ProviderResult<Vec<StructuralRelationship>> {
        ProviderResult::unsupported(&self.capabilities().name, "get_relationships")
    }

    /// Caminos acotados de `source` a `target` compatibles con `kind`:
    /// saltos intermedios de delegación (`calls`) y un último salto del
    /// mismo tipo. Nunca alcanzabilidad arbitraria: `max_hops` es un techo.
    fn find_paths(
        &mut self,
        _repo_path: &str,
        _source: &NodeBinding,
        _target: &NodeBinding,
        _kind: RelationKind,
        _max_hops: usize,
    ) -> ProviderResult<Vec<StructuralPath>> {
        ProviderResult::unsupported(&self.capabilities().name, "find_paths")
    }

    /// Quién depende de `node` (entrante), acotado por profundidad.
    fn impact(
        &mut self,
        repo_path: &str,
        node: &NodeBinding,
        _depth: usize,
    ) -> ProviderResult<StructuralSubgraph> {
        self.get_neighbors(
            repo_path,
            node,
            &NeighborQuery {
                direction: Direction::Inbound,
                kinds: vec![],
                limit: 40,
            },
        )
    }

    fn get_code_snippet(
        &mut self,
        _repo_path: &str,
        _node: &NodeBinding,
    ) -> ProviderResult<CodeSnippet> {
        ProviderResult::unsupported(&self.capabilities().name, "get_code_snippet")
    }

    fn get_file_outline(
        &mut self,
        _repo_path: &str,
        _file_path: &str,
    ) -> ProviderResult<Vec<StructuralNode>> {
        ProviderResult::unsupported(&self.capabilities().name, "get_file_outline")
    }

    fn get_architecture(&mut self, _repo_path: &str) -> ProviderResult<ArchitectureSummary> {
        ProviderResult::unsupported(&self.capabilities().name, "get_architecture")
    }

    fn changed_nodes(
        &mut self,
        _repo_path: &str,
        _since: &str,
    ) -> ProviderResult<Vec<StructuralNode>> {
        ProviderResult::unsupported(&self.capabilities().name, "changed_nodes")
    }
}

/// Etiquetas de proveedor que casi nunca aportan contexto de cambio: se
/// excluyen de los vecindarios por defecto. Confirmado en CBM 0.8.1: aristas
/// `USAGE` hacia secciones Markdown y hacia `site/astro.config.mjs` desde
/// funciones Rust que no tienen ninguna relación con ellos.
pub const LOW_SIGNAL_LABELS: [&str; 6] = [
    "section",
    "decorator",
    "folder",
    "project",
    "package",
    "envvar",
];

/// Envoltorio sobre una sesión (spawneada o fallida) del proveedor
/// estructural. Existe para que `pipeline::prepare`/`pipeline::health` no
/// decidan CUÁNDO se spawnea el proceso — solo lo usan. Esto es lo que
/// permite que el servidor MCP (Fase E5) mantenga una sesión persistente
/// durante toda su vida en vez de spawnear un proceso nuevo por llamada,
/// cerrando el gap real que la revisión adversarial encontró en ADR-0002
/// (`docs/work-items/adversarial-review-adr-0001-0002-0006.md`): la CLI de
/// un solo disparo nunca puede amortizar, un servidor sí.
pub enum ProviderHandle {
    Live(codebase_memory::CodebaseMemoryClient),
    /// Cualquier otra implementación del contrato: el proveedor de fixtures
    /// (tests deterministas, `RATIONALE_PROVIDER=fixture:<ruta>`) o uno
    /// nativo futuro.
    Custom(Box<dyn CodeIntelligenceProvider + Send>),
    /// Guarda el mensaje de error del spawn fallido — nunca se reintenta
    /// automáticamente dentro de la misma sesión (fail open, Arquitectura
    /// §13.5); una sesión nueva del servidor sí reintentaría.
    Unavailable(String),
}

impl ProviderHandle {
    /// El proveedor vivo como trait object, o `None` si la sesión no
    /// existe. Los callers dejan de hacer `match` sobre la implementación
    /// concreta: el mismo pipeline sirve para Codebase Memory, un proveedor
    /// de fixtures o uno nativo futuro.
    pub fn as_provider(&mut self) -> Option<&mut dyn CodeIntelligenceProvider> {
        match self {
            ProviderHandle::Live(client) => Some(client),
            ProviderHandle::Custom(provider) => Some(provider.as_mut()),
            ProviderHandle::Unavailable(_) => None,
        }
    }

    pub fn unavailable_reason(&self) -> Option<&str> {
        match self {
            ProviderHandle::Unavailable(reason) => Some(reason),
            _ => None,
        }
    }

    /// Spawnea la sesión una sola vez. La CLI la llama por invocación (igual
    /// costo que antes); el servidor MCP la llama una sola vez al arrancar.
    ///
    /// `RATIONALE_PROVIDER` selecciona el proveedor explícitamente:
    /// - ausente: Codebase Memory desde `PATH`;
    /// - `none`: sin proveedor — cobertura `unknown`, el mismo camino que un
    ///   Codebase Memory ausente (CI, tests que no deben indexar directorios
    ///   temporales en la instalación real del usuario);
    /// - `fixture:<ruta.json>`: grafo estático determinista para tests y
    ///   demos, siempre etiquetado como `fixture` en la cobertura.
    pub fn spawn() -> Self {
        match std::env::var("RATIONALE_PROVIDER").as_deref() {
            Ok("none") => {
                return ProviderHandle::Unavailable(
                    "proveedor estructural desactivado por RATIONALE_PROVIDER=none".to_string(),
                );
            }
            Ok(value) if value.starts_with("fixture:") => {
                let path = &value["fixture:".len()..];
                return match fixture::FixtureProvider::load(std::path::Path::new(path)) {
                    Ok(provider) => ProviderHandle::Custom(Box::new(provider)),
                    Err(error) => ProviderHandle::Unavailable(format!(
                        "no se pudo cargar el fixture estructural '{path}': {error}"
                    )),
                };
            }
            _ => {}
        }
        match codebase_memory::CodebaseMemoryClient::spawn() {
            Ok(client) => ProviderHandle::Live(client),
            Err(e) => ProviderHandle::Unavailable(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(file: &str, qn: &str) -> NodeBinding {
        NodeBinding {
            file_path: file.to_string(),
            qualified_name: qn.to_string(),
            symbol_kind: Some("function".to_string()),
        }
    }

    #[test]
    fn node_binding_serializes_without_provider_ids() {
        let node = binding("src/payments.rs", "src.payments.create_link");
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "file_path": "src/payments.rs",
                "qualified_name": "src.payments.create_link",
                "symbol_kind": "function"
            })
        );
        let back: NodeBinding = serde_json::from_value(json).unwrap();
        assert_eq!(back, node);
    }

    #[test]
    fn node_keys_are_deterministic_and_ignore_symbol_kind() {
        let with_kind = binding("src/a.rs", "src.a.f");
        let mut without_kind = with_kind.clone();
        without_kind.symbol_kind = None;
        assert_eq!(with_kind.key("p"), without_kind.key("p"));
        assert_ne!(with_kind.key("p"), with_kind.key("other-project"));
        assert!(with_kind.key("p").starts_with("n_"));
        assert_eq!(with_kind.short_name(), "f");
    }

    #[test]
    fn relation_kinds_roundtrip_and_inferred_kinds_are_not_structural() {
        for kind in RelationKind::STRUCTURAL {
            assert_eq!(RelationKind::parse(kind.as_str()), Some(kind));
            assert!(!kind.is_inferred());
        }
        assert!(RelationKind::SimilarTo.is_inferred());
        assert!(!NeighborQuery::default().accepts(RelationKind::SemanticallyRelated));
        assert!(NeighborQuery::default().accepts(RelationKind::Calls));
    }

    #[test]
    fn subgraph_merge_deduplicates_by_key() {
        let a = binding("src/a.rs", "src.a.f");
        let b = binding("src/b.rs", "src.b.g");
        let node = |binding: &NodeBinding| StructuralNode {
            key: binding.key("p"),
            binding: binding.clone(),
            name: binding.short_name().to_string(),
            label: "function".to_string(),
            start_line: None,
            end_line: None,
            signature: None,
            is_test: false,
        };
        let edge = StructuralRelationship {
            key: StructuralRelationship::relationship_key("p", &a, RelationKind::Calls, &b),
            source: a.clone(),
            kind: RelationKind::Calls,
            target: b.clone(),
            provider_kind: "CALLS".to_string(),
        };
        let mut left = StructuralSubgraph {
            nodes: vec![node(&a)],
            relationships: vec![edge.clone()],
            truncated: false,
        };
        left.merge(StructuralSubgraph {
            nodes: vec![node(&a), node(&b)],
            relationships: vec![edge],
            truncated: true,
        });
        assert_eq!(left.nodes.len(), 2);
        assert_eq!(left.relationships.len(), 1);
        assert!(left.truncated);
    }
}
