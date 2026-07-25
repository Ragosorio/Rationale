# 11 — Performance observations (CBM-011: measure CLI vs MCP)

## Observed

### CLI (medición formal con `time`, ver `04-cli-contracts.md`)

| Escenario | Wall clock | CPU usuario |
|---|---:|---:|
| `cli index_status`, sin daemon (crea uno temporal por invocación) — corrida 1 | 6.811s | 2.19s |
| `cli index_status`, sin daemon — corrida 2 | 6.873s | 2.26s |
| `cli index_status`, con `daemon start` previo — corrida 1 | 2.283s | 2.17s |
| `cli index_status`, con `daemon start` previo — corrida 2 | 2.275s | 2.18s |

El propio binario advierte activamente sobre este costo (`hint: this command started a temporary CBM daemon...`).

### MCP (esta sesión)

**No se realizó una medición formal de wall-clock por llamada individual** — las herramientas MCP en esta sesión se invocan a través de un servidor ya conectado y persistente durante toda la investigación (mismo patrón que "daemon caliente"), por lo que ninguna llamada individual mostró una demora perceptible de varios segundos como sí ocurrió con la CLI en frío. Esto es una observación cualitativa, no una medición instrumentada con timestamps — **queda pendiente una medición formal con instrumentación propia antes de cerrar ADR-0002** (ej. medir con `time` un roundtrip MCP real usando un cliente mínimo, no a través de esta sesión de agente).

## Claimed

Ninguna documentación de CBM publica benchmarks de latencia CLI vs MCP.

## Verified

Las cuatro mediciones de CLI son reproducibles (ver comandos en `04-cli-contracts.md`); se repitieron dos veces cada escenario con resultados consistentes entre sí (diferencia <5%).

## Unknown

- Latencia real de una llamada MCP fría (arrancando el servidor MCP desde cero, sin sesión previa) — no medida.
- Cuánto del ~2.2s de CPU con daemon caliente es arranque del propio binario cliente (296MB, carga de gramáticas) vs. IPC real al daemon — no perfilado.
- Si existe una diferencia de latencia entre el transporte MCP stdio y una eventual variante de red — fuera de alcance, CBM parece operar únicamente sobre stdio local.

## Risk

**Alto para el diseño del fast path de Rationale, ya capturado en `04-cli-contracts.md`.** No se repite aquí en detalle; ver esa sección de Decision impact.

## Decision impact

1. Refuerza la recomendación ya registrada en `04-cli-contracts.md`: el **fast path baseline de Rationale no debe invocar un subproceso CLI de Codebase Memory por cada operación**. Una sesión MCP persistente es estructuralmente más compatible con las metas de latencia de Rationale, pero **debe medirse formalmente antes de fijar ADR-0002**, no asumirse solo por la experiencia cualitativa de esta sesión de investigación.
2. Próximo research item concreto antes de ADR-0002: escribir un cliente MCP mínimo (en el lenguaje que resulte del spike de `docs/research/language/`) que mida explícitamente: tiempo de arranque del servidor MCP, tiempo de primera respuesta, tiempo de respuestas subsecuentes en la misma sesión — replicando la metodología ya aplicada aquí a la CLI.

## Reproducir

Ver comandos en `04-cli-contracts.md`. La parte de MCP queda como trabajo futuro explícito, no reproducible con lo capturado en esta sesión.
