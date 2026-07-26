# Install

La alfa se distribuye como binario verificable desde GitHub Releases. El camino
de compilación desde fuente queda disponible para desarrollo; los usuarios no
necesitan instalar Rust.

## Instalación desde GitHub

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
```

Variables soportadas:

- `RATIONALE_VERSION=v0.1.0-alpha.1` fija una versión concreta.
- `RATIONALE_INSTALL_DIR=$HOME/.local/bin` cambia el destino.
- `RATIONALE_SKIP_AGENT_CONFIG=1` evita editar la configuración MCP de Codex.

El script descarga el artefacto de la plataforma, comprueba SHA-256, instala
`rationale` y deja intactos todos los `.rationale/` de los proyectos.

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

Crea `.rationale/{subjects,records,bindings,approvals,schemas,migrations}/`. `proposals/` se crea automáticamente la primera vez que `finalize_change` escribe una propuesta — no hace falta crearlo a mano.

**`init` también detecta y avisa a los agentes de código presentes** (paso 4, ver abajo) — no es un paso manual separado a menos que lo desactives con `rationale init --skip-agent-config` o `RATIONALE_SKIP_AGENT_CONFIG=1`.

## 3. Proveedor estructural (opcional pero recomendado)

Rationale funciona sin un proveedor de inteligencia de código, pero con cobertura degradada (`provider_status: unavailable`, nunca bloquea). Para cobertura completa, instala [`codebase-memory-mcp`](../research/codebase-memory/) y verifica que esté en el `PATH`:

```bash
which codebase-memory-mcp
```

## 4. Registrar el servidor MCP para un agente

`rationale init` ya lo hace por ti (ver paso 2) — este paso es solo para volver a ejecutarlo a mano, en un proyecto donde instalaste un agente nuevo después del `init`, o para revisar exactamente qué escribiría antes de tocar nada:

```bash
rationale install-agent                     # detecta claude-code/codex/cursor-agent y escribe/actualiza sus instrucciones + registro MCP
rationale install-agent --dry-run           # imprime qué haría sin escribir nada
rationale install-agent --project-root <p>  # apunta a un proyecto distinto del cwd
```

Detecta el agente por binario en `PATH` (`claude`, `codex`, `cursor-agent`) o por configuración ya presente en el proyecto, y escribe un bloque delimitado e idempotente en `CLAUDE.md`/`AGENTS.md`/`.cursor/rules/rationale.mdc` más el registro MCP correspondiente (`.mcp.json`, `.cursor/mcp.json`, o `codex mcp add` global). Nunca sobrescribe contenido previo del usuario. Revertir: `rationale uninstall-agent` — ver [`uninstall.md`](uninstall.md).

**Requiere reiniciar la sesión del agente** para que cargue la configuración nueva.

## Actualizar y desinstalar

Reinstalar con `RATIONALE_VERSION` vacío actualiza a la Release más reciente.
El rollback consiste en reinstalar una versión anterior fijando esa variable.
La desinstalación elimina solo el binario; nunca borra `.rationale/` de un
proyecto automáticamente. Ver [`uninstall.md`](uninstall.md).

## Verificar la instalación

```bash
target/release/rationale health
```

Debe imprimir JSON con `project_id`, `git_revision`, `provider_status`. Ver [`diagnostics.md`](diagnostics.md) si algo falla.
