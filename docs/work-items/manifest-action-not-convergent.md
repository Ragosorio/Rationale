# Work item: manifest-action-not-convergent

**Estado: `implemented — pending integration and pilot validation`.**
Bloqueaba `v0.1.0-alpha.8`. No bloqueó la migración de los pilotos.

## Problema

`install-agent` informa que todo está al día mientras reescribe el manifest.

En la segunda ejecución sobre un proyecto ya instalado, el reporte dice
`instrucciones ya presentes y al día` y `skill al día en …` para las nueve
entradas — y aun así `.rationale-local/installed-agent-files.json` cambia:
cada entrada pasa de `"action": "created"` a `"action": "modified"`.

La causa es que `record_entry` / `record_owned_entry` se llaman con
`outcome.action`, que describe el estado del archivo **en esta ejecución**
(existía → `Modified`), no lo que pasó cuando Rationale lo instaló. La
primera pasada crea el archivo y registra `Created`; la segunda lo encuentra
existente y sobrescribe ese hecho histórico con `Modified`.

El reporte al usuario y el efecto sobre el disco se contradicen.

## Objetivo

Que la segunda ejecución de `install-agent` sobre un proyecto sin cambios sea
**byte-idéntica** en todos los archivos que toca, manifest incluido.

La corrección debe hacer una de dos cosas, explícitamente:

1. **Preservar la semántica original**: si ya existe una entrada para ese
   path, conservar su `action` en vez de sobrescribirla — `action` significa
   «qué hizo Rationale la primera vez», y eso no cambia al reinstalar.
2. **Redefinir qué significa `action`** y documentarlo en el propio tipo, si
   se decide que debe reflejar la última ejecución.

No se acepta cerrar esto dejando ambiguo qué significa el campo.

## Non-goals

- No se cambia `ReversalStrategy` ni la lógica de reversión. `action` ya está
  documentado como algo que «nunca decide cómo se revierte» (`src/agents.rs`,
  comentario en `uninstall`), y eso es correcto — por eso este defecto es
  inerte y no corrompe desinstalaciones.
- No se toca la exclusión de ADR-0014 ni la migración de la Decision #9.

## Base revision

`3830191` (`main`, tras el merge del dogfood pre-beta).

## Evidencia

Observado en el repo piloto real Monorepo, 2026-07-28, durante la migración a
la configuración portable:

- Pasada 1: los seis skills se registran con `action: created`.
- Pasada 2: reporte `skill al día` en los seis, y sin embargo el manifest
  cambia — `created` → `modified` en las nueve entradas. `md5` de `.mcp.json`,
  `CLAUDE.md` y `AGENTS.md` idénticos; el del manifest, no.
- Pasada 3: **idéntica a la 2**. Converge; no es churn perpetuo.

Impacto acotado, y por eso no bloqueó el commit de Monorepo: el archivo
afectado es local-only y desde ADR-0014 está fuera del índice, así que el
diff fantasma ya no llega a Git en un proyecto migrado. En un proyecto que
todavía lo tenga versionado, sí produce un diff sin cambio real.

## Riesgos

- **De no arreglarlo:** el reporte de `install-agent` deja de ser fiable como
  descripción de lo que hizo. Es la superficie que un usuario mira para
  decidir si algo cambió, y ya mintió una vez.
- **De arreglarlo mal:** conservar `action` de una entrada previa sin validar
  que corresponde al mismo archivo administrado podría arrastrar metadata
  histórica incorrecta. La validación de `resolve_managed_entry_path` ya cubre
  ese caso y debe seguir aplicándose.

## Plan

1. Decidir entre las opciones 1 y 2 del Objetivo, y dejar la decisión escrita
   en el doc-comment de `FileAction`.
2. Implementar en `record_entry` / `record_owned_entry`.
3. Añadir el test de convergencia (ver Tests).
4. Verificar sobre una copia de un piloto que la segunda pasada no ensucia
   nada.

## Tests

Un test nuevo que **exija convergencia completa en la segunda ejecución**:
correr `install` dos veces sobre un repo temporal y afirmar que el manifest
serializado es byte-idéntico entre ambas. Debe fallar con el código actual —
si no falla, no está probando esto.

Los tests existentes de idempotencia (`instructions_block_is_idempotent`,
`mcp_json_migrates_a_legacy_absolute_command_to_the_logical_one`) no cubren el
manifest: por eso este defecto llegó a un piloto real sin que nadie lo viera.

## Docs

`CHANGELOG.md` bajo «Sin publicar», si el arreglo redefine `action`.

## Criterio de éxito

`install-agent` corrido dos veces seguidas sobre un proyecto sin cambios deja
el árbol de trabajo idéntico, y el test de convergencia lo garantiza contra
regresión.

## Resolución (2026-07-28)

Implementado en `fix/manifest-action-convergence`. **Eran dos causas, no una.**
El diagnóstico original de este work item nombraba solo la primera; la segunda
apareció al exigir byte-identidad en los tests, y no se habría visto comparando
campos uno a uno.

**Causa 1 — sobrescritura del `action` histórico.** `record_entry` y
`record_owned_entry` recibían `outcome.action`, que describe el estado del
archivo *en la ejecución actual* (existía → `Modified`), y lo escribían encima
del hecho histórico. La primera pasada registraba `Created`; la segunda lo
volteaba a `Modified`.

**Causa 2 — reordenamiento de entradas por `remove` + `push`.** Preservar
`action` no bastaba. En una segunda pasada solo *algunas* entradas se
re-registran —los skills se reescriben siempre; instrucciones y MCP solo si
cambiaron— y con `remove` + `push` esas pocas saltaban al final, desplazando
al resto. `AGENTS.md` pasaba de la posición 9 a la 3 sin que ningún dato
cambiara.

**Corrección.** `action` queda definido como hecho histórico —cómo adquirió
Rationale la administración del archivo la primera vez— y documentado en el
doc-comment de `FileAction`, que era el requisito de no dejar el campo
ambiguo. `upsert_entry` sustituye a `remove` + `push`: actualiza la entrada
existente **en su posición**, y solo añade al final lo que es nuevo. Se
actualizan hash, `reversal` y la normalización de ruta; nunca el `action`. La
migración desde rutas absolutas también lo preserva.

**Evidencia.** Seis tests nuevos, escritos antes del arreglo:

| Test | Cubre |
|---|---|
| `fresh_install_manifest_is_byte_identical_on_the_second_run` | Instalación nueva converge |
| `legacy_manifest_migrates_once_then_converges` | Manifest de alpha.7: migra una vez, luego byte-idéntico |
| `a_historically_created_entry_stays_created` | `created` sobrevive dos reinstalaciones |
| `a_historically_modified_entry_stays_modified` | `modified` sobrevive incluso si se borra el archivo y la pasada observa una creación |
| `uninstall_after_convergence_still_preserves_user_content` | La reversión sigue conservando lo del usuario |
| `convergence_does_not_weaken_the_arbitrary_path_guard` | Las rutas arbitrarias se siguen rechazando |

Los dos primeros **fallaron con el arreglo parcial** (solo causa 1) y son los
que encontraron el reordenamiento. Suite completa: 247 tests, clippy y fmt
limpios.

Pendiente para cerrar: integración en `main` y comprobación de convergencia en
disco sobre el piloto Monorepo.
