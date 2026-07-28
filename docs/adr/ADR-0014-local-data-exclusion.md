# ADR-0014: Local data exclusion in consumer projects

**Status:** proposed — pendiente de revisión cruzada independiente y aprobación humana antes de `accepted`.
**Date:** 2026-07-28
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** propone reemplazar la garantía de exclusión local de ADR-0012. Mientras ambos sigan en `proposed`, este ADR **no** supersede formalmente a aquel — una propuesta sin aprobar no adquiere autoridad sobre otra propuesta.

## Context

ADR-0012 fijó que toda la telemetría de Rationale es local-only, y presentó
como evidencia que `.rationale-local/` «ya está en `.gitignore` desde Fase A».
Esa verificación se hizo dentro del repo de Rationale, donde ese `.gitignore`
está escrito a mano, y se generalizó a los proyectos consumidores sin
comprobarlo.

La migración de `alpha.7` a `main` sobre copias de Monorepo y BoostAPI
demostró que la generalización era falsa (`ADR-0012 §Validation update —
2026-07-28`). En ambos pilotos, tres archivos están versionados y presentes en
`origin/main`:

```
.rationale-local/installed-agent-files.json
.rationale-local/runs/review-decisions.ndjson
.rationale-local/runs/vertical-slice.ndjson
```

Ni `init` ni `install-agent` escriben nunca una exclusión en el proyecto del
usuario. Dos de dos pilotos reprodujeron el defecto: es el flujo normal del
producto, no un accidente de un repo.

Este ADR decide cómo Rationale protege sus datos locales en un repo ajeno.
**La resolución del ejecutable en `.mcp.json` queda explícitamente fuera de
alcance** y se decide en ADR-0015: es un tradeoff distinto (herencia de `PATH`
en clientes MCP lanzados como app gráfica) y mezclarlos bloquearía este ADR
contra un problema sin resolver.

## Decision

1. **`.rationale-local/` es dato estrictamente local y nunca debe quedar
   versionado en un proyecto consumidor.** Incluye la telemetría NDJSON de
   `runs/` y el manifest `installed-agent-files.json`, que almacena rutas
   absolutas bajo el `$HOME` del usuario.

2. **La exclusión se instala en `.git/info/exclude`, no en `.gitignore`.**
   `.gitignore` es un archivo compartido y versionado del proyecto ajeno;
   escribir ahí es una modificación visible del repo que Rationale no tiene
   por qué imponer al equipo. `info/exclude` es local al clon, logra el mismo
   efecto, y cada persona que corra `init` o `install-agent` obtiene la
   exclusión en su propio clon.

3. **La exclusión se escribe *antes* de crear cualquier contenido bajo
   `.rationale-local/`**, y de forma idempotente: si la entrada ya existe, no
   se duplica ni se reescribe el archivo.

4. **El gitdir se resuelve de verdad, no se asume `<root>/.git/info/exclude`.**
   En submódulos y worktrees, `.git` es un *archivo* con un puntero `gitdir:`,
   no un directorio. La ruta correcta se obtiene resolviendo ese puntero (o
   vía `git rev-parse --git-common-dir`, que en un worktree apunta al
   directorio común compartido — que es donde `info/exclude` debe vivir para
   aplicar a todos los worktrees).

5. **Fuera de un repositorio Git, Rationale no falla ni bloquea.** Si no hay
   gitdir, no hay nada que excluir: se omite el paso en silencio y el resto de
   la instalación procede igual.

6. **Rationale detecta si `.rationale-local/` ya está seguido por Git y
   advierte, pero nunca toca el índice.** Modificar el estado versionado de un
   proyecto ajeno no es una decisión del instalador. La advertencia nombra el
   comando exacto y deja la ejecución al humano:

   ```
   aviso: .rationale-local/ contiene archivos seguidos por Git.
   Rationale no modificará el índice automáticamente.
   Para dejar de versionarlos:
       git rm -r --cached .rationale-local
   ```

