# Adversarial review: Phase F (capture, signals, Subject Resolver, `finalize_change`, `rationale review`)

**Role:** independent Review Agent (`Proceso §4.4` — "another session" as a valid reviewer), with no prior context from the session that implemented Phase F.
**Assignment:** try to refute `src/storage.rs`, `src/capture.rs`, `src/signals.rs`, `src/subjects.rs`, `src/pipeline.rs::finalize`, `src/review.rs`, and the full proposal→review→approval cycle, without self-approving anything.
**This session approved or rejected nothing** — the verdict is left to human review, following the same pattern as `docs/work-items/adversarial-review-adr-0001-0002-0006.md` and `docs/work-items/adversarial-review-fase-e5-e6.md`.

Methodology: a full reading of `src/storage.rs`, `src/capture.rs`, `src/signals.rs`, `src/subjects.rs`, `src/pipeline.rs`, `src/review.rs`, `src/main.rs::cmd_review`, and `src/project.rs`; a review of commit `c9fd5b6` (the path traversal fix applied while closing Phase F); `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test --release` (95 tests); and, above all, an empirical attack against the **real compiled binary** (`target/release/rationale serve` / `rationale review`) using a Python client speaking the `Content-Length` framing directly (the same approach as the E5/E6 review) plus direct CLI invocations with `subprocess`, on throwaway synthetic Git projects in `/tmp/`. No production file was modified; `git status` was still clean at the end.

Starting commit: `c9fd5b6` (`fix(security): path traversal real vía record_id en finalize_change/review`), the most recent in `git log` when this review started.

---

## Executive summary

| Area | Finding | Severity |
|---|---|---|
| `subjects::list_subjects` / `storage::list_records` + `pipeline::finalize` | A single corrupt YAML file in `.rationale/subjects/` or `.rationale/records/` disables the WHOLE Subject Resolver for every future proposal, silently, with no diagnostic | **High** |
| Proposal→review cycle (`review.rs` + `pipeline::finalize`) | A real TOCTOU: a proposal can be lost silently (never moved to `.rejected/`) if it is overwritten during the human review window; a second concurrent approval of the same proposal overwrites the first with no collision detection | **High** |
| `review::describe_effect` (`println!` of content controlled by the MCP client) | ANSI/control escape sequences in `finalize_change`'s `intent`/`statement`/`risks` survive intact all the way to the human's terminal in `rationale review` | **High** |
| `signals::signals_from_paths` (substring matching) | Confirmed false positive: a cosmetic file unrelated to real authorization (`auth_helper_unrelated.rs`) triggers the `Authorization` signal and generates a noise proposal | **Medium** |
| `signals::determine_level` | Confirmed false negative: a real double-charge payments bug, with no path keyword or normative language, is classified at the lowest level (`Intent`), like a trivial change | **Medium** |
| `subjects::resolve` — `ALIAS_SIMILARITY_THRESHOLD = 0.85` | Confirmed false positive: two genuinely different concepts sharing a long phrase template reach 0.86 lexical similarity and BLOCK the whole proposal | **Medium** |
| `subjects::resolve` — `CANDIDATE_MIN_THRESHOLD = 0.2` | Confirmed false negative: the same real concept (no double charge) expressed with different vocabulary generates no candidate — silent Subject fragmentation | **Medium** |
| Path traversal in `record_id` (fix `c9fd5b6`) | Re-verified independently: the fix holds; no bypass or equivalent uncovered place was found | Holds |
| Never self-approves (`review::approve` is the only function that builds `Approval{status:"approved"}`) | Re-verified independently, without trusting the earlier finding | Holds |
| EOF / non-interactive stdin in `rationale review` | Never interpreted as implicit approval — it falls into the "Skipped" path | Holds |
| An empty corrected `statement` in the `'c'` flow | Rejected by `storage::validate()` before touching disk; the proposal stays pending and is not lost | Holds |
| Concurrent writes across OS processes on the same `record_id` | The atomic rename holds even with 12 real concurrent processes — it never corrupts the file | Holds |
| `cargo fmt` / `cargo clippy -D warnings` / `cargo test --release` | 95/95 tests, zero warnings | Holds |

**7 actionable findings** (0 new critical, 3 high, 4 medium) + **6 confirmations that hold** under a real empirical attack.

---

## Full findings

### 1. A single corrupt file in `.rationale/subjects/` or `.rationale/records/` turns the Subject Resolver off completely, silently (High)

