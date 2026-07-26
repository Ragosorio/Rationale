# Work item: alpha-release-mcp-cleanroom-hardening

## Problema

La prueba cleanroom del binario público encontró tres superficies que hoy se
confunden entre sí:

1. La landing instala `v0.0.0-dogfood.7` porque esa sigue siendo la Release
   pública más reciente. El código de `release/v0.1.0-alpha.1`, incluyendo
   `install-agent`, `uninstall-agent` y el `--help` no mutante, todavía no está
   publicado.
2. `rationale serve` permanece abierto correctamente cuando se ejecuta a mano,
   pero no completa el handshake con Codex. El servidor de Rationale usa
   framing `Content-Length`; el transporte stdio MCP estándar usa mensajes JSON
   delimitados por newline.
3. Una ruta explícita sin `.rationale/` llega a `expect(...)` y produce panic en
   comandos como `health`, en vez de un error de usuario controlado.

Hay además una ambigüedad de versión: `0.9.0` corresponde a Codebase Memory en
la conversación cleanroom, no a una Release de Rationale. El siguiente tag
planeado en este repositorio es `v0.1.0-alpha.1`; no existen tags públicos
`dogfood.8`, `dogfood.9` ni `v0.9.0`.

## Objetivo

Publicar una alpha instalable cuya web, instalador y binario declaren la misma
versión; cuyo servidor MCP complete un handshake real con Codex; y cuyo CLI
convierta entradas inválidas en errores claros sin panic ni backtrace.

## Non-goals

- No rediseñar la landing.
- No modificar Codebase Memory ni leer sus internals.
- No adoptar `rmcp` automáticamente: primero se evalúa la corrección mínima
  conforme al transporte stdio estándar.
- No agregar un daemon ni hacer que `serve` se desacople del proceso padre.
- No ocultar que `rationale serve` espera tráfico por stdin cuando se ejecuta
  directamente.
- No aprobar automáticamente ADRs, Subjects, propuestas ni Records.
- No continuar el piloto funcional hasta que el handshake MCP real pase.

## Base revision

`fa1c1cacb188b87081ec670466cdf43f36123169`
(`release/v0.1.0-alpha.1`).

## Evidencia

### Cobertura y revisión

- Codebase Memory estaba `ready` para
  `Users-roor.osorio-Desktop-Rationale`, con 3.202 nodos y 6.672 aristas.
- Se usó el grafo para localizar `src/mcp/server.rs::run`,
  `ProviderHandle::spawn`, `CodebaseMemoryClient::spawn_with`,
  `pipeline::health` y los tests MCP.
- Se revisaron directamente los archivos no cubiertos por el grafo:
  instaladores, workflow de Release, landing, runbooks y ADRs.
- Warning de cobertura: no se pudo resolver el deployment exacto de Sites;
  el repositorio no contiene `.openai/hosting.json` y la cuenta conectada no
  enumeró ningún proyecto. La fuente local y el `dist` sí declaran
  `dogfood.7` y usan `releases/latest`.

### Release e instalador

- GitHub reportó como única Release `Latest` a
  `v0.0.0-dogfood.7`, publicada el 2026-07-26, con los instaladores y paquetes
  esperados.
- `git tag` local termina también en `v0.0.0-dogfood.7`.
- La landing ejecuta
  `releases/latest/download/rationale-installer.sh`; el instalador consulta
  `releases/latest`. Por tanto, descargar `dogfood.7` es hoy el comportamiento
  correcto de esas dos piezas, no un fallo de caché de Sites.
- El commit base tiene dos ejecuciones de CI verdes (push y pull request), pero
  no tiene tag ni Release.

### Handshake MCP

- La especificación MCP para stdio exige JSON-RPC delimitado por newline.
- `src/mcp/framing.rs`, `src/mcp/server.rs` y `tests/mcp_server.rs` usan
  exclusivamente `Content-Length`.
- Reproducción contra el binario real del commit base:
  - un `initialize` delimitado por newline terminó sin respuesta;
  - el mismo `initialize` con `Content-Length` recibió una respuesta válida.
- Esto explica el síntoma exacto de Codex: cliente y servidor quedan esperando
  delimitadores distintos hasta que el cliente reporta timeout.
- Aumentar `startup_timeout_sec` no corrige una incompatibilidad de framing.
- El servidor además crea `ProviderHandle` antes de leer `initialize`; el
  handshake del servidor no debería depender del arranque ni de la salud de
  Codebase Memory.

### Panic de CLI

- `cmd_health` acepta directamente el valor de `--project-root`.
- `pipeline::health` llama
  `configuration::load(project_root).expect("cargar configuración")`.
- Una ruta equivocada produce `NoRationaleDirFound` y panic. El error ya tiene
  una representación legible en `ConfigError`; se está descartando por el
  `expect`.

### Help público

