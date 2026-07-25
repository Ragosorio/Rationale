# ADR-0004: Derived database

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

`Rationale_v0.5.md §26.2` especifica la capa derivada como "SQLite. Regenerable. Optimizado. No necesariamente versionado. Invalidado por revisión, schema o generación del proveedor." `Rationale_Arquitectura_Conceptual_v0.1.md §11.7` asigna a esta capa: indexar Records, FTS, aliases, scope paths, binding resolutions, candidate retrieval, deduplicación, cache de assessments, invalidación por revisión.

Hoy (Fase D completa) Rationale no tiene capa derivada — lee YAML directamente en cada invocación (`src/storage.rs`). Esto es correcto para una vertical slice de una sola constraint, pero Fase E2/E3 introduce `Assessment` (que debe recalcularse, no reescribirse sobre el Record original) y un Context Compiler con budget y ranking sobre potencialmente muchos Records — leer y parsear YAML completo en cada consulta deja de ser sostenible.

## Decision

**SQLite (vía `rusqlite`, feature `bundled`)** es el motor de la capa derivada: assessments calculados, índice FTS5 sobre statements/títulos, y resoluciones de binding cacheadas. Se añade como dependencia real del core en Fase E3 (hoy solo existe en el spike de lenguaje).

## Evidence

- `rusqlite 0.31` (feature `bundled`) ya fue validado end-to-end en `spikes/language/rust/`: build exitoso, 6 tests unitarios incluyendo un roundtrip real de creación de tabla + insert + select (`docs/research/language/candidates.md`), sin requerir SQLite del sistema (vendorizado en C).
- Codebase Memory, el proveedor estructural que Rationale ya consume, usa el mismo patrón (SQLite por proyecto en `~/.cache/codebase-memory-mcp/*.db`, con modo WAL activo) — confirmado por inspección directa de archivos en `docs/research/codebase-memory/07-storage-and-cache.md`. Es un precedente real, no solo una preferencia, de que SQLite es adecuado para este tipo de índice derivado de una herramienta de desarrollador local.
- `v0.5 §26.2` ya prescribe SQLite explícitamente para esta capa — este ADR no introduce una alternativa nueva, formaliza con evidencia una decisión ya apuntada en el contrato conceptual.

## Alternatives considered

- **Sin base de datos, releer YAML siempre**: es lo que hace hoy la Fase D (correcto para su alcance mínimo). Descartado para Fase E porque el Context Compiler necesita ranking y filtrado sobre un volumen creciente de Records/Assessments sin re-parsear YAML en cada consulta — no escala con el número de Records de un proyecto real.
- **Un motor embebido distinto (sled, redb)**: descartado sin evaluación propia — `v0.5 §26.2` ya fija SQLite, y no hay evidencia de que Rationale necesite las garantías específicas de un KV store embebido en Rust puro sobre SQLite, que además ya tiene FTS5 nativo (necesario para `retrieval` según `v0.5 §19.1`).
- **PostgreSQL/base de datos administrada**: descartado explícitamente por `Arquitectura §4.1` ("no deberá requerir obligatoriamente... base de datos administrada").

## Consequences

- Se añade `rusqlite = { version = "0.31", features = ["bundled"] }` al `Cargo.toml` raíz en Fase E3.
- El binario crece en tamaño (SQLite vendorizado en C se compila dentro del binario Rust) — ya medido indirectamente en el spike, sin cifra específica para el core real todavía; se mide en la verificación de Fase E.
- Introduce una dependencia con un componente C compilado — coherente con la ventaja ya evaluada en ADR-0001 ("interoperabilidad con procesos C", 5% del criterio ponderado, a favor de Rust).
- La capa derivada nunca es la única copia de una decisión (`Arquitectura §11.7`) — todo lo que SQLite almacena debe ser reconstruible desde `.rationale/` (verificado explícitamente en Fase E3 con un test de "cache rebuild desde cero").

## Risks

- Corrupción del archivo SQLite (energía, proceso matado a mitad de escritura) — mitigación: modo WAL (mismo patrón observado en Codebase Memory) y regenerabilidad completa como red de seguridad, no como excepción.
- Crecimiento del cache sin límite con el tiempo — pendiente de política de invalidación/expiración explícita en Fase E3 (no existe todavía ni en Rationale ni, según `07-storage-and-cache.md`, se confirmó que exista en Codebase Memory).

## Validation

`rusqlite` ya está validado por el spike (Fase C). La integración real en el core se valida en Fase E3 con: test de creación de tabla + insert + select sobre datos reales, test de regeneración completa desde `.rationale/`, y medición de tamaño de binario resultante.

**Este ADR está en estado `proposed`**, pendiente de revisión cruzada y aprobación humana.

## Revisit trigger

Reabrir si la medición real de Fase E3 muestra un tamaño de binario o tiempo de compilación inaceptable, o si aparece un caso de corrupción reproducible que WAL no mitigue.
