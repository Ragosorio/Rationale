# ADR-0012: Telemetry and privacy

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

`Rationale_Arquitectura_Conceptual_v0.1.md §20` exige instrumentación "desde la primera vertical, no al final", y `§11.14` es explícito: "No enviar datos automáticamente". `Rationale_v0.5.md §30.1.12` añade, para el futuro piloto en un repositorio laboral: telemetría local por defecto, sin envío de código/prompts/diffs/Records a servicios adicionales sin autorización.

Esto ya está implementado de facto desde Fase D (`src/evaluation.rs`), sin que existiera un ADR formal que lo fijara como decisión, no como accidente de implementación. Antes del piloto (Fase H) y de que Fase F añada captura de diffs y señales (con más superficie de datos potencialmente sensibles), conviene formalizarlo.

## Decision

1. **Toda telemetría es local-only por defecto y sin excepción configurable en Fase E/F.** Ningún dato sale del filesystem del usuario mediante ninguna llamada de red iniciada por Rationale.
2. **Formato:** NDJSON en `.rationale-local/runs/*.ndjson`, un archivo por tipo de evento, append-only.
3. **Campos permitidos:** metadata operacional (latencia, revisión Git, estado de consistencia, estado y cobertura del proveedor, tamaño del packet en bytes, timestamp). **Campos prohibidos por defecto:** contenido de código, prompts de agente, diffs completos, contenido de Records/Evidence, cualquier secreto.

## Evidence

- Implementación real ya existente (`src/evaluation.rs`, Fase D): `RunLog` solo contiene `event`, `timestamp`, `latency_ms`, `git_revision` (un SHA, no contenido), `consistency`, `provider_status`, `provider_coverage`, `packet_bytes` (un conteo, no el packet mismo). Ningún campo de texto libre, código o prompt.
- `.rationale-local/` ya está en `.gitignore` desde Fase A — verificado que los logs de instrumentación nunca se versionan ni se publican junto con el repo.
- Verificado en la sesión de Fase D: el log real generado (`.rationale-local/runs/vertical-slice.ndjson`) no contiene ninguna cadena de código, ruta de archivo del usuario más allá de lo estrictamente necesario, ni ningún dato más allá de los campos declarados.

## Alternatives considered

- **Enviar telemetría agregada a un servicio propio para mejorar el producto**: descartado explícitamente — contradice `Arquitectura §11.14` y no hay ningún consentimiento de usuario implementado que lo autorizaría. Podría reconsiderarse en el futuro solo como opt-in explícito, nunca por defecto, y fuera del alcance de este ADR.
- **No instrumentar hasta que exista un caso de uso concreto**: descartado — `Arquitectura §20` es explícito en que la instrumentación va desde la primera vertical, no se pospone, precisamente para que las métricas de `v0.5 §30` (tokens del packet, latencia, tasa de recall) tengan datos desde el principio del proyecto.
- **Registrar el diff completo o el contenido del Record en cada log**: descartado — violaría el principio de minimización (`v0.5 §4.11`, `§26.5`) y aumentaría innecesariamente el riesgo si un log llegara a compartirse por error.

## Consequences

- Fase F (captura de diffs/señales) debe respetar la misma regla: cualquier instrumentación nueva que esa fase introduzca sigue local-only, y cualquier campo que se plantee añadir debe pasar la misma prueba ("¿es metadata operacional o es contenido potencialmente sensible?").
- El futuro piloto (Fase H, monorepo laboral) hereda esta política sin necesitar una decisión nueva — ya está resuelta aquí.
- Los reportes agregados de `v0.5 §30.1.12` ("los reportes podrán utilizar IDs y métricas agregadas") siguen siendo responsabilidad de un proceso manual y explícito de análisis, no de envío automático.

## Risks

- Un desarrollador que copie manualmente `.rationale-local/` a otro lugar (ej. para debugging compartido) podría exponer rutas de archivo del sistema — mitigación: los campos ya excluyen contenido, y `git_revision` es un SHA público de todos modos en cualquier repo compartido.
- Si Fase F añade un campo nuevo al log sin revisar esta política, podría filtrarse contenido sensible sin que nadie lo note — mitigación: este ADR se cita explícitamente como gate de revisión para cualquier cambio a `RunLog` o estructuras equivalentes.

## Validation

Verificado por inspección directa del log real generado en Fase D (ver Evidence). Fase E6 añade un test explícito que falla si `RunLog` (o su equivalente ampliado) incluye un campo de tipo texto libre no acotado (protección contra regresión futura).

**Este ADR está en estado `proposed`**, pendiente de revisión cruzada y aprobación humana.

## Revisit trigger

Reabrir si Fase F necesita registrar algo que hoy está prohibido (ej. un fragmento de diff para debugging) — requeriría una decisión explícita de opt-in, no una ampliación silenciosa de este ADR.