- El commit base ya incluye ayuda global y por subcomando, junto con tests que
  verifican que `--help` no crea `.rationale/`, archivos de agentes ni
  configuración MCP.
- La Release `dogfood.7` antecede esos cambios; por eso el usuario instalado no
  ve `install-agent` ni `uninstall-agent`.

## Riesgos

- Cambiar el framing compartido sin separar roles rompería el cliente de
  Rationale hacia Codebase Memory, que actualmente usa `Content-Length`.
- Responder `initialize` con una versión de protocolo incompatible puede hacer
  que Codex cierre la sesión aun después de corregir el framing.
- Marcar la alpha como GitHub prerelease y conservar `releases/latest` puede
  hacer que la landing siga instalando `dogfood.7`: GitHub no trata prereleases
  como la Release estable `latest`.
- Publicar antes del smoke cleanroom repetiría la instalación de un binario
  cuyo contenido no coincide con la documentación.
- Convertir solo `health` a `Result` dejaría panics equivalentes en `prepare`,
  `review`, `review-record`, `install-agent` y `uninstall-agent`.

## Plan

### P0 — bloquear promoción y fijar el contrato de versión

1. No crear `v0.1.0-alpha.1` hasta cerrar P1-P4.
2. Definir en ADR-0010 dos canales explícitos:
   - `stable`: puede usar GitHub `releases/latest`;
   - `preview`: debe apuntar a un tag alpha explícito o a un manifiesto
     versionado que incluya prereleases.
3. Elegir una única fuente de versión para paquete, binario, instalador,
   landing, runbook y notas de Release.
4. Añadir `rationale --version` y hacer que el paquete de Release reporte el
   tag real, no únicamente `Cargo.toml = 0.0.0`.

### P1 — corregir el servidor MCP

1. Reabrir ADR-0007 con la evidencia cleanroom. La decisión compartida
   `Content-Length` no es conforme para el lado servidor stdio.
2. Separar los codecs por frontera:
   - cliente Rationale → Codebase Memory: conservar el framing que exige el
     proveedor mientras ese sea su contrato real;
   - servidor Rationale ← Codex: JSON por línea conforme a MCP stdio.
3. Responder `initialize` y `tools/list` antes de arrancar Codebase Memory.
   Crear el provider de forma lazy en la primera herramienta que lo necesite.
4. Negociar y probar una versión de protocolo aceptada por Codex; no inferir
   compatibilidad por número de versión del proveedor.
5. Mantener stdout exclusivamente para mensajes MCP y stderr para
   diagnósticos.

### P2 — eliminar panics de entradas de usuario

1. Hacer que `pipeline::health`, `prepare`, `explain` y `finalize` propaguen
   errores tipados en lugar de usar `expect` para configuración o I/O
   esperable.
2. Centralizar la resolución y validación de `--project-root`.
3. Convertir errores CLI en mensaje breve + exit code distinto de cero, sin
   `thread 'main' panicked` ni sugerencia de `RUST_BACKTRACE`.
4. Convertir los mismos errores en `isError: true` para MCP sin depender de
   `catch_unwind` como control de flujo.
5. Auditar los demás subcomandos que aceptan `--project-root`.

### P3 — completar ayuda y diagnóstico

1. Conservar los tests actuales de `--help` no mutante.
2. Ampliar `rationale serve --help` para explicar que el proceso usa stdio,
   permanece abierto y normalmente lo inicia el agente.
3. Añadir `--version` a la ayuda global y a la prueba de paquete.
4. Documentar cómo distinguir una llamada MCP real de una ejecución del CLI:
   `Called rationale.health` frente a `Ran rationale health`.
5. No imprimir banners en stdout durante `serve`.

### P4 — hacer coherentes web, Release e instalador

1. Quitar el string `dogfood.7` duplicado en Astro y JavaScript y derivarlo de
   una sola fuente versionada.
2. Para la etapa alpha, mostrar e instalar explícitamente
   `v0.1.0-alpha.1`; no depender de `releases/latest` si la Release será
   prerelease.
3. Hacer que el instalador falle claramente si no puede resolver un tag o si
   faltan assets, y que imprima versión, origen y ruta instalados.
4. Añadir una verificación CI que compare tag, nombres de assets, salida de
   `rationale --version`, comando de la landing y runbook.
5. Recuperar o persistir `.openai/hosting.json` antes del siguiente deploy para
   que la versión publicada sea auditable desde el repositorio.

### P5 — validación y publicación

1. Ejecutar formatter, clippy, tests release, audit y build de la landing.
2. Ejecutar conformance MCP con un cliente newline independiente y, después,
   con Codex real:
   - handshake sin timeout;
   - `tools/list` devuelve cuatro herramientas;
   - llamada nativa a `health`;
   - provider degradado sin Codebase Memory y completo tras indexar.
3. Ejecutar smoke del instalador sobre los paquetes producidos antes de
   publicar.
4. Obtener revisión adversarial independiente del transporte, errores y
   release contract.