7. **El manifest solo puede contener rutas relativas al proyecto.** Hoy guarda
   absolutas (`/Users/<quien-sea>/Desktop/BoostAPI/CLAUDE.md`). Aunque la
   exclusión impida que se versione, una ruta absoluta en un archivo que ya
   viajó dos veces es un dato que no necesitaba existir. La ruta relativa es
   suficiente para lo único que el manifest hace: saber qué archivos
   administra y si siguen intactos.

8. **La validación es un test de regresión, no una inspección manual.** Un
   test crea un repo Git temporal, corre `install-agent`, y falla si
   `git status --porcelain` reporta cualquier ruta bajo `.rationale-local/`
   como seguida o sin ignorar. La inspección manual fue exactamente lo que
   falló en ADR-0012.

9. **`install-agent` normaliza las entradas heredadas del manifest cuyo
   destino es reconocible.** No basta con que las entradas *nuevas* sean
   relativas: un proyecto de `alpha.7` que se haya movido o copiado conserva
   rutas absolutas apuntando a la ubicación anterior, la guarda las rechaza
   —correctamente— y el rechazo aborta `uninstall-agent` entero.

   La normalización **no relaja la guarda**. Solo reescribe una entrada cuando
   su ruta termina en un destino administrado conocido (`CLAUDE.md`,
   `AGENTS.md`, `.mcp.json`, la regla de Cursor, o un `SKILL.md` bajo el
   directorio de skills), derivados de `TARGETS` y `prompts::ACTIONS` como
   fuente única. Una ruta arbitraria —`~/Documents/notas.md`— no coincide con
   ninguno, se conserva intacta, y la guarda la sigue rechazando. El destino
   resultante queda siempre dentro del `project_root` por ser relativo.

   Esto es lo que hace que `install-agent` sea de verdad la vía de migración:
   repara el estado administrativo, no solo los bloques. Sin ello, un piloto
   movido quedaría permanentemente sin poder desinstalarse.

   **Radio de acción declarado:** si el manifest heredado apuntaba al
   `CLAUDE.md` de *otro* proyecto, tras normalizar apunta al de éste. No es una
   escalada de privilegio: `uninstall` solo extirpa el bloque delimitado de
   Rationale, y si este proyecto está instalado ese archivo ya tenía su propia
   entrada. Se declara en vez de dejarlo implícito.

   Queda **fuera** de esta decisión que una entrada irreconocible siga
   abortando la operación entera en vez de saltarse. Con la normalización, ese
   caso deja de producirse por mover un proyecto y pasa a señalar un manifest
   corrupto o manipulado, donde fallar ruidosamente es defendible.

## Evidence

- **Reproducción en dos pilotos reales**, sobre copias, sin tocar los
  originales: `install-agent` corrido dos veces sobre Monorepo y BoostAPI.
  Ambos partían de `.rationale-local/` ya versionado con los mismos tres
  archivos, y ambos lo dejaron modificado en el árbol de trabajo.
- **Exposición efectiva, no potencial**: `git branch -r --contains` sitúa los
  commits que introdujeron esos archivos en `origin/main` de ambos repos
  (`812f7fe`, `b78357d` en Monorepo; `0346b91`, `bcfc9fe` en BoostAPI).
- **Ninguno de los dos proyectos tenía entrada `rationale` en su
  `.gitignore`** — confirmado por inspección directa. Nada en el producto la
  escribe.
- **El contenido filtrado incluye dato conductual**: `review-decisions.ndjson`
  registra `time_to_confirm_ms` por Record (hasta ~300 s) y la decisión tomada.
  `installed-agent-files.json` registra rutas absolutas bajo `$HOME`.
- **La migración en sí es correcta y no está en cuestión**: las dos pasadas de
  `install-agent` dejaron `CLAUDE.md`, `AGENTS.md` y `.mcp.json` byte-idénticos
  entre sí, con un solo bloque administrado y sin borrados. El defecto es
  exclusivamente de exclusión de datos locales.

## Alternatives considered

