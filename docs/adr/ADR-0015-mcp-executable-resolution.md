# ADR-0015: Executable resolution in per-project MCP configuration

**Status:** proposed — pendiente de revisión cruzada independiente y aprobación humana antes de `accepted`.
**Date:** 2026-07-28
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno. Complementa ADR-0014, que sacó la ruta del binario de `CLAUDE.md` y `AGENTS.md` y dejó explícitamente este problema fuera de su alcance.

## Context

`install-agent` escribe el ejecutable de Rationale como **ruta absoluta** en los
archivos de configuración MCP por proyecto:

```json
{ "mcpServers": { "rationale": {
  "command": "/Users/roor.osorio/.local/bin/rationale", "args": ["serve"] } } }
```

Esos archivos —`.mcp.json` para Claude Code, `.cursor/mcp.json` para Cursor—
son configuración **compartida y versionada del proyecto**. En los dos repos
piloto, `.mcp.json` está comiteado y presente en `origin/main` con el `$HOME`
de una persona concreta dentro. Cualquier otro integrante que clone obtiene un
`command` que no existe en su máquina.

La justificación que se venía dando para la ruta absoluta es que un cliente MCP
lanzado como aplicación gráfica puede no heredar el `PATH` del shell, y que por
tanto un `"command": "rationale"` pelado fallaría con «command not found». Esa
premisa nunca se comprobó contra los clientes que Rationale realmente soporta.
Este ADR la comprueba.

## Decision

1. **`.mcp.json` (Claude Code) usa el comando lógico `"rationale"`, no una ruta
   absoluta.** La premisa del `PATH` no se sostiene para este cliente: se
   refutó empíricamente (ver Evidence). El archivo es compartido por diseño y
   debe ser portable.

2. **`.cursor/mcp.json` (Cursor) usa también el comando lógico `"rationale"`**,
   por consistencia y porque el archivo es igualmente compartido — pero **la
   herencia de `PATH` en Cursor no está verificada** y queda como riesgo
   declarado con `Revisit trigger` propio, no como supuesto silencioso.

3. **La configuración MCP compartida no contiene nada dependiente de la
   máquina.** Si en el futuro algún cliente exige una ruta absoluta
   demostrada, esa configuración va a un archivo local no versionado, nunca al
   compartido.

4. **El fallo de resolución debe ser diagnosticable, no misterioso.**
   `rationale doctor` es el lugar donde un `command not found` del cliente MCP
   debe traducirse en «el binario no está en el `PATH` que ve tu cliente; está
   en `<ruta>`». Un fallo silencioso de arranque de servidor MCP es
   indistinguible de otras diez causas.

5. **No se introduce ningún wrapper, script intermedio ni archivo
   `.mcp.json.example`.** Ver Alternatives.

## Evidence

**La premisa del `PATH` se refutó sobre este mismo repositorio, en ejecución.**
El `.mcp.json` de Rationale declara un comando **pelado**, sin ruta:

```json
{ "command": "cargo", "args": ["run", "--quiet", "--release", "--", "serve"] }
```

Claude Code arrancó el servidor con esa configuración y respondió a una llamada
real de la herramienta `health` durante la sesión en que se escribió este ADR:

```json
{"project_id":"rationale","provider_status":"successful",
 "git_revision":"0ecc5a275055ab1b7c7391cfb2a5625217614c76", ...}
```

`cargo` vive en `~/.cargo/bin/cargo`, un directorio que **no** está en el `PATH`
por defecto de macOS: lo añade `~/.profile` vía `. "$HOME/.cargo/env"`.

**Alcance exacto de lo que esto prueba, y lo que no.** Prueba que Claude Code
resuelve un comando pelado contra el `PATH` de su entorno, y que ese entorno
incluía `~/.cargo/bin`. **No prueba que Claude Code procese `~/.profile`**: el
mecanismo casi con certeza es que heredó el entorno del proceso que lo lanzó
—un shell interactivo, que sí había cargado el perfil—. La distinción importa,
porque un cliente lanzado desde Finder o Spotlight recibiría el entorno de
`launchd`, sin ninguna extensión de perfil, y ahí un comando pelado fallaría.

Tampoco prueba el directorio que importa: el binario de Rationale vive en
`~/.local/bin`, no en `~/.cargo/bin`. Que ambos estén en el `PATH` observado en
esta máquina, y que los añada el mismo mecanismo de perfil, hace la inferencia
razonable pero **no es comprobación directa**.

Dicho de forma exacta: la evidencia **refuta que la ruta absoluta sea
necesaria** en el modo de lanzamiento que se probó, y no más que eso. Las
validaciones #1 y #2 cierran ambos huecos.

**El daño de la alternativa actual sí está medido**, no supuesto: `.mcp.json`
comiteado con `/Users/roor.osorio/.local/bin/rationale` en Monorepo y BoostAPI,
ambos en `origin/main`. Y reinstalar con un binario distinto lo reescribe:
durante esta investigación pasó a `/Users/roor.osorio/Desktop/Rationale/target/
release/rationale`, produciendo churn en un archivo versionado por máquina *y*
por binario.

