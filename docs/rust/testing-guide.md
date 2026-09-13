# Rust — testing guide

## Applicable pyramid (the subset of `Rationale_Arquitectura_Conceptual_v0.1.md §19.1` relevant today)

```text
Unit          cargo test — pure functions, without external I/O when possible
Integration   cargo test with real fixtures and the compiled binary (tests/)
Contract      fixtures against Codebase Memory (Arquitectura §19.3)
Property      no extra dependency (manual invariant tests); evaluate `proptest`
              when a concrete case justifies it — do not add it preemptively
              (Proceso §19: "is it necessary?")
Golden packet determinism of the complete Context Packet (Arquitectura §19.4)
```

## Commands

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test                    # the whole suite
cargo test <partial_name>     # filter by name
cargo test -- --nocapture     # show test stdout (useful to debug emitted JSON)
```

## Test conventions verified in the spike and kept in the core

- One test per pipeline operation when the operation has non-trivial logic
  (`test_op4_*`, `test_op5_*` in the spike). Do not test operations that are
  pure I/O without decision logic: "read a file" needs no dedicated test, but
  "what is decided from what was read" does.
- Real on-disk fixtures (`fixtures/record.yaml`) instead of YAML strings
  embedded in the test, so the same fixture feeds both the real pipeline and the
  test and they cannot diverge.
- Invariant tests (`test_severity_weight_monotonic_property`) when a
  property-testing dependency is not justified. State in the test name that it
  is a manual property test, so it is clear it does not replace a dedicated
  framework if one is added later.
- `std::env::temp_dir()` with a unique suffix for tests that touch the disk or
  SQLite — never a fixed path shared between tests, because `cargo test` runs in
  parallel by default.
- Tests that protect a contract with agents (skill content, prompt text, tool
  descriptions) assert the property that matters — a language rule, a link that
  exists, a list that mirrors the gate — rather than whole paragraphs.

## Tests required before Phase D (reminder of `Arquitectura §19.2`)

These belonged to the real vertical slice, not the language spike: schema
validation, atomic writes, revision consistency states, provider
timeout/unavailability, partial coverage, token budget, deduplication, the
critical blocking predicate, prompt-injection sanitization, path traversal,
concurrent reads, write locks, cache rebuild, monorepo cross-package relevance,
baseline deadline, and context-packet determinism. See Phase D5 of the kickoff
plan for the subset required in the vertical slice.

## What the spike did prove viable

- A deadline with real subprocess cancellation, verified with a manual timing
  test (`--demo-timeout`) rather than `cargo test`, because it measures
  wall-clock time rather than asserting a value. This kind of latency test
  belongs in the "performance" category of the pyramid (`Arquitectura §19.1`),
  separate from the fast unit suite.
