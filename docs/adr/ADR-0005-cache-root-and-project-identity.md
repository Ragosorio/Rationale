# ADR-0005: Cache root and project identity

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

ADR-0004 decide que la capa derivada vive en SQLite. Falta decidir **dónde** en el filesystem y **cómo se nombra** por proyecto. `Rationale_Arquitectura_Conceptual_v0.1.md §10.2` propone `<user-cache>/rationale/projects/<project-id>/`, con la ruta exacta "pendiente de decisión durante implementación" y sugiere `~/Library/Caches/Rationale/` en macOS o "un root configurable compatible con XDG".

`src/configuration.rs` ya resuelve un `project_id` (de `config.yaml` o, por defecto, el nombre del directorio), pero solo se usa hoy para mostrarlo en `health` — no existe todavía ninguna ruta de cache real que nombrar.

## Decision

1. **Cache root:** `~/.cache/rationale/projects/<project-id-sanitizado>/` en macOS y Linux — no la ruta nativa de macOS (`~/Library/Caches/`). Windows queda como gap documentado, no bloqueante (ver Risks).
2. **Sanitización del nombre de proyecto:** la ruta absoluta del `project_root`, con separadores reemplazados por guiones — mismo esquema observado en Codebase Memory.
3. **`project_id` lógico** (el valor usado dentro de Records/logs, no el nombre de carpeta de cache) sigue siendo el mecanismo ya implementado: `config.yaml → project.id`, con fallback al nombre del directorio. No cambia con este ADR.

## Evidence

- **Precedente real, no solo teórico:** Codebase Memory (mismo dominio de herramienta: análisis local de código, cache derivado) usa exactamente `~/.cache/codebase-memory-mcp/<ruta-sanitizada>.db`, confirmado por inspección directa de archivos en `docs/research/codebase-memory/07-storage-and-cache.md` — no una ruta nativa de macOS. Siete proyectos reales indexados en esa máquina confirman el patrón de nombrado (`Users-roor.osorio-Desktop-Monorepo.db`, etc.).
- Usar `~/.cache/` uniformemente (en vez de `~/Library/Caches/` en macOS vs `~/.cache/` en Linux) evita añadir una dependencia nueva (`dirs` u otro crate de resolución de paths por plataforma) solo para esta decisión — consistente con el principio de minimizar dependencias (`Proceso §19`).
- La separación canónica/derivada (`v0.5 §4.19`, Subject `storage.canonical-vs-derived`) significa que **la corrección nunca depende de esta ruta** — perder o mover el cache solo dispara una reconstrucción, nunca pérdida de una decisión real. Esto reduce el riesgo de cualquier elección de ruta imperfecta.

## Alternatives considered

- **`~/Library/Caches/Rationale/` en macOS, XDG en Linux (rutas nativas por plataforma)**: descartado por ahora — requeriría el crate `dirs` (o `directories`) sin que exista todavía una razón concreta más allá de "seguir la convención del SO". Se reconsiderará si Fase J (empaquetado) encuentra un requisito real de integración con herramientas del sistema (Finder, indexación de Spotlight, etc.) que dependa de la ubicación nativa.
- **Cache dentro del repo (`.rationale-local/` ya existe con este propósito)**: descartado para la capa SQLite específicamente — `.rationale-local/` ya se usa para logs de ejecución efímeros (Fase D), pero mezclar ahí un índice SQLite de mayor volumen contradice la intención original de esa carpeta y complica `.gitignore` selectivo. Se mantiene separado.
- **Project ID basado en hash del remote de Git o del commit raíz** (más estable que un nombre de directorio ante renombres/movimientos): evaluado, no descartado — es una mejora real pendiente, pero no bloquea este ADR porque la corrección no depende de la estabilidad del `project_id` (solo su legibilidad en logs). Queda anotado como mejora futura, no como decisión de este documento.

## Consequences

- Nuevo módulo o extensión de `configuration.rs` para calcular la ruta de cache: `cache_root(project_root) -> PathBuf`.
- Si el usuario mueve o renombra la carpeta del proyecto, el cache derivado bajo la ruta anterior queda huérfano (nunca se borra automáticamente en este ADR) — aceptable porque es 100% regenerable, pero deja basura en disco a largo plazo. Política de limpieza de cache huérfano queda fuera de alcance de Fase E.
- Windows usará una ruta distinta cuando se implemente (`%LOCALAPPDATA%\rationale\projects\...` es el candidato natural, sin verificar todavía) — no bloquea Fase E, que se desarrolla y valida en macOS.

## Risks

- **Gap de Windows explícito**: este ADR no resuelve la ruta de cache en Windows. Mismo patrón de riesgo ya registrado para file locking (`docs/dependencies/inventory.yaml known_gaps`) — se añade aquí como segundo gap de la misma naturaleza, a resolver junto con Fase J (empaquetado), no antes.
- Colisión de nombres si dos proyectos distintos sanitizan a la misma ruta (extremadamente improbable con rutas absolutas completas, pero no matemáticamente imposible con symlinks) — mitigación diferida, mismo riesgo ya aceptado implícitamente por Codebase Memory sin incidentes reportados en la investigación de Fase B.

## Validation

Se valida en Fase E3 con un test que calcula la ruta de cache para el propio repo de Rationale y para el fixture de la vertical slice, confirmando que no colisionan y que ambas son reconstruibles borrando el directorio de cache.

**Este ADR está en estado `proposed`**, pendiente de revisión cruzada y aprobación humana.

## Revisit trigger

Reabrir cuando Fase J (empaquetado) necesite resolver Windows de forma real, o si aparece un caso concreto donde la estabilidad del `project_id` ante renombres de carpeta cause un problema medible (no solo teórico).
