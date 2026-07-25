# ADR-0001: Core language and toolchain

**Status:** proposed — pendiente de revisión cruzada independiente (`AGENTS.md §Roles y revisión cruzada`) antes de `accepted`. Ningún agente debe autoaprobar esta decisión (`evaluation.no-self-certification`, `.rationale/subjects/`).
**Date:** 2026-07-25
**Deciders:** Claude Code (implementación del spike y propuesta); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

`Rationale_Arquitectura_Conceptual_v0.1.md §8` prohíbe elegir el lenguaje del núcleo por preferencia — exige un spike con carga idéntica entre candidatos reales, medido con evidencia, antes de escribir producción. `docs/research/language/spike-protocol.md` fijó el protocolo y los criterios ponderados **antes** de implementar nada, precisamente para evitar el sesgo de calibrar el criterio después de ver el resultado.

Los candidatos evaluados fueron Rust y Go (C y TypeScript/Node.js se descartaron por decisión explícita ya documentada en `spike-protocol.md §Candidatos`, no por evaluación).

## Decision

**Rust** es el lenguaje del núcleo de Rationale, con evidencia del spike ejecutado en `spikes/language/rust/` y `spikes/language/go/`.

## Evidence

Implementaciones completas de las 6 operaciones obligatorias en ambos candidatos, con carga idéntica verificada (`docs/research/language/candidates.md`). Mediciones crudas en `docs/research/language/benchmark-results.json`.

### Puntuación ponderada (`Arquitectura_Conceptual_v0.1.md §8.2`)

| Criterio | Peso | Rust | Go | Base empírica |
|---|---:|---:|---:|---|
| Seguridad de memoria y confiabilidad | 20% | 9 | 6 | Rust correcto al primer intento en las 6 operaciones + demos. Go tuvo un **fallo real y medido**: la implementación idiomática de cancelación de subproceso (`exec.CommandContext` + `cmd.Output()`) tardó **5016ms en vez de ~500ms** por el problema Unix del nieto huérfano que sostiene el pipe de stdout abierto — requirió reescribirse con process groups (`Setpgid` + `kill(-pid)`) para cumplir el deadline. |
| Distribución como binario | 15% | 9 | 7 | Binario Rust release: 2.216.304 bytes. Binario Go release (stripped): 7.072.994 bytes — 3.2× más grande. Ambos son ejecutables estáticos Mach-O arm64 sin runtime externo. |
| Rendimiento y latencia | 15% | 9 | 7 | Memoria residente pico: Rust ~3.0MB, Go ~12.7-12.9MB (4.2× más). Latencia MCP (`initialize`/`tools call`): Rust 3.5ms/15.6ms, Go 8.4ms/9.0ms — ambos negligibles a esta escala, sin diferencia práctica. |
| MCP y JSON-RPC | 10% | 8 | 8 | Ambos implementaron el mismo framing `Content-Length` (verificado contra el mismo protocolo usado por Codebase Memory, `docs/research/codebase-memory/11-performance-observations.md`) con esfuerzo equivalente en la std de cada lenguaje. Sin diferenciador real. |
| SQLite y filesystem | 10% | 7 | 8 | Rust usó `rusqlite` con SQLite vendorizado en C (`bundled`); Go usó `modernc.org/sqlite`, pure-Go sin cgo — mejor historia de cross-compilation nativa para Go en este aspecto específico. |
| Compatibilidad macOS/Linux/Windows | 10% | 7 | 5 | **Gap real encontrado, no solo teórico:** el file locking usado en el spike de Go (`syscall.Flock`) es POSIX-only y el propio código falla explícitamente en Windows; Rust usó una ruta también POSIX-only en el spike por simplicidad, pero tiene una alternativa portable sin ejercitar en la std (`std::fs::File::lock`, disponible desde Rust 1.89). Ver `docs/research/language/compatibility-matrix.md`. |
| Mantenibilidad con agentes | 10% | 7 | 7 | Rust "falla más temprano y ruidoso" (errores de compilación); Go "falla más tarde y en silencio" (el bug de cancelación no lo atrapó el compilador ni un linter, solo la medición empírica). Empate cualitativo, con matiz a favor de Rust por alinearse con el principio general de este proyecto de preferir fallos explícitos sobre silenciosos (`docs/research/codebase-memory/10-failure-modes.md`). |
| Tiempo de compilación y desarrollo | 5% | 5 | 9 | Build release limpio: Rust 31.94s, Go 9.59s — Go 3.3× más rápido. Ventaja real de Go para el ciclo de iteración con agentes. |
| Interoperabilidad con procesos C | 5% | 8 | 5 | Rust usó FFI directo a `flock()` de forma natural; Go evitó deliberadamente cgo para SQLite, lo cual reduce fricción de cross-compilation pero también indica menor comodidad nativa con interop C si llegara a necesitarse. |

