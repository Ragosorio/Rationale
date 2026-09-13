---
lang: es
slug: evidence
title: Evidencia y estado de la release
description: Contra qué se verificó 1.0, cómo, y qué sigue abierto — con la cobertura y la incertidumbre adjuntas.
section: Proyecto
order: 13
---

## Estado de la release

`v1.0.0` es la primera release estable: el ciclo completo — contexto antes de
un cambio, captura autónoma después, autoridad humana sobre las reglas fijadas
y el Control Room en vivo — está implementado, probado y en uso sobre el propio
repositorio de Rationale.

## Cómo se verificó

- **Gates automáticos.** Formato, Clippy con warnings denegados, 366 tests de
  Rust (unitarios y de integración, incluida una cadena de gobierno en la que un
  agente captura un Record, una persona lo fija y un intento posterior de
  reemplazarlo se vuelve un conflicto que se resuelve), auditoría RustSec,
  validación de schemas, el typecheck, los tests y el build del Control Room, y
  los chequeos estáticos de este sitio.
- **Checkout limpio.** Los mismos gates pasan desde un checkout nuevo del commit
  de la release, donde el binario sirve su página de fallback sin `ui/dist`.
- **Artefacto de release.** El binario empaquetado se extrajo y se ejercitó: la
  CLI, `doctor --check`, el Control Room embebido (lista de Hosts permitidos,
  métodos de solo lectura, path traversal rechazado) y las cinco herramientas
  MCP contra el Codebase Memory real.
- **Migración del registro.** Instalar sobre registros anteriores a 1.0 de
  Claude Code, Cursor y Codex (con el CLI real de `codex`, en homes aislados)
  migró los tres, convergió en una segunda ejecución y desinstaló limpio
  conservando los servidores ajenos.
- **Dogfood.** Un cambio real al propio Rationale pasó por una sesión de agente
  persistente: preparación, gobierno declarado, cambio, tests, captura de tres
  Records con bindings confirmados por el proveedor, recuperación de la regla
  nueva como gobernante y la operación completa visible en vivo en el Control
  Room.

Los registros detallados viven en el repositorio, en `docs/work-items/`.

## Qué sigue abierto

- Windows se compila y se prueba en CI; la verificación local de la release se
  hizo en macOS.
- La polaridad léxica de los conflictos con la intención es una heurística
  ruidosa conocida (ver [Límites conocidos](/es/docs/limits)).
- Varios ADRs detrás de 1.0 están implementados pero siguen `proposed` a la
  espera de una revisión independiente.
