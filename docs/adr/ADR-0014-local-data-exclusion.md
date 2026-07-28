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

## Validation

Pendiente de implementación. La validación exigida es:

1. Test de regresión con repo Git temporal (Decision #8), que falle si
   `.rationale-local/` queda seguido o sin ignorar tras `install-agent`.
2. Fixtures `tests/fixtures/alpha7-consumer/` que reproduzcan el estado
   histórico real observado en los pilotos — un solo formato, porque Monorepo y
   BoostAPI resultaron idénticos.
3. Test específico de gitdir resuelto vía puntero `gitdir:` (worktree o
   submódulo), no solo el caso `.git/` directorio.
4. Test de que un manifest antiguo con rutas absolutas sigue siendo legible por
   `uninstall-agent`.

**Explícitamente, la validación no puede consistir en inspección manual del
repo de Rationale.** Ese fue el error de ADR-0012: verificar en el repo de
desarrollo y generalizar a los consumidores.

## Revisit trigger

Reabrir si aparece un consumidor donde `info/exclude` no sea suficiente —
por ejemplo, un flujo donde `.rationale-local/` deba compartirse
deliberadamente entre miembros de un equipo (debugging conjunto, auditoría de
decisiones). Eso requeriría una decisión explícita sobre qué campos son
publicables, no una relajación silenciosa de esta exclusión.
