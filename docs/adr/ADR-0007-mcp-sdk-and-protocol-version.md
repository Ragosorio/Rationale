# ADR-0007: MCP SDK and protocol version

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

Fase E5 convierte a Rationale en servidor MCP (`prepare_change`, `explain_target`, `health`), además de cliente (ya implementado en `src/providers/codebase_memory.rs` desde Fase D). Hace falta decidir si usar un SDK externo o continuar con el framing JSON-RPC 2.0 `Content-Length` escrito a mano que el cliente ya usa.

## Decision

1. **Continuar con la implementación manual del framing `Content-Length`** para el servidor MCP de Fase E5, reutilizando `read_mcp_message`/`write_mcp_message` de `src/providers/codebase_memory.rs` (generalizados a un módulo compartido).
2. **Protocolo `2024-11-05`** — la misma versión que el cliente ya declara y que Codebase Memory ya acepta en producción real.
3. **`rmcp` (SDK oficial) queda como candidato documentado para una migración futura**, no para Fase E5.

## Evidence

- **La implementación manual ya está probada dos veces contra un servidor MCP real** (Codebase Memory): una vez en el spike de lenguaje (`spikes/language/rust/src/main.rs`, con cliente Python de prueba) y otra vez en el cliente real de producción (`src/providers/codebase_memory.rs`, Fase D, con `initialize`/`tools/call` funcionando end-to-end, latencia de 15-30ms medida en sesión cálida). Cero incidencias de framing en ninguna de las dos pruebas.
- **`rmcp` (`github.com/modelcontextprotocol/rust-sdk`) es el SDK oficial** de la organización que define el protocolo — se verificó que compila limpiamente en este entorno (`cargo build` exitoso, 26.5s).
- **Pero `rmcp` arrastra una dependencia sustancial**: al compilarlo se observaron ~15 crates transitivos nuevos, incluyendo `tokio` (runtime async completo, feature `full`), `futures`, `async-trait`, `schemars`, `chrono`, `darling`, `tracing` — un cambio de naturaleza, no solo de tamaño, frente al enfoque síncrono actual de Rationale (hoy solo `serde`/`serde_json`/`serde_yaml`, sin runtime async en ningún módulo).
- **`rmcp` está en beta** (`3.0.0-beta.2`) — su superficie de API todavía puede cambiar antes de una release estable.

## Alternatives considered

- **Adoptar `rmcp` ahora**: descartado para Fase E5. Requeriría convertir `main.rs`, `providers/codebase_memory.rs` y todo el flujo síncrono actual a `async`/`await` con un runtime Tokio — un cambio arquitectónico transversal, no una decisión aislada de "qué SDK usar para el servidor", justo cuando Fase E ya introduce cambios grandes en el store canónico y la capa derivada. Acumular ambos riesgos en la misma fase viola el principio de cambios pequeños y verificables (`Proceso §6.4`).
- **Otro SDK de terceros** (`rust-mcp-sdk`, `mcp-attr`, `tower-mcp`): no evaluados con la misma profundidad — ninguno tiene el respaldo de ser el SDK de la organización que define el protocolo, y adoptar cualquiera de ellos tendría el mismo costo de conversión a async sin la ventaja de ser "oficial".
- **Seguir sin servidor MCP** (solo CLI): descartado — es exactamente el límite que este plan (Fase E) busca resolver; sin superficie MCP, ningún agente puede consumir Rationale.

## Consequences

- El servidor MCP de Fase E5 se implementa como una extensión del framing ya escrito: leer un mensaje `Content-Length`, despachar por `method`, escribir la respuesta — todo síncrono, sin runtime async.
- **Regla operativa crítica heredada de `Arquitectura §11.1`**: stdout queda reservado exclusivamente para el protocolo MCP; todo log va a stderr o archivo. Se verifica con un test explícito en Fase E6.
- Si Rationale necesita en el futuro atender múltiples sesiones/agentes concurrentes de forma no bloqueante, la migración a `rmcp` (o a un runtime async propio) se vuelve más atractiva — pero no es una necesidad actual, es una capacidad hipotética.

## Risks

- Mantener el framing a mano significa que Rationale es responsable de seguir cualquier evolución futura del protocolo MCP manualmente, sin las garantías de compatibilidad que un SDK oficial mantenido ofrecería. Mitigación: la superficie de Fase E5 es pequeña (3 herramientas, sin streaming, sin cancelación de requests en curso) — el riesgo de divergencia del protocolo es bajo para este alcance.
- Retrasar la adopción de `rmcp` significa que, si se decide migrar más adelante, el costo de conversión a async sigue pendiente — no desaparece, solo se pospone a un momento con menos cambios simultáneos.

## Validation

Framing verificado dos veces contra Codebase Memory real (Fase C y D). La extensión a modo servidor se valida en Fase E5/E6 con un cliente de prueba (mismo patrón Python usado en `docs/research/codebase-memory/11-performance-observations.md`) que llama `prepare_change`/`explain_target`/`health` contra el binario real de Rationale.

**Este ADR está en estado `proposed`**, pendiente de revisión cruzada y aprobación humana.

## Revisit trigger

Reabrir cuando: (a) `rmcp` alcance una release estable (no beta) Y exista una necesidad real de concurrencia/async no cubierta por el enfoque síncrono; o (b) el protocolo MCP introduzca una capacidad (streaming, cancelación) que el framing manual no pueda soportar razonablemente.
