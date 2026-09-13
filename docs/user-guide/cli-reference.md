# Referencia de la CLI

La ayuda del binario es la referencia ejecutable:

```bash
rationale --help
rationale <comando> --help
```

## Configuración

| Comando | Uso |
|---|---|
| `init [--skip-agent-config]` | Crea `.rationale/` y configura los agentes detectados. |
| `install-agent [--dry-run] [--refresh-skills] [--global-only]` | Registra el servidor MCP por usuario y escribe protocolo y skills en el proyecto. Idempotente. |
| `uninstall-agent [--global-only]` | Revierte solamente lo escrito por `install-agent`. |
| `update` | Instala la última Release del canal mediante el helper local. |

## Contexto

| Comando | Uso |
|---|---|
| `health` | Proyecto, revisión Git, working tree, estado y cobertura del proveedor. |
| `prepare <target> [--intent "…"] [--repo-path <path>]` | Compila el packet de contexto para un path o símbolo. |
| `serve [--client <claude-code\|codex\|cursor>]` | Servidor MCP persistente por stdio. |
| `ui [--port <n>] [--no-open]` | Control Room de solo lectura en `127.0.0.1`. |

## Autoridad humana

| Comando | Uso |
|---|---|
| `pin <record-id> [--reason "…"]` | Fija un Record: los agentes lo usan pero no pueden reemplazarlo. |
| `unpin <record-id> [--reason "…"]` | Devuelve un Record fijado a autoridad normal. |
| `conflicts [--json]` | Lista conflictos pendientes con reglas fijadas. |
| `resolve <conflict-id> <keep-pinned\|adopt-new>` | Decide un conflicto. |
| `review-record <record-id>` | Lifecycle de un Record: corregir, disputar, revocar, reemplazar, autoridad, evidencia. |

`pin` y `unpin` exigen terminal interactiva, un actor declarado en
`.rationale/config.yaml` y confirmar escribiendo el id. `resolve` exige terminal
interactiva; `adopt-new` además exige actor declarado.

## Migración y mantenimiento

| Comando | Uso |
|---|---|
| `migrate [--dry-run] [--json]` | Pasa propuestas anteriores a 1.0 por el gate de captura. Nunca borra nada. |
| `review` | Legado: confirma propuestas anteriores a 1.0 una a una. |
| `doctor [--check] [--repair] [--json]` | Integridad del canon. `--check` sale con 1 si hay hallazgos; `--repair` pregunta por cada uno. |

## Opciones frecuentes

```bash
rationale health --project-root /ruta/proyecto
rationale prepare "src/lib.rs::funcion" --intent "qué quiero cambiar"
rationale install-agent --project-root /ruta/proyecto --dry-run
rationale ui --port 9800 --no-open
```

`--no-mascot` o `RATIONALE_NO_MASCOT=1` silencian a Chestie. La CLI no ofrece una
vía para que un agente fije Records ni decida conflictos. Si una versión
publicada muestra comandos distintos, reporta el desvío antes de actualizar la
documentación.
