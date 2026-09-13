---
lang: es
slug: architecture
title: Arquitectura factual
description: Cómo encajan Git, Codebase Memory, el canon, el compilador de contexto, MCP, la actividad local y el Control Room.
section: Proyecto
order: 12
---

## El flujo

```text
Codebase Memory (dónde / cómo) ─┐
Git (qué / cuándo) ─────────────┼─> compilador de contexto ─> packet ─> agente
canon .rationale (por qué) ─────┘            │
                                   snapshot de operación
cambio del agente ─> finalize_change ─> gate de captura ─> Records canónicos
                                              │
                        conflicto con un Record fijado ─> decisión humana

sesiones ─> .rationale-local/activity ─┐
operaciones ───────────────────────────┴─> rationale ui (solo lectura)
```

## Componentes

| Módulo | Responsabilidad |
| --- | --- |
| `pipeline` | Orquesta `prepare`, `explain`, `health` y `finalize`. |
| `providers` | Un modelo estructural normalizado; el adaptador de Codebase Memory solo usa sus herramientas MCP públicas. Un proveedor de fixtures sostiene los tests deterministas. |
| `context` + `retrieval` | Seleccionan la vecindad estructural, derivan el estado de las relaciones y compilan el packet bajo un techo de tokens. |
| `canon` + `storage` | El gate de captura, procedencia, autoridad, reemplazo, conflictos, migración y escrituras YAML atómicas bajo un lock. |
| `relationships` | Explicaciones de relaciones y su estado derivado. |
| `operations` + `activity` | Snapshots de operación y el stream de actividad por sesión (ADR-0017). |
| `mcp::server` | JSON-RPC delimitado por líneas sobre stdio: cinco herramientas, seis prompts. |
| `ui` | Un servidor HTTP localhost sin dependencias externas, con vistas REST y Server-Sent Events. |
| `agents` | Registro de agentes convergente y reversible, e instrucciones del proyecto. |
| `doctor` | Chequeos de integridad del canon y reparación guiada. |

## Canónico frente a derivado

El YAML en `.rationale/` es la única autoridad. La cache SQLite de búsqueda,
los snapshots de operación y la actividad se pueden reconstruir o borrar.
Ningún componente lee el almacenamiento privado de un proveedor
(`constraint.no-provider-internal-access`).

## Fronteras que se sostienen

- El proveedor informa, el canon decide. La ausencia de una arista no es prueba.
- Los agentes escriben memoria, las personas tienen la autoridad: fijar,
  desfijar y reemplazar una regla fijada son actos humanos con autoridad
  declarada.
- El Control Room observa y nunca escribe; el servidor MCP es la única frontera
  con los agentes.
- Las trazas locales se quedan locales y mínimas: identificadores y una
  intención de una línea.

## Dónde leer más

El repositorio conserva la arquitectura conceptual, un mapa factual del código
en `docs/architecture/` y los ADRs en `docs/adr/` con su estado de aprobación.
