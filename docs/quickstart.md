# Quickstart

Cinco minutos, sin haber visto el proyecto antes.

## Qué hace por ti

Tu memoria de código (Codebase Memory) sabe **dónde** está el código y **cómo**
se conecta. Rationale sabe **por qué** existe y **qué** debe seguir siendo
cierto. Juntos evitan que un agente derribe una valla sin saber qué protegía —
la [valla de Chesterton](https://es.wikipedia.org/wiki/Valla_de_Chesterton): no
quites algo hasta saber por qué está ahí.

Es local-first: sin servidor, sin cuenta, sin API de pago. Codebase Memory es
opcional — sin él Rationale sigue funcionando con cobertura degradada.

## Instalar

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash   # recomendado
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-installer.sh | sh
```

Esto coloca el binario en `~/.local/bin` (o `$RATIONALE_INSTALL_DIR`), verifica
SHA-256 y registra el servidor MCP para los agentes que detecta (Claude Code,
Codex, Cursor) con la ruta absoluta del binario. Todavía no toca ningún
proyecto. En Windows, usa el instalador de PowerShell del
[`README`](../README.md#windows-powershell).

Las actualizaciones siguientes:

```bash
rationale update
```

## Inicializar un proyecto

Dentro del proyecto que quieres proteger:

```bash
rationale init
rationale health
```

`init` crea `.rationale/` (el canon del proyecto) y escribe el protocolo de
invocación para los agentes que detecta. Si prefieres hacerlo después, usa
`rationale init --skip-agent-config` y corre `rationale install-agent` cuando
quieras.

## Qué se instaló y dónde vive cada cosa

| Qué | Dónde | Se versiona en Git |
|---|---|---|
| El binario `rationale` | `~/.local/bin/rationale` | No — es una herramienta |
| **El canon del proyecto** | `<tu-proyecto>/.rationale/` — Records, Subjects, configuración | **Sí** — se revisa en PR y se comparte con el equipo |
| Instrucciones para tu agente | `CLAUDE.md` / `AGENTS.md` / `.cursor/rules/rationale.mdc`, en un bloque delimitado, y skills en `.claude/skills/` | Sí |
| Registro del servidor MCP | `~/.claude.json`, `~/.cursor/mcp.json` o `codex mcp` — por usuario | No — nunca en el proyecto |
| Actividad y operaciones | `<proyecto>/.rationale-local/` | No — excluido automáticamente, nunca sale de la máquina |

## El flujo real

Le pides a tu agente algo como:

> "Quiero que los reintentos de pago se detengan en tres intentos."

Sin que lo menciones, el agente (guiado por el protocolo instalado):

1. Usa Codebase Memory para encontrar `charge`, sus callers y su ubicación.
2. Llama a `prepare_change(target, intent)` y recibe las reglas y decisiones que
   gobiernan ese código, por qué existen sus relaciones y qué conflictos tiene
   tu intención.
3. Hace el cambio y corre los tests.
4. Llama a `finalize_change` con el conocimiento que sigue siendo cierto — por
   ejemplo, «los pagos que ya llegaron al banco nunca se reintentan». Rationale
   lo escribe en `.rationale/records/` en esa misma llamada.

La próxima conversación, con cualquier agente, recibe esa regla antes de tocar
`charge`.

## Míralo en vivo

```bash
rationale ui
```

El [Control Room](user-guide/control-room.md) abre en `127.0.0.1:9748`: el
subgrafo que recibió el agente, la memoria que lo explica y la actividad de cada
sesión mientras ocurre.

## Fija lo que no debe moverse

```bash
rationale pin <record-id>
```

Un agente puede usar una regla fijada, pero no reemplazarla. Si lo intenta, no
se escribe nada y recibes un conflicto:

```bash
rationale conflicts
rationale resolve <conflict-id> keep-pinned
```

Fijar exige que tu actor de Git esté declarado bajo `authority:` en
`.rationale/config.yaml`.

## Prompt maestro

`install-agent` escribe el [prompt maestro](prompt-master.md) en las
instrucciones de cada agente. La [versión en español](prompt-master.es.md) sirve
para leerlo o pegarlo a mano en un cliente que Rationale no configura.

## Verificar que quedó bien

```bash
rationale --version
rationale health
rationale doctor --check
```

`health` imprime JSON con `project_id`, `git_revision` y `provider_status`. Si
algo falla, ver [`docs/runbooks/diagnostics.md`](runbooks/diagnostics.md).

## Quitarlo

```bash
rationale uninstall-agent                # revierte instrucciones y skills de este proyecto
rationale uninstall-agent --global-only  # revierte los registros MCP del usuario
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

Ninguno toca `.rationale/` — es tu canon, y borrarlo es una decisión tuya. Ver
[`docs/runbooks/uninstall.md`](runbooks/uninstall.md).

## Siguiente paso

Si vas a construir sobre Rationale, sigue con el
[índice de documentación](README.md), [CONTRIBUTING.md](../CONTRIBUTING.md) y
los documentos fundacionales listados en el [`README`](../README.md#documentos-fundacionales).