**Claude Desktop no es un consumidor de este archivo.** Usa
`claude_desktop_config.json`, no `.mcp.json`. El escenario de app gráfica que
motivaba la ruta absoluta no aplica al archivo que Rationale escribe para
Claude Code.

## Alternatives considered

- **Ruta absoluta versionada (statu quo).** Descartado: garantiza el fallo para
  todo el que no sea quien instaló, mete el `$HOME` de una persona en un
  archivo compartido, y produce churn por máquina y por binario. El único
  beneficio alegado —inmunidad al `PATH`— se refutó para Claude Code.

- **Wrapper estable (`./scripts/rationale` comiteado que resuelve el binario).**
  Descartado: Rationale añadiría un archivo ejecutable al repositorio del
  usuario para resolver un problema suyo. Es más invasivo que el problema, y
  traslada la resolución de `PATH` a un script que tiene exactamente la misma
  dificultad.

- **`.mcp.json` local no versionado + `.mcp.json.example` compartido.**
  Descartado por ahora: `.mcp.json` es el mecanismo que Claude Code define
  *como* configuración de proyecto compartida; sacarlo de Git rompe el «clonas
  y funciona» para el equipo, y obliga a cada integrante a correr
  `install-agent` antes de tener herramientas. Con la Decision #1 el archivo
  ya es portable y el problema que motivaba sacarlo desaparece. Reconsiderar
  solo si aparece un cliente que exija una ruta absoluta demostrada — ahí la
  parte dependiente de máquina va a un archivo local, no se ignora todo el
  compartido.

- **Configuración compartida + override local por integrante.** Descartado como
  diseño base: es la solución correcta para un problema que, tras la Decision
  #1, ya no existe. Añadir dos archivos y una precedencia entre ellos para un
  caso hipotético es complejidad sin evidencia que la pida.

- **Detección específica por cliente (absoluta para unos, lógica para otros).**
  Descartado: produce dos comportamientos que mantener y documentar, y el
  cliente donde la premisa se refutó es justamente el mayoritario. Si Cursor
  resulta necesitar otra cosa, se decide entonces con evidencia — el
  `Revisit trigger` lo cubre.

## Consequences

- Un integrante que clone cualquiera de los pilotos obtiene un `.mcp.json`
  funcional en cuanto tenga `rationale` instalado, sin correr nada.
- `.mcp.json` deja de producir diffs al cambiar de máquina o de binario. Deja
  de ser un archivo que ensucia el árbol de trabajo del equipo.
- El repositorio de Rationale conserva `cargo run --quiet --release -- serve`
  en su propio `.mcp.json`: aquí el servidor se construye desde el fuente, no
  se instala. `install-agent` lo reescribirá a `"rationale"` si se corre en
  este repo, y hay que revertirlo — la misma fricción que ya existe y que este
  ADR no resuelve.
- Si el binario no está en el `PATH` del cliente, el servidor no arranca. La
  Decision #4 existe para que eso sea diagnosticable en vez de silencioso.

## Risks

- **Cursor podría no heredar el `PATH`.** Es una app Electron y su
  comportamiento no se verificó. Mitigación: la Decision #2 lo declara como
  riesgo abierto, no como supuesto; la validación #2 lo cierra. Si falla, la
  corrección es acotada — un solo `AgentTarget`.

- **Un usuario con Rationale fuera del `PATH`.** Instalación manual en una ruta
  no estándar, o `~/.local/bin` sin exportar. Antes «funcionaba» porque la ruta
  absoluta lo tapaba; ahora falla. Mitigación: Decision #4, y el instalador ya
  avisa cuando el directorio de instalación no está en el `PATH`.

