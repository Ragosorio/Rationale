# Changelog

Los cambios importantes se registran aquí por Release. El detalle técnico de
cada cambio vive en commits, ADRs y work items enlazados.

## Sin publicar

- **`windows-latest` y CI sin agentes instalados encontraron cuatro defectos
  que la máquina del desarrollador nunca podía ver.** `cargo test --locked`
  estaba rojo en las tres plataformas (`main` en `eb35bfb5`), y eran cuatro
  causas distintas:
  - **JSON inválido en Windows.** `cmd_init` y `cmd_health` interpolaban la
    ruta del proyecto en un literal JSON escrito a mano; `\` no es un escape
    válido, así que el contrato de una línea salía corrupto. Ambos comandos
    generan ahora el JSON con `serde_json`.
  - **Comparación de rutas no portable en Windows.** `/otro/proyecto/CLAUDE.md`
    **no es** `is_absolute()` en Windows — le falta la letra de unidad — así
    que la migración de manifests heredados la trataba como ya relativa y la
    saltaba, dejando una entrada externa que después abortaba
    `uninstall-agent`. La migración decide ahora por el destino al que
    apunta la ruta, no por su forma, comparando componentes portables (`/` y
    `\` como separador en cualquier plataforma) en vez de `is_absolute()` o
    subcadenas.
  - **Tests de `agents::install` dependientes del PATH del desarrollador.**
    `install()` detecta un agente si su binario está en `PATH` o si el
    proyecto ya usa su configuración. En la Mac del desarrollador `claude` y
    `codex` estaban en `PATH`, así que la detección siempre ocurría y
    `install` siempre llegaba a escribir el manifest; en los runners de CI no
    existe ninguno de los tres binarios, `install` retornaba temprano, y tres
    tests que asumían un manifest ya escrito fallaban con `NotFound`. Un
    cuarto test pasaba, pero por la razón equivocada: la guarda contra rutas
    arbitrarias nunca se ejercitó porque `install` no hizo nada. Los tests
    ahora siembran explícitamente la condición de detección (`AGENTS.md`,
    `.cursor/mcp.json`, el directorio de skills) en vez de depender de qué
    tenga instalado quien los corre.
  - **`install-agent` abortaba por completo si Codex se detectaba sin
    binario invocable.** Sembrar la detección de arriba expuso un defecto de
    producción real, no solo de tests: al detectar `codex` por configuración
    del proyecto (`AGENTS.md` heredado) sin que el binario `codex` exista en
    esta máquina, `install-agent` intentaba de todos modos ejecutar
    `codex mcp list` para el registro global y abortaba la instalación
    completa — incluida la de los demás agentes detectados en la misma
    pasada — con un error de proceso. Ahora, sin binario invocable, el
    registro global de MCP se omite con un aviso; los demás agentes se
    instalan con normalidad.

  Los cuatro defectos son independientes entre sí. Los dos tests de
  `tests/cli.rs` que ya fallaban en Windows antes de esta sesión se
  corrigieron en el mismo esfuerzo para que la matriz de alpha.8 pudiera
  quedar completamente verde.

- **Dogfood corrigió una falsa idempotencia de `init` y añadió acciones
  pre-hechas.** Si `.rationale/` ya existía, `cmd_init` emitía
  `already-initialized` y retornaba antes de `agents::install`; `update`
  solo registra Codex globalmente, así que un repo inicializado antes de
  instalar Claude Code quedaba permanentemente sin `.mcp.json`, bloque en
  `CLAUDE.md` ni manifest. El defecto se observó en Monorepo y afecta los
  cuatro repos piloto. Ahora `init` conserva el contrato JSON de una línea,
  respeta los dos mecanismos de skip y converge la configuración de agentes
  también al reinicializar. Rationale expone seis acciones desde una fuente
  única: prompts MCP (`preflight`, `explain`, `capture`, `review`, `health`,
  `protocol`) y skills de Claude Code `/rationale-*`. Los skills se escriben
  atómicamente, se registran con hash SHA-256 y `uninstall-agent` borra solo
  los intactos; un archivo editado por el usuario se conserva. La operación
  destructiva ya no comprueba un path y luego lo borra: reclama la identidad
  mediante rename atómico y publica reemplazos sin sobrescribir destinos que
  reaparezcan, cerrando la carrera TOCTOU de pathname documentada en ADR-0008.

- **Landing final para la validación alpha → beta.** Inglés y español ahora
  son rutas Astro estáticas (`/` y `/es/`) en vez de mutaciones de texto en
  JavaScript. Documentación abre el manual localizado, Instalar conserva su
  anchor, el Hero enlaza a un Quick Start que distingue slash commands reales
  de Claude Code de solicitudes escritas para Codex, y las navegaciones usan
  View Transitions MPA nativas con fallback normal y reduced motion.

- **`windows-latest` entró a CI real por primera vez y encontró dos defectos
  reales que ubuntu/macOS nunca podían detectar.** `cache::cache_root`
  usaba `$HOME` directo — inexistente en Windows — cuando ADR-0005 ya
  documentaba `%LOCALAPPDATA%\rationale\projects\...` como el candidato a
  implementar "cuando Fase J necesite resolver Windows de forma real"; ese
  momento es este. Implementado tal cual el ADR lo nombraba, sin decisión
  nueva. El test de timeout de proveedor (`provider_timeout_reports_unavailable_and_kills_process`)
  usa un mock en bash (`dd`, framing byte-exacto) que Windows no puede
  ejecutar como binario nativo — se salta explícitamente en Windows con la
  razón documentada en el propio test: es una limitación del fixture de
  prueba, no del código de producción bajo prueba (`spawn_with` siempre
  lanza un binario real, nunca un script, en cualquier plataforma).

- **Corrige que `prepare_change` descartaba `intent` en silencio.** El MCP
  exigía `mode: "intent-aware"` explícito además de `intent` para activar
  detección de conflictos; sin ese flag, `intent` se ignoraba sin ningún
  diagnóstico. `Rationale_v0.5.md §4.18` define el modo por la presencia de
  intención, no por un flag separado, y el prompt maestro documentado
  (`docs/prompt-master.md`) solo enseña `prepare_change(target, intent)` —
  nunca `mode`. Cualquier agente siguiendo el protocolo oficial reproducía
  exactamente el síntoma del bug real que motivó el proyecto: una intención
  contradictoria pasaba sin que Rationale la señalara. `mode: "baseline"`
  explícito sigue forzando retrieval puro sin detección, como override.

- **Fase A — instalación/actualización sin pérdida de datos.** Windows:
  `package.ps1` empaquetaba el ZIP con los archivos en la raíz mientras
  `rationale-installer.ps1` los buscaba en un subdirectorio (CI nunca lo
  detectó: solo compilaba ubuntu+macOS); ahora se corrige y `windows-latest`
  se agrega a CI con un smoke test real del layout del archivo. Un merge
  conflict que invierte los marcadores `rationale:begin`/`rationale:end`
  hacía panicar `install-agent`; ahora falla con un error legible sin tocar
  el archivo. `uninstall-agent` borraba el archivo entero para toda entrada
  que Rationale hubiera creado, incluso si el usuario le agregó contenido
  propio después (otro servidor MCP en el mismo `.mcp.json`, texto bajo el
  bloque de `CLAUDE.md`); ahora extirpa solo lo que Rationale escribió.
  `.mcp.json` no se actualizaba si el binario cambiaba de ruta. Las
  escrituras de `agents.rs` (`CLAUDE.md`, `AGENTS.md`, `.mcp.json`, el
  manifest) pasan a ser atómicas, mismo patrón que el canon YAML.

- **Fase B — corrección silenciosa en la cadena de gobernanza.**
  `finalize_change` solo ataba bindings desde el diff mecánico, nunca desde
  el target declarado — si el cambio real ya estaba commiteado antes de
  `base_revision`, el Record resultante nunca podía gobernar su propio
  target (mismo síntoma que el bug original del dogfood, causa distinta).
  Ahora se ata también el target declarado cuando resuelve a un archivo
  real. Además, `finalize_change` capturaba `AGENTS.md`/`CLAUDE.md`/
  `.mcp.json` como si fueran parte del cambio del usuario cuando `init`/
  `install-agent` los deja untracked (confirmado en dogfood real) — ahora
  se excluyen, igual que `.rationale/`. `schema_version` se escribía pero
  nunca se leía: `rationale doctor` ahora detecta versiones desconocidas.
  `approved_at` nunca se escribía en una `Approval` — de una aprobación solo
  quedaba el "quién", nunca el "cuándo". Una propuesta reclamada por
  `rationale review` cuyo proceso muere antes de promoverla/rechazarla
  quedaba huérfana para siempre en `.rationale/proposals/.in-review/`, sin
  ningún camino de recuperación pese a que los comentarios prometían que
  "queda recuperable"; `rationale doctor --repair` ahora la devuelve a
  `proposals/`.

- **Fase C — camino de migración para el canon legado.** `doctor` detectaba
  `RecordWithoutBindings` pero se negaba a repararlo ("inventar un binding
  sería peor que ninguno") — correcto como principio, pero sin salida para
  el canon que el productor roto de Fase 1 ya dejó escrito en cuatro repos.
  `rationale doctor --repair` ahora pide la ruta (y símbolo opcional) al
  humano, escribe el binding marcado `declared_by: human` — nunca
  indistinguible de uno que un proveedor estructural confirmó — con su
  propio evento de lifecycle. Rescata la evidencia de dogfood en vez de
  descartarla.

- **Fase D — observabilidad del ciclo de vida.** `rationale review-record`
  imprime ahora `approvals[]` (actor, autoridad, estado, `approved_at`) y el
  historial completo de `lifecycle.events[]` antes del menú de acción —
  antes había que leer el YAML a mano para auditar quién aprobó una
  decisión y cuándo. `kind: "exception"` estaba en el enum de
  `record.schema.json` desde el principio pero era inalcanzable:
  `finalize_change` no tenía parámetro `kind` y el productor hardcodeaba
  `"constraint"` siempre; ahora es un parámetro opcional validado, con
  `"constraint"` como default implícito (el comportamiento de antes).
  `review-record --project-root <ruta> <id>` ataba `record_id` al VALOR del
  flag en vez del id real cuando el flag venía primero — solo funcionaba en
  el orden documentado por casualidad; ahora el parseo de posicionales sabe
  qué flags llevan valor.

- **Fase E — cobertura que faltaba para poder afirmar beta con evidencia.**
  Cuatro áreas sin ningún test: que `uninstall-agent` de verdad conserva
  `.rationale/` (ambos instaladores lo *imprimían*, ninguno lo probaba);
  el exit code de `doctor --check` y la forma real de `doctor --json`;
  y `project_root` distinto de `repo_path` (canon en un repo, código en
  otro) — cableado desde el principio pero nunca ejercitado con dos repos
  Git reales. Las cuatro ya pasan.

## v0.1.0-alpha.7 — 2026-07-27

- Corrige el canal por defecto de `rationale-installer.sh/.ps1` y
  `rationale-update.sh/.ps1`: por defecto usaban `RATIONALE_CHANNEL=stable`,
  que resuelve la versión vía `GET /releases/latest` de GitHub. Ese endpoint
  excluye prereleases por diseño, y todas las alfas (incluida alpha.6) están
  marcadas como prerelease — así que "stable" resolvía silenciosamente a
  `v0.0.0-dogfood.7`, una Release anterior a `rationale-update.sh`. El
  instalador fallaba con un 404 real al pedir ese archivo a esa Release
  vieja. El canal por defecto pasa a `preview` mientras el proyecto sea
  pre-1.0, tal como ya establecía `docs/work-items/alpha-release-mcp-cleanroom-hardening.md`.
  Verificado end-to-end contra los assets reales de GitHub.

## v0.1.0-alpha.6 — 2026-07-27

- Cadena de gobernanza completa: bindings de archivo y símbolo, Subjects
  materializados, severidad tolerante, captura de cambios sin commit,
  resolución de conflictos y `rationale doctor` para canon legado.
- Chestie aparece en la revisión humana, `health`, la preparación y los
  instaladores, con globo dinámico y salida sobria para CI, pipes y
  `--no-mascot`.
- El prompt maestro vive en [`docs/prompt-master.md`](docs/prompt-master.md) y
  es la fuente que también consume `install-agent`.
- El sitio documental Astro queda disponible en `/docs/*` y `/es/docs/*`, con
  prompt maestro bilingüe, navegación, TOC y contenido operativo.

## v0.1.0-alpha.5 — 2026-07-27

Sin cambios funcionales sobre alpha.4 — release generada por el proceso de
fusión de PRs de la rama de release; el contenido real llegó en alpha.6.

## v0.1.0-alpha.4 — 2026-07-26

- Corrige el reporte de versión del servidor MCP para que las herramientas
  expongan la Release real del binario.
- Mantiene el pipeline de actualización preview robusto de alpha.2 y alpha.3.

## v0.1.0-alpha.3 — 2026-07-26

- Evita fallos por `SIGPIPE` en el helper de actualización preview al dejar de
  cerrar prematuramente el stream de Releases.

## v0.1.0-alpha.2 — 2026-07-26

- Endurece la actualización preview y la selección de Releases prerelease en
  los helpers Unix y PowerShell.

## v0.1.0-alpha.1 — 2026-07-26

- Publica la primera alfa empaquetada con binarios, checksums, instaladores y
  helper de actualización.
- Corrige el transporte del servidor MCP: Rationale habla JSON por línea en
  su frontera stdio, mientras conserva `Content-Length` únicamente como
  cliente hacia Codebase Memory.
- Hace `--help`, `--version` y las opciones inválidas no mutantes; antes,
  `rationale init --help` podía crear `.rationale/` y modificar archivos de
  instrucciones del agente.
- Añade `install-agent`, `uninstall-agent`, integración MCP e instrucciones de
  invocación con Chestie.

### Historia que precede la alfa

Las siete Releases `v0.0.0-dogfood.1`–`v0.0.0-dogfood.7` compartieron el bug
de framing que motivó [8ec97ea](https://github.com/Ragosorio/Rationale/commit/8ec97ea):
el servidor esperaba `Content-Length`, aunque el transporte stdio de MCP usa
un objeto JSON por línea. El proceso podía arrancar y parecer sano mientras
el handshake nunca completaba. La corrección fue separar ambos codecs; no
fue un fallo de Codebase Memory ni un problema de latencia.

El incidente de `SIGPIPE` fue posterior y distinto: lo introdujo el
endurecimiento de alpha.1 en el pipeline preview y quedó corregido en alpha.3.
La landing que apuntaba a `releases/latest` también era un problema separado:
GitHub entregaba `dogfood.7`, anterior a `install-agent`, al `--help` seguro y
al helper de actualización.

## v0.0.0-dogfood.7 — 2026-07-26

- MVP local instalable desde GitHub Release.
- CLI para `init`, `health`, `prepare`, `review` y `review-record`.
- Servidor MCP con `health`, `prepare_change`, `explain_target` y
  `finalize_change`.
- Lifecycle auditable de Records y autoridad declarada por proyecto.
- Artefactos multi-plataforma, checksums SHA-256 e instaladores.

Fue la última iteración dogfood antes de la alfa empaquetada. No debe usarse
como referencia pública actual: la evidencia de Release vigente es
`v0.1.0-alpha.6`.
