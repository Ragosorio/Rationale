//! Structural Provider Adapter — Rationale_v0.5.md §21, Arquitectura §11.5.
//!
//! Frontera de responsabilidad (Arquitectura §7.2, confirmada con evidencia
//! en docs/research/codebase-memory/): nunca leer el almacenamiento interno
//! del proveedor, nunca inferir capacidades desde un string de versión,
//! nunca tratar un resultado vacío como "la relación no existe".

pub mod codebase_memory;

use serde::Serialize;

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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedTarget {
    pub qualified_name: String,
    pub file_path: String,
}

/// Subconjunto del `CodeIntelligenceProvider` de Rationale_v0.5.md §21
/// necesario para la vertical slice de Fase D. `get_relationships`,
/// `get_impact`, `changed_targets`, `classify_change` y
/// `find_lineage_candidates` pertenecen a Fase E/F — no se implementan
/// todavía (Arquitectura §23: "no debe incluir toda la visión").
pub trait CodeIntelligenceProvider {
    fn health(&mut self, repo_path: &str) -> ProviderResult<()>;
    fn resolve_target(
        &mut self,
        repo_path: &str,
        symbol_name: &str,
    ) -> ProviderResult<ResolvedTarget>;
}