`src/subjects.rs:82-90` (`list_subjects`) uses `?` on `read_subject(&path)` inside the `for` that walks the directory: if A SINGLE `.yaml` file does not parse or lacks `id`/`title`, the whole function returns `Err`, also discarding the Subjects that had been read correctly up to that point. The same applies to `storage::list_records` for Records.

`src/pipeline.rs:542-543`:
```rust
let existing_subjects = subjects::list_subjects(&subjects_dir).unwrap_or_default();
let existing_records = storage::list_records(&records_dir).unwrap_or_default();
```
Both lines silence that `Err` with `.unwrap_or_default()` — with no `diagnostics.push(...)`. This contrasts directly with `pipeline::prepare` (same module, lines 87–105), which does report explicitly when reading Subjects fails (`"advertencia: no se pudieron leer Subjects: {e}"`). `finalize` does not take the same care.

**Reproducible evidence — the Subjects case.** A project with a Subject almost identical in title to the proposed one (it should block as `Alias`):
```
=== BEFORE corrupting subjects/ : exact-title duplicate should be detected ===
action: alias blocked_reason: candidato de Subject fuerte sin novelty_reason — ver subject_resolution.candidates

=== AFTER adding one malformed subjects/broken.yaml : same exact-title duplicate ===
action: create blocked_reason: None
candidates: []
proposal_written: True
ALL diagnostics: ['target declarado: .../src/auth/authz.rs',
 'propuesta escrita en .../proposals/constraint.dup-after.yaml (nivel=OperationalKnowledge, subject=authz.new-duplicate-attempt)']
```
The corrupt file added was trivial: `.rationale/subjects/broken.yaml` with the content `"id: \ntitle: \n"`. No diagnostic mentions that anything failed while reading Subjects.

