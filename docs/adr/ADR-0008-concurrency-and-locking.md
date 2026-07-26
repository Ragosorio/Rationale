# ADR-0008: Concurrency and locking

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-26
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

Fase F1 convierte `src/storage.rs` de solo-lectura a lectura/escritura (`write_record`), el prerequisito para que `finalize_change` (Fase F5) y `rationale review` (Fase F6) puedan persistir propuestas y aprobaciones en `.rationale/`. `Arquitectura §11.6` exige "escribir cambios atómicos" y `§15.3` exige "usar archivos temporales y rename atómico" para proteger writes — pero ninguno de los dos especifica qué pasa bajo escritura **concurrente** real (dos invocaciones de la CLI, o un servidor MCP y una revisión humana, escribiendo el mismo Record al mismo tiempo).

`docs/dependencies/inventory.yaml` ya registraba un `known_gap` desde el spike de lenguaje: la ruta de file locking del candidato Rust era POSIX-only (`flock` vía FFI), sin ejercitar en Windows. Hasta ahora esto no importaba porque no existía ninguna escritura real que lockear.

## Decision

1. **`rename` atómico es la única garantía de corrección** para escritura concurrente en Fase F — no se añade un lock de archivo (`flock`, `File::lock` portable, ni un lock externo) todavía. Dos escrituras concurrentes al mismo Record producen "last-rename-wins": el resultado final es exactamente uno de los dos Records completos, nunca una fusión ni un archivo corrupto o truncado.
2. **El `known_gap` de Windows queda formalmente diferido**, no resuelto: no bloqueante hasta que exista evidencia de un caso real de escritura concurrente entre procesos (no solo hilos) en ese SO.
3. El nombre del archivo temporal de `write_record` se hizo único **por invocación**, no solo por proceso (PID + contador atómico), cerrando un bug real encontrado durante esta misma implementación (ver Evidence).

## Evidence

- **Bug real encontrado y corregido durante F1**: la primera versión de `write_record` nombraba el archivo temporal solo con el PID del proceso (`.{file}.tmp-{pid}`). Dos hilos del *mismo* proceso escribiendo el mismo Record a la vez habrían reutilizado el mismo nombre temporal, permitiendo que un hilo truncara el archivo temporal del otro a mitad de escritura — una corrupción real, no hipotética. Corregido añadiendo un contador atómico (`AtomicU64`) por proceso al nombre temporal.
- **Test de concurrencia real** (`storage::tests::concurrent_writes_to_same_record_never_corrupt_the_file`): 8 hilos escriben el mismo Record simultáneamente con contenidos distintos. Verificado 15 corridas consecutivas sin fallos: el archivo final siempre es un Record completo y válido (nunca corrupto), su contenido es exactamente uno de los 8 candidatos (nunca una mezcla), y no queda ningún archivo temporal huérfano tras la contención.
- **Test de escritura atómica bajo reemplazo** (`write_record_leaves_no_tmp_file_and_fully_replaces_existing`): una segunda escritura reemplaza la primera por completo, sin fusionar campos antiguos y nuevos.
- **Precedente ya existente en la capa derivada** (`cache::tests::concurrent_reads_do_not_corrupt_cache`, Fase E3): lecturas concurrentes sobre SQLite en modo WAL ya están cubiertas; este ADR cubre la pieza que faltaba, escritura concurrente sobre el canon YAML.

## Alternatives considered

- **File locking real (`flock` vía FFI, o `std::fs::File::lock` portable desde Rust 1.89)**: descartado *por ahora*. Añadir un lock exige decidir su alcance (¿por Record? ¿por proyecto completo?), su comportamiento ante procesos muertos que no liberan el lock, y ejercitarlo en Windows — trabajo real que hoy no tiene un caso de uso concreto que lo justifique (`AGENTS.md`: "no crear un daemon antes de medir necesidad" aplica por el mismo principio a locking). Se revisita si aparece evidencia de pérdida de escrituras en uso real.
- **Un lock de archivo advisory simple (`.rationale/.lock`) por proyecto**: descartado por ahora — serializaría toda escritura del proyecto (no solo del Record en conflicto), un costo desproporcionado sin evidencia de que las colisiones sean frecuentes. Candidato razonable si el revisit trigger se activa.
- **Base de datos transaccional para el canon** (en vez de archivos YAML): fuera de alcance — contradice `Arquitectura §26.1` (el canon debe ser legible y revisable en PR sin herramienta, ADR-0003).

## Consequences

- `write_record` no bloquea nunca esperando un lock — coherente con "fail open" y con la ausencia de un daemon persistente en esta fase.
- Bajo colisión real (dos escrituras al mismo Record en la misma ventana de tiempo), una de las dos se pierde silenciosamente desde la perspectiva de quien la emitió — no hay notificación de "tu escritura fue sobrescrita". Esto es aceptable para el patrón de uso actual (un agente + un humano revisando secuencialmente, `rationale review` de Fase F6), no para escritura verdaderamente concurrente multi-agente.
- El `known_gap` de Windows sigue abierto y ahora vive en este ADR en vez de solo en `inventory.yaml`.

## Risks

- **Pérdida silenciosa de una escritura bajo colisión real** — mitigado parcialmente porque el flujo previsto (`finalize_change` escribe en `.rationale/proposals/`, nunca directo a `records/`; `rationale review` es la única vía que escribe en `records/`) reduce drásticamente la ventana de colisión real: normalmente hay un solo proceso humano ejecutando `rationale review` a la vez.
- **El gap de Windows podría manifestarse antes de lo esperado** si Rationale se usa ahí con dos procesos concurrentes reales. Mitigación: el revisit trigger de abajo es concreto y verificable.

## Validation

Tests descritos en Evidence, corridos como parte de `cargo test` en cada verificación de fase. La prueba de concurrencia usa hilos (no procesos separados) porque ejercita la misma ruta de código (`std::fs::rename` sobre el mismo filesystem) con muchísimo menos overhead de test — la garantía de atomicidad de `rename` en POSIX es a nivel de sistema de archivos, no de proceso, así que la evidencia generaliza.

## Revisit trigger

Reabrir cuando: (a) aparezca un caso real de dos procesos (no hilos) escribiendo el mismo Record en una ventana de colisión medible — por ejemplo, dos agentes trabajando el mismo proyecto simultáneamente en Fase G/H; o (b) el piloto en monorepo (Fase H) corra en Windows y se necesite verificar que `rename` tiene la misma garantía atómica ahí (en NTFS, `MoveFileEx` con `MOVEFILE_REPLACE_EXISTING` es atómico para reemplazo de archivo en el mismo volumen — no verificado empíricamente todavía en este proyecto).
