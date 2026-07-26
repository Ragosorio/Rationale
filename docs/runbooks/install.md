# Install

No hay empaquetado ni distribución todavía (Fase J, pendiente) — se corre desde el código fuente.

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

Cópialo (ajustando el `command` si no vas a correr `cargo run` desde el propio directorio del proyecto) al `.mcp.json` del proyecto donde quieras que un agente MCP (Claude Code, etc.) pueda llamar `prepare_change`/`explain_target`/`health`/`finalize_change`. **Requiere reiniciar la sesión del agente** para que cargue el archivo.

## Verificar la instalación

```bash
target/release/rationale health
```

Debe imprimir JSON con `project_id`, `git_revision`, `provider_status`. Ver [`diagnostics.md`](diagnostics.md) si algo falla.
