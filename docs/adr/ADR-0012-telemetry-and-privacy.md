# ADR-0012: Telemetry and privacy

**Status:** proposed — validation failed in part (ver «Validation update — 2026-07-28»). Reemplazo propuesto en ADR-0014.
**Date:** 2026-07-25
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno todavía — ADR-0014 propone reemplazar la garantía de exclusión local, pero ambos siguen en `proposed`: una propuesta sin aprobar no supersede a otra. Cuando ADR-0014 pase revisión cruzada y aprobación humana, este campo pasa a `partially superseded by ADR-0014`.

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

## Validation update — 2026-07-28

La migración de `alpha.7` a `main` sobre copias de los repos piloto Monorepo y
BoostAPI invalidó parte de la validación original. **No se reescribe el texto
anterior**: queda tal como se escribió, y esta sección registra qué falló y por
qué. El error también es evidencia.

**Qué sigue siendo válido.** La Decision #1 no fue refutada: Rationale no
inicia ninguna llamada de red y ningún dato sale del filesystem por acción del
producto. La Decision #2 (formato NDJSON append-only) tampoco.

**Fallo 1 — la Evidence generalizó desde el repo de desarrollo a los repos
consumidores.** La línea «`.rationale-local/` ya está en `.gitignore` desde
Fase A — verificado que los logs de instrumentación nunca se versionan ni se
publican junto con el repo» es cierta **solo dentro del repo de Rationale**,
donde ese `.gitignore` se escribió a mano. `init` e `install-agent` nunca
escriben esa entrada en el proyecto del usuario. En Monorepo y BoostAPI los
tres archivos de `.rationale-local/` están versionados, y `git branch -r
--contains` los sitúa en `origin/main` de ambos: la exposición fue efectiva
hacia esos remotos, no solo potencial. Dos de dos pilotos — es el flujo normal,
no contaminación accidental.

**Fallo 2 — el inventario de campos estaba incompleto.** La Evidence solo
auditó `RunLog` (`src/evaluation.rs`). El evento `review_decision`
(`src/review.rs:636`) emite `record_id`, `decision` y `time_to_confirm_ms`
—cuánto tardó un humano en resolver cada Record— y **ninguno de los tres
aparece en la lista de campos permitidos de la Decision #3**. Es dato
conductual sobre una persona, no metadata operacional de máquina. Nunca pasó la
prueba que este mismo ADR exige. `installed-agent-files.json` tampoco fue
considerado, y almacena rutas absolutas bajo el `$HOME` del usuario.

**Fallo 3 — el vector de exposición previsto no fue el real.** Risks anticipó
«un desarrollador que copie manualmente `.rationale-local/`». El vector real no
requirió ninguna acción humana: fue `git add` sobre un directorio que nadie
había excluido. Un riesgo redactado alrededor de un descuido manual no cubrió
el caso automático.

**Qué queda invalidado**, y pasa a ADR-0014: la garantía de que
`.rationale-local/` queda excluido en proyectos consumidores, la conclusión de
que los datos nunca llegan a un remoto, y la completitud del inventario de
datos locales considerado en el análisis de privacidad.

**Remediación en los pilotos.** Corregir Rationale no saca los archivos ya
seguidos por Git. Cada repo afectado necesita `git rm -r --cached
.rationale-local` de forma manual y explícita. No se reescribe el historial:
lo filtrado es metadata operacional y rutas personales — sin credenciales,
secretos, código ni contenido de Records — y el costo de reescribir historia
compartida es desproporcionado frente a ese contenido.

**Alcance confirmado de la exposición (2026-07-28, por el dueño del
proyecto):** Boost y BoostAPI son repositorios privados. La exposición fue
real pero acotada a esos remotos privados y a las personas con acceso a ellos;
no hubo publicación pública. Esto **no** equivale a «sin exposición»: los datos
llegaron a colaboradores y persisten en el historial remoto. Reduce la
gravedad, no la existencia del incidente, y es lo que hace proporcionada la
decisión de no reescribir el historial. Reabrir esta evaluación si cambia la
visibilidad de alguno de los dos repos o si se identifica contenido más
sensible del ya inventariado.
