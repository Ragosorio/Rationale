# Fase H — piloto read-only y alfa instalable

## Gate de entrada

- Fase G completa y Release dogfood vigente `v0.0.0-dogfood.6` publicada.
- Security baseline revisado y sin P0/P1 abiertos.
- Instalador probado en una máquina limpia.
- El dueño del proyecto autoriza expresamente los paths de Rationale,
  Monorepo y BoostAPI que pueden leerse.

## Casos y condiciones

Se ejecutarán 20–30 cambios históricos internos autorizados, distribuidos entre
los tres repositorios. Cada caso tendrá ground truth preregistrado y comparará:

1. código/Git;
2. código + documentación;
3. Codebase Memory;
4. Codebase Memory + Rationale.

La primera pasada es read-only. Solo tras superar sus métricas se habilita
captura asistida; no se activan bloqueos ni aprobaciones automáticas.

## Métricas de salida

- recall de restricciones críticas `>= 90%`;
- precisión del contexto `>= 80%`;
- contexto perjudicial `< 2%`;
- falsos bloqueos `0`;
- paquete mediano `<= 600` tokens y P95 `<= 1000`;
- reducción de contexto manual `>= 50%`;
- ningún assessment obsoleto presentado como exacto.

Cada fallo se conserva como evidencia, issue o Record disputado; no se elimina
para mejorar artificialmente la métrica.
