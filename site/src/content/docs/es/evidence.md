---
lang: es
slug: evidence
title: Evidencia y estado de release
description: Lee la evidencia actual del proyecto con cobertura, incertidumbre y gates abiertos incluidos.
section: Proyecto
order: 12
---

## Evidencia actual

El test dogfood de la cadena de gobernanza reproduce el camino crítico: captura
un cambio sin commit, materializa un Subject, una persona aprueba la propuesta
y una sesión MCP nueva recupera el mismo Record gobernante. El control negativo
no inventa un Record para un target no relacionado.

Los gates Rust del repositorio incluyen tests debug y release, Clippy con
warnings denegados, formato, validación de schemas y auditoría RustSec. La
landing y este manual se construyen por separado con Astro.

## Qué demuestra

La cadena se ejercita contra el binario real y el proveedor se trata como
cobertura opcional. El test demuestra recuperación y fronteras de autoridad; no
demuestra por sí solo detección semántica de contradicciones.

## Qué sigue abierto

El proyecto sigue en `pre-1.0 / alpha`. Instalación en máquinas limpias,
matrices de plataformas, revisión del piloto y repositorios de formas inusuales
siguen siendo gates visibles, no afirmaciones ocultas.
