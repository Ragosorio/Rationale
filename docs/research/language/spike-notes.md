# Spike notes — qualitative observations

Notes from the agent that implemented both candidates in the same session, with the same declared level of effort (`Proceso §9.2`).

## Maintainability with agents

- **Rust** requires resolving the ownership/borrowing model even in a small program (for example `child.stdout.take()`, explicit handling of `Option`/`Result` at every step). This produced more verbose code whose type errors appear at compile time — the compiler rejected any attempt to use a value after moving it, before anything ran. For an agent iterating quickly, this means more "fix the compile error" cycles but also more confidence that, if it compiles, a certain class of logic error (using a moved value, forgetting a `Result`) is already ruled out.
- **Go** allowed writing the first version faster and with less syntactic friction. The cost appeared later: the subprocess cancellation bug (`candidates.md`) was caught by neither the compiler nor the linter — only by empirical timing measurement. That is, Go moved the cost of "catching the error" from compile time to test/measurement time.
- For an agent workflow that relies heavily on `cargo test`/`go test` and automated measurement (exactly the pattern this project already follows, `Proceso §12`), both languages are viable, but **Rust fails earlier and louder; Go fails later and silently** unless there is deliberate instrumentation like this spike's.

## Ergonomics

- Go's standard library for JSON (`encoding/json`) and subprocesses (`os/exec`) is more direct to use than Rust's crate ecosystem (`serde_json` + manual `Command` handling), at the cost of fewer compile-time guarantees.
- Rust's error handling (`Result<T, E>` mandatory at every failure point) made writing fast more uncomfortable, but also made it impossible to silently ignore an I/O failure — in Go, an ignored error (`_`) compiles without warning unless an external linter is used (`errcheck`, not included in this spike for dependency parity).
- MCP framing (`Content-Length` + JSON-RPC) was implemented almost identically in both languages — it was not a real differentiator; both have sufficient support in their standard libraries (`std::io`/`io.Reader` with byte-by-byte reading and manual header parsing).

## What this spike does NOT evaluate

- Long-term maintainability in a project of tens of thousands of lines (this spike is deliberately small, `spike-protocol.md`).
- The availability and quality of agent-specific skills/documentation for each language — an assessment pending until after choosing (`Proceso §9.3`).
- Real behavior on Linux/Windows (`compatibility-matrix.md`) — assessed only by design, not verified in execution.
- Real packaging and distribution (Phase J, much later).

## Overall impression

Neither candidate showed a limitation that rules it out. The decision in ADR-0001 must explicitly weigh memory safety/reliability demonstrated empirically in this spike (in Rust's favor, because of the cancellation finding) against iteration speed and native fuzzing (in Go's favor) — exactly the tension that the weighted criteria in `Arquitectura_Conceptual_v0.1.md §8.2` already anticipated by giving "memory safety and reliability" the highest weight (20%).
