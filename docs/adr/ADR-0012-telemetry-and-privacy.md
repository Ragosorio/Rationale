# ADR-0012: Telemetry and privacy

**Status:** proposed — validation failed in part (see "Validation update — 2026-07-28"). Replacement proposed in ADR-0014.
**Date:** 2026-07-25
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none yet — ADR-0014 proposes replacing the local-exclusion guarantee, but both are still `proposed`: an unapproved proposal does not supersede another. When ADR-0014 passes cross-review and human approval, this field becomes `partially superseded by ADR-0014`.

## Context

`Rationale_Arquitectura_Conceptual_v0.1.md §20` requires instrumentation "from the first vertical, not at the end", and `§11.14` is explicit: "Do not send data automatically". `Rationale_v0.5.md §30.1.12` adds, for the future pilot in a work repository: local telemetry by default, with no code, prompts, diffs, or Records sent to additional services without authorization.

This has been implemented de facto since Phase D (`src/evaluation.rs`), without a formal ADR fixing it as a decision rather than an implementation accident. Before the pilot (Phase H), and before Phase F adds capture of diffs and signals (with more potentially sensitive data surface), it is worth formalizing.

## Decision

1. **All telemetry is local-only by default, with no configurable exception in Phase E/F.** No data leaves the user's filesystem through any network call initiated by Rationale.
2. **Format:** NDJSON in `.rationale-local/runs/*.ndjson`, one file per event type, append-only.
3. **Allowed fields:** operational metadata (latency, Git revision, consistency state, provider status and coverage, packet size in bytes, timestamp). **Fields forbidden by default:** code content, agent prompts, full diffs, Record/Evidence content, any secret.

## Evidence

- A real implementation already exists (`src/evaluation.rs`, Phase D): `RunLog` contains only `event`, `timestamp`, `latency_ms`, `git_revision` (a SHA, not content), `consistency`, `provider_status`, `provider_coverage`, and `packet_bytes` (a count, not the packet itself). No free-text, code, or prompt field.
- `.rationale-local/` has been in `.gitignore` since Phase A — verified that the instrumentation logs are never versioned or published with the repository.
- Verified in the Phase D session: the real generated log (`.rationale-local/runs/vertical-slice.ndjson`) contains no code string, no user file path beyond what is strictly necessary, and no data beyond the declared fields.

## Alternatives considered

- **Sending aggregated telemetry to our own service to improve the product**: explicitly discarded — it contradicts `Arquitectura §11.14`, and no user consent that would authorize it is implemented. It could be reconsidered in the future only as an explicit opt-in, never by default, and outside the scope of this ADR.
- **Not instrumenting until a concrete use case exists**: discarded — `Arquitectura §20` is explicit that instrumentation starts from the first vertical and is not postponed, precisely so the metrics in `v0.5 §30` (packet tokens, latency, recall rate) have data from the beginning of the project.
- **Logging the full diff or the Record content in every log**: discarded — it would violate the minimization principle (`v0.5 §4.11`, `§26.5`) and needlessly increase the risk if a log were ever shared by mistake.

## Consequences

- Phase F (capture of diffs and signals) must respect the same rule: any new instrumentation that phase introduces remains local-only, and any field proposed for addition must pass the same test ("is this operational metadata or potentially sensitive content?").
- The future pilot (Phase H, a work monorepo) inherits this policy without needing a new decision — it is settled here.
- The aggregated reports of `v0.5 §30.1.12` ("reports may use IDs and aggregated metrics") remain the responsibility of a manual, explicit analysis process, not of automatic sending.

## Risks

- A developer who manually copies `.rationale-local/` elsewhere (for example, for shared debugging) could expose system file paths — mitigation: the fields already exclude content, and `git_revision` is a SHA that is public anyway in any shared repository.
- If Phase F adds a new field to the log without reviewing this policy, sensitive content could leak without anyone noticing — mitigation: this ADR is cited explicitly as a review gate for any change to `RunLog` or equivalent structures.

## Validation

Verified by direct inspection of the real log generated in Phase D (see Evidence). Phase E6 adds an explicit test that fails if `RunLog` (or its extended equivalent) includes an unbounded free-text field (protection against future regression).

**This ADR is `proposed`**, pending cross-review and human approval.

## Revisit trigger

Reopen if Phase F needs to record something forbidden today (for example, a diff fragment for debugging) — it would require an explicit opt-in decision, not a silent extension of this ADR.

## Validation update — 2026-07-28

The migration from `alpha.7` to `main` on copies of the Monorepo and BoostAPI
pilot repositories invalidated part of the original validation. **The earlier
text is not rewritten**: it stays as written, and this section records what
failed and why. The error is evidence too.

**What is still valid.** Decision #1 was not refuted: Rationale initiates no
network call and no data leaves the filesystem through the product's action.
Decision #2 (append-only NDJSON format) was not refuted either.

**Failure 1 — the Evidence generalized from the development repository to
consumer repositories.** The line "`.rationale-local/` has been in `.gitignore`
since Phase A — verified that the instrumentation logs are never versioned or
published with the repository" is true **only inside Rationale's repository**,
where that `.gitignore` was written by hand. `init` and `install-agent` never
write that entry into the user's project. In Monorepo and BoostAPI, the three
files of `.rationale-local/` are versioned, and `git branch -r --contains` places
them on `origin/main` in both: the exposure to those remotes was real, not only
potential. Two of two pilots — it is the normal flow, not accidental
contamination.

**Failure 2 — the field inventory was incomplete.** The Evidence audited only
`RunLog` (`src/evaluation.rs`). The `review_decision` event
(`src/review.rs:636`) emits `record_id`, `decision`, and `time_to_confirm_ms`
— how long a human took to resolve each Record — and **none of the three
appears in Decision #3's list of allowed fields**. It is behavioral data about a
person, not machine operational metadata. It never passed the test this very ADR
requires. `installed-agent-files.json` was not considered either, and it stores
absolute paths under the user's `$HOME`.

**Failure 3 — the anticipated exposure vector was not the real one.** Risks
anticipated "a developer who manually copies `.rationale-local/`". The real
vector required no human action: it was `git add` on a directory nobody had
excluded. A risk written around a manual slip did not cover the automatic case.

**What is invalidated**, and moves to ADR-0014: the guarantee that
`.rationale-local/` stays excluded in consumer projects, the conclusion that the
data never reaches a remote, and the completeness of the local data inventory
considered in the privacy analysis.

**Remediation in the pilots.** Fixing Rationale does not remove files already
tracked by Git. Each affected repository needs `git rm -r --cached
.rationale-local`, run manually and explicitly. History is not rewritten: what
leaked is operational metadata and personal paths — no credentials, secrets,
code, or Record content — and the cost of rewriting shared history is
disproportionate to that content.

**Confirmed scope of the exposure (2026-07-28, by the project owner):** Boost
and BoostAPI are private repositories. The exposure was real but limited to those
private remotes and the people with access to them; there was no public
disclosure. This does **not** mean "no exposure": the data reached collaborators
and persists in the remote history. It reduces the severity, not the existence,
of the incident, and it is what makes the decision not to rewrite history
proportionate. Reopen this assessment if the visibility of either repository
changes or if content more sensitive than what is already inventoried is
identified.
