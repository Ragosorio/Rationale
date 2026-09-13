# Install

Rationale se distribuye como binario verificable desde GitHub Releases para
macOS (ARM64 y x86_64), Linux (x86_64 y ARM64) y Windows x86_64. El camino de
compilación desde fuente queda para desarrollo; los usuarios no necesitan Rust.

## Instalación desde GitHub

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
```

Variables soportadas:

- `RATIONALE_CHANNEL`: `stable` (por defecto desde 1.0) resuelve
  `GET /releases/latest` de GitHub, que excluye pre-releases; `preview` resuelve
  la Release más reciente, sea pre-release o no. ADR-0010 distinguía ambos
  canales mientras el proyecto era pre-1.0; con una versión estable publicada,
  una `-rc` posterior no debe llegar a quien instala sin pedirla.
- `RATIONALE_VERSION=v1.0.0` fija una versión concreta.
- `RATIONALE_INSTALL_DIR=$HOME/.local/bin` cambia el destino.
- `RATIONALE_SKIP_AGENT_CONFIG=1` evita registrar los agentes.

El script descarga el artefacto de la plataforma, comprueba SHA-256, instala
`rationale` y el helper `rationale-update`, ejecuta
`rationale install-agent --global-only` para registrar el servidor MCP en los
agentes detectados y deja intactos todos los `.rationale/`.

## 1. Compilar

```bash
cd /ruta/a/Rationale
cargo build --release
```

Produce `target/release/rationale`.

## 2. Inicializar un proyecto

Desde la raíz del proyecto que quieras gobernar con Rationale (puede ser el propio repo de Rationale — así es como se hizo dogfooding desde Fase D):

```bash
/ruta/a/Rationale/target/release/rationale init
```

Crea `.rationale/{subjects,records,approvals,schemas,migrations}/`. Los Records aparecen en `records/` cuando un agente captura conocimiento durable con `finalize_change`.

**`init` también detecta y avisa a los agentes de código presentes** (paso 4, ver abajo) — no es un paso manual separado a menos que lo desactives con `rationale init --skip-agent-config` o `RATIONALE_SKIP_AGENT_CONFIG=1`.

## 3. Proveedor estructural (opcional pero recomendado)

Rationale funciona sin un proveedor de inteligencia de código, pero con cobertura degradada (`provider_status: unavailable`, nunca bloquea). Para cobertura completa, instala [`codebase-memory-mcp`](../research/codebase-memory/) y verifica que esté en el `PATH`:

```bash
which codebase-memory-mcp
```

## 4. Registrar el servidor MCP para un agente

El instalador ya registró el servidor MCP y `rationale init` escribió las instrucciones del proyecto — este paso es para volver a ejecutarlo a mano, en un proyecto donde instalaste un agente nuevo después del `init`, o para revisar exactamente qué escribiría antes de tocar nada:

```bash
rationale install-agent                     # detecta claude-code/codex/cursor-agent y escribe/actualiza instrucciones, skills y registro MCP
rationale install-agent --dry-run           # imprime qué haría sin escribir nada
rationale install-agent --project-root <p>  # apunta a un proyecto distinto del cwd
```

Detecta el agente por binario en `PATH` (`claude`, `codex`, `cursor-agent`) o
por configuración ya presente. Escribe un bloque delimitado e idempotente en
`CLAUDE.md`/`AGENTS.md`/`.cursor/rules/rationale.mdc` y registra el MCP
globalmente para Claude Code, Codex y Cursor con la ruta absoluta del binario
instalado, como `serve --client <agente>`. Un registro anterior del mismo
binario (`serve` a secas) se migra; las entradas MCP por proyecto de versiones
antiguas se extirpan solo si conservan la forma conocida de Rationale, y otros
servidores se preservan.
Revertir el proyecto: `rationale uninstall-agent`. Revertir el registro del
usuario: `rationale uninstall-agent --global-only` — ver
[`uninstall.md`](uninstall.md).

**Requiere reiniciar la sesión del agente** para que cargue la configuración nueva.

## Actualizar y desinstalar

Después de una instalación nueva, actualiza el binario con:

```bash
rationale update
```

Para un usuario que todavía tiene una versión anterior a la que incluye el
helper, ejecuta una vez:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-update.sh | sh
```

El helper respeta `RATIONALE_CHANNEL` y `RATIONALE_VERSION` si se fijan para
rollback o preview.
El rollback consiste en reinstalar una versión anterior fijando esa variable.
La desinstalación elimina solo el binario; nunca borra `.rationale/` de un
proyecto automáticamente. Ver [`uninstall.md`](uninstall.md).

## Verificar la instalación

```bash
rationale health
```

Debe imprimir JSON con `project_id`, `git_revision`, `provider_status`. Después
de migrar desde una versión anterior a 1.0, ejecuta también
`rationale migrate --dry-run` por si quedan propuestas pendientes. Ver
[`diagnostics.md`](diagnostics.md) si algo falla.
