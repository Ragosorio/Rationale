# Changelog

Los cambios importantes se registran aquí por Release. El detalle técnico de
cada cambio vive en commits, ADRs y work items enlazados.

## Unreleased

- Cadena de gobernanza completa: bindings de archivo y símbolo, Subjects
  materializados, severidad tolerante, captura de cambios sin commit,
  resolución de conflictos y `rationale doctor` para canon legado.
- Chestie aparece en la revisión humana, `health`, la preparación y los
  instaladores, con globo dinámico y salida sobria para CI, pipes y
  `--no-mascot`.
- El prompt maestro vive en [`docs/prompt-master.md`](docs/prompt-master.md) y
  es la fuente que también consume `install-agent`.

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
`v0.1.0-alpha.4`.
