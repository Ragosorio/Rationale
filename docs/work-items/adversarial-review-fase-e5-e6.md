# Adversarial review: Phase E5/E6 (Context Compiler, MCP surface)

**Role:** independent Review Agent (`Proceso §4.4` — "another session" as a valid reviewer), with no prior context from the session that implemented Phase E.
**Assignment:** try to refute the Context Compiler (`src/retrieval.rs`), the MCP surface (`src/mcp/`, `src/pipeline.rs`, `src/providers/mod.rs`), and Phase E5's claim of real amortization, without self-approving anything.
**This session approved or rejected nothing** — the verdict is left to human review, following the same pattern as `docs/work-items/adversarial-review-adr-0001-0002-0006.md`.

Methodology: a full reading of the code in `src/retrieval.rs`, `src/mcp/server.rs`, `src/mcp/framing.rs`, `src/pipeline.rs`, `src/providers/mod.rs`, and `src/providers/codebase_memory.rs`; `cargo test`/`cargo clippy --all-targets -- -D warnings`/`cargo fmt --check`; an empirical attack against the compiled binary (`target/release/rationale serve`) with a Python client speaking the `Content-Length` framing directly; a synthetic Rationale project (`.rationale/records/` with control Records) to isolate `compile_packet` without depending on the repository's real content; a worktree of the commit before the pipeline refactor (`f774db7`) to verify byte-identity independently; and direct latency measurement (CLI versus MCP server versus raw `codebase-memory-mcp`).

Commits reviewed: `f774db7` (Context Compiler), `74a16b3` (MCP surface), `b7d978a` (expanded E6 suite), `a6a78b4` (docs). `git status` was clean at the start of this review; no production file was modified.

---

## Executive summary

| Area | Findings | Highest severity |
|---|---|---|
| `src/mcp/framing.rs` (framing without an async runtime) | 2 | **Critical** — process abort and unbounded memory growth, both trivial to trigger, neither covered by tests |
| Silent session termination on invalid/nested JSON | 1 | **High** — contradicts Phase E5's central premise (an amortized persistent session) |
| `compile_packet` — token budget (`src/retrieval.rs`) | 1 | Medium |
| `compile_packet` — `additional_history_available` (`src/retrieval.rs`) | 1 | Medium |
| `detect_conflict` (`src/retrieval.rs`) | 1 | Medium |
| `token_estimate` (`src/retrieval.rs`) | 1 | Minor |
| Measured real amortization versus the 6.8 s narrative | 1 (nuance, not a bug) | — |
| The MCP server's `catch_unwind` | Holds | — |
| Byte-identity of the pipeline refactor | Holds | — |
| `cargo test`/`clippy`/`fmt` | Holds | — |
| `write_message` with `.expect()` outside `catch_unwind` | 1 | Minor/theoretical |

**9 actionable findings** (2 critical, 1 high, 3 medium, 2 minor, 1 nuance without bug severity) + **4 confirmations that hold**.

---

## Full findings

### A. `src/mcp/framing.rs` — the framing has no limits (Critical)

The module comment says: "it never blocks indefinitely on its own". That is true for *blocking*, but the framing imposes no size limit at all, neither on the header nor on the body, and both paths are reachable by any client (or client bug) before the pipeline or `catch_unwind` come into play — the framing runs in `run()`'s main loop (`src/mcp/server.rs:34`), outside any `catch_unwind`.

**A1 — an extreme `Content-Length` aborts the process (SIGABRT); it is not a catchable panic.**

`src/mcp/framing.rs:36`: `let mut body = vec![0u8; length];` — `length` comes straight from the header, with no upper bound. A value exceeding available memory triggers `handle_alloc_error`, which in Rust **aborts the process** (it is not a normal `panic!`; `catch_unwind` does not catch it under any circumstance).