- **Un cliente podría recibir un `PATH` sin el directorio de instalación.**
  Riesgo residual real y **abierto**. La validación #1 lo cierra únicamente
  para Claude Code en la máquina y modalidad de lanzamiento probadas; como el
  mecanismo por el que ese cliente obtuvo su `PATH` no se determinó, el
  resultado no puede extrapolarse a otros clientes, otras máquinas ni otras
  formas de arranque. Cursor sigue sin probar (validación #2).

  **Se acepta a sabiendas, no por descuido**, porque la comparación es
  asimétrica: la ruta absoluta falla para *todo* integrante que no sea quien
  instaló —con certeza, ya medido en dos pilotos— mientras que el comando
  lógico falla solo en los modos de lanzamiento donde el `PATH` no se hereda, y
  falla de forma diagnosticable (Decision #4). Cambiar un fallo seguro y
  silencioso por uno condicional y diagnosticable es una mejora aunque el
  segundo no sea cero. Las validaciones #1 y #2 acotan cuánto vale ese «solo».

- **Regresión silenciosa al reinstalar en el repo de Rationale.** Ver
  Consequences. Riesgo aceptado y documentado; el arreglo real sería que
  `install-agent` detecte que el proyecto *es* Rationale, y eso no justifica
  código de producción hoy.

## Validation

Pendiente de implementación. Exigida antes de `accepted`:

**Binario bajo prueba**, para que la evidencia quede atada a un ejecutable
concreto y no a «alguna instalación»:

```
command -v rationale  → /Users/roor.osorio/.local/bin/rationale
rationale --version   → rationale v0.1.0-alpha.7
shasum -a 256         → 1933981a5dafdf020fcb4f2060c5bf0acd61678b2cd953ab4a643517b4f063fe
```

**Confundidor ya eliminado.** Se verificó que ese binario habla MCP
correctamente al invocarse directamente: `initialize` devuelve
`{"name":"rationale","version":"v0.1.0-alpha.7"}` y `tools/call health`
responde, sobre el banco `~/Desktop/rationale-path-test`, usando el transporte
stdio de un objeto JSON por línea (`src/mcp/framing.rs` — *no*
`Content-Length`, que es el codec de Codebase Memory). El binario de `main`
hace lo mismo. Por tanto, un fallo en las pruebas de cliente aísla a la
resolución del `PATH` y a nada más.

1. **Comprobación directa con `~/.local/bin` en Claude Code — ✅ PASÓ
   (2026-07-28).** Banco `~/Desktop/rationale-path-test` con `.mcp.json`
   conteniendo `"command": "rationale"`, tras cerrar y reabrir el cliente.

   La comprobación no se hizo llamando a `health` desde el chat —el cliente no
   completaba la generación, por causas ajenas a Rationale— sino **inspeccionando
   el proceso que el cliente había lanzado**, que es evidencia más directa:

   ```
   PID 52676   rationale serve          ← comando pelado, tal como en .mcp.json
   cwd         ~/Desktop/rationale-path-test
   txt         /Users/roor.osorio/.local/bin/rationale
   PATH        …:/Users/roor.osorio/.local/bin:…
   ```

   El descriptor `txt` es el ejecutable que el kernel cargó: prueba que el
   cliente resolvió `rationale` **a `~/.local/bin`**, el directorio que la
   Evidence no cubría. Cierra ese hueco.

   **Qué se observó exactamente sobre el `PATH`:** el proceso recibió un
   `PATH` que contenía `~/.local/bin` (aparecía dos veces). **El mecanismo por
   el cual el cliente obtuvo ese `PATH` no se determinó.** Podría ser
   resolución del perfil del usuario, herencia del entorno de un proceso
   ancestro, o configuración propia del cliente; nada de lo medido distingue
   entre esas hipótesis, y la duplicación por sí sola no prueba ninguna.

   **Alcance de lo que esta validación cierra:** el riesgo de resolución de
   `PATH` queda cerrado **para Claude Code, en esta máquina y en esta
   modalidad de lanzamiento**, y nada más. No se cierra para Cursor
   (validación #2, pendiente), ni para otras modalidades de lanzamiento de
   este mismo cliente, ni para clientes MCP no probados. El riesgo declarado
   en §Risks se mantiene abierto para todos ellos.

2. **Resolución en runtime en Cursor — ⏳ `pending validation`.** No ejecutada:
   `cursor-agent` no está en el `PATH` de la máquina de prueba. Se declara
   pendiente, **no** comprobada, y ADR-0015 no puede pasar a `accepted` sin
   ella o sin una decisión explícita de aceptar el riesgo para Cursor.

   Banco con `.cursor/mcp.json` escrito
   **a mano** con el mismo comando pelado, tras cerrar y reabrir Cursor.

   **Alcance exacto de esta prueba:** demuestra únicamente que Cursor resuelve
   y ejecuta el comando lógico desde `~/.local/bin`. **No** demuestra que
   `install-agent` detecte Cursor ni que genere ese archivo — en la máquina de
   prueba `cursor-agent` no está en el `PATH`, así que la detección no se
   ejercitó. Esa segunda propiedad queda cubierta por
   `no_target_writes_an_absolute_command_into_shared_mcp_config`, que recorre
   `TARGETS` incluyendo Cursor, y por `only_claude_code_declares_a_skills_directory`.
   Registrar la prueba manual como si también hubiera comprobado la detección
   sería exactamente el tipo de generalización que invalidó ADR-0012.

   Si no hay Cursor funcional disponible, esta validación queda como
   **`pending validation`** explícito — nunca como comprobada.
3. **Test de regresión** que falle si `upsert_mcp_json` escribe una ruta
   absoluta en cualquier `mcp_config_file` de `TARGETS`.
4. **Instalación en dos rutas distintas del mismo proyecto** (copiado), para
   confirmar que el `.mcp.json` resultante es byte-idéntico — hoy no lo es.

**La validación no puede ser inspección del archivo generado.** Que el JSON
«se vea bien» no prueba que el cliente arranque el servidor; solo una llamada
real a una herramienta MCP lo prueba.

## Revisit trigger

Reabrir si: (a) la validación #2 muestra que Cursor no resuelve comandos por
`PATH`; (b) se añade a `TARGETS` un cliente lanzado como app gráfica que
consuma un archivo versionado; o (c) aparece un informe real de «command not
found» al arrancar el servidor MCP en una instalación estándar.
