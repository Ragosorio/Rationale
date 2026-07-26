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
e idempotentes y registra el servidor MCP correspondiente. Reinicia la sesión
del agente después de instalar una configuración nueva.

`rationale serve` es un servidor stdio: permanece abierto y no muestra un
banner en stdout. El agente debe enviarle JSON-RPC por líneas; una ejecución
manual que parece silenciosa está esperando tráfico, no bloqueada.

Para revertir exactamente esos cambios:

```bash
rationale uninstall-agent
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

El repositorio incluye `.mcp.json` como configuración de desarrollo. Para una
instalación global de Codex, registra el binario instalado con:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve
```

Revisa el archivo de configuración de tu agente antes de versionarlo: nunca
incluyas tokens, claves privadas ni rutas sensibles.

## Proveedor estructural

Codebase Memory es opcional. Si no está disponible, `health` reportará cobertura
degradada y las respuestas incluirán advertencias honestas. Rationale nunca lee
la SQLite interna del proveedor.
