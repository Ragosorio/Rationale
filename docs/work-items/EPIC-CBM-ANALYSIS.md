# EPIC-CBM-ANALYSIS

## Problema

Rationale integrará Codebase Memory como su primer proveedor estructural, pero la arquitectura no puede fijar el adaptador, el transporte, ni el modelo de revisión/cobertura sin evidencia reproducible del comportamiento real de Codebase Memory (`Rationale_Arquitectura_Conceptual_v0.1.md §0, §6, §7`).

## Objetivo

Producir los 13 documentos de investigación de `docs/research/codebase-memory/` con evidencia reproducible, y cerrar con una recomendación de frontera de adaptador (`12-integration-recommendation.md`).

## Non-goals

- No se implementa el adaptador todavía.
- No se elige lenguaje aquí (eso es el spike de `docs/research/language/`).
- No se modifica el clon de Codebase Memory.

## Base revision

Codebase Memory: `97ce23f9827177fff3858831156e9795c6832b18` (`DeusData/codebase-memory-mcp`, 2026-07-23). Ver `docs/research/codebase-memory/00-source-lock.md`.

## Subtareas (`Rationale_Proceso_Construccion_Agentes_v0.1.md §8`)

| Tarea | Descripción | Estado | Notas |
|---|---|---|---|
| CBM-001 | Clone and lock revision | Clon ✅ · lock doc pendiente | Clon ya existe en `~/Desktop/codebase-memory-mcp`; falta `00-source-lock.md` |
| CBM-002 | Build on MacBook Air M4 | Pendiente | Fuente completa en el clon; build no ejecutado (no hay `build/`, `bin/`, `out/`). El binario en `~/.local/bin/` viene de `install.sh`, no de build local |
| CBM-003 | Run tests | Pendiente | Depende de CBM-002 |
| CBM-004 | Index itself | ✅ hecho | 20.747 nodos / 77.956 edges, vía MCP en esta sesión |
| CBM-005 | Map modules | Pendiente | Usar `get_architecture` sobre el propio índice + lectura de código |
| CBM-006 | Inspect MCP | Pendiente | Contratos de herramientas expuestas |
| CBM-007 | Inspect CLI | Pendiente | Comandos, salida estructurada |
| CBM-008 | Inspect revision and coverage | **Pendiente — prioridad máxima** | Ver pregunta gobernante abajo |
| CBM-009 | Inspect daemon and watcher | Pendiente | |
| CBM-010 | Inspect workspace support | Pendiente | Fixture real disponible: `~/Desktop/Monorepo` ya indexado (12.367 nodos / 23.331 edges) |
| CBM-011 | Measure CLI vs MCP | Pendiente | Decide transporte del adaptador (ADR-0002) |
| CBM-012 | Recommend adapter boundary | Pendiente | Entregable final; depende de CBM-001 a CBM-011 |

## La pregunta que gobierna toda la arquitectura (CBM-008)

> ¿Codebase Memory expone la revisión que indexó, distingue working tree de HEAD, y reporta cobertura parcial de forma verificable?

Toda la garantía de consistencia por revisión de Rationale depende de esto (`Rationale_v0.5.md §4.8, §12.5, §20.3`; Subject `architecture.revision-consistency` en `.rationale/subjects/`). Si la respuesta es negativa o parcial, no se disimula: se documenta como hallazgo, se reproduce, y se convierte en ADR — puede obligar a derivar la revisión desde Git en vez de confiar en el proveedor.

## Riesgos

- El proveedor puede tener relaciones falsas, trazas vacías o resultados silenciosamente vacíos (`Rationale_v0.5.md §20.6`) — no asumir que ausencia de relación significa inexistencia.
- Confundir "no encontré una relación" con "la relación no existe" (`Rationale_Arquitectura_Conceptual_v0.1.md §4.6`).

## Plan

Ver `Rationale_Arquitectura_Conceptual_v0.1.md §7` para la estructura de los 13 documentos y sus 6 secciones obligatorias (`Observed / Claimed / Verified / Unknown / Risk / Decision impact`).

## Tests

No aplica código todavía. "Test" en esta epic significa: cada comando de investigación debe ser reproducible por otro agente con los mismos resultados declarados.

## Docs

Los 13 archivos en `docs/research/codebase-memory/00-source-lock.md` … `12-integration-recommendation.md`.

## Criterio de éxito

Los 13 documentos existen, con comandos reproducibles, secciones completas, y CBM-008 respondido explícitamente con evidencia — incluso si la respuesta es "no lo expone".
