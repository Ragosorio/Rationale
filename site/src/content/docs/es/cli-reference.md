---
lang: es
slug: cli-reference
title: Referencia de la CLI
description: Los comandos que inicializan, inspeccionan, preparan, revisan y mantienen un proyecto Rationale.
section: Operar
order: 4
---

## Comandos

| Comando | Propósito |
| --- | --- |
| `init` | Crea el canon `.rationale/` y ofrece integrar agentes. |
| `health` | Reporta identidad, revisión Git, proveedor y cobertura. |
| `prepare <target>` | Compila contexto para un path o símbolo. |
| `review` | Revisa propuestas pendientes con una persona. |
| `review-record <id>` | Muta un Record aprobado mediante su lifecycle auditable. |
| `install-agent` | Añade instrucciones y registro MCP idempotentes. |
| `uninstall-agent` | Revierte únicamente lo que escribió `install-agent`. |
| `update` | Instala el preview seleccionado por el helper local. |
| `serve` | Ejecuta el servidor MCP persistente por stdio. |

## Opciones frecuentes

```bash
rationale health --project-root /ruta/proyecto
rationale prepare "src/auth.rs::resolve" --project-root /ruta/proyecto
rationale install-agent --project-root /ruta/proyecto --dry-run
```

`prepare` toma como target el primer argumento posicional que no sea una
opción. `init --help` y las opciones inválidas no tienen efectos laterales.
`init` no acepta `--project-root`; ejecútalo desde la raíz del proyecto.

## Frontera de salida

Los comandos con contrato de máquina conservan JSON en stdout. Los diagnósticos
y Chestie van por stderr o se suprimen cuando la salida no es una terminal.
`serve` nunca imprime un banner en stdout.
