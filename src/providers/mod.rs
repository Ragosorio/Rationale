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
    /// Guarda el mensaje de error del spawn fallido — nunca se reintenta
    /// automáticamente dentro de la misma sesión (fail open, Arquitectura
    /// §13.5); una sesión nueva del servidor sí reintentaría.
    Unavailable(String),
}

impl ProviderHandle {
    /// Spawnea la sesión una sola vez. La CLI la llama por invocación (igual
    /// costo que antes); el servidor MCP la llama una sola vez al arrancar.
    pub fn spawn() -> Self {
        match codebase_memory::CodebaseMemoryClient::spawn() {
            Ok(client) => ProviderHandle::Live(client),
            Err(e) => ProviderHandle::Unavailable(e.to_string()),
        }
    }
}
