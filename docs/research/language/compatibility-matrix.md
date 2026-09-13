# Compatibility matrix — macOS / Linux / Windows

This spike ran only on the reference machine (macOS arm64, `docs/environment/reference-development-machine.md`). Linux and Windows **were not tested in real execution** — no runners were available in this bootstrap environment. What follows is a feasibility assessment based on the dependencies and APIs chosen for each candidate, not an empirical verification. What is verified and what is assessed by design is marked explicitly.

## Rust

| Platform | Status | Basis of the assessment |
|---|---|---|
| macOS arm64 | ✅ **Verified** | Build + tests + demos run in this session |
| macOS x86_64 | Viable (not verified) | `rustc`/`cargo` officially support the target; `rusqlite` with the `bundled` feature vendors its own SQLite in C, without depending on a system version |
| Linux (amd64/arm64) | Viable (not verified) | The same argument: an official Tier 1 toolchain, and `bundled` SQLite avoids depending on the system package |
| Windows (amd64) | Viable, with a caveat (not verified) | `rusqlite bundled` compiles the vendored SQLite with the MSVC/MinGW linker — officially supported by the crate, but not tested here. Using `std::fs::File` for locking (not used in the spike, which used `flock` through direct FFI with `cfg(unix)`) does have a stable cross-platform API in `std` since Rust 1.89 that **was not exercised** in this spike for simplicity — to be tested before committing to Phase D |

## Go

| Platform | Status | Basis of the assessment |
|---|---|---|
| macOS arm64 | ✅ **Verified** | Build + tests + fuzzing + demos run in this session |
| macOS x86_64 | Viable (not verified) | Go's native cross-compilation (`GOOS`/`GOARCH`), with no additional toolchain |
| Linux (amd64/arm64) | Viable (not verified) | `modernc.org/sqlite` is pure Go (no cgo) — chosen deliberately in this spike to avoid depending on a C compiler on the target, which improves the cross-compilation story compared with `mattn/go-sqlite3` (cgo-based) |
| Windows (amd64) | **A real gap found, not only theoretical** | `syscall.Flock` (used in `demo-lock`) is **POSIX-only** — the spike's own code fails explicitly on Windows (`runtime.GOOS == "windows"` returns an error). Windows would require a separate implementation through `LockFileEx` (the `golang.org/x/sys/windows` package or similar), not included in this spike |

## Cross-cutting finding

Neither candidate was tested in real execution outside macOS arm64 in this phase. The real, already confirmed (not hypothetical) difference is file locking: Rust used a POSIX-only path for implementation simplicity in the spike, but has an unused standard cross-platform alternative (`std::fs::File::lock`); Go used `syscall.Flock`, which **has no Windows equivalent in the standard library** and would require additional platform-specific code.

**Before committing to Phase D with either candidate**, if the chosen language is Go, writing the Windows file-locking path must be part of ADR-0001 as an explicit risk, not assumed solved.
