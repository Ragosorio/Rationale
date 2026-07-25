# Spike notes — observaciones cualitativas

Notas del agente que implementó ambos candidatos en la misma sesión, con el mismo nivel de esfuerzo declarado (`Proceso §9.2`).

## Mantenibilidad con agentes

- **Rust** exige resolver el modelo de propiedad/borrowing incluso en un programa pequeño (ej. `child.stdout.take()`, manejo explícito de `Option`/`Result` en cada paso). Esto produjo código más verboso pero cuyos errores de tipo aparecen en tiempo de compilación — el compilador rechazó cualquier intento de usar un valor después de moverlo, antes de llegar a ejecutar nada. Para un agente que itera rápido, esto significa más ciclos de "corregir error de compilación" pero también mayor confianza en que, si compila, cierto tipo de error de lógica (usar un valor movido, olvidar un `Result`) ya está descartado.
- **Go** permitió escribir la primera versión más rápido y con menos fricción sintáctica. El costo apareció después: el bug de cancelación de subproceso (`candidates.md`) no lo detectó el compilador ni el linter — lo detectó únicamente la medición empírica de tiempo. Es decir, Go movió el costo de "atrapar el error" de tiempo de compilación a tiempo de prueba/medición.
- Para un flujo de trabajo con agentes que se apoya fuertemente en `cargo test`/`go test` y en medición automatizada (exactamente el patrón que este proyecto ya sigue, `Proceso §12`), ambos lenguajes son viables, pero **Rust falla más temprano y de forma más ruidosa; Go falla más tarde y en silencio** salvo que exista instrumentación deliberada como la de este spike.

## Ergonomía

- La biblioteca estándar de Go para JSON (`encoding/json`) y subprocess (`os/exec`) es más directa de usar que el ecosistema de crates en Rust (`serde_json` + manejo manual de `Command`), a costa de menos garantías en tiempo de compilación.
- El manejo de errores de Rust (`Result<T, E>` obligatorio en cada punto de fallo) hizo más incómodo escribir rápido, pero también hizo imposible ignorar silenciosamente un fallo de I/O — en Go, un error ignorado (`_`) compila sin advertencia a menos que se use un linter externo (`errcheck`, no incluido en este spike por paridad de dependencias).
- El framing MCP (`Content-Length` + JSON-RPC) se implementó de forma casi idéntica en ambos lenguajes — no fue un diferenciador real; ambos tienen soporte suficiente en la std (`std::io`/`io.Reader` con lectura byte a byte y parsing manual de headers).

## Lo que este spike NO evalúa

- Mantenibilidad a largo plazo en un proyecto de decenas de miles de líneas (este spike es deliberadamente pequeño, `spike-protocol.md`).
- Disponibilidad y calidad de skills/documentación específica para agentes en cada lenguaje — evaluación pendiente para después de elegir (`Proceso §9.3`).
- Comportamiento real en Linux/Windows (`compatibility-matrix.md`) — solo evaluado por diseño, no verificado en ejecución.
- Empaquetado y distribución real (Fase J, muy posterior).

## Impresión general

Ninguno de los dos candidatos mostró una limitación que lo descarte. La decisión en ADR-0001 debe ponderar explícitamente: seguridad de memoria/confiabilidad demostrada empíricamente en este spike (a favor de Rust, por el hallazgo de cancelación), contra velocidad de iteración y fuzzing nativo (a favor de Go) — exactamente la tensión que los criterios ponderados de `Arquitectura_Conceptual_v0.1.md §8.2` ya anticipaban al ponderar "seguridad de memoria y confiabilidad" con el peso más alto (20%).
