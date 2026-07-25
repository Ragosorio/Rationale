# Candidatos del spike de lenguaje — notas

Implementaciones completas en `spikes/language/rust/` y `spikes/language/go/`, ambas ejecutando exactamente las 6 operaciones de `spike-protocol.md`, ambas con servidor MCP mínimo, file locking, subprocess con deadline/cancelación, y suite de tests. Mediciones crudas en `benchmark-results.json`.

## Rust (`spikes/language/rust/`)

**Dependencias:** `rusqlite` (con feature `bundled`, vendoriza su propio SQLite en C), `serde` + `serde_json`, `serde_yaml`.

**Lo que salió bien al primer intento:**
- Las 6 operaciones + MCP server + file lock + deadline funcionaron correctamente en la primera implementación, sin necesidad de corregir nada después de escribirlas.
- La cancelación de subproceso (deadline de 500ms) se implementó con un poll manual (`try_wait()` en loop + `kill()` al expirar) que **nunca intenta leer stdout en el camino de timeout** — esto evitó naturalmente el problema del "nieto huérfano que sostiene el pipe abierto" que sí afectó a la primera versión de Go (ver abajo). No fue una decisión consciente de evitar el bug; fue una consecuencia del estilo de implementación manual, pero el resultado es relevante para el criterio de confiabilidad.
- Binario más pequeño (2.2MB) y con memoria residente pico más baja (~3MB) que Go, consistente con un runtime mínimo sin garbage collector.

**Fricciones:**
- Tiempo de compilación en frío notablemente más largo (31.94s vs 9.59s de Go) — mayor costo por iteración durante desarrollo activo con agentes.
- No existe fuzzing/property testing nativo en la std; se implementó un test manual de invariante monótona en vez de usar `proptest`/`cargo-fuzz`, para no introducir una dependencia extra que Go no necesitaría (mantener paridad de carga).
- El file locking multiplataforma real (`std::fs::File::lock`, disponible desde Rust 1.89) no se usó en el spike — se usó una ruta POSIX-only vía FFI directo a `flock()` por simplicidad, dejando sin verificar la ruta que sí sería portable.

## Go (`spikes/language/go/`)

**Dependencias:** `modernc.org/sqlite` (pure-Go, sin cgo — elegido deliberadamente por su mejor historia de cross-compilation frente a `mattn/go-sqlite3`), `gopkg.in/yaml.v3`.

**Lo que salió bien:**
- Compilación en frío 3.3× más rápida que Rust (9.59s vs 31.94s) — ventaja real para el ciclo de iteración de agentes.
- Fuzzing nativo (`go test -fuzz`) sin ninguna dependencia externa: 384.496 ejecuciones en 10 segundos, cero fallos. Este es un diferenciador genuino del lenguaje, no del spike — Rust necesitaría un crate externo para lo mismo.
- Servidor MCP y file locking (en la plataforma soportada) funcionaron correctamente.

**Fricción real, no hipotética — requirió una corrección:**
- La primera implementación del subproceso con deadline usó el patrón idiomático estándar de la librería (`exec.CommandContext` + `cmd.Output()`). Con un deadline de 500ms contra un proveedor mock que duerme 5s, **la cancelación tardó 5016ms, no ~500ms** — el contexto expiraba y el proceso hijo directo (el script `bash`) recibía la señal, pero el nieto (`sleep`, hijo de bash) heredaba el descriptor de escritura del pipe de stdout y seguía vivo; `cmd.Output()` bloquea leyendo hasta EOF, que no llega hasta que **todos** los tenedores del pipe cierran su copia — es decir, hasta que `sleep` termina por sí solo.
- La corrección requirió abandonar la conveniencia de `cmd.Output()` y usar manualmente: `Setpgid: true` en `SysProcAttr`, capturar stdout en un `bytes.Buffer` (no un pipe leído tras `Wait()`, que tiene su propia carrera de cierre), y matar el **grupo de procesos completo** (`syscall.Kill(-pid, ...)`) en vez de solo el proceso hijo directo. Con esa corrección, la cancelación sí ocurre en ~503ms.
- Esta es una diferencia real de ergonomía/confiabilidad bajo el criterio de "seguridad de memoria y confiabilidad" (20% del peso): el camino idiomático más simple en Go tenía un footgun conocido de Unix que produjo un resultado incorrecto silencioso (sin error, solo lento) hasta que se corrigió explícitamente.
- File locking (`syscall.Flock`) es POSIX-only; Windows quedó fuera de alcance del spike y requeriría trabajo adicional (ver `compatibility-matrix.md`).

## Resumen para ADR-0001

Ningún candidato fue descartado por incapacidad — ambos completaron las 6 operaciones y las pruebas adicionales exigidas. La diferencia central no está en "qué se puede hacer" sino en **el camino idiomático por defecto**: Rust obligó a un diseño manual (poll loop) que resultó en corrección desde el primer intento; Go permitió una implementación más corta con una función de conveniencia (`cmd.Output()`) que ocultó un bug real de cancelación hasta la verificación empírica. Compensando en la otra dirección: Go compila 3.3× más rápido y tiene fuzzing nativo sin dependencias, dos ventajas reales para el ciclo de desarrollo con agentes.
