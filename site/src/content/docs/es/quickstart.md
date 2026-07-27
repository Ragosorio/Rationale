---
lang: es
slug: quickstart
title: Quickstart de cinco minutos
description: Instala Rationale, inicializa un proyecto, conecta el proveedor estructural opcional y ejecuta la primera revisión de salud.
section: Empezar
order: 1
---

## Qué necesitas

Rationale funciona localmente en macOS, Linux y los targets publicados de
preview. Necesitas Git y una shell. Codebase Memory es recomendable para la
búsqueda estructural, pero opcional: sin él, Rationale reporta cobertura
degradada y sigue funcionando.

## Instalar

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/download/v0.1.0-alpha.7/rationale-installer.sh | sh
rationale --version
```

Para instalar primero el proveedor compañero:

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash
```

## Inicializar un proyecto

Ejecuta estos comandos desde el repositorio que quieres gobernar:

```bash
rationale init
rationale health
rationale install-agent --dry-run
rationale install-agent
```

`init` crea `.rationale/`, conserva los YAML canónicos en Git y ofrece integrar
el agente. `install-agent` escribe un bloque idempotente de instrucciones y
registra el servidor MCP donde el agente detectado lo soporta.

## Hacer el primer cambio gobernado

Pide al agente que localice el target con Codebase Memory y luego llama a
`prepare_change(target, intent)` antes de un cambio no trivial. Después debe
llamar a `finalize_change(...)`. Eso crea una propuesta pendiente cuando el
cambio tiene una señal de alto valor; no aprueba nada.

Revisa tú la propuesta:

```bash
rationale review
```

La aprobación, corrección, disputa, revocación, superseder y los cambios de
autoridad siguen siendo acciones humanas interactivas.

## Verificar la instalación

```bash
rationale --help
rationale --version
rationale health
```

La CLI emite el contrato JSON documentado por stdout. Los diagnósticos van por
stderr, y `rationale serve` reserva stdout para mensajes JSON-RPC de MCP.
