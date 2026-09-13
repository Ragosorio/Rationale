# Rust — style guide

Applies to all Rust code in the repository (the core in `src/` and the
throwaway spike in `spikes/language/rust/`). It complements, and does not
repeat, `AGENTS.md`.

## Required tools

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # needed in non-interactive shells, see docs/environment/
cargo fmt --check                       # formatting (rustfmt.toml at the repository root)
cargo clippy --all-targets -- -D warnings   # lint, zero warnings tolerated
```

No commit may introduce code that fails `cargo fmt --check` or produces a new
Clippy warning. Both must run before closing any work item
(`AGENTS.md §Quality gates`).

## Conventions

- **Edition 2021**, `stable` toolchain (not nightly) — set by ADR-0001; do not
  use nightly-only features.
- The spike's `Cargo.toml` sets `panic = "abort"` for release builds to avoid
  unwinding overhead in a distributed binary. The core does not: the MCP server
  relies on `catch_unwind` to turn a tool panic into `isError` without ending the
  session.
- Name functions that implement a step of an external contract with that
  contract's vocabulary (for example `op1_read_record` in the spike, mapped 1:1
  to `spike-protocol.md`). It makes traceability easier during cross-review.
- Prefer an explicit `Result<T, E>` over `.unwrap()`/`.expect()` in production
  code. The spike used `.expect()` liberally because it is short-lived research
  code; that is **not** the standard for the core.
- Comments only when they explain a non-obvious why (an invariant, a
  workaround, a design decision) — the project's general policy. Do not repeat
  in a comment what the function name already says. Existing code comments are
  in Spanish; follow the surrounding file.
- Text an agent reads (skills, pre-made actions, MCP tool descriptions) is
  written in English and asks the agent to reply in the user's language.

## Dependencies

Before adding a dependency, follow `docs/dependencies/inventory.yaml` and
`Rationale_Proceso_Construccion_Agentes_v0.1.md §19`: is it necessary? Is there
an alternative in `std`? Is the license compatible? Is it maintained? Does it
build on the three target operating systems?

Dependencies validated in the spike (`docs/research/language/candidates.md`)
were a starting point for Phase D, not a final decision:

| Crate | Purpose | Note |
|---|---|---|
| `rusqlite` (feature `bundled`) | Embedded SQLite | Vendors its own SQLite in C — no system dependency |
| `serde` + `serde_json` | JSON serialization | The ecosystem's de facto standard |
| `serde_yaml` | Parsing YAML Records | Marked `deprecated` upstream; ADR-0003 replaced it with `yaml_serde` in the core |

## References

- [The Rust Book](https://doc.rust-lang.org/book/) (official)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- `rustfmt.toml` at the repository root — the current formatting configuration
