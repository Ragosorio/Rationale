# Language spike candidates — notes

Complete implementations in `spikes/language/rust/` and `spikes/language/go/`, both running exactly the 6 operations of `spike-protocol.md`, both with a minimal MCP server, file locking, a subprocess with deadline/cancellation, and a test suite. Raw measurements in `benchmark-results.json`.

## Rust (`spikes/language/rust/`)

**Dependencies:** `rusqlite` (with the `bundled` feature, which vendors its own SQLite in C), `serde` + `serde_json`, `serde_yaml`.

**What went right on the first attempt:**
- The 6 operations + MCP server + file lock + deadline worked correctly in the first implementation, with nothing to fix after writing them.
- Subprocess cancellation (a 500 ms deadline) was implemented with a manual poll (`try_wait()` in a loop + `kill()` on expiry) that **never tries to read stdout on the timeout path** — this naturally avoided the "orphaned grandchild holding the pipe open" problem that did affect Go's first version (see below). It was not a conscious decision to avoid the bug; it was a consequence of the manual implementation style, but the result is relevant to the reliability criterion.
- A smaller binary (2.2 MB) and lower peak resident memory (~3 MB) than Go, consistent with a minimal runtime without a garbage collector.

**Friction:**
- Noticeably longer cold compilation time (31.94 s versus Go's 9.59 s) — a higher cost per iteration during active development with agents.
- There is no native fuzzing/property testing in `std`; a manual monotonic invariant test was implemented instead of using `proptest`/`cargo-fuzz`, to avoid introducing an extra dependency Go would not need (keeping workload parity).
- Real cross-platform file locking (`std::fs::File::lock`, available since Rust 1.89) was not used in the spike — a POSIX-only path through direct FFI to `flock()` was used for simplicity, leaving the path that would be portable unverified.

## Go (`spikes/language/go/`)

**Dependencies:** `modernc.org/sqlite` (pure Go, without cgo — chosen deliberately for its better cross-compilation story compared with `mattn/go-sqlite3`), `gopkg.in/yaml.v3`.

**What went right:**
- Cold compilation 3.3× faster than Rust (9.59 s versus 31.94 s) — a real advantage for the agents' iteration cycle.
- Native fuzzing (`go test -fuzz`) without any external dependency: 384,496 executions in 10 seconds, zero failures. This is a genuine differentiator of the language, not of the spike — Rust would need an external crate for the same.
- The MCP server and file locking (on the supported platform) worked correctly.

**Real, not hypothetical, friction — it required a fix:**
- The first implementation of the subprocess with a deadline used the library's standard idiomatic pattern (`exec.CommandContext` + `cmd.Output()`). With a 500 ms deadline against a mock provider sleeping 5 s, **cancellation took 5016 ms, not ~500 ms** — the context expired and the direct child process (the `bash` script) received the signal, but the grandchild (`sleep`, a child of bash) inherited the stdout pipe's write descriptor and stayed alive; `cmd.Output()` blocks reading until EOF, which does not arrive until **every** holder of the pipe closes its copy — that is, until `sleep` finishes on its own.
- The fix required giving up the convenience of `cmd.Output()` and manually using `Setpgid: true` in `SysProcAttr`, capturing stdout in a `bytes.Buffer` (not a pipe read after `Wait()`, which has its own close race), and killing the **whole process group** (`syscall.Kill(-pid, ...)`) instead of only the direct child. With that fix, cancellation does happen in ~503 ms.
- This is a real ergonomics/reliability difference under the "memory safety and reliability" criterion (20% of the weight): Go's simplest idiomatic path had a known Unix footgun that produced a silently wrong result (no error, just slow) until it was fixed explicitly.
- File locking (`syscall.Flock`) is POSIX-only; Windows was out of the spike's scope and would require additional work (see `compatibility-matrix.md`).

## Summary for ADR-0001

Neither candidate was discarded for inability — both completed the 6 operations and the additional required tests. The central difference is not in "what can be done" but in **the default idiomatic path**: Rust forced a manual design (a poll loop) that was correct from the first attempt; Go allowed a shorter implementation with a convenience function (`cmd.Output()`) that hid a real cancellation bug until empirical verification. Compensating in the other direction: Go compiles 3.3× faster and has native fuzzing without dependencies, two real advantages for the development cycle with agents.