**Reproducible evidence — the Records case (binding overlap).** The same pattern, but through `.rationale/records/`:
```
=== BEFORE corrupting records/: binding-overlap should surface the existing subject as a candidate ===
action: alias candidates: [{'id': 'authz.entity-scoped-staff-access', 'signals': {'binding_overlap': 1.0, ...}}]

=== AFTER adding one malformed records/broken.yaml: same binding-overlap scenario ===
action: create candidates: []
diagnostics: ['target declarado: .../src/auth/authz.rs', 'propuesta escrita en .../proposals/constraint.overlap-after.yaml (nivel=OperationalKnowledge, subject=authz.new-attempt)']
```
The corrupt file: `.rationale/records/broken.yaml` with `statement: ""` (any invalid `Record`, exactly the kind of file that already exists today in this repository's `.rationale/proposals/.rejected/` after my own tests — see finding 3 — or that a failed manual edit could leave behind).

**Why it is high, not medium:** `Rationale_Arquitectura_Conceptual_v0.1.md §27` explicitly forbids "Hiding partial coverage". This is exactly that case: the whole Subject Resolver (Phase F4, the central piece of this phase together with `finalize_change`) goes blind to ALL of the existing canon of Subjects and Records — not only the corrupt file — without `FinalizeOutcome` carrying any signal of it. It does not require an attacker: a human editing a Subject by hand with malformed YAML, or a future interrupted write to Subjects, produces the same effect completely by accident and silently. The observable result is indistinguishable from "no similar Subjects/Records exist" when the reality is "I could not read them".

**Suggested fix (not applied):** have `list_subjects`/`list_records` accumulate errors per file instead of aborting on the first (there is already a precedent: `review::list_pending` silently skips entries that do not parse, but at least does not discard the others); and have `pipeline::finalize` report in `diagnostics` any error reading Subjects/Records, as `pipeline::prepare` already does.

---

### 2. A real TOCTOU in the proposal→review cycle: silent loss of proposals and approvals (High)

`review::list_pending` (`src/review.rs:40-65`) reads all of `.rationale/proposals/` **once** at the start of `cmd_review` (`src/main.rs:204`) and keeps each proposed `Record` **in memory** while waiting for human input (which can take minutes). `review::approve` (`src/review.rs:111-148`) never reads the file from disk again before promoting it: it writes the in-memory `Record` directly to `records/<id>.yaml` and then deletes `proposals/<id>.yaml` — without checking that the file still contains what was shown to the human, or even that it still exists.

**2a. A new proposal on the same `record_id`, written while the first waits for review, is lost without a trace.**

A sequence reproduced against the real binary:
1. `finalize_change` writes `proposals/constraint.race-test.yaml` with `statement: "FIRST VERSION..."`.
2. A real `rationale review` is started; its `list_pending()` has already loaded the "FIRST VERSION" proposal into memory and is blocked waiting for the human's answer (confirmed by reading its stdout up to the prompt).
3. **Meanwhile**, a second `finalize_change` call with the SAME `record_id="constraint.race-test"` but `statement: "SECOND VERSION..."` overwrites `proposals/constraint.race-test.yaml` on disk (a successful atomic write, `proposal_written: true`).
4. "approve" is confirmed to the review process, which promotes what it had in memory.

```
=== proposals/constraint.race-test.yaml on disk RIGHT BEFORE approving ===
statement: 'SECOND VERSION: staff must never receive global super_admin (overwritten during review window).'
...

=== rest of review stdout ===
Aprobado -> /tmp/rationale-atk-race/.rationale/records/constraint.race-test.yaml

records/ contents: ['constraint.race-test.yaml']
proposals/ contents: []
=== promoted record statement line ===
statement: 'FIRST VERSION: staff must never receive global super_admin.'
```
`records/` ends with "FIRST VERSION" (consistent with what the human saw and approved — there is no deception about what they approved), but `proposals/` is left **empty**: the "SECOND VERSION" proposal — which did exist on disk, with `proposal_written: true` — disappears completely. It is not moved to `.rejected/` (which `review::reject` does use explicitly, "never deleted silently"), it remains in no log, and there is no diagnostic. It is indistinguishable from that proposal never having existed.

**2b. Two concurrent human reviewers on the same proposal: the second `approve` overwrites the first without detecting the collision.**

Two real `rationale review` processes were launched almost simultaneously against the same project with ONE pending proposal (both run `list_pending()` before either approves). The first approves and promotes successfully. The second, still holding the SAME in-memory copy (from before the first promoted it), also approves:
```
=== Reviewer A result ===
Aprobado -> .../records/constraint.double-reviewer-test.yaml

=== Reviewer B result (same proposal, approved after A already promoted it) ===
Aprobado -> .../records/constraint.double-reviewer-test.yaml
```
Both report success (`Aprobado -> ...`, without any error), and reviewer B never checks that `proposals/constraint.double-reviewer-test.yaml` no longer existed when it tried to promote it (`std::fs::remove_file` in `approve()` uses `let _ = ...`, ignoring the failure). In this experiment both processes share the same local Git identity (`user.name`/`user.email`), so the final `approvals` shows only one entry — but the second `write_record` **replaces the whole file**, it does not merge `approvals`: if the second reviewer had a different Git identity (two real people, or two agents with different configurations — the natural scenario of Phase G, dogfooding with more than one collaborator), the second write would have discarded the real `Approval` the first reviewer had already persisted, without warning either of them.

**Why it is high:** the whole Phase F6 mechanism exists so that "it never self-approves, always with a visible and deliberate effect" (`review.rs:1-21`). Both variants of this race violate that guarantee in its most silent form: it is not that something gets approved unintentionally (the human did consciously approve what they saw), it is that the **result persisted on disk does not correspond to the single expected source of truth** — a real proposal disappears without evidence (2a), or a real, already persisted approval can be trampled by another without collision detection (2b). Neither case is covered by the existing tests of `review.rs` or `tests/mcp_server.rs`, which only test sequential, non-overlapping invocations.

**Suggested fix:** before writing in `approve()`, re-read `proposal.path` and compare it with the in-memory copy (or simply compare mtime/hash); if it differs or the file no longer exists, abort with an explicit error instead of proceeding silently. Also consider a file lock (`flock`) on `proposals/<id>.yaml` during the review window.

---

### 3. Injection of control/ANSI sequences into the human reviewer's terminal through `intent`/`statement`/`risks` (High)

`finalize_change` accepts `intent`, `statement`, and `risks` as free text coming from the MCP client (an agent, potentially compromised or simply buggy) — it is never sanitized. These fields are persisted as-is in the proposed `Record` and, later, `review::describe_effect` (`src/review.rs:70-92`) prints them with `println!` straight to the human's terminal during `rationale review` — exactly the moment Phase F6 designed for the human to see "the practical effect" before approving (v0.5 §15.5).

**A vector ruled out first (file names through Git):** it was tested whether a real file name with ANSI escape sequences (legal in POSIX, which only forbids `/` and NUL) survived through `capture::diff_since` → `path_hint`. Confirmed that it does **not**: Git automatically quotes any control byte in a path with `--name-status`, even with `core.quotepath=false` (verified with `xxd` on the raw diff — the escape bytes appear as the literal text `\033`, never as the real `0x1b` byte). This concrete vector does not apply — Git already neutralizes it before Rationale sees it.

**A confirmed vector (the agent's own free text):** `intent`/`statement` go through no Git filter. `finalize_change` was called with:
```python
malicious_statement = ("Staff must never receive global super_admin.\x1b[2K\r"
                        "\x1b[32mAUTO-APPROVED BY SECURITY TEAM - no review needed\x1b[0m")
malicious_intent = "Normal intent text \x1b[8mhidden-instruction-here\x1b[28m end"
```
`yaml_serde` does escape these bytes correctly when persisting the YAML (`\e[2K` as a string escape, a valid YAML 1.1 form) — persistence on disk is safe. But when re-reading and displaying with `rationale review`:
```
=== raw stdout bytes from `rationale review` ===
b"...Afirmaci\xc3\xb3n propuesta: Staff must never receive global super_admin.\x1b[2K\r\x1b[32mAUTO-APPROVED BY SECURITY TEAM - no review needed\x1b[0m\nRaz\xc3\xb3n: Normal intent text \x1b[8mhidden-instruction-here\x1b[28m end\n..."
Contains raw ESC byte (0x1b) in review's stdout: True
```
The real escape bytes (`0x1b`) reach the stdout the human reads intact — `\x1b[2K\r` erases the current line and moves the cursor, `\x1b[32m...\x1b[0m` can paint a fake "AUTO-APPROVED BY SECURITY TEAM" message in green, and `\x1b[8m...\x1b[28m` (SGR "conceal") can hide text in terminals that support it.

**The real scope, without exaggerating:** this does NOT allow skipping the confirmation word itself — the human still has to type `approve`/`approve-critical` literally, and that check (`src/main.rs:242`) compares the typed string, unaffected by what is displayed. The real risk is **visual deception**: the human may approve believing they read something different from what the system will actually persist (the real statement hidden or visually replaced by a fake banner), which directly attacks the only safeguard Phase F6 offers ("one statement per screen... never the full YAML" — but the "screen" itself can be faked).

**Why it is high:** the human trust point is the only real control against self-approval in the whole system (`review.rs:1-21`); a vector that allows manipulating exactly what that human sees, without anything in the pipeline detecting or sanitizing it, attacks that control at its root.

**Suggested fix:** strip control characters (`\x00-\x1f` except `\n`/`\t` if intentional multi-line formatting is to be preserved) from `statement`/`rationale`/`risks` before writing them in `describe_effect`, or at least before persisting them — the simplest option is to sanitize at the input boundary (`pipeline::finalize`), where `record_id` is already sanitized.

---

### 4. `signals::signals_from_paths` — a real false positive from substring matching (Medium)

`PATH_KEYWORDS` (`src/signals.rs:37-56`) uses `path_lower.contains(kw)` — a pure substring, not a whole word. A purely cosmetic file whose name simply contains the substring `"auth"` triggers the `Authorization` signal, with no relationship at all to real authorization logic.

Reproducible evidence against the real binary:
```
=== FP test: auth_helper_unrelated.rs (decorative banner, no real auth logic) ===
signals: ['authorization']
level: decision
proposal_written: True
```
The real file used: `src/auth_helper_unrelated.rs` with the content `// renders a decorative header banner for the CLI splash screen` — zero relationship to authorization. The real commit generated a full `Decision`-level proposal.

**Why it matters (medium, not high):** the module declares itself honest ("deliberately short and extensible... substring matching... cheap"), and the mechanism is additive (it never blocks, it only generates noise) — but the noise erodes exactly Phase F's central promise ("Rationale does not ask about every change; it activates assisted capture only when it detects concrete signals", `signals.rs:4-10`). Words like `"client"` (under `ExternalIntegration`) are even broader — they would match almost any file named `*_client.rs`, including this repository's own `src/providers/*.rs`.

**Suggested fix:** use whole-word matching on path segments (split by `/`, `_`, `-`, `.` and compare exact tokens) instead of a raw substring — the same pattern `contains_word` already uses for `NORMATIVE_WORDS` in the same file (lines 68–75), which does avoid the analogous case (`avoid` inside `avoidance-list`, covered by the test `does_not_false_positive_on_substring_of_normative_word`). The inconsistency between the two functions of the same module (one matches by word, the other by raw substring) is not justified in the comments.

---

### 5. `signals::determine_level` — a real false negative: a critically dangerous change is classified at the lowest level (Medium)

A real change in payment settlement logic that doubles the amount charged under certain conditions, with no `PATH_KEYWORDS` keyword in the path and no `NORMATIVE_WORDS` in `intent`/`statement`, is classified as `Intent` — the same level as a trivial refactor with no signal.

Reproducible evidence:
```
=== FN test: real critical payment-doubling bug, no keyword path, no normative language ===
signals: []
level: intent
proposal_written: True
```
Path used deliberately without any keyword: `src/core/ledger_math.rs`. `intent`: *"Updated the settlement calculation used when closing out international customer orders."* `statement`: *"International order settlement now doubles the charged amount when currency mismatch is detected."* `risks`: *"Customers could be charged twice the correct amount for international orders."* — none of these texts contains `must`/`never`/`because`/`avoid`/`do not`. The `severity: "critical"` field the caller did pass **has no influence at all** on `determine_level` — it is a completely separate field that only affects the confirmation word in `rationale review`, not the capture level.

**Why it matters (medium, not high):** the mechanism still writes a proposal (`proposal_written: true`; the event is never lost), so there is no data loss — but the explicit purpose of the levels (v0.5 §16) is to prioritize where a human with limited time should look first, and this is precisely the case — a real, serious financial bug — where getting the priority wrong costs the most.

**Suggested fix:** nothing trivial without expanding the keyword taxonomy (which the module already admits is deliberately incomplete) — the cheapest option is for `determine_level` to consider the caller's declared `severity` as an additional signal (not authoritative, but visible) when there is no domain match or normative language, instead of ignoring it entirely.

---

### 6. `subjects::resolve` — the `ALIAS_SIMILARITY_THRESHOLD = 0.85` threshold blocks two genuinely different concepts for sharing a phrase template (Medium)

Jaccard over normalized tokens (`lexical_similarity`, `src/subjects.rs:169-182`) does not distinguish "the same sentence with a different domain word" from "the same idea". Two governance titles with a long shared template, describing real and different constraints (schema migration governance versus audit logging governance), reach 0.86 — above the `Alias` threshold (0.85) — and the whole proposal is BLOCKED (not just marked as a candidate).

Reproducible evidence against the real binary:
```
=== Jaccard FP test: distinct concept (migration governance vs audit-logging governance) ===
subject_resolution action: alias
candidates: [{"id": "db.migration-governance", "signals": {"binding_overlap": 0.0, "lexical_similarity": 0.8636363636363636, "scope_compatible": true}}]
blocked_reason: candidato de Subject fuerte sin novelty_reason — ver subject_resolution.candidates
proposal_written: False
```
Existing title: *"Ensure that the system never allows a background job to write directly to the production database without going through the approved **migration** pipeline"*. Proposed title: the same sentence, replacing only *"migration"* with *"audit logging"*. `binding_overlap: 0.0` (no file in common) — the only signal that triggers the block is purely lexical.

**Why it matters (medium, not high):** unlike `retrieval::detect_conflict` (which only adds a warning and never blocks — v0.5 §19.1), here `finalize_change` does block writing the proposal completely (`proposal_written: false`) unless the caller provides an explicit `novelty_reason`. This means any organization using consistent drafting templates for its constraints (reasonable, even recommendable) will generate recurring false blocks, training agents to fill in `novelty_reason` almost reflexively — exactly the "accept everything" that v0.5 §294 wants to avoid, only in the opposite direction (accepting the override of the block, not the approval).

**Suggested fix:** weight `lexical_similarity` with something more than raw token Jaccard — for example, excluding structural stopwords from the computation (`ensure`, `that`, `the`, `system`, `never`, `allows`, `to`, `without`, `going`, `through`, `approved`, `pipeline` are pure syntactic scaffolding, not a concept signal) before applying the threshold. It is not a trivial change without additional evidence about how common repeated templates are in real Records — but the current threshold (0.85, with no documented justification beyond the number) does not withstand this counterexample.

---

### 7. `subjects::resolve` — the same real concept with different vocabulary never surfaces as a candidate (Medium)

The exact counterpart of finding 6: an existing Subject (`payments.no-double-charge`, titled *"Payments must never be processed twice for the same order"*) and a new proposal describing the SAME concept (avoiding a double charge, this time in checkout retries) with completely different vocabulary (*"Idempotent settlement retries must not re-bill the customer's card on transient network failures"*) do not share enough tokens even to pass `CANDIDATE_MIN_THRESHOLD` (0.2).

Reproducible evidence:
```
=== Jaccard FN test: same real concept (no double billing), different vocabulary ===
subject_resolution action: create
candidates: []
proposal_written: True
```
`candidates: []` — it does not even appear as a weak candidate for a human to review in `rationale review`; the new Subject is created without any signal that a Subject already governs the same real concern.

**Why it matters (medium, not high):** this is silent fragmentation of the canon — exactly what the Subject Resolver (Phase F4) exists to prevent (v0.5 §9.1, steps 2–5). It blocks nothing and corrupts no data, but it erodes the phase's central guarantee over time: each real concept would end up with N different Subjects depending on which agent drafted it first, without anyone noticing until a manual audit.

**Suggested fix:** the module itself already recognizes this as a known, deferred limit (`resolve()` doc: "5. Local semantic similarity. -> deferred, §28.3 (embeddings)") — consistent with v0.5's architectural decision not to use embeddings yet. It is less an implementation bug than an already documented structural limitation; it is included here because the assignment asked for the explicit counterexample, and it is confirmed with real, not only theoretical, data.

---

## What holds under attack

1. **The path traversal fix (`c9fd5b6`) holds, and no equivalent uncovered place was found.** `storage::validate_safe_id` was re-verified independently (not only by reading code): `../../../../etc/pwned`, `..`, `.`, `sub/dir`, `back\slash`, `nul\0byte` — all rejected by the existing tests, reconfirmed with `cargo test`. It was explicitly searched, through `grep`, whether `subject_id`/`subject_title` are ever used to build a path — confirmed that they are NOT (`src/pipeline.rs:86,376,540` only do `config.rationale_dir.join("subjects")`, a fixed literal; Phase F does not write new Subjects yet, consistent with `docs/architecture/code-map.md`). It was also tested whether a real Git file name with control sequences could reach `path_hint` unescaped (see finding 3) — Git neutralizes it before Rationale sees it, even with `core.quotepath=false` (verified with `xxd`).

2. **The "never self-approves" guarantee was re-verified independently, without trusting this review's earlier finding.** `grep -rn "status: \"approved\"" src/` finds 4 places; the 3 that are not `review.rs:130` are inside `#[cfg(test)]` (fixtures in `retrieval.rs` and `assessment.rs` to test that already approved Records are displayed correctly — never production code). `grep` over `src/mcp/server.rs` confirms that the `review` module is never referenced from the MCP surface (neither `finalize_change` nor any other tool) — the only way to produce a real `Approval` is still `rationale review`, a separate interactive CLI process.

3. **EOF / non-interactive stdin is never interpreted as implicit approval.** `rationale review --project-root <dir>` was run with `stdin=/dev/null` against a real `critical` proposal:
   ```
   returncode: 0
   ...
   Escribe 'approve-critical' para aprobar tal cual...
   Saltado — la propuesta sigue pendiente.
   records/ contents: []
   proposals/ contents: ['constraint.eof-test.yaml']
   ```
   `stdin.read_line` returns `Ok(0)` on EOF (it is not an error), the resulting empty string matches no valid confirmation word, and it falls into the `else` path ("Skipped"). The critical proposal stays intact and pending — the design holds against this concrete vector.

4. **An empty corrected `statement` in the `'c'` flow is rejected before touching disk, without losing the proposal.** An empty line was sent as the new statement followed by the real confirmation word:
   ```
   error aprobando: Record inválido: falta campo obligatorio 'statement'
   records/: []
   proposals/: ['constraint.empty-correction-test.yaml']
   ```
   `storage::validate()` (shared between reading and writing) rejects the `Record` before `write_record` touches disk; `approve()` propagates the error through `?`, so `std::fs::remove_file(&proposal.path)` never runs — the original proposal stays intact in `proposals/` and is not lost.

5. **Concurrent writes ACROSS real OS processes on the same `record_id` never corrupt the file.** Unlike the existing unit test (`concurrent_writes_to_same_record_never_corrupt_the_file`, which uses threads within one process), **12 real, separate `rationale serve` processes** were launched, each calling `finalize_change` with the same `record_id` simultaneously:
   ```
   12/12 finalize_change calls reported proposal_written=True
   === final proposal file content ===
   statement: statement-from-writer-11
   ... (complete, well-formed YAML, a single clean candidate)
   leftover tmp files: []
   ```
   The final result is exactly one of the 12 candidates, complete and valid — never a mixture or a half-written file, and with no orphaned temporary files. The atomic write pattern (temporary file + `rename` in the same directory) also holds across OS processes, not only across threads.

6. **`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --release` pass cleanly**: 79 unit tests + 8 MCP integration + 8 schema validation = 95/95, zero warnings, in this environment.

---

## Severity summary

| Severity | Count | Findings |
|---|---:|---|
| Critical | 0 (new) | The only critical finding of this phase (path traversal through `record_id`) was already found and fixed before this review (`c9fd5b6`); re-verified independently, it holds. |
| High | 3 | 1 (corrupt Subjects/Records reading silently turns the Resolver off), 2 (proposal↔review TOCTOU: loss of proposals and approvals), 3 (control/ANSI injection into the reviewer's terminal) |
| Medium | 4 | 4 (`signals_from_paths` false positive from substrings), 5 (`determine_level` false negative on a critical change without a keyword/normative language), 6 (Jaccard `Alias` false positive blocks a legitimate proposal), 7 (Jaccard false negative allows silent Subject fragmentation) |
| Minor | 0 | — |
| Holds | 6 | path traversal fix re-verified; no-self-approval guarantee re-verified independently; EOF/non-interactive stdin never approves; empty correction rejected without loss; cross-process concurrency never corrupts; full suite clean |

**Recommendation on blocking (in this review's judgment; the final decision belongs to the human owner):** findings 1, 2, and 3 (the three "High") share a trait that makes them more urgent than the "Medium" ones: all three are **silent** — none produces a visible error, a caught panic, or even an entry in `diagnostics`; in all three cases the system reports success (`proposal_written: true` or `Aprobado -> ...`) while doing something different from what its own documentation promises (full coverage, no silent loss, "one statement per screen" faithful to what is persisted). Findings 4–7 are real and deserve fixing, but they are noise or known precision gaps already partly acknowledged in the code's own comments (`signals.rs`/`subjects.rs` declare themselves "deliberately crude"), not silent violations of a guarantee already promised as met.

The decision on what to fix, and whether Phase F is considered closed as is or requires an additional security iteration (in the style of `c9fd5b6` itself, which fixed a finding of this same nature while closing this phase), rests entirely with the project's human owner (`evaluation.no-self-certification`).

---

## Appendix F8 — revalidation after Codex's audit

The later independent audit confirmed three P1 and four P2 findings against the
state of this report. F8 applied the following fixes:

| Finding | Fix | Current evidence |
|---|---|---|
| `authority: reviewer` outside the schema | An `AuthorityRole` enum, authority declared per actor in `.rationale/config.yaml`, default `contributor`, validation in `storage::validate` | `storage::tests::approval_authority_must_match_declared_schema_enum`, `tests/schema_validation.rs` |
| Free-form, unpersisted `novelty_reason` | A structured object (`contrasted_subject`, `difference_kind`, `difference`, `evidence`), a mandatory candidate, persistence in `Resolution` and YAML | `subjects::tests::novelty_reason_requires_a_real_candidate_and_auditable_difference`, `novelty_reason_is_structured_validated_and_persisted` |
| TOCTOU between checking and promoting | An atomic claim through `rename` into `.rationale/proposals/.in-review/`; only one consumer wins and the intermediate state is recoverable | `tests/review_concurrency.rs`, `approve_detects_proposal_already_promoted_by_another_session` |
| A corrupt proposal skipped in review | `list_pending_detailed` accumulates errors and `cmd_review` reports them on stderr | `list_pending_reports_corrupt_yaml_instead_of_hiding_it` |
| Documentation drift | Updated `Cargo.toml`, the spike protocol, the security guide, bindings, and the schemas README | Markdown link review: 0 broken |
| No cross-platform CI | Added `.github/workflows/ci.yml` for Linux and macOS | versioned workflow; remote execution pending on GitHub |

The suite in this working state is **110 tests run**: 88 unit, 11 MCP, 1 real
process concurrency, and 10 schema tests. The local revalidation runs every test
and keeps the prohibition on self-approval; accepting foundational decisions
remains human.
