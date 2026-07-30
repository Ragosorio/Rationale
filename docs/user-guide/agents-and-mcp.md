# Agentes y MCP

Rationale puede funcionar solo por CLI, pero su flujo principal está pensado
para que un agente consulte contexto sin recibir autoridad.

## Integración automática

Después de instalar el binario:

```bash
rationale init --skip-agent-config
rationale install-agent --dry-run
rationale install-agent
```

`install-agent` detecta Claude Code, Codex o Cursor, escribe bloques delimitados
e idempotentes en el proyecto y registra el servidor MCP una vez en la
configuración global del usuario. El registro usa la ruta absoluta del binario
instalado para no depender del `PATH` de una aplicación gráfica. Reinicia la
sesión del agente después de instalar una configuración nueva.

El texto que `install-agent` escribe por defecto es el [prompt maestro](../prompt-master.md);
la [versión en español](../prompt-master.es.md) está disponible para equipos que
trabajan en español. La landing cambia entre ambos al cambiar de idioma y cada
uno tiene una fuente canónica, para que la instrucción instalada, la
documentación y el bloque copiable no se separen.

`rationale serve` es un servidor stdio: permanece abierto y no muestra un
banner en stdout. El agente debe enviarle JSON-RPC por líneas; una ejecución
manual que parece silenciosa está esperando tráfico, no bloqueada.

Para revertir exactamente esos cambios:

```bash
rationale uninstall-agent
rationale uninstall-agent --global-only
```

## Herramientas MCP

El servidor expone:

- `health`
- `prepare_change`
- `explain_target`
- `finalize_change`

MCP no expone aprobación, revocación, superseder ni cambio de autoridad. Esas
operaciones requieren la CLI interactiva y una persona.

## Configuración manual

El repositorio incluye `.mcp.json` como configuración de desarrollo. El
instalador administra automáticamente `~/.claude.json`,
`~/.cursor/mcp.json` y Codex. Para registrar Codex manualmente:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve
```

Revisa el archivo de configuración de tu agente antes de versionarlo: nunca
incluyas tokens, claves privadas ni rutas sensibles.

## Proveedor estructural

Codebase Memory es opcional. Si no está disponible, `health` reportará cobertura
degradada y las respuestas incluirán advertencias honestas. Rationale nunca lee
la SQLite interna del proveedor.