- **Escribir la entrada en el `.gitignore` del proyecto.** Descartado: es una
  modificación visible y versionada de un archivo compartido del equipo, para
  resolver un problema que es local a cada clon. Rationale impondría un cambio
  de repo para proteger sus propios artefactos. `info/exclude` obtiene el mismo
  resultado sin tocar nada compartido. Queda como opción si aparece un caso
  donde la exclusión deba propagarse a quien nunca corre Rationale — hoy no
  existe: sin correr Rationale no hay `.rationale-local/` que excluir.
- **Ejecutar `git rm -r --cached .rationale-local` automáticamente al detectar
  archivos seguidos.** Descartado: altera el estado versionado de un proyecto
  ajeno sin consentimiento, y en un producto cuya tesis es «no derribes una
  valla sin saber por qué está ahí», hacerlo en silencio sería contradictorio.
  Se advierte y se entrega el comando.
- **No escribir nada bajo `.rationale-local/` hasta que exista exclusión.**
  Descartado como política general: convertiría un problema de higiene en un
  bloqueo funcional, y ADR-0012 §Decision ya establece que la instrumentación
  no se pospone. La Decision #3 de este ADR (excluir *antes* de escribir)
  consigue el efecto sin bloquear nada.
- **Dejar de emitir `review_decision`.** Descartado: es dato legítimo para las
  métricas de `v0.5 §30`. El problema no era que se generara, sino que se
  publicara — y que nunca pasara el filtro de campos permitidos de ADR-0012.
  Que ese filtro se aplique a todo emisor, no solo a `RunLog`, es trabajo de la
  revisión de ADR-0012, no de este ADR.

## Consequences

- Cualquier emisor nuevo de datos bajo `.rationale-local/` hereda la protección
  sin decisión adicional: la exclusión cubre el directorio, no archivos
  concretos.
- Los repos piloto ya afectados **no se arreglan solos**. Requieren remediación
  manual con `git rm -r --cached .rationale-local`, una vez por repo. Los datos
  históricos permanecen en commits anteriores; ver ADR-0012 §Validation update
  para por qué no se reescribe el historial.
- Rationale escribe dentro de `.git/`, lo cual no hacía antes. Se limita a
  `info/exclude` y es la única escritura permitida ahí.
- El manifest cambia de formato (rutas relativas). `uninstall-agent` lee ese
  archivo: debe tolerar manifests antiguos con rutas absolutas o los pilotos ya
  instalados perderían la capacidad de desinstalar limpiamente.

## Risks

- **`info/exclude` no se propaga.** Alguien que clone y nunca corra `init` ni
  `install-agent` no tendrá la exclusión. Mitigación: tampoco tendrá
  `.rationale-local/`, porque solo Rationale lo crea. El riesgo es nulo en la
  práctica y se vuelve real solo si alguien copia el directorio a mano — el
  mismo caso que ADR-0012 §Risks ya contempla.
- **Un `.git` inesperado.** Repos con `core.worktree`, submódulos anidados o
  setups no estándar podrían resolver un gitdir que no es el esperado.
  Mitigación: si la resolución no produce un directorio escribible, se omite y
  se advierte, nunca se falla la instalación.
- **La advertencia se ignora.** El humano puede no correr el `git rm --cached`
  y quedarse con los archivos seguidos indefinidamente. Mitigación aceptada: la
  alternativa es actuar sobre el índice ajeno, que es peor. La advertencia se
  repite en cada ejecución mientras la condición persista.

