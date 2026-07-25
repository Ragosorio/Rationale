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
| CBM-001 | Clone and lock revision | ✅ hecho | `00-source-lock.md` + `source-lock.yaml`. Halló discrepancia de versión: binario instalado 0.8.1 vs clon 338 commits por delante de v0.9.0 |
| CBM-002 | Build on MacBook Air M4 | ✅ hecho | `01-build-and-test.md`. Build exitoso, 2m49s, binario 296MB, reporta versión `dev` |
| CBM-003 | Run tests | ✅ hecho (parcial) | `01-build-and-test.md`. `test-foundation` falla en link (símbolos `_suite_*`); suite completa no ejecutada por costo — documentado como limitación conocida, no responsabilidad de Rationale |
| CBM-004 | Index itself | ✅ hecho | `02-module-map.md`. 20.747 nodos / 77.956 edges |
| CBM-005 | Map modules | ✅ hecho | `02-module-map.md`. Hallazgo: `packages` en clusters no distingue módulos reales |
| CBM-006 | Inspect MCP | ✅ hecho | `03-mcp-contracts.md`. 14 herramientas documentadas; ADR de CBM es documento único, no log de decisiones |
| CBM-007 | Inspect CLI | ✅ hecho | `04-cli-contracts.md`. Latencia medida: 6.8s fría, 2.2s con daemon caliente |
| CBM-008 | Inspect revision and coverage | ✅ hecho | `05-revision-and-coverage.md`. **Hallazgo crítico:** `detect_changes` devolvió vacío ante 200 archivos realmente modificados |
| CBM-009 | Inspect daemon and watcher | ✅ hecho | `06-daemon-and-watcher.md`. `hook_augment.c` confirma el patrón no-bloqueante de `v0.5 §20.7` con evidencia de código |
| CBM-010 | Inspect workspace support | ✅ hecho | `08-workspaces-and-monorepos.md`. **Hallazgo crítico:** cero relaciones `IMPORTS` cruzan paquetes en el Monorepo real (8 paquetes npm) |
| CBM-011 | Measure CLI vs MCP | ✅ hecho (parcial) | `11-performance-observations.md`. CLI medido formalmente; MCP solo cualitativo — medición formal queda como research item antes de ADR-0002 |
| CBM-012 | Recommend adapter boundary | ✅ hecho | `12-integration-recommendation.md`. Síntesis y recomendación de frontera de adaptador |

Documentos adicionales completados fuera de la lista original de subtareas: `07-storage-and-cache.md`, `09-installation-and-agents.md`, `10-failure-modes.md` (consolidado transversal).

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

**Estado: completo.** Los 13 documentos (`00`–`12`) más `source-lock.yaml` están escritos con evidencia reproducible. Dos hallazgos críticos con impacto arquitectónico directo:

1. `detect_changes` no detectó 200 archivos realmente modificados (CBM-008) → el Revision Coordinator de Rationale debe derivar su propia verdad de revisión desde Git, nunca confiar en la señal del proveedor.
2. Cero relaciones `IMPORTS` cruzan paquetes en un monorepo real de 8 paquetes npm (CBM-010) → la recuperación cross-workspace de Rationale no puede depender únicamente de edges del proveedor; necesita bindings manuales/contractuales como fallback.

Research items abiertos para antes de ADR-0002: medición formal de latencia MCP, lectura de `pass_pkgmap.c`, y confirmar si el hallazgo de cobertura vía CLI en HEAD también aparece vía MCP. Ver `12-integration-recommendation.md §Próximos research items`.