5. Publicar la alpha, desplegar la landing exacta y repetir el cleanroom desde
   un usuario macOS nuevo.

## Tests

### MCP

- `initialize` newline responde dentro de un deadline corto aun cuando
  Codebase Memory no existe o tarda.
- `notifications/initialized`, `tools/list` y llamadas consecutivas usan un
  JSON por línea y mantienen stdout limpio.
- `health`, `prepare_change`, `explain_target` y `finalize_change` aparecen en
  una sesión real de Codex.
- Un mensaje malformado devuelve error JSON-RPC y la sesión sigue viva.
- El cliente interno hacia Codebase Memory conserva sus contract tests
  separados; ningún test comparte implícitamente el framing de ambas
  fronteras.

### CLI

- `health --project-root <sin-.rationale>` sale no-cero con error legible y sin
  panic.
- La misma invariante cubre `prepare`, `review`, `review-record`,
  `install-agent` y `uninstall-agent`.
- Todos los `--help` salen cero y no escriben archivos.
- `serve --help` termina de inmediato.
- `--version` coincide con el tag empaquetado.

### Release y web

- Cada asset esperado existe y su checksum pasa.
- El instalador pinned instala exactamente el tag pedido.
- El comando mostrado por la landing instala esa misma versión.
- Reinstalar es idempotente y no duplica MCP ni instrucciones.
- Desinstalar preserva `.rationale/`.
- El HTML construido no contiene referencias stale a `dogfood.7`.

### Quality gates

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release
cargo audit
npm --prefix site run build
```

## Docs

- Revisar ADR-0007 y ADR-0010 sin autoaceptarlos.
- Actualizar `docs/architecture/code-map.md` para separar ambos codecs.
- Actualizar `docs/runbooks/diagnostics.md`: stdio newline, proceso silencioso
  esperado y prueba nativa desde Codex.
- Actualizar runbooks de instalación, Release y rollback con los canales
  `stable`/`preview`.
- Actualizar quickstart, referencia CLI y guía de agentes.
- Registrar el resultado cleanroom con versión, SHA, OS, Codex y evidencia de
  llamada MCP nativa.

## Criterio de éxito

- La landing instala el tag que muestra.
- `rationale --version` confirma ese mismo tag.
- `rationale --help` muestra todos los comandos públicos.
- Ningún subcomando hace panic ante una ruta de usuario inválida.
- Codex inicia Rationale sin alerta de timeout y llama `rationale.health` como
  herramienta MCP, no mediante shell.
- La prueba sin proveedor degrada honestamente y la prueba con índice reporta
  cobertura real.
- CI y todos los quality gates pasan.
- Una revisión independiente intenta falsificar la solución.
- Los ADRs afectados quedan revisados pero solo el humano decide su estado.
- La prueba cleanroom completa pasa desde instalación hasta persistencia tras
  reinstalación, sin borrar `.rationale/`.

## Estado de implementación — 2026-07-26

Se implementaron los cambios P1-P4 y la validación local de P5:

- `serve` usa JSON por línea para su frontera MCP y mantiene `Content-Length`
  únicamente para el cliente interno de Codebase Memory; el proveedor se
  inicia de forma lazy después del handshake.
- `health`, `prepare`, `explain`, `finalize`, `review`, `review-record`,
  `install-agent`, `uninstall-agent` e `init` convierten rutas o errores de
  configuración esperables en mensajes controlados, sin panic.
- `--help`, `--version` y `rationale update` están cubiertos por tests; el
  instalador instala el helper de actualización junto al binario.
- El helper distingue `stable` y `preview`: la alfa empaquetada busca la
  prerelease más reciente y no puede degradar silenciosamente a `latest`
  estable.
- La landing y el quickstart apuntan explícitamente a `v0.1.0-alpha.4` para
  no confundir una prerelease con GitHub `latest`; el workflow marca tags con
  guion como prerelease y publica los helpers.
- Pasaron `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --release`, `cargo audit --no-fetch --no-yanked`,
  `bash scripts/check-docs.sh` y `npm run build` en `site/`.
- El smoke independiente `initialize` por stdin/stdout respondió en menos de
  un segundo y los 11 tests MCP mantuvieron sesiones persistentes.

Las primeras publicaciones de la alfa (`v0.1.0-alpha.1` a
`v0.1.0-alpha.3`) expusieron defectos en el
pipeline preview del helper (`curl` recibía SIGPIPE por un `head` prematuro).
La corrección se publicará como `v0.1.0-alpha.4`; la landing y los runbooks
apuntan a ese tag para que la ruta de actualización sea realmente usable.

Pendiente fuera del checkout local: repetir la instalación desde una cuenta
macOS limpia y desplegar la landing mediante Sites. El conector de Sites no
expuso un proyecto ni un `.openai/hosting.json` en este entorno; no se simuló
ese despliegue.