Reproducible evidence:
```
$ python3 - <<'EOF'
# (full script in the report; sends after 'initialize':)
# Content-Length: 999999999999999999\r\n\r\n{}
EOF
huge_content_length: process died with code=-6; stderr=memory allocation of 999999999999999999 bytes failed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
Exit code -6 = `SIGABRT`. A single malformed message of ~40 bytes brings the MCP server process down completely — exactly what Phase E5 says it avoids ("an invalid target... never brings down the whole session"), but for a vector different from the one the test covers.

**A2 — without a header terminator, the buffer grows without bound (unbounded memory).**

`src/mcp/framing.rs:16-26`: the loop looking for `\r\n\r\n` pushes bytes into a `Vec<u8>` indefinitely if the terminator never arrives. There is no header size limit in the production code (the integration test does impose one — `assert!(header.len() < 4096, ...)` in `tests/mcp_server.rs:66` — but that assertion lives only in the *test*, not in `framing.rs`).

Reproducible evidence: send 150 MB of `X` bytes without ever completing `\r\n\r\n`:
```
Process RSS after sending 150MB without a header terminator: 154.0 MB (pid=64175, alive=True)
```
The process stays alive, consuming memory proportional to whatever the client decides to send — no bound, no timeout, no disconnection. A client that hangs mid-message (a slow connection, a bug, or an adversary) can grow the server's memory indefinitely.

**Why this is critical and not a nuance:** both vectors are completely outside `catch_unwind` (they live in `framing::read_message`, called before any tool dispatch), neither is covered by `tests/mcp_server.rs` or any other test in the repository, and both are triggered with trivial payloads (40 bytes and 150 MB respectively — neither requires sophistication). The comment in `server.rs:10-13` promises that "stdout is EXCLUSIVELY for the protocol" and that tool panics are caught — but an aborted process (A1) does not even leave that: there is no stdout to corrupt because there is no process.

**Suggested fix (not applied — the human owner's decision):** impose an explicit upper limit on `Content-Length` (for example a few MB, far above any real packet observed — the largest `token_estimate` measured in this report was a few hundred) and a header size limit before attempting `vec![0u8; length]`; return `None` (or a JSON-RPC error) instead of aborting.

---

### B. A single invalid JSON message ends the whole persistent session, silently (High)

Phase E5 exists explicitly to amortize the cost of a long-lived session (`src/mcp/server.rs:1-8`, `src/providers/mod.rs:62-69`). If an incoming message does not parse as JSON — because it is malformed, truncated, or exceeds **`serde_json`'s default recursion limit (128 levels, `de.rs:63`)** — `read_message` returns `None` (line 38: `serde_json::from_slice(&body).ok()?`), indistinguishable from EOF. The main loop `while let Some(msg) = framing::read_message(...)` (`server.rs:34`) ends and `run()` returns: the process exits cleanly (`exit code 0`), without writing any error message to the client and without any diagnostic on stderr.

Reproducible evidence — a **260-byte** payload, syntactically valid JSON (130 levels of nested arrays, far below any imaginable "attack" — it could occur with a real nested data structure, such as an AST or deeply nested config):
```python
depth = 130
body = ("[" * depth + "]" * depth).encode()  # 260 bytes
# sent after initialize + notifications/initialized
```
Result:
```
returncode: 0
stdout leftover: b''
stderr: ''
```
Also confirmed with directly malformed JSON (`{not valid json!!!`) — the same result: `returncode: 0`, no stderr.

**Why it is high, not just a nuance:** this is not a panic — `catch_unwind` never gets a chance to act, because the problem occurs before the message reaches `handle_tools_call`. The whole session (the asset Phase E5 exists to amortize) dies on the first message that fails to parse, without any warning — a real MCP client with a minor serialization bug (or one sending a legitimately nested data structure above 128 levels) loses the whole session without knowing why, and must reconnect, paying again the startup cost it was trying to avoid. **No test in the repository (`tests/mcp_server.rs`) covers this path** — the E6 suite only tests an unknown tool, a nonexistent target, and a `project_root` without `.rationale/`, the three cases that do go through `catch_unwind` successfully.

**Suggested fix:** in `read_message`, explicitly distinguish real EOF (the client closing stdin) from "a message could not be parsed" — return an error variant instead of `None` for the second case, and answer with a JSON-RPC error (`-32700 Parse error`) without ending the session.

---

### C. `compile_packet` can exceed `max_tokens` silently (Medium)

The trimming loop (`src/retrieval.rs:218-230`) can only reduce `affected_targets` and `known_risks` — never `protected_tokens` (levels 0–3: critical constraints, intent conflicts, primary reason), by explicit and correct design (v0.5 §30.1.7: never omit a critical constraint). But if `protected_tokens` alone already exceeds `budget.max_tokens`, the loop empties `affected_targets` and `known_risks` completely and then `break`s — the final packet is served with `token_estimate > max_tokens`, and **no warning is added to `warnings`** saying the budget was not respected.

Reproducible evidence (a control project with one Record whose simple `statement`+`rationale` add up to more than the budget):
```
=== Case D: max_tokens=1 ===
critical_constraints count: 1
known_risks: []
affected_targets: []
token_estimate: 39  (exceeds max_tokens=1: True)
warnings: ['no se encontró el símbolo dentro de la cobertura disponible; no implica que no exista']
```
The only warning present is an unrelated one (symbol not found); nothing in `warnings` mentions that `token_estimate` (39) exceeds `max_tokens` (1). The existing test `tiny_budget_never_drops_critical_constraints` (`retrieval.rs:425`) verifies that critical constraints are not trimmed — correct — but does not check for the absence of an over-budget warning, so this behavior went undetected.

**Why it matters:** a caller (an agent or downstream tool) that relies on `token_estimate` to decide whether the packet fits its context window has no way of knowing, just by looking at the packet, that the requested budget was not met — it would have to compare `token_estimate` against the `max_tokens` it requested itself, a check the protocol itself should make unnecessary.

**Suggested fix:** if `token_estimate_total > budget.max_tokens` at the end of `compile_packet`, add an entry to `warnings` (for example `"token budget exceeded: N > max_tokens M — protected content (levels 0-3) is never trimmed"`).

---

### D. `additional_history_available` systematically underestimates what was trimmed (Medium)

The counter (`src/retrieval.rs:198-239`) has two parts: (1) critical constraints not included because of `max_critical_constraints` — this part is correct and well tested (`budget_caps_critical_constraints`); and (2) a **fixed `+1` flag** if `known_risks.len()` ended below the minimum of the total risks and `max_risks` — regardless of how many risks were really trimmed, and **without considering at all how many `affected_targets` were trimmed by the budget**, because the trimming loop (lines 218–230) pops first from `affected_targets` and only then from `known_risks`.

Reproducible evidence (a control project, one Record with 6 distinct `binding_declarations` and 5 `risks`, varying `max_tokens`):
```
max_tokens=104: known_risks=5 affected_targets=5 additional_history_available=0
max_tokens=100: known_risks=5 affected_targets=4 additional_history_available=0
max_tokens=95:  known_risks=5 affected_targets=2 additional_history_available=0
max_tokens=90:  known_risks=5 affected_targets=0 additional_history_available=0   <-- 6 targets removed, counter at 0
max_tokens=85:  known_risks=4 affected_targets=0 additional_history_available=1   <-- 1 risk removed -> "+1"
max_tokens=40:  known_risks=0 affected_targets=0 additional_history_available=1   <-- 5 risks + 6 targets removed -> still "+1"
```
At `max_tokens=90`, the 6 `affected_targets` (real structural bindings to `src/one.rs`...`src/six.rs`) disappear from the packet completely, and the field designed exactly to signal "more is available, expand if you need it" (v0.5 §18.2, progressive disclosure) reports **0** — the caller has no signal that anything was omitted. At `max_tokens=40`, 11 elements in total were removed (5 risks + 6 targets) and the counter only reaches "1".

**Why it matters:** it breaks the "progressive disclosure" guarantee the field claims to implement — an agent relying on `additional_history_available == 0` to decide it saw all the relevant context would be wrong in the most common trimming case (affecting `affected_targets`, which is precisely where the impacted code structure lives).

**Suggested fix:** keep two separate counters of "elements trimmed by the budget" (one for `affected_targets`, one for `known_risks`), summing how many were really removed in the trimming loop, not a boolean flag.

---

### E. `detect_conflict` produces real false positives and false negatives (Medium)

The code is already honest in its comment ("deliberately crude... never claims semantic understanding"), but the packet propagates no confidence qualification to the string that does sound definitive: `"La intención puede entrar en conflicto con '{id}': {statement}"` ("The intent may conflict with '{id}': {statement}").

**Confirmed false positive** — two unrelated topics, overlapping in generic domain vocabulary ("checkout", "page"):
```
intent: "Update the login button color and add a loading spinner for the checkout page"
constraint: "The checkout page must load a fraud-detection script before allowing payment."
-> intent_conflicts: ["La intención puede entrar en conflicto con 'constraint.conflict-test': ..."]
```
Changing the color of a login button has no real relationship to a fraud constraint on checkout; the overlap is accidental ("checkout" + "page", both domain words, not a semantic conflict).

**Confirmed false negative** — the same dangerous concept (leaking secrets through logs), different vocabulary:
```
intent: "I'm going to dump auth secrets into the debug console output for troubleshooting"
constraint: "Passwords must never be written to the application log files for any reason."
-> intent_conflicts: []   (empty — no conflict detected)
```
"Auth secrets"/"debug console output" share no word longer than 3 letters with "Passwords"/"log files" — the `overlap >= 2` threshold (line 125) never fires, even though this is, on any reasonable reading, exactly the scenario the constraint tries to prevent.

**Why it matters (medium, not critical):** the level-2 mechanism is additive — it never blocks anything on its own (correct, v0.5 §19.1: deterministic retrieval, no semantic heuristics that decide). The real risk is **false confidence in both directions**: an agent could dismiss a genuinely irrelevant conflict warning as "noise" (training itself to ignore them), and symmetrically, a real attempt to violate the constraint would pass without any signal. Neither case is covered by the existing tests (`intent_conflict_detected_by_word_overlap` only tests a direct overlap of shared vocabulary, not an adversarial one).

**Suggested fix:** nothing here is trivial without introducing semantic heuristics (explicitly out of scope, §28.3) — the cheapest option is to qualify the served string (for example `"possible lexical overlap, not verified semantically"`) so the consumer knows it is a cheap recall signal, not a verdict.

---

### F. `token_estimate` (chars/4) is not a stable proxy — the direction of the error changes with the content (Minor)

Measured against `tiktoken` (`cl100k_base`, the same reference vocabulary widely used for models of this family) on representative samples from the repository itself:

| Sample | chars | real tokens | estimate (chars/4) | error |
|---|---:|---:|---:|---:|
| English prose (a real statement from the repository) | 289 | 52 | 72 | **+38.5%** (overestimates) |
| Short English prose | 82 | 12 | 20 | **+66.7%** (overestimates) |
| Spanish equivalent | 90 | 25 | 22 | **-12.0%** (underestimates) |
| Technical ID (`constraint.no-provider-internal-access`) | 38 | 6 | 9 | **+50.0%** (overestimates) |
| Path + symbol (`src/providers/....rs::Cliente::método`) | 70 | 14 | 17 | +21.4% (overestimates) |

The code comment (`retrieval.rs:69-72`) is already honest — it is a "proxy", not an exact measurement — but the direction of the error **is not consistent**: for English prose and technical IDs with punctuation, chars/4 overestimates considerably (up to +66%); for Spanish text, it underestimates (-12%). This contradicts this assignment's initial hypothesis (that Spanish would underestimate *more* than English because of accents) — the real result is subtler: English is overestimated strongly, Spanish is underestimated moderately, and neither is "close".

**Why it matters (minor on its own, but it compounds with finding C):** since finding C already shows that exceeding the budget produces no warning, a ±12–66% estimation error widens the range of possible silent overruns of the real token budget compared with what the packet reports. The repository's current Records are in English, so the "Spanish underestimates" case is theoretical today for this particular project — but the repository itself and its documentation are in Spanish, so future Records in Spanish cannot be ruled out.

---

### G. Real amortization: the documented "6.8 s" does not reproduce in this environment (nuance, not a code bug)

I measured directly in this environment (not only by reading the code):

| Scenario | Measured time |
|---|---:|
| `rationale prepare` (CLI, real provider spawned every time) | ~130–190 ms (5 runs) |
| `rationale prepare` (CLI, provider forced to `Unavailable` through a `PATH` without `codebase-memory-mcp`) | ~30–40 ms |
| `rationale serve`, a warm `prepare_change` call (session already initialized) | ~32–37 ms (10 runs) |
| A direct, **fresh** `initialize` against `codebase-memory-mcp` 0.8.1 (a new process each time, without going through Rationale) | **~15–20 ms** (3 independent runs) |

The last data point directly contradicts `docs/research/codebase-memory/11-performance-observations.md`, which documents `initialize` at **6.79–6.86 s** against the same binary (version not confirmed as different). In this environment, the pure `initialize` handshake — the cost Phase E5 claims to amortize — no longer costs 6.8 s: it costs ~15–20 ms, whatever the session.

This **does not invalidate the mechanism** of Phase E5 (the persistent session is still correct and does reduce the per-call cost from ~150 ms to ~33 ms, a real, measured ~4–5x factor) — but the dramatic magnitude (200x, "6.8 s -> 33 ms") that motivates the Phase E5 commit and finding #1 of the ADR-0002 adversarial review **did not reproduce in this session**. I explicitly checked that it is not an illusion of silent fallback: `provider_status` was `"successful"` and `provider_coverage` `"complete"`/`"unknown"` in every call — the real provider is being invoked and responding, not silently falling back to `Unavailable`.

**Cause of the discrepancy: `Unknown`.** Candidates not ruled out: (a) already-warm on-disk caches in `~/.cache/codebase-memory-mcp/*.db` (confirmed to exist, several tens of MB, accumulated from earlier sessions against this and other repositories) that would avoid the indexing cost the original research may have measured under the name "`initialize`"; (b) a version or measurement environment different from `11-performance-observations.md`'s; (c) a real behavior change in `codebase-memory-mcp` 0.8.1 between the date of that research and today. **Risk:** if the 6.8 s figure was specific to a cold-cache environment that no longer reproduces in development, Phase E5's dramatic benefit may be overstated in the current documentation — without this being a defect of Phase E5's code itself. **Suggested next experiment:** measure `initialize` against `codebase-memory-mcp` in an environment with an empty `~/.cache/codebase-memory-mcp/` (a clean container) to isolate the cache variable.

---

## What holds under attack

1. **The `catch_unwind` in `src/mcp/server.rs:142-163` correctly catches the tool panics that do reach it** (`.expect("cargar configuración")`, `.expect("leer records")`, `.expect("no hay Records...")` in `src/pipeline.rs`). Verified empirically with a `project_root` without `.rationale/`: the call returns `isError: true` with the expected generic message, and the session correctly answers a `health` call immediately afterwards. This holds exactly as commit `74a16b3` describes it — **for the panics that occur inside the tool's execution**. Findings A and B of this report show that there are panics/aborts/terminations that happen *before* that boundary (in `framing::read_message`), outside this guarantee's scope — the commit does not declare that false, but it does not delimit the real scope of the protection either.

2. **Byte-identity of the pipeline refactor, verified independently.** I built a worktree of the immediately preceding commit (`f774db7`, before `feat(mcp)`) and compared `rationale health`, `rationale prepare src/main.rs --project-root . --repo-path .`, and the same call with `--intent "test intent phrase"`, against the current binary (`a6a78b4`). An empty `diff` on stdout and stderr in all three cases. Commit `74a16b3`'s claim ("Verified byte-identical against the pre-refactor binary in the CLI") reproduces independently.

3. **`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` (50 tests: 40 unit + 2 MCP integration + 8 schema validation) pass cleanly**, with no warnings or failures, in this environment.

4. **The rule "stdout is exclusively for the protocol" holds in the strict sense the test verifies**: in none of the cases that do produce a response (unknown tool, invalid target, caught panic) was the framing corrupted — every message the process actually emitted was a valid `Content-Length` message. Findings A/B do not contradict this: in A1 the process aborts without emitting anything else; in A2/B the process stays silent or exits cleanly, but emits no corrupt bytes.

---

## Severity summary

| Severity | Count | Findings |
|---|---:|---|
| Critical | 2 | A1 (astronomical Content-Length → SIGABRT), A2 (header without terminator → unbounded memory) |
| High | 1 | B (invalid/nested JSON silently kills the persistent session) |
| Medium | 3 | C (budget exceeded without a warning), D (`additional_history_available` underestimates), E (`detect_conflict` false positives/negatives) |
| Minor | 2 | F (`token_estimate` not stable), `write_message` with `.expect()` outside `catch_unwind` (theoretical; no practical way to trigger it was found with the current types) |
| Nuance (not a bug) | 1 | G (the 6.8 s amortization narrative not reproduced in this environment; cause `Unknown`) |

**Recommendation on blocking:** in this review's judgment, **`src/mcp/framing.rs` (findings A1, A2) and the handling of unparseable messages in `src/mcp/server.rs`/`src/mcp/framing.rs` (finding B) should be treated as blocking before considering Phase E closed**, because they directly contradict the robustness guarantee that the Phase E6 test suite itself (`tests/mcp_server.rs`) claims to cover ("if a single byte of stdout stopped being a well-formed Content-Length message... that is the real assertion") without actually exercising the paths where the whole process dies or grows without bound. Findings C, D, and E are real and should be fixed, but they do not block on their own: levels 0–3 of the packet (the most important guarantee, v0.5 §30.1.7) were never compromised in any experiment. Finding G is not a code defect — it is an evidence discrepancy between the historical research and the current environment that merits a note in `docs/research/codebase-memory/11-performance-observations.md` or a new research item, not a code fix.

The decision on what to fix, and whether any ADR or piece of code moves to `accepted`, rests entirely with the project's human owner (`evaluation.no-self-certification`).
