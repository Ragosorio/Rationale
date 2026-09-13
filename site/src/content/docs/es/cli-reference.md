---
lang: es
slug: cli-reference
title: Referencia de la CLI
description: Todos los comandos de rationale — configuración, contexto, el Control Room, autoridad humana, migración y mantenimiento.
section: Operar
order: 8
---

## Configuración

| Comando | Propósito |
| --- | --- |
| `init [--skip-agent-config]` | Crea el canon `.rationale/`, o lo conserva si ya existe, y configura los agentes detectados. |
| `install-agent [--dry-run] [--refresh-skills] [--global-only]` | Registra el servidor MCP por usuario y escribe el protocolo y las skills en el proyecto. Idempotente. |
| `uninstall-agent [--global-only]` | Revierte exactamente lo que escribió `install-agent`. |
| `update` | Instala la última release de tu canal mediante el helper instalado. |

## Contexto

| Comando | Propósito |
| --- | --- |
| `health` | Identidad del proyecto, revisión de Git, working tree, estado y cobertura del proveedor. |
| `prepare <target> [--intent "…"] [--repo-path <path>]` | Compila el mismo packet de contexto que recibe un agente. |
| `serve [--client <claude-code\|codex\|cursor>]` | Ejecuta el servidor MCP persistente por stdio. |
| `ui [--port <n>] [--no-open]` | Abre el Control Room de solo lectura en `127.0.0.1`. |

## Autoridad humana

| Comando | Propósito |
| --- | --- |
| `pin <record-id> [--reason "…"]` | Fija un Record: los agentes lo usan, pero no pueden reemplazarlo sin tu decisión. |
| `unpin <record-id> [--reason "…"]` | Devuelve un Record fijado a autoridad normal. |
| `conflicts [--json]` | Lista las afirmaciones de agentes que intentaron reemplazar una regla fijada. |
| `resolve <conflict-id> <keep-pinned\|adopt-new>` | Decide un conflicto. `adopt-new` exige autoridad declarada. |
| `review-record <record-id>` | Corrige, disputa, revoca, reemplaza, cambia la autoridad o añade evidencia a un Record, con un evento de lifecycle auditado. |

`pin` y `unpin` exigen una terminal interactiva y un actor declarado bajo
`authority:` en `.rationale/config.yaml`, y te piden escribir el id del Record
para confirmar. `resolve` también exige una terminal interactiva; `adopt-new`
además exige un actor declarado. Un agente nunca puede ejecutarlos por ti.

## Migración y mantenimiento

| Comando | Propósito |
| --- | --- |
| `migrate [--dry-run] [--json]` | Pasa las propuestas pendientes anteriores a 1.0 por el gate de captura: las válidas se vuelven Records con procedencia `migrated` y las ruidosas se archivan con su motivo. No borra nada. |
| `review` | Legado: confirma propuestas anteriores a 1.0 una a una. El trabajo normal nunca crea propuestas. |
| `doctor [--check] [--repair] [--json]` | Detecta severidades o autoridades inválidas, Records sin bindings, rutas rotas, Subjects colgantes y propuestas sin migrar. `--check` sale con 1 si hay hallazgos; `--repair` pregunta por cada uno. |

## Opciones comunes

```bash
rationale health --project-root /ruta/al/proyecto
rationale prepare "src/auth.rs::resolve" --intent "aceptar tokens vencidos durante un minuto"
rationale install-agent --project-root /ruta/al/proyecto --dry-run
```

`--project-root` lo acepta todo comando de proyecto excepto `init`, que se
ejecuta desde la raíz del proyecto. `--no-mascot` (o `RATIONALE_NO_MASCOT=1`)
silencia a Chestie. `--help` en cualquier comando no tiene efectos.

## Entorno

| Variable | Efecto |
| --- | --- |
| `RATIONALE_PROVIDER=none` | Desactiva el proveedor estructural. |
| `RATIONALE_ACTIVITY=off` | Desactiva la actividad local y los snapshots de operación. |
| `RATIONALE_SKIP_AGENT_CONFIG=1` | Omite la configuración de agentes en `init`. |
| `RATIONALE_CHANNEL=stable\|preview` | Canal de releases del instalador y de `update` (por defecto `stable`). |
| `RATIONALE_VERSION`, `RATIONALE_INSTALL_DIR` | Fija una versión en el instalador o cambia el directorio del binario. |

## Frontera de salida

Los comandos con contrato de máquina dejan el JSON en stdout; los diagnósticos
y Chestie van a stderr. `serve` nunca imprime en stdout nada que no sea un
mensaje MCP.
