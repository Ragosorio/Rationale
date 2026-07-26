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

## 3. Proveedor estructural (opcional pero recomendado)

Rationale funciona sin un proveedor de inteligencia de código, pero con cobertura degradada (`provider_status: unavailable`, nunca bloquea). Para cobertura completa, instala [`codebase-memory-mcp`](../research/codebase-memory/) y verifica que esté en el `PATH`:

```bash
which codebase-memory-mcp
```

## 4. Registrar el servidor MCP para un agente

El repo de Rationale mismo trae [`.mcp.json`](../../.mcp.json) de ejemplo:

```json
{
  "mcpServers": {
    "rationale": {
      "command": "cargo",
      "args": ["run", "--quiet", "--release", "--", "serve"]
    }
  }
}
```

Cópialo (ajustando el `command` si no vas a correr `cargo run` desde el propio directorio del proyecto) al `.mcp.json` del proyecto donde quieras que un agente MCP (Claude Code, etc.) pueda llamar `prepare_change`/`explain_target`/`health`/`finalize_change`. Para Codex global usa `codex mcp add rationale -- /ruta/al/rationale serve`. **Requiere reiniciar la sesión del agente** para que cargue el archivo.

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
