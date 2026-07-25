# Rust — testing guide

## Pirámide aplicable (subconjunto de `Rationale_Arquitectura_Conceptual_v0.1.md §19.1` relevante hoy)

```text
Unit          cargo test — funciones puras, sin I/O externo cuando sea posible
Integration   cargo test con fixtures reales (ver spikes/language/rust/fixtures/)
Contract      Fase D: fixtures propios contra Codebase Memory (Arquitectura §19.3)
Property      sin dependencia extra en el spike (test manual de invariante);
              evaluar `proptest` cuando un caso concreto lo justifique — no
              añadir preventivamente (Proceso §19: "¿es necesaria?")
Golden packet Fase D — determinismo del Context Packet completo (Arquitectura §19.4)
```

## Comandos

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test                    # toda la suite
cargo test <nombre_parcial>   # filtrar por nombre
cargo test -- --nocapture     # ver stdout de los tests (útil para depurar JSON emitido)
```

## Convenciones de test verificadas en el spike

- Un test por operación del pipeline cuando la operación tiene lógica no trivial (`test_op4_*`, `test_op5_*`) — no testear operaciones que son I/O puro sin lógica de decisión (ej. no hace falta un test dedicado para "leer un archivo", pero sí para "qué se decide con lo leído").
- Fixtures reales en disco (`fixtures/record.yaml`) en vez de strings YAML embebidos en el test — permite que el mismo fixture sirva de entrada tanto al pipeline real como al test, evitando que diverjan.
- Tests de invariante (`test_severity_weight_monotonic_property`) cuando no se justifica una dependencia de property-testing — documentar explícitamente en el nombre del test que es un property-test manual, para que quede claro que no reemplaza cobertura real de un framework dedicado si se añade después.
- `tempdir`/`std::env::temp_dir()` con un sufijo único (`process::id()`) para tests que tocan SQLite en disco — nunca un path fijo compartido entre tests (evita colisión si `cargo test` corre en paralelo, que es el comportamiento por defecto).

## Tests obligatorios antes de Fase D (recordatorio de `Arquitectura §19.2`)

No implementados todavía en el spike (pertenecen a la vertical slice real, no al spike de lenguaje): schema validation, atomic writes, revision consistency states, provider timeout/unavailable, partial coverage, token budget, deduplication, critical blocking predicate, prompt injection sanitization, path traversal, concurrent reads, write locks, cache rebuild, monorepo cross-package relevance, baseline deadline, context packet determinism. Ver Fase D5 del plan de arranque para el subconjunto exigido en la vertical slice.

## Qué el spike sí demostró como viable

- Deadline + cancelación real de subproceso, verificado con un test manual de tiempo (`--demo-timeout`), no con `cargo test` — porque requiere medir wall-clock, no solo un assert de valor. Para Fase D, este tipo de test de latencia debe vivir en la categoría "performance" de la pirámide (`Arquitectura §19.1`), separado de la suite unitaria rápida.
