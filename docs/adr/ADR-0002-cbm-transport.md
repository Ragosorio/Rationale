# ADR-0002: Codebase Memory transport

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

`Rationale_Arquitectura_Conceptual_v0.1.md §7.1` exige responder explícitamente si "MCP cliente-a-servidor es mejor que CLI subprocess para la primera vertical", con evidencia medida, no preferencia. `docs/research/codebase-memory/04-cli-contracts.md` y `11-performance-observations.md` ya produjeron mediciones formales de ambos transportes contra el mismo binario de Codebase Memory.

## Decision

El adaptador `CodeIntelligenceProvider` de Rationale (`Rationale_v0.5.md §21`) usará una **sesión MCP persistente de larga duración** (un proceso hijo de Codebase Memory iniciado una vez por vida del proceso de Rationale, no un subproceso CLI por operación) como transporte primario hacia Codebase Memory.

## Evidence

Mediciones formales (`04-cli-contracts.md`, `11-performance-observations.md`, cliente MCP stdio propio construido en B1.1):

| Transporte | Escenario | Latencia medida |
|---|---|---:|
| CLI | Sin daemon (proceso nuevo por invocación) | 6.811s – 6.873s |
| CLI | Con `daemon start` previo | 2.275s – 2.283s |
| MCP | `initialize` (handshake, una vez por proceso) | 6.791s – 6.859s |
| MCP | `tools/call` subsecuente, misma sesión | 15ms – 30ms |

**Hallazgo central:** el costo de ~6.8s es idéntico entre CLI fría y el handshake `initialize` de MCP — es el mismo costo de arranque del binario de Codebase Memory (carga de ~180 gramáticas tree-sitter, verificación de índices SQLite existentes), no una diferencia de transporte. La diferencia real aparece **después** del arranque: cada llamada MCP subsecuente en la misma sesión cuesta 15-30ms, mientras que cada invocación CLI repite un costo de al menos 2.2s (con daemon precalentado) porque no existe el concepto de "sesión" entre invocaciones de `cli <tool>`.

## Alternatives considered

- **CLI subprocess por operación**: descartado. Incluso con el daemon de CBM precalentado (`daemon start`), cada invocación de `cli <tool>` cuesta ~2.2s — dos órdenes de magnitud por encima del presupuesto de baseline de `Rationale_v0.5.md §20.5.2` (P95 ≤ 150ms) y del propio presupuesto intent-aware (~2s, `Arquitectura_Conceptual_v0.1.md §13.2`, ya al límite con una sola llamada).
- **Sesión MCP nueva por operación**: descartado por la misma razón — pagaría el handshake de 6.8s en cada operación, sin ninguna ventaja sobre la CLI fría.
- **Conectar al daemon persistente de Codebase Memory** (`06-daemon-and-watcher.md`): **no descartado, sino diferido**. Si Rationale pudiera conectar una sesión MCP nueva contra un daemon de CBM ya corriendo (en vez de arrancar su propio proceso hijo), el costo de 6.8s podría pagarse una sola vez por máquina en vez de una vez por proceso de Rationale. Esto no se probó en esta epic (research item explícito, no bloqueante para Fase D).

## Consequences

- El adaptador de Rationale debe gestionar el ciclo de vida de un proceso hijo de larga duración (spawn una vez, mantener vivo, terminar limpiamente al cerrar Rationale) — más complejidad de gestión de proceso que un subprocess CLI stateless, pero necesaria para cumplir el presupuesto de latencia.
- El primer `prepare_change`/`explain_target` de una sesión de Rationale pagará inevitablemente el costo de ~6.8s de arranque de Codebase Memory — debe comunicarse honestamente al usuario/agente como "warm-up", no ocultarse ni presentarse como parte del presupuesto de baseline.
- El **fast path baseline** (`Rationale_v0.5.md §20.5.1`) sigue sin poder depender de esta sesión MCP para su primera invocación en frío — debe depender exclusivamente de bindings ya resueltos localmente por Rationale, tal como ya concluía `12-integration-recommendation.md`. Esta decisión de transporte resuelve el modo intent-aware, no el baseline.

## Risks

- Un proceso hijo de larga duración puede quedar huérfano o zombi si Rationale termina de forma anormal — mitigación: manejo de señales explícito y verificación de salud (`health`) al inicio de cada sesión de Rationale.
- Si Codebase Memory actualiza su binario mientras la sesión MCP está viva, el adaptador podría quedar hablando con una versión obsoleta — mitigación: negociación de capacidades (`capabilities()`) al reconectar, no asumir que una sesión larga es siempre válida.

## Validation

Medición reproducible con el cliente MCP stdio de `docs/research/codebase-memory/11-performance-observations.md §Reproducir`.

**Este ADR está en estado `proposed`.** Pendiente: medir si conectar al daemon persistente de CBM evita pagar el handshake de 6.8s por proceso (research item de `12-integration-recommendation.md`) — de confirmarse, actualizaría este ADR con una ruta aún más rápida sin cambiar la decisión central (MCP sobre CLI).

## Revisit trigger

Reabrir si: (a) se confirma que conectar al daemon persistente de CBM evita el costo de 6.8s, cambiando el diseño de gestión de proceso del adaptador; (b) una versión futura de Codebase Memory reduce drásticamente el costo de `initialize`, lo cual podría hacer viable una sesión MCP por operación después de todo.
