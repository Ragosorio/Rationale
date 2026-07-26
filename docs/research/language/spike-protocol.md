# Protocolo del spike de lenguaje — Rust vs Go

**Estado:** ejecutado en Fase C. Este documento conserva el criterio de evaluación que se congeló antes del spike; los resultados están en [`candidates.md`](candidates.md), [`compatibility-matrix.md`](compatibility-matrix.md), [`benchmark-results.json`](benchmark-results.json) y [`spike-notes.md`](spike-notes.md), precisamente para evitar el sesgo de fijar el criterio después de ver qué lenguaje "se sintió mejor" (`Rationale_Arquitectura_Conceptual_v0.1.md §8`).

El ADR-0001 conserva estado `proposed` hasta la aprobación humana; la implementación posterior no constituye autoaprobación (`Rationale_Proceso_Construccion_Agentes_v0.1.md §9`).

## Candidatos

**Rust vs Go**, según decisión del equipo. El documento de arquitectura conceptual (`Arquitectura_Conceptual_v0.1.md §8.1`) también lista C y TypeScript/Node.js como candidatos posibles; se descartan de este spike por decisión explícita, no por evaluación:

- **C**: Codebase Memory ya está escrito en C. Se descarta como candidato del núcleo de Rationale precisamente para preservar la frontera de protocolo/adaptador en vez de compartir lenguaje o proceso (`Arquitectura_Conceptual_v0.1.md §28.1` del doc conceptual, y §8.1 de este documento: "no debe elegirse únicamente porque Codebase Memory usa C").
- **TypeScript/Node.js**: reservado para prototipos, tooling o harnesses de evaluación, no para el núcleo distribuido (`Arquitectura_Conceptual_v0.1.md §8.1`).

Si el resultado de este spike es insatisfactorio para ambos candidatos, se documentará como tal y se reabrirá la comparación — no se fuerza una elección entre dos opciones débiles.

## Carga idéntica (`Proceso §9.1`)

Cada candidato debe implementar exactamente la misma función mínima, sin atajos ni funcionalidad adicional en ninguno de los dos:

```text
Input:
  target + intent + revision

Operations:
  1. Leer un Record (YAML) desde disco.
  2. Abrir una base SQLite (crear si no existe, insertar y leer una fila).
  3. Llamar o mockear un proveedor estructural externo (subprocess o llamada MCP simulada).
  4. Verificar una revisión (comparar dos strings de revisión, ej. Git SHA).
  5. Rankear una constraint (ordenar una lista pequeña por un campo numérico).
  6. Emitir JSON a stdout.

Measurements:
  - Startup time (cold).
  - Latency end-to-end de la operación completa.
  - Memoria residente pico.
  - Tamaño del binario compilado (release, sin símbolos de debug).
  - Velocidad de la suite de tests.
  - Viabilidad de cross-compilation / estrategia de CI para macOS, Linux y Windows.
```

**Regla de igualdad de carga:** no se permite que un candidato implemente un demo trivial y el otro una versión más completa (`Proceso §9.2`). Ambos deben construir exactamente las seis operaciones, ni más ni menos.

## Criterios ponderados (`Arquitectura_Conceptual_v0.1.md §8.2`)

```text
20% Seguridad de memoria y confiabilidad
15% Distribución como binario
15% Rendimiento y latencia
10% MCP y JSON-RPC
10% SQLite y filesystem
10% Compatibilidad macOS/Linux/Windows
10% Mantenibilidad con agentes
5%  Tiempo de compilación y desarrollo
5%  Interoperabilidad con procesos C
```

Cada candidato debe probar además, más allá de la carga mínima:

- Servidor MCP mínimo.
- Cliente hacia Codebase Memory o wrapper CLI.
- File locking.
- Subprocess.
- Cancelación.
- Deadline.
- Build arm64.
- Binary size.
- Test tooling.
- Fuzzing o property tests (viabilidad, no necesariamente implementación completa en el spike).
- Packaging (viabilidad).

## Entregables esperados del spike

```text
docs/research/language/
├── spike-protocol.md        (este documento)
├── candidates.md            (notas por candidato tras ejecutar el spike)
├── benchmark-results.json   (mediciones crudas)
├── compatibility-matrix.md  (macOS/Linux/Windows por candidato)
├── spike-notes.md           (observaciones cualitativas: mantenibilidad con agentes, ergonomía)
└── ADR-0001-core-language.md → vive en docs/adr/, no aquí
```

Ninguno de estos archivos existe todavía. Este documento únicamente fija el protocolo.

## Qué invalida el spike

Según `Arquitectura_Conceptual_v0.1.md §22`, el ADR resultante no puede decir solamente "elegimos X porque es rápido". Debe registrar evidencia, tradeoffs, alternativas descartadas y por qué, riesgo de reversión y fecha de revisión.

## Próximo paso

Ejecutar el spike (Fase C, fuera de alcance de este plan de bootstrap) implementando la carga idéntica en ambos candidatos, midiendo con el mismo hardware (`docs/environment/reference-development-machine.md`) y bajo las mismas condiciones.
