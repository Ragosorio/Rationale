# Agentes y MCP

Rationale puede funcionar solo por CLI, pero su flujo principal es que un agente
prepare contexto y capture conocimiento sin recibir autoridad sobre las reglas
fijadas.

## Integración automática

El instalador registra el servidor MCP para los agentes que detecta. Dentro de
un proyecto:

```bash
rationale install-agent --dry-run
rationale install-agent
```

| Agente | Registro MCP (por usuario) | En el proyecto |
|---|---|---|
| Claude Code | `~/.claude.json` | bloque en `CLAUDE.md` + seis skills en `.claude/skills/` |
| Codex | `codex mcp add` | bloque en `AGENTS.md` |
| Cursor | `~/.cursor/mcp.json` | `.cursor/rules/rationale.mdc` |

Cada registro ejecuta `rationale serve --client <agente>` con la ruta absoluta
del binario instalado (ADR-0016): funciona desde aplicaciones gráficas sin
depender de su `PATH` y atribuye cada sesión al agente correcto en el Control
Room. La instalación es convergente — repetirla no cambia nada y un registro
anterior del mismo binario se migra — y los archivos del proyecto nunca llevan
una ruta personal. Reinicia el agente después de instalar.

El texto que se instala es el [prompt maestro](../prompt-master.md); la
[versión en español](../prompt-master.es.md) mantiene los mismos pasos.

Para revertir exactamente esos cambios:

```bash
rationale uninstall-agent
rationale uninstall-agent --global-only
```

Las skills que editaste se conservan.

## Skills de Claude Code

- `/rationale-preflight <target> <intent>`
- `/rationale-explain <target>`
- `/rationale-capture [statement]`
- `/rationale-conflicts` — solo humana: presenta los conflictos y te entrega la decisión.
- `/rationale-health`
- `/rationale-protocol`

La skill `rationale-review` de versiones anteriores se retira sola al
reinstalar si nadie la editó.

## Herramientas MCP

El servidor expone cinco herramientas:

- `health`
- `prepare_change`
- `explain_target`
- `finalize_change`
- `resolve_conflict` — solo con la respuesta literal de la persona (`human_answer`)

Y seis prompts con las mismas acciones que las skills. `rationale serve` es un
servidor stdio de JSON-RPC delimitado por líneas: permanece abierto y no muestra
un banner en stdout; una ejecución manual silenciosa está esperando tráfico.

MCP no puede fijar ni desfijar Records, y reemplazar una regla fijada exige un
actor declarado en `.rationale/config.yaml`.

## Configuración manual

Para registrar Codex a mano:

```bash
codex mcp add rationale -- "$HOME/.local/bin/rationale" serve --client codex
```

Sin `--client`, la sesión usa el nombre que el cliente declara en `initialize`,
o `unknown`. Revisa la configuración de tu agente antes de versionarla: nunca
incluyas tokens, claves privadas ni rutas sensibles.

## Proveedor estructural

Codebase Memory es opcional. Si no está disponible, `health` reporta cobertura
degradada y los packets incluyen advertencias honestas. Rationale nunca lee la
SQLite interna del proveedor.
