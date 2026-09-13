# ADR-0017: Local activity stream and operation snapshots

**Status:** proposed — pendiente de revisión cruzada independiente y aprobación humana antes de `accepted`.
**Date:** 2026-09-12
**Deciders:** Claude Code (análisis e implementación), por encargo del dueño del proyecto (brief de Rationale vNext, 2026-09-12); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno. Acota ADR-0012 §Decision 3 para dos emisores nuevos —el flujo de actividad y los snapshots de operación— y retira `RunLog`. Mientras ADR-0012 y este ADR sigan en `proposed`, este no adquiere autoridad sobre aquel: declara la tensión en vez de resolverla en silencio.

## Context

El brief de vNext pide que Rationale sea observable mientras un agente
trabaja: una vista de actividad con el cliente, la intención, el target, lo
considerado y lo seleccionado, el tamaño del packet, y un grafo que reacciona
a eventos (`ActivityEvent`). El `RunLog` de Fase D (`runs/vertical-slice.ndjson`)
solo tenía latencia, revisión, consistencia, estado del proveedor y bytes: no
alcanza para nada de eso.

ADR-0012 §Decision 3 prohíbe por defecto en la telemetría local los prompts de
agente y el contenido de Records, y su *revisit trigger* exige una decisión
explícita —no una ampliación silenciosa— para registrar algo prohibido. La
intención declarada en `prepare_change` se parece a un prompt. Además:

- Los snapshots de operación de vNext (`.rationale-local/operations/`) ya
  guardan la intención, el target y el subgrafo seleccionado, porque
  `finalize_change` y la UI los necesitan.
- Los conflictos pendientes (`.rationale-local/conflicts/`) guardan los dos
  statements en tensión, porque `resolve_conflict` debe poder continuar.
- El test guardián que ADR-0012 §Validation prometía («falla si hay un campo de
  texto libre no acotado») nunca se escribió.
- Los escritores nuevos bajo `.rationale-local/` no aplicaban la exclusión de
  Git antes de escribir; solo `init` e `install-agent` la instalaban
  (ADR-0014 §Decision 3).

## Decision

1. **Flujo de actividad** en `.rationale-local/activity/<session-id>.ndjson`:
   append-only, un archivo por proceso (`rationale serve`, cada invocación de
   la CLI), esquema `rationale/activity/1`. Cada evento lleva
   `schema_version`, `session_id`, `seq`, `trace_id`, `operation_id`,
   `timestamp` (RFC3339 UTC con milisegundos), `actor`, `project`, `kind` y
   `payload`. Un archivo por sesión evita que Claude Code, Codex y Cursor
   compitan por el mismo archivo.
2. **Inventario de contenido permitido:** identificadores (ids de sesión,
   traza, operación, Record y conflicto; claves de nodo y arista; rutas;
   nombres calificados; el spec del target, ≤200 caracteres), estados,
   conteos, latencias, tamaños, versión y nombre del cliente. **Un solo texto
   libre:** la intención declarada, en una línea y ≤280 caracteres.
   **Prohibido:** código y snippets, diffs, statements, rationale, evidencia,
   preguntas de conflicto, respuestas humanas y resúmenes. El contenido de un
   Record o de un conflicto viaja por referencia: la UI lo lee del canon.
3. **Snapshots de operación** (`.rationale-local/operations/<op>.json`): estado
   derivado funcional, no telemetría. Guardan el subgrafo (nombres, rutas,
   claves, roles, ids de Records), la selección, el actor, la revisión base y
   la intención; nunca el código del target. Retención: 200 operaciones.
4. **Exclusión antes de escribir:** los escritores de vNext —el de
   actividad, y el pipeline antes de un snapshot de operación o de un
   conflicto— instalan la exclusión de `.rationale-local/` (ADR-0014) antes
   de su primer contenido en cada proyecto, también con la actividad
   desactivada.
5. **Opt-out:** `RATIONALE_ACTIVITY=off` desactiva el flujo por completo. Por
   defecto está activo, porque la vista de actividad es parte del producto.
6. **Retención:** sesiones de más de 14 días se eliminan, y nunca quedan más de
   500 archivos. Se usa antigüedad y no solo conteo, para que una ráfaga de
   sesiones cortas (una suite de tests) no desaloje la historia real.
7. **`RunLog` se retira.** `review-decisions.ndjson` (flujo de revisión legado)
   no cambia en este ADR.

## Evidence

- Brief de vNext (2026-09-12): modelo `ActivityEvent`, eventos requeridos y
  vista de actividad con intención y target.
- `src/activity.rs`: los payloads solo se construyen con `activity::payload::*`;
  el test `payloads_are_bounded_and_carry_content_only_by_reference` recorre
  cada constructor con entradas de 10.000 caracteres con secuencias ANSI y
  falla ante un string sin techo, bytes de control, listas sin límite o claves
  de contenido (`statement`, `rationale`, `detail`, `question`, `source`…).
- `git_exclusion_is_installed_before_the_first_event` comprueba que, en un
  repo recién inicializado, `.git/info/exclude` contiene `.rationale-local/`
  y que `git status` no ve la actividad.

## Alternatives considered

- **Conservar solo `RunLog`:** la UI no podría mostrar qué hace el agente ni
  encender el subgrafo seleccionado. Descartado: contradice el brief.
- **Incluir statements en los eventos:** duplicaría el contenido del canon en
  un segundo almacén no versionado, contra la minimización (v0.5 §4.11). La UI
  ya puede leer el canon.
- **Un único `activity.ndjson`:** varios procesos de agente escribirían a la
  vez; habría que coordinar locks entre procesos sin necesidad.
- **Desactivado por defecto (opt-in):** la vista de actividad quedaría vacía
  en el flujo normal. Se prefiere activo por defecto, con contenido mínimo,
  local, excluido de Git y con opt-out explícito.

## Consequences

- La UI observa sesiones de varios agentes sin coordinación entre procesos.
- Cualquier evento nuevo debe pasar por `activity::payload`, así que el test
  guardián lo cubre automáticamente.
- Una intención escrita por una persona queda en disco local, acotada, hasta
  por 14 días.

## Risks

- **La intención puede contener texto sensible.** Mitigación: una línea, ≤280
  caracteres, local-only, excluida de Git, retención de 14 días y
  `RATIONALE_ACTIVITY=off`.
- **Rutas y nombres calificados revelan la estructura del código.** Ya están
  en el canon versionado y en Git; los eventos no añaden contenido de código.
- **Proyectos con `.rationale-local/` ya versionado:** `info/exclude` no saca
  archivos seguidos. `install-agent` sigue advirtiendo con el comando de
  remediación (ADR-0014 §Decision 6).

## Validation

Tests unitarios de `src/activity.rs` (orden y `seq`, combinación de sesiones,
*tail* de líneas completas, exclusión, opt-out, retención, minimización) y el
test de integración MCP del ciclo de vida de una operación, que verifica la
secuencia de eventos de `prepare_change` y `finalize_change`.

## Revisit trigger

Reabrir si la UI necesita un texto libre más allá de la intención, si se
propone transmitir actividad fuera de la máquina (eso requiere además la
decisión de opt-in de ADR-0012) o si la actividad debe compartirse entre
clones.