**Total ponderado: Rust 8.05/10, Go 6.80/10.**

Esta puntuación es una síntesis explícita de evidencia, no una ley — los pesos y escalas están sujetos al mismo principio de sensibilidad y revisión que `Rationale_v0.5.md §30.1.3` exige para `context_utility_density`. Se documentan aquí precisamente para que puedan auditarse y disputarse con datos, no aceptarse por autoridad de quien las calculó.

## Alternatives considered

- **Go**: descartado no por incapacidad (completó las 6 operaciones y las pruebas adicionales) sino por peor puntuación ponderada, dominada por el criterio de mayor peso (seguridad de memoria y confiabilidad, 20%) donde se encontró un fallo real y reproducible. Go retiene ventajas reales documentadas (compilación 3.3× más rápida, fuzzing nativo sin dependencias, SQLite pure-Go) que deben pesarse en el "Revisit trigger" si la evidencia cambia.
- **C**: descartado sin evaluación, por decisión explícita anterior a este spike (`spike-protocol.md §Candidatos`) — preservar la frontera de protocolo/adaptador frente a Codebase Memory (escrito en C) en vez de compartir lenguaje o proceso.
- **TypeScript/Node.js**: descartado sin evaluación, reservado para prototipos y tooling de evaluación, no para el núcleo distribuido (`Arquitectura_Conceptual_v0.1.md §8.1`).

## Consequences

- Se habilita continuar a Fase C5 (toolchain: formatter, linter, testing guide) y Fase D (vertical slice) en Rust.
- El adaptador de Codebase Memory (`Rationale_v0.5.md §21`) se implementará en Rust, con FFI/subprocess hacia el binario de CBM (escrito en C) — la interoperabilidad C ya demostrada en el spike (`flock` vía FFI) es un precedente directo.
- Se pierde la ventaja de compilación 3.3× más rápida de Go — mitigable parcialmente con `cargo check` incremental durante desarrollo activo, no medido en este spike.
- El fuzzing/property testing en Rust requerirá una dependencia externa (`proptest` o `cargo-fuzz`) cuando se necesite — no está en el toolchain inicial de Fase C5 salvo que un caso concreto lo justifique.
- El file locking en Fase D/E debe usar explícitamente la ruta portable de la std (`std::fs::File::lock`), no la ruta POSIX-only vía FFI usada en el spike por simplicidad — pendiente de verificar en Windows antes de Fase J (empaquetado).

## Risks

- El compilador de Rust y su ecosistema de crates pueden ser menos familiares para algunos agentes que Go — mitigación: `Proceso §9.3` ya exige crear style guide, testing guide y security guide específicos del lenguaje elegido (Fase C5).
- La puntuación ponderada es una síntesis de un solo spike pequeño, no de un proyecto de producción — un hallazgo distinto en Fase D (vertical slice, alcance mayor) podría matizar esta decisión; ver Revisit trigger.

## Validation

Spike ejecutado completo en ambos candidatos, con las 6 operaciones obligatorias, servidor MCP, file locking, subprocess con deadline real, y suite de tests (6 tests unitarios en cada uno, más fuzzing nativo en Go). Reproducible: ver comandos en `spikes/language/rust/` y `spikes/language/go/`, y `docs/research/language/benchmark-results.json` para las mediciones crudas.

**Este ADR está en estado `proposed`, no `accepted`.** Requiere revisión cruzada de otro agente (idealmente Codex, per `Proceso §13`) que intente falsificar la puntuación ponderada y las conclusiones antes de pasar a `accepted`, y aprobación humana explícita antes de comprometerse en Fase D.

## Revisit trigger

Reabrir este ADR si:
- El adaptador de Codebase Memory (Fase E) revela una necesidad de interop C tan intensiva que la ventaja de Rust en ese criterio se vuelve dominante (reforzaría la decisión) o, inversamente, si aparece una limitación de Rust no anticipada aquí (debilitaría la decisión).
- Fase D (vertical slice) descubre que el tiempo de compilación de Rust (31.94s en este spike pequeño) escala mal y afecta materialmente la velocidad de iteración de los agentes que construyen Rationale.
- Se identifica un bug de seguridad de memoria en la propia implementación Rust del núcleo que contradiga la premisa central de esta decisión.
