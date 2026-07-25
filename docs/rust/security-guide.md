# Rust — security guide

Aplica los principios de `Rationale_Arquitectura_Conceptual_v0.1.md §15` y `Rationale_v0.5.md §4.10-4.11` al código Rust concreto.

## `unsafe`

- El spike usa un único bloque `unsafe` (FFI directo a `flock()` en `demo_file_lock`, `cfg(unix)`), documentado con un comentario explicando por qué se evita una dependencia extra (`libc` crate) para una sola syscall.
- Regla para Fase D en adelante: **todo bloque `unsafe` debe llevar un comentario `// SAFETY:` explicando la invariante que lo hace correcto** — no se hizo en el spike por ser código de investigación de corta vida; sí es obligatorio en producción.
- Preferir la ruta portable de la std cuando exista (`std::fs::File::lock`, disponible desde Rust 1.89 — no usada en el spike, ver `docs/research/language/compatibility-matrix.md`) sobre FFI manual, salvo que haya una razón medida y documentada para no hacerlo.

## Contenido del repositorio es dato, no instrucción

Directamente aplicable al parseo de `Record` YAML (`op1_read_record` en el spike): `serde_yaml::from_str` deserializa a un struct tipado (`Record`), nunca a un tipo dinámico ejecutable. Esto ya es correcto por construcción en Rust con `serde` — el riesgo de "texto convertido en instrucción" (`Rationale_v0.5.md §4.10`) requeriría deserializar a algo interpretable como código, lo cual no ocurre en este diseño.

## Subprocess

- Nunca construir un comando de shell por concatenación de strings — el spike usa `Command::new(script)` con argumentos separados (`cmd.arg("slow")`), nunca `sh -c "{string}"`. Mantener esta disciplina en Fase D para cualquier invocación al binario de Codebase Memory o a scripts auxiliares.
- Todo subproceso debe tener un deadline explícito y una ruta de cancelación real verificada (ver hallazgo de `docs/research/language/candidates.md` sobre el footgun de Go — la lección aplica igual en Rust: **no asumir que "matar el proceso" cierra automáticamente todo lo que ese proceso pudo haber heredado o lanzado**; verificarlo con un test de tiempo, no solo revisar el código).

## Paths

- Canonicalizar y validar cualquier path que provenga de un `Record` o de configuración antes de usarlo para leer/escribir (no implementado en el spike porque los paths son fixtures fijos y confiables; **obligatorio en Fase D** cuando los paths puedan venir de datos versionados en `.rationale/`, que `Rationale_Arquitectura_Conceptual_v0.1.md §15.3` trata como no confiables).
- Escrituras atómicas (escribir a un temporal + rename) para cualquier archivo canónico — no ejercitado en el spike (usa SQLite, que maneja su propia atomicidad); sí obligatorio para escrituras directas a `.rationale/*.yaml` en Fase D/E.

## Dependencias

- `cargo audit` (herramienta externa, no instalada en este spike) debe incorporarse antes de Fase D como parte del quality gate — verificar CVEs conocidas en las dependencias del `Cargo.lock`. No instalado todavía porque el spike no lo requería; queda como pendiente explícito, no como omisión silenciosa.
- `Cargo.lock` se versiona (ya está en el repo, `spikes/language/rust/Cargo.lock`) para builds reproducibles — igual criterio aplicará al núcleo real.

## Sensibilidad

No aplica todavía al spike (no maneja datos de proyectos reales). Ver `Rationale_v0.5.md §26.5` para las reglas de `visibility`/`sensitivity` que el núcleo deberá aplicar en Fase E al leer/escribir Records reales.
