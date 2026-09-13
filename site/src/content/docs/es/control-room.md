---
lang: es
slug: control-room
title: Control Room
description: rationale ui — el subgrafo de trabajo, la memoria causal y la actividad de los agentes en vivo, servido en solo lectura desde tu máquina.
section: Operar
order: 5
---

## Abrirlo

```bash
rationale ui
rationale ui --port 9800 --no-open
```

El Control Room escucha solo en `127.0.0.1` (puerto `9748` por defecto; sin
`--port` prueba los siguientes si está ocupado) y abre tu navegador. La
interfaz va embebida en el binario. Observa el mismo estado local que la CLI y
el servidor MCP y **nunca escribe**: `rationale serve` sigue siendo la frontera
con los agentes.

## Grafo

El subgrafo de trabajo de las operaciones recientes, en 3D:

- el color del nodo es su **rol** en el cambio — target, caller, callee,
  dependencia, dependiente, test, contexto;
- el color de la arista es su **estado estructural** — observed, indirect,
  orphaned, unknown;
- un anillo causal marca los nodos y relaciones **explicados por un Record**.

Selecciona un nodo para leer los Records que lo gobiernan, sus relaciones y las
operaciones en las que apareció. Selecciona una relación para ver por qué
existe y cómo cambió su estado en el tiempo.

## Actividad

Cada sesión de agente en tiempo real: qué cliente se conectó (Claude Code,
Codex, Cursor o la CLI), la intención y el target de cada operación, la
latencia del proveedor, el tamaño del packet, los candidatos capturados y
descartados, los conflictos y las explicaciones en riesgo. Los eventos llegan
por Server-Sent Events; no hace falta recargar.

## Memoria

El canon como navegador: filtra por tipo, estado y autoridad; lee statement,
rationale, procedencia, bindings y relaciones explicadas; ve los conflictos
pendientes con Records fijados. Los Records recién capturados se marcan al
llegar.

## Sistema

Proyecto, revisión de Git, conteos del canon, estado del stream y la tabla de
sesiones. También recuerda la frontera de privacidad de abajo.

## Qué se guarda y dónde

La actividad es un archivo NDJSON por sesión en `.rationale-local/activity/`;
los snapshots de operación viven en `.rationale-local/operations/`. Ambos se
excluyen de Git automáticamente y nunca salen de la máquina. Guardan
identificadores, la intención declarada (una línea, como máximo 280
caracteres) y referencias a Records — nunca el contenido de un Record, código
ni conversaciones (ADR-0017). La retención conserva los últimos 14 días, con un
máximo de 500 sesiones y 200 operaciones.

`RATIONALE_ACTIVITY=off` desactiva por completo la actividad y los snapshots.

## Seguridad

El servidor acepta solo `GET` y `HEAD`, valida la cabecera `Host` contra
nombres de loopback (defensa contra DNS rebinding), acota el tamaño y el tiempo
de las peticiones, envía una Content-Security-Policy restrictiva y sirve solo
assets embebidos — nunca una ruta del sistema de archivos.
