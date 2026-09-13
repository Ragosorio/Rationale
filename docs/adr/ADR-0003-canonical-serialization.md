# ADR-0003: Canonical serialization

**Status:** proposed — pending independent cross-review before `accepted`.
**Date:** 2026-07-25
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none

## Context

`Rationale_v0.5.md §26.1` requires the canonical layer to be "reviewable in a PR" and "readable without the tool". `Rationale_Arquitectura_Conceptual_v0.1.md §21` (proposed structure) and all the product documentation (`v0.5 §5, §9, §27`) already model Subjects, Records, Bindings, and Approvals in YAML — it is not an abstract decision; a real corpus of canonical YAML data already exists: 9 Subjects and 2 Records (one real, one fixture) in `.rationale/` and `fixtures/vertical-slice/.rationale/`.

Regardless of the format, `docs/rust/style-guide.md` already recorded a concrete risk: the `serde_yaml` dependency is marked `+deprecated` upstream (confirmed in the `cargo build` output of the language spike itself, Phase C). Phase E multiplies the volume of YAML parsing (Subjects, Evidence, Assessments) — migrating the dependency after that costs substantially more than migrating it now, before the code depends on it in more than three modules.

## Decision

1. **YAML remains the canonical format** of `.rationale/` (Subjects, Records, Bindings, Approvals, Evidence, and Assessments where appropriate). JSON is reserved for the Context Packet served over MCP (an exchange format, not a storage format) and for validation schemas (`.rationale/schemas/*.json`; JSON Schema is the de facto standard for that role).
2. **`serde_yaml` is replaced by `yaml_serde` (0.10.4)** in the root `Cargo.toml`, before Phase E2 expands the canonical model.

## Evidence

- The existing canonical corpus (`.rationale/subjects/*.yaml`, `.rationale/records/*.yaml`, `fixtures/vertical-slice/.rationale/`) is already YAML — rewriting it to JSON would have a real migration cost with no clear benefit, since no requirement in `v0.5 §26.1` favors JSON over YAML for this use.
- Long text fields (`statement`, `rationale`, `problem.statement`) use YAML block scalars (`>`) extensively in the examples in `v0.5 §27` — JSON has no equally readable equivalent for multi-line text without escapes.
- **A real compatibility test run in this session**: a throwaway Cargo project (`/tmp/yaml-serde-probe`, since deleted) deserialized a real Subject from the repository (`policy.local-first.yaml`) with `yaml_serde::from_str` and serialized it back with `yaml_serde::to_string`. Both operations have the same signature as in `serde_yaml` (`from_str<T>(&str) -> Result<T>`, `to_string<T>(&T) -> Result<String>`), and the round trip succeeded on real, not synthetic, data.
- `yaml_serde` is backed by "The YAML Organization" (`github.com/yaml/yaml-serde`), licensed `MIT OR Apache-2.0` — the same license range as `serde_yaml`, with no change to the dependency policy.

## Alternatives considered

- **Migrating all of `.rationale/` to JSON**: discarded — the cost of rewriting the existing corpus with no benefit (`v0.5 §26.1` does not require JSON), and a loss of readability for long text fields.
- **Staying on `serde_yaml`**: discarded — it is a dependency whose maintenance was discontinued; Phase E multiplies its usage surface, and migrating after that (with the Subject Resolver, Evidence, and Assessments already depending on its API) would cost more than migrating now.
- **`serde_yaml_ng`** (a single-maintainer fork, `acatton`): discarded in favor of `yaml_serde` because the latter has organizational backing (`github.com/yaml`) rather than a single maintainer — more durable in the long run, the same criterion `Arquitectura §18.4` already applied to dependency risk.
- **`serde_yaml_bw`** ("panic-free parsing, including malformed YAML"): interesting for error tolerance, but not tested in this session — not discarded for the future, only not this ADR's decision without its own evidence.

## Consequences

- Change `serde_yaml = "0.9"` to `yaml_serde = "0.10.4"` in the root `Cargo.toml`. The API used in the current code (`src/configuration.rs`, `src/storage.rs`) uses `serde_yaml::from_str` — the same pattern works with `yaml_serde::from_str` without logic changes, only the crate name and the `use`.
- The 7 JSON schemas planned for Phase E2 (`.rationale/schemas/*.json`) are not affected — they are JSON Schema, a validation format, not canonical storage.
- The Context Packet served over MCP (Phase E5) is serialized as JSON, not YAML — it is JSON-RPC 2.0's native format, not an independent choice.

## Risks

- `yaml_serde` is a relatively new crate (recent publication metadata) — less production history than `serde_yaml` in its prime. Mitigation: the round trip was already tested against real data; `Cargo.lock` stays pinned and is reviewed before every version update (`Arquitectura §18.4`).
- The dependency change touches every file that uses `serde_yaml::` today — it requires a mechanical find-and-replace pass when implemented, with `cargo test` as the safety net (19 existing tests, several of which read real YAML).

## Validation

The compatibility test was run and is described in Evidence. The real migration of `Cargo.toml` and the code happens in Phase E2 (`docs/dependencies/inventory.yaml` is updated in the same commit), with `cargo test` passing as the acceptance criterion.

**This ADR is `proposed`**, pending cross-review and human approval.

## Revisit trigger

Reopen if `yaml_serde` stops being maintained (the same pattern that led to this ADR with `serde_yaml`), or if a concrete Phase F/G need shows that JSON would be preferable for a specific portion of the canon (for example, if the volume of Records makes YAML parsing a measured, not hypothetical, bottleneck).
