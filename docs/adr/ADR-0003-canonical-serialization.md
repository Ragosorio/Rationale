# ADR-0003: Canonical serialization

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

`Rationale_v0.5.md §26.1` exige que la capa canónica sea "revisable en PR" y "legible sin la herramienta". `Rationale_Arquitectura_Conceptual_v0.1.md §21` (estructura propuesta) y toda la documentación de producto (`v0.5 §5, §9, §27`) ya modelan Subjects, Records, Bindings y Approvals en YAML — no es una decisión abstracta, ya existe un corpus real de datos canónicos en YAML: 9 Subjects, 2 Records (uno real, uno de fixture) en `.rationale/` y `fixtures/vertical-slice/.rationale/`.

Independientemente del formato, `docs/rust/style-guide.md` ya registró un riesgo concreto: la dependencia `serde_yaml` está marcada `+deprecated` upstream (confirmado en la propia salida de `cargo build` del spike de lenguaje, Fase C). Fase E multiplica el volumen de parseo YAML (Subjects, Evidence, Assessments) — migrar la dependencia después de eso cuesta sustancialmente más que migrarla ahora, antes de que el código dependa de ella en más de tres módulos.

## Decision

1. **YAML se mantiene como formato canónico** de `.rationale/` (Subjects, Records, Bindings, Approvals, Evidence, Assessments cuando corresponda). JSON se reserva para el Context Packet servido por MCP (formato de intercambio, no de almacenamiento) y para los schemas de validación (`.rationale/schemas/*.json`, JSON Schema es el estándar de facto para esta función).
2. **`serde_yaml` se reemplaza por `yaml_serde` (0.10.4)** en el `Cargo.toml` raíz, antes de que Fase E2 amplíe el modelo canónico.

## Evidence

- El corpus canónico existente (`.rationale/subjects/*.yaml`, `.rationale/records/*.yaml`, `fixtures/vertical-slice/.rationale/`) ya está en YAML — reescribirlo a JSON tendría un costo de migración real sin beneficio claro, dado que ningún requisito de `v0.5 §26.1` favorece JSON sobre YAML para este uso.
- Los campos de texto largo (`statement`, `rationale`, `problem.statement`) usan bloques YAML (`>`) extensivamente en los ejemplos de `v0.5 §27` — JSON no tiene un equivalente igual de legible para texto multilínea sin escapes.
- **Prueba de compatibilidad real ejecutada en esta sesión**: se creó un proyecto Cargo desechable (`/tmp/yaml-serde-probe`, ya eliminado) que deserializa un Subject real del repo (`policy.local-first.yaml`) con `yaml_serde::from_str` y lo serializa de vuelta con `yaml_serde::to_string`. Ambas operaciones tienen la misma firma que `serde_yaml` (`from_str<T>(&str) -> Result<T>`, `to_string<T>(&T) -> Result<String>`) y el roundtrip fue exitoso sobre datos reales, no sintéticos.
- `yaml_serde` está respaldado por "The YAML Organization" (`github.com/yaml/yaml-serde`), licencia `MIT OR Apache-2.0` — mismo rango de licencia que `serde_yaml`, sin cambio de política de dependencias.

## Alternatives considered

- **Migrar todo `.rationale/` a JSON**: descartado — costo de reescritura del corpus existente sin beneficio (`v0.5 §26.1` no exige JSON), y pérdida de legibilidad para campos de texto largo.
- **Quedarse en `serde_yaml`**: descartado — es una dependencia con mantenimiento descontinuado; Fase E multiplica su superficie de uso, y migrar después de eso (con Subject Resolver, Evidence, Assessments ya dependiendo de su API) sería más costoso que migrar ahora.
- **`serde_yaml_ng`** (fork de un solo mantenedor, `acatton`): descartado en favor de `yaml_serde` por tener respaldo organizacional (`github.com/yaml`) en vez de un mantenedor individual — más duradero a largo plazo, mismo criterio que ya aplicó `Arquitectura §18.4` sobre riesgo de dependencias.
- **`serde_yaml_bw`** ("panic-free parsing, including malformed YAML"): interesante para tolerancia a errores, pero no probado en esta sesión — no se descarta para el futuro, solo no es la decisión de este ADR sin evidencia propia.

## Consequences

- Cambiar `serde_yaml = "0.9"` por `yaml_serde = "0.10.4"` en `Cargo.toml` raíz. La API usada en el código actual (`src/configuration.rs`, `src/storage.rs`) usa `serde_yaml::from_str` — el mismo patrón funciona con `yaml_serde::from_str` sin cambios de lógica, solo el nombre del crate y el `use`.
- Los 7 schemas JSON planeados para Fase E2 (`.rationale/schemas/*.json`) no se ven afectados — son JSON Schema, formato de validación, no de almacenamiento canónico.
- El Context Packet servido por MCP (Fase E5) se serializa en JSON, no YAML — es el formato nativo de JSON-RPC 2.0, no una elección independiente.

## Risks

- `yaml_serde` es un crate relativamente nuevo (metadatos de publicación recientes) — menor historial de producción que `serde_yaml` en su mejor época. Mitigación: el roundtrip ya se probó contra datos reales; se mantiene `Cargo.lock` fijado y se revisa antes de cada actualización de versión (`Arquitectura §18.4`).
- El cambio de dependencia toca todo archivo que hoy usa `serde_yaml::` — requiere una pasada de find-and-replace mecánica al implementarlo, con `cargo test` como red de seguridad (19 tests existentes, varios leen YAML real).

## Validation

Prueba de compatibilidad ejecutada y descrita en Evidence. La migración real del `Cargo.toml` y el código se ejecuta en Fase E2 (`docs/dependencies/inventory.yaml` se actualiza en el mismo commit), con `cargo test` pasando como criterio de aceptación.

**Este ADR está en estado `proposed`**, pendiente de revisión cruzada y aprobación humana.

## Revisit trigger

Reabrir si `yaml_serde` deja de mantenerse (mismo patrón que llevó a este ADR con `serde_yaml`), o si una necesidad concreta de Fase F/G demuestra que JSON sería preferible para una porción específica del canon (por ejemplo, si el volumen de Records vuelve el parseo YAML un cuello de botella medido, no hipotético).
