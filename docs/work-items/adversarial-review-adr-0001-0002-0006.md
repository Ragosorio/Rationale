# Adversarial review: ADR-0001, ADR-0002, ADR-0006

**Role:** independent Review Agent (`Proceso §4.4` — "another session" as a valid reviewer).
**Assignment:** try to refute the three `proposed` ADRs, without self-approving. Methodology: a full reading of the ADRs, the cited research notes, the code of both spikes, the Git history, and the real implementation (`src/`).
**This session approved or rejected nothing** — the verdict is left to human review.

---

## Executive summary

| ADR | Verdict | Most severe finding |
|---|---|---|
| ADR-0001 (Rust) | Holds, with nuances | Nothing blocking; the conclusion withstands aggressive re-weighting (verified by recomputing 5 different scenarios — Rust wins in all of them, even inverting the disputed criterion). A real weakness: the highest-weight criterion ("memory safety and reliability", 20%) is justified with evidence about subprocess management, not memory in the strict sense; and the central finding (Go's footgun) was not recorded in an auditable commit, only in prose. |
| ADR-0002 (persistent MCP session) | Holds, with significant nuances | **The real Phase D implementation (`src/main.rs`) does not achieve the amortization the ADR claims** — every CLI invocation (`rationale prepare`) pays the full ~6.8 s handshake, because the binary today is a single-use process, not a daemon. The measured advantage (15–30 ms per call) is realized only *within* an invocation, never *between* invocations. It depends on the architectural question still open in `Arquitectura §28`: "one process per session or a shared daemon?". |
| ADR-0006 (revision from Git) | Holds | The most robust of the three. The central evidence is doubly corroborated (`05` and `08`, independently). Implementation nuances: test gaps for symbolic links/submodules/a repository without commits; and the ADR describes "content hashes where applicable" but the real code only computes a `working_tree_dirty` boolean. |

## Full findings (by ADR)

### ADR-0001

1. Go's footgun (5016 ms) does not exist as an auditable commit — there is only a final, already fixed commit (`6b5d47e`). The "before" is only narrated in `candidates.md`/`benchmark-results.json`. *(nuance)*
2. `spike-notes.md` admits the result was a "consequence of the manual implementation style", not a structural guarantee of the language — the ADR generalizes more than its own evidence supports. *(nuance)*
3. The evidence cited for "memory safety and reliability" (20%, the highest weight) is really about the ergonomics of subprocess management (`os/exec` versus a manual poll), not about memory in the traditional sense. *(nuance — methodological rigor)*
4. Double counting: the same fact (Go avoids cgo with `modernc.org/sqlite`) is rewarded under "SQLite and filesystem" and penalized under "Interoperability with C processes" — symmetrically for Rust with `flock` FFI. The effects roughly cancel out; the result does not change. *(cosmetic)*
5. **Sensitivity check of the score (a direct answer to the assignment's central question):** recomputed under 5 different re-weighting scenarios (including removing the disputed criterion entirely, or inverting its score). Rust wins in all of them. *(holds — the conclusion is robust)*
6. "Distribution as a binary" (15%) is scored only by file size — code signing, installers, and real packaging were never tested, although this is already acknowledged transparently in `spike-notes.md`/`compatibility-matrix.md`. *(nuance)*

### ADR-0002

1. **[The most important finding of this report]** `cmd_health`/`cmd_prepare` in `src/main.rs` call `CodebaseMemoryClient::spawn()` at the start of each function, and the binary returns when finished. Every CLI run is a new OS process that pays the full handshake. That is exactly the profile of the "new MCP session per operation" alternative that ADR-0002 itself explicitly discards. *(blocking for the current realization, a nuance for the transport decision itself)*
2. No response `id` correlation — the design depends on calls being strictly sequential (documented in a comment, but not verified with a real `id`). If Rationale ever needs concurrency, it would attribute responses incorrectly. *(nuance)*
3. The promised risk mitigation ("explicit signal handling") is not implemented — there is no `ctrlc`/`signal-hook` in the code, only a `Drop` that does not run on an unhandled `SIGINT`/`SIGTERM`. *(nuance)*
4. SQLite contention between multiple concurrent Rationale processes (two terminals, two agents) is not discussed in Risks. *(nuance)*
5. The central measurement (15–30 ms warm versus 2.2–6.8 s cold, 3 runs, <5% variance) is solid. *(holds)*

### ADR-0006

1. The central evidence is doubly corroborated: `05-revision-and-coverage.md` (detect_changes fails) and `08-workspaces-and-monorepos.md` (a present capability that fails silently) are independent findings that reinforce each other. *(holds)*
2. The root cause of `detect_changes` is still "Unknown" in the evidence itself — the ADR generalizes from a causally unisolated anomaly to a permanent architectural principle. Reasonable as a conservative default, but it should be more honest about that. *(nuance)*
3. **A direct question of the assignment — is there a legitimate case of a "virtual" revision that the ADR discards without justification?** No counterexample was found: `v0.5 §4.19` already limits the scope to Git upstream, and the ADR allows the provider's signal as diagnostic metadata, just not as authority. *(holds)*
4. Untested edge cases: symbolic links, submodules (`git status --short` may not walk them depending on configuration), a freshly `git init`ed repository with no commits yet. *(nuance — a test gap, not a confirmed bug)*
5. `check_consistency` can label `WorkingTreeAhead` when there are really two overlapping problems (dirty + a different revision) — functionally harmless, misleading as a diagnostic. *(cosmetic)*
6. The ADR describes "content hashes where applicable" but the real code only computes a `dirty` boolean — any uncommitted file (even an irrelevant one) invalidates every Record equally. Consistent with "fail with humility", but the ADR promises more precision than the code has. *(nuance)*

## Pending decision

The project's human owner decides whether these ADRs move to `accepted`, are corrected first, or stay `proposed` with the corrections applied. ADR-0002's finding (#1) is incorporated directly into the ADR itself as an explicit acknowledgment, since Phase E5 (the MCP surface, next in the plan) is precisely what closes that gap: an MCP server is, by construction, the long-lived process that a single-command CLI is not.
