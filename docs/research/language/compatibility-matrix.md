# Compatibility matrix — macOS / Linux / Windows

Este spike se ejecutó únicamente en la máquina de referencia (macOS arm64, `docs/environment/reference-development-machine.md`). Linux y Windows **no se probaron en ejecución real** — no hay runners disponibles en este entorno de bootstrap. Lo que sigue es una evaluación de viabilidad basada en las dependencias y APIs elegidas para cada candidato, no una verificación empírica. Se marca explícitamente qué es verificado y qué es evaluado por diseño.

## Rust

| Plataforma | Estado | Base de la evaluación |
|---|---|---|
| macOS arm64 | ✅ **Verificado** | Build + tests + demos ejecutados en esta sesión |
| macOS x86_64 | Viable (no verificado) | `rustc`/`cargo` soportan el target oficialmente; `rusqlite` con feature `bundled` vendoriza su propio SQLite en C, sin depender de una versión de sistema |
| Linux (amd64/arm64) | Viable (no verificado) | Mismo argumento: toolchain oficial de Tier 1, `bundled` SQLite evita depender del paquete del sistema |
| Windows (amd64) | Viable con matiz (no verificado) | `rusqlite bundled` compila el SQLite vendorizado con el linker de MSVC/MinGW — soportado oficialmente por el crate, pero no probado aquí. El uso de `std::fs::File` para locking (no usado en el spike, se usó `flock` vía FFI directo con `cfg(unix)`) sí tiene una API estable multiplataforma en la std desde Rust 1.89 que **no se ejercitó** en este spike por simplicidad — pendiente de probar antes de comprometerse en Fase D |

## Go

| Plataforma | Estado | Base de la evaluación |
|---|---|---|
| macOS arm64 | ✅ **Verificado** | Build + tests + fuzzing + demos ejecutados en esta sesión |
| macOS x86_64 | Viable (no verificado) | Cross-compilation nativa de Go (`GOOS`/`GOARCH`), sin toolchain adicional |
| Linux (amd64/arm64) | Viable (no verificado) | `modernc.org/sqlite` es pure-Go (sin cgo) — elegido deliberadamente en este spike para evitar la dependencia de un compilador C en el target, lo cual mejora la historia de cross-compilation frente a `mattn/go-sqlite3` (cgo-based) |
| Windows (amd64) | **Gap real encontrado, no solo teórico** | `syscall.Flock` (usado en `demo-lock`) es **POSIX-only** — el propio código del spike falla explícitamente en Windows (`runtime.GOOS == "windows"` devuelve error). Windows requeriría una implementación separada vía `LockFileEx` (paquete `golang.org/x/sys/windows` o similar), no incluida en este spike |

## Hallazgo transversal

Ningún candidato fue probado en ejecución real fuera de macOS arm64 en esta fase. La diferencia real y ya confirmada (no hipotética) es el file locking: Rust usó una ruta POSIX-only por simplicidad de implementación en el spike, pero tiene una alternativa estándar multiplataforma sin usar (`std::fs::File::lock`); Go usó `syscall.Flock`, que **no tiene equivalente en la stdlib para Windows** y requeriría código adicional específico de plataforma.

**Antes de comprometerse en Fase D con cualquiera de los dos candidatos**, si el lenguaje elegido es Go, escribir el path de Windows para file locking debe ser parte del ADR-0001 como riesgo explícito, no asumirse resuelto.
