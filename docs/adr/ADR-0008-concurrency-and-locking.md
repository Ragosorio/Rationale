# ADR-0008: Concurrency and locking

**Status:** proposed — pendiente de revisión cruzada independiente antes de `accepted`.
**Date:** 2026-07-26; ampliado 2026-07-28
**Deciders:** Claude Code (análisis e implementación); pendiente aprobación humana y/o revisión cruzada de otro agente
**Supersedes / Superseded by:** ninguno

## Context

Fase F1 convierte `src/storage.rs` de solo-lectura a lectura/escritura (`write_record`), el prerequisito para que `finalize_change` (Fase F5) y `rationale review` (Fase F6) puedan persistir propuestas y aprobaciones en `.rationale/`. `Arquitectura §11.6` exige "escribir cambios atómicos" y `§15.3` exige "usar archivos temporales y rename atómico" para proteger writes — pero ninguno de los dos especifica qué pasa bajo escritura **concurrente** real (dos invocaciones de la CLI, o un servidor MCP y una revisión humana, escribiendo el mismo Record al mismo tiempo).

`docs/dependencies/inventory.yaml` ya registraba un `known_gap` desde el spike de lenguaje: la ruta de file locking del candidato Rust era POSIX-only (`flock` vía FFI), sin ejercitar en Windows. Hasta ahora esto no importaba porque no existía ninguna escritura real que lockear.

## Decision

1. **`rename` atómico es la única garantía de corrección** para escritura concurrente en Fase F — no se añade un lock de archivo (`flock`, `File::lock` portable, ni un lock externo) todavía. Dos escrituras concurrentes al mismo Record producen "last-rename-wins": el resultado final es exactamente uno de los dos Records completos, nunca una fusión ni un archivo corrupto o truncado.
2. **El `known_gap` de Windows queda formalmente diferido**, no resuelto: no bloqueante hasta que exista evidencia de un caso real de escritura concurrente entre procesos (no solo hilos) en ese SO.
3. El nombre del archivo temporal de `write_record` se hizo único **por invocación**, no solo por proceso (PID + contador atómico), cerrando un bug real encontrado durante esta misma implementación (ver Evidence).
4. Los archivos completos administrados por `install-agent` (hoy, los skills de
   Claude Code) usan **claim por identidad**, no un lock general: Rationale
   renombra atómicamente la entrada existente a un nombre único hermano,
   verifica el hash de esa identidad reclamada y solo entonces la retira o
   publica el reemplazo.
5. La publicación de un skill usa semántica **no-clobber** mediante un archivo
   completo y sincronizado seguido de `hard_link` al destino. Si otro proceso
   recrea el path durante la operación, la publicación falla cerrada, conserva
   el contenido concurrente y reporta dónde quedó la copia reclamada.
6. No se añade un lock de proyecto ni de skill. Un lock advisory solo
   coordinaría procesos cooperativos de Rationale; no impediría que un editor u
   otro agente sustituyera el path entre la comprobación y el borrado.
7. El manifest no tiene autoridad para elegir libremente cómo se revierte un
   path. `uninstall-agent` deriva de código la tupla exacta
   `path + agent + reversal`: archivos de instrucciones y MCP siempre usan
   `managed-part`; únicamente los `SKILL.md` enumerados usan `owned-file` y
   estos requieren hash no vacío.
8. Los nombres de claim incluyen PID, timestamp nanosegundo y contador atómico.
   Antes del `rename`, un destino de claim ya existente se trata como colisión
   y se busca otro nombre; nunca se reutiliza ni sobrescribe una cuarentena
   abandonada.

## Evidence

- **Bug real encontrado y corregido durante F1**: la primera versión de `write_record` nombraba el archivo temporal solo con el PID del proceso (`.{file}.tmp-{pid}`). Dos hilos del *mismo* proceso escribiendo el mismo Record a la vez habrían reutilizado el mismo nombre temporal, permitiendo que un hilo truncara el archivo temporal del otro a mitad de escritura — una corrupción real, no hipotética. Corregido añadiendo un contador atómico (`AtomicU64`) por proceso al nombre temporal.
- **Test de concurrencia real** (`storage::tests::concurrent_writes_to_same_record_never_corrupt_the_file`): 8 hilos escriben el mismo Record simultáneamente con contenidos distintos. Verificado 15 corridas consecutivas sin fallos: el archivo final siempre es un Record completo y válido (nunca corrupto), su contenido es exactamente uno de los 8 candidatos (nunca una mezcla), y no queda ningún archivo temporal huérfano tras la contención.
- **Test de escritura atómica bajo reemplazo** (`write_record_leaves_no_tmp_file_and_fully_replaces_existing`): una segunda escritura reemplaza la primera por completo, sin fusionar campos antiguos y nuevos.
- **Precedente ya existente en la capa derivada** (`cache::tests::concurrent_reads_do_not_corrupt_cache`, Fase E3): lecturas concurrentes sobre SQLite en modo WAL ya están cubiertas; este ADR cubre la pieza que faltaba, escritura concurrente sobre el canon YAML.
- **Carrera TOCTOU reproducida como unidad**:
  `claimed_removal_never_deletes_a_recreated_destination` reclama el archivo,
  recrea el path con contenido de usuario y verifica que la eliminación solo
  retire la identidad reclamada.
- **Publicación sin sobrescritura**:
  `no_clobber_publish_preserves_a_destination_that_reappeared` verifica que un
  destino concurrente sobreviva byte por byte.
- **Edición conservada y restaurada**:
  `edited_claim_is_restored_without_overwrite` verifica que un hash inesperado
  restaure el archivo y no deje una cuarentena en el caso normal.