- **Un manifest con rutas absolutas fuera del proyecto aborta `uninstall-agent`
  entero.** Observado al verificar este ADR sobre una copia de BoostAPI: si el
  proyecto se movió o copió después de instalar, las entradas absolutas del
  manifest apuntan a la ubicación anterior, `resolve_managed_entry_path` las
  rechaza —correctamente, es la guarda que impide que un manifest manipulado
  haga tocar archivos arbitrarios— y el rechazo **cancela toda la
  desinstalación**, incluidas las entradas legítimas.

  Es un defecto **preexistente**, no introducido por este ADR: la guarda y su
  comportamiento de aborto son anteriores. Este ADR lo reduce hacia adelante
  (una ruta relativa sobrevive a mover el proyecto) pero no lo elimina para los
  manifests ya escritos. Se documenta aquí en vez de darlo por cubierto: la
  Consequence de este ADR afirma compatibilidad de `uninstall-agent` con
  manifests heredados, y esa afirmación es cierta solo mientras el proyecto no
  se haya movido. Dar por buena una compatibilidad sin acotarla sería
  exactamente el error que ADR-0012 cometió.

  **Resuelto por la Decision #9**, añadida después de detectar esto: en vez de
  tocar la guarda, `install-agent` normaliza las entradas heredadas cuyo
  destino es reconocible, de modo que el caso «proyecto movido» deja de
  producir entradas externas. Verificado end-to-end sobre una copia de BoostAPI
  con el manifest apuntando a la ubicación original.

  Lo que **no** se resolvió, deliberadamente: una entrada irreconocible sigue
  abortando la operación entera. Tras la Decision #9 ese caso ya no lo produce
  un proyecto movido, sino un manifest corrupto o manipulado, donde fallar
  ruidosamente es la respuesta defendible. Si aparece un caso real de entrada
  irreconocible legítima, se reabre.

## Validation

Implementado y verificado. Siete tests de regresión en `src/agents.rs`, todos
sobre repositorios Git temporales reales:

1. `install_leaves_no_local_data_visible_to_git` — falla si `git status
   --porcelain --untracked-files=all` reporta cualquier ruta bajo
   `.rationale-local/` tras instalar (Decision #8).
2. `exclude_entry_is_idempotent_and_preserves_existing_rules` — segunda pasada
   sin reescritura, entrada sin duplicar, reglas previas del usuario intactas.
3. `exclude_resolves_the_real_gitdir_in_a_worktree` — worktree real, `.git`
   como archivo con puntero, exclusión aterrizando en el directorio común.
4. `install_warns_when_local_data_is_already_tracked` — advertencia con el
   comando exacto, y comprobación de que el índice **no** se modificó.
5. `install_outside_a_git_repository_does_not_fail` (Decision #5).
6. `manifest_stores_project_relative_paths` — ninguna ruta absoluta.
7. `uninstall_still_reads_a_legacy_absolute_path_manifest` — compatibilidad
   hacia atrás.
8. `install_migrates_a_moved_projects_legacy_manifest_and_uninstall_then_works`
   — el escenario completo de Decision #9: `uninstall` falla antes de migrar,
   `install-agent` normaliza, `uninstall` completa.
9. `migration_never_normalizes_an_arbitrary_path` — `~/Documents/notas.md` se
   conserva intacta y la guarda la sigue rechazando.
10. `migration_recognizes_every_managed_destination` — recorre `TARGETS` y
    `prompts::ACTIONS` para que un agente o acción nuevos no queden fuera de la
    normalización sin que nadie lo note.

**Desviación deliberada respecto al plan original de validación**, que pedía
fixtures estáticos en `tests/fixtures/alpha7-consumer/`: se descartaron a favor
de construir los repos con `git init` dentro de cada test. Un fixture estático
no puede llevar un `.git/` real versionado dentro de este repositorio, así que
no podría reproducir lo único que importa aquí —qué archivos están *seguidos
por el índice*— que es precisamente la condición que falló en los pilotos. Los
tests programáticos cubren ese estado; los fixtures no podían.

Verificación end-to-end adicional, fuera de la suite: `install-agent` corrido
dos veces sobre copias frescas de Monorepo y BoostAPI. Bloques byte-idénticos
entre pasadas, exclusión sin duplicar, cero rutas absolutas en `CLAUDE.md` y
`AGENTS.md`, advertencia emitida en ambos por los tres archivos ya seguidos.

**Explícitamente, la validación no puede consistir en inspección manual del
repo de Rationale.** Ese fue el error de ADR-0012: verificar en el repo de
desarrollo y generalizar a los consumidores.

## Revisit trigger

Reabrir si aparece un consumidor donde `info/exclude` no sea suficiente —
por ejemplo, un flujo donde `.rationale-local/` deba compartirse
deliberadamente entre miembros de un equipo (debugging conjunto, auditoría de
decisiones). Eso requeriría una decisión explícita sobre qué campos son
publicables, no una relajación silenciosa de esta exclusión.
