# Changelog

Los cambios importantes se registran aquí por Release. El detalle técnico de
cada cambio vive en commits, ADRs y work items enlazados.

## Sin publicar

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