- **Claim abandonado conservado**:
  `claim_skips_an_abandoned_destination_instead_of_overwriting_it` demuestra
  que un nombre ocupado no se reemplaza y que tanto el archivo actual como la
  cuarentena previa quedan byte por byte intactos.
- **Manifest sin autoridad destructiva**:
  `uninstall_rejects_owned_file_reversal_for_managed_part_files` cubre
  `CLAUDE.md` y `.mcp.json`; aunque el manifest incluya el hash correcto, no
  puede convertirlos en archivos completos propiedad de Rationale.

## Alternatives considered

- **File locking real (`flock` vía FFI, o `std::fs::File::lock` portable desde Rust 1.89)**: descartado *por ahora*. Añadir un lock exige decidir su alcance (¿por Record? ¿por proyecto completo?), su comportamiento ante procesos muertos que no liberan el lock, y ejercitarlo en Windows — trabajo real que hoy no tiene un caso de uso concreto que lo justifique (`AGENTS.md`: "no crear un daemon antes de medir necesidad" aplica por el mismo principio a locking). Se revisita si aparece evidencia de pérdida de escrituras en uso real.
- **Un lock de archivo advisory simple (`.rationale/.lock`) por proyecto**: descartado por ahora — serializaría toda escritura del proyecto (no solo del Record en conflicto), un costo desproporcionado sin evidencia de que las colisiones sean frecuentes. Candidato razonable si el revisit trigger se activa.
- **Lock por skill**: descartado para cerrar este riesgo. Evitaría dos
  instalaciones de Rationale simultáneas si ambas cooperan, pero no protege
  frente a editores, scripts u otros agentes que no toman el lock. Podría
  añadirse más adelante para mejorar mensajes y evitar trabajo duplicado, pero
  no sustituye la captura atómica de identidad.
- **Comprobar dos veces el hash antes de `remove_file(path)`**: descartado. Solo
  estrecha la ventana; sigue existiendo un instante entre la segunda
  comprobación y el borrado por nombre.
- **Base de datos transaccional para el canon** (en vez de archivos YAML): fuera de alcance — contradice `Arquitectura §26.1` (el canon debe ser legible y revisable en PR sin herramienta, ADR-0003).

## Consequences

- `write_record` no bloquea nunca esperando un lock — coherente con "fail open" y con la ausencia de un daemon persistente en esta fase.
- Bajo colisión real (dos escrituras al mismo Record en la misma ventana de tiempo), una de las dos se pierde silenciosamente desde la perspectiva de quien la emitió — no hay notificación de "tu escritura fue sobrescrita". Esto es aceptable para el patrón de uso actual (un agente + un humano revisando secuencialmente, `rationale review` de Fase F6), no para escritura verdaderamente concurrente multi-agente.
- El `known_gap` de Windows sigue abierto y ahora vive en este ADR en vez de solo en `inventory.yaml`.
- El uninstall de un archivo completo ya no borra el nombre que observó antes:
  borra el nombre único reclamado. Un archivo que reaparezca en el destino
  pertenece al proceso concurrente y queda intacto.
- Sistemas de archivos que no admitan hard links o publicación segura hacen
  fallar la operación sin sobrescribir. Esto prioriza conservar datos sobre
  completar silenciosamente la instalación.

## Risks

- **Pérdida silenciosa de una escritura bajo colisión real** — mitigado parcialmente porque el flujo previsto (`finalize_change` escribe en `.rationale/proposals/`, nunca directo a `records/`; `rationale review` es la única vía que escribe en `records/`) reduce drásticamente la ventana de colisión real: normalmente hay un solo proceso humano ejecutando `rationale review` a la vez.
- **El gap de Windows podría manifestarse antes de lo esperado** si Rationale se usa ahí con dos procesos concurrentes reales. Mitigación: el revisit trigger de abajo es concreto y verificable.
- **Un proceso con un descriptor ya abierto puede seguir escribiendo la
  identidad reclamada después del rename.** Rationale nunca confundirá esa
  identidad con un path nuevo, pero la coordinación perfecta con writers no
  cooperativos requeriría primitives exclusivas específicas del SO. El
  comportamiento actual evita sobrescribir la entrada concurrente y conserva
  la copia reclamada si no puede restaurarla; ese límite debe permanecer
  visible.
- **Existe una ventana mínima entre comprobar que el nombre único de claim está
  libre y ejecutar `rename`.** La identidad incluye timestamp nanosegundo y
  contador atómico, y los nombres abandonados detectados se saltan, por lo que
  una colisión accidental deja de depender de reutilizar un PID. Cerrar también
  una creación hostil exactamente en esa ventana exigiría una primitiva
  `rename-no-replace` específica del SO o locking cooperativo; no se presenta
  la comprobación actual como garantía frente a un atacante local.

## Validation

Tests descritos en Evidence, corridos como parte de `cargo test` en cada
verificación de fase. La prueba de concurrencia del canon usa hilos (no procesos
separados) porque ejercita la misma ruta de código (`std::fs::rename` sobre el
mismo filesystem) con muchísimo menos overhead de test. Los tests de skills
separan explícitamente claim, recreación del destino y finalización para hacer
determinista la ventana TOCTOU que sería probabilística con sleeps.

## Revisit trigger

Reabrir cuando: (a) aparezca un caso real de dos procesos (no hilos) escribiendo
el mismo Record en una ventana de colisión medible — por ejemplo, dos agentes
trabajando el mismo proyecto simultáneamente en Fase G/H; (b) el piloto en
monorepo (Fase H) corra en Windows y se necesite verificar empíricamente
`rename` y `hard_link` en NTFS; o (c) aparezca evidencia de writers que mantienen
un descriptor abierto sobre un skill mientras `install-agent` lo reemplaza. El
caso (c) justificaría estudiar handles exclusivos por plataforma; no un lock
advisory presentado como garantía universal.
