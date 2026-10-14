# Research note: context integrity, retrieval scope, and agent integration (1.2)

**Consulted:** 2026-10-14. **Author:** Claude Code, for the 1.2 work in
`docs/work-items/context-integrity-1.2.md`.

Evidence labels: **[Measured]** a paper or benchmark with numbers;
**[Vendor]** numbers from the vendor's own evaluation; **[Guidance]** author
advice without measurement; **[Doc]** documented product behavior;
**[Inference]** this note's own reasoning.

## Ideas adopted

Each entry: the problem it addresses, the evidence, how it fits Rationale, its
cost, and how this work checks it.

### 1. A current view by default; history on request

- **Problem:** superseded Records served as current.
- **Evidence:** Chroma, *Context Rot* (2025-07-14,
  https://www.trychroma.com/research/context-rot) **[Measured]**: distractors
  degrade accuracy, more of them more. Claude Code memory docs
  (https://code.claude.com/docs/en/memory) **[Doc]**: with contradictory
  instructions "Claude may pick one arbitrarily". Graphiti/Zep (arXiv
  2501.13956, Apache-2.0) **[Vendor]** keeps invalidated facts with validity
  intervals instead of deleting them. adr-tools and MADR 4.0 **[Guidance]**:
  two-way supersede links and a status line.
- **Fit:** `lifecycle::StatusIndex` derives the status from both sides of the
  supersession; `history` keeps the past without serving it as authority. No
  LLM contradiction detection (Rationale stays deterministic).
- **Cost:** agents need `include_history` for the past decision; one-sided
  supersessions written by older binaries need `doctor --repair` for those
  binaries to see them.
- **Check:** synthetic scenarios Q1, Q2, Q13; superseded/revoked served as
  authority 7 → 0.

### 2. Symbol-level scope instead of the whole file

- **Problem:** a symbol query returned the whole file's Records.
- **Evidence:** Agentless (arXiv 2407.01489) **[Measured]**: localization
  with file skeletons kept the ground-truth location 58.33% of the time versus
  53.67% with complete files, at about a seventh of the cost. Serena
  **[Guidance]** works at symbol granularity.
- **Fit:** companion file bindings no longer spread a symbol Record over the
  file; a lexical symbol binding keeps the scope when the index lags.
- **Cost:** a v1.1 Record that really governed the whole file through a
  companion binding stops governing other symbols until re-bound.
- **Check:** Q3, Q9, the capture suite; Records of another symbol 3 → 0.

### 3. A hard, honest budget over what the agent actually receives

- **Problem:** responses over their own budget (reported median ≈ 3,800 tokens
  for 2,400).
- **Evidence:** Anthropic, *Writing effective tools for agents* (2025-09-11,
  https://www.anthropic.com/engineering/writing-tools-for-agents)
  **[Guidance]**: concise responses, truncation that steers to a narrower
  query. Claude Code MCP docs (https://code.claude.com/docs/en/mcp) **[Doc]**:
  a warning at 10,000 tokens and a 25,000 default limit. Anthropic token
  counting docs (https://platform.claude.com/docs/en/build-with-claude/token-counting)
  **[Doc]**: the count is an estimate; Claude 4.7 and later produce about 30%
  more tokens for the same text. playcode.io (2026-07-14, re-verified
  2026-09-02, one fixture per type) **[Measured, third party]**: about 3.0
  characters per token for JSON, 3.3 for English, 2.2–2.45 for code with the
  current tokenizer. Aider's repo map **[Guidance]** fits a budget by binary
  search but accepts overshooting by 15%.
- **Fit:** `src/tokens.rs` counts the complete serialized response as an
  upper bound; regenerable sections are trimmed first and listed in `omitted`;
  governing knowledge is never cut. **[Inference]** an upper bound is safer than
  a midpoint because the budget is a promise to the agent.
- **Cost:** not validated against `count_tokens` (no network or key in this
  work); dense non-ASCII text is counted pessimistically.
- **Check:** Q10, Q10b; real repository p50 3,068 → 2,370 estimated tokens.

### 4. Explicit folder scope

- **Problem:** folder queries returned nothing.
- **Evidence:** Claude Code's `.claude/rules/` with `paths:` globs and
  on-demand subdirectory `CLAUDE.md` files **[Doc]** scope instructions to a
  part of the tree.
- **Fit:** a folder target returns the Records bound inside it, by priority; a
  folder binding states an invariant below it.
- **Cost:** large folders need the priority trimming and the `omitted` list.
- **Check:** Q4, Q8; `src/providers` on the real repository 0 → 3 Records.

### 5. Invariants along observed dependencies

- **Problem:** a constraint from an incident did not reach modules sharing the
  invariant.
- **Evidence:** RepoCoder (arXiv 2303.12570) **[Measured]**: iterative
  repository-level retrieval beat in-file baselines by more than 10%. Aider
  ranks with personalized PageRank over a dependency graph **[Guidance]**.
  **[Inference]** an explicit, observed edge is the strongest justification a
  deterministic tool can offer for crossing a module boundary.
- **Fit:** `inherited_constraints` from observed `calls`/`uses`/`imports`/…
  edges, `critical`/`high`/pinned only; folder bindings for invariants without
  a call.
- **Cost:** depends on the provider; noisy name-based call resolution could
  bring a constraint across a false edge (limited to grave constraints).
- **Check:** Q5.

### 6. Operation leases, made visible, closed only explicitly

- **Problem:** a fifth of the operations never closed.
- **Evidence:** Anthropic, *Effective harnesses for long-running agents*
  (2025-11-26) **[Guidance]**: end each session in a clean state. Beads
  (MIT; `bd stale`, `bd prime` as a SessionStart hook) **[Guidance]**.
  claude-mem uses lifecycle hooks to capture and close **[Guidance]**.
- **Fit:** `open | finalized | abandoned`, `rationale operations`, `health`,
  pruning that never deletes open ones, and the Stop hook.
- **Cost:** another field in the local operation file; agents must call
  `abandon` instead of leaving operations.
- **Check:** the operations suite and the hook tests.

### 7. Retirement with referential integrity

- **Problem:** 116 active Records describing a retired module.
- **Evidence:** Beads' delete refuses when dependents exist unless `--cascade`
  or `--force` is explicit **[Guidance]**; MADR's `deprecated` status.
- **Fit:** `rationale retire` plans, applies only on request, never touches
  pinned Records, lists exceptions it would orphan, and undoes byte for byte.
- **Cost:** a human command; agents propose it.
- **Check:** the repair suite.

### 8. A deterministic boot with directive codes, and hooks as the backstop

- **Problem:** instructions alone did not close operations or surface canon
  problems.
- **Evidence:** Impeccable (https://github.com/pbakaus/impeccable,
  Apache-2.0, about 81k stars) **[Guidance, verified in its source]**: one
  entry skill with a router, a context command run once per session that prints
  directive codes (`CONTEXT_STALE`, …), and SessionStart/PostToolUse/Stop hooks
  installed to `settings.local.json`; behavior tests assert on tool-call
  traces. Its quality claims are author-sourced. Claude Code hooks reference
  (https://code.claude.com/docs/en/hooks) **[Doc]**: `additionalContext`
  (10,000-character cap), Stop `decision: block` with `stop_hook_active`, MCP
  tool matchers.
- **Fit:** `rationale hook` with `OPEN_OPERATIONS` and `CANON_FINDINGS` lines
  in SessionStart; reminders, never blocks.
- **Cost:** a local settings file per machine.
- **Check:** hook tests; a live SessionStart run with Claude Code 2.1.296.

## Ideas not adopted, and why

- **Embeddings, hybrid vector retrieval, LLM contradiction detection** (Zep,
  Mem0, cognee): they need models or remote services Rationale excludes, and
  Mem0's own LOCOMO results put full context (72.9) above its memory (66.9)
  **[Vendor]**.
- **Contextual Retrieval's contextual BM25** (Anthropic, 2024-09-19) **[Measured,
  combined with embeddings]**: the FTS index only feeds diagnostics today; not
  worth the cost until lexical search selects Records. Recorded as an option.
- **A codebase-memory-style compressed graph as the main context:** the
  codebase-memory paper (arXiv 2603.27277) **[Measured]** reports 10× fewer
  tokens but answer quality 83% versus 92% for a file-exploring agent — a
  warning against cutting governing context to hit a number.

## Model selection facts used by `execution_advice`

From the Claude models overview and Claude Code model configuration
(https://platform.claude.com/docs/en/about-claude/models/overview,
https://code.claude.com/docs/en/model-config) **[Doc]**: aliases `haiku`,
`sonnet`, `opus`, `fable` resolve to Haiku 5.5, Sonnet 5.5, Opus 5.5, and Fable
5.1 on the Anthropic API (other providers differ); effort levels `low` …
`max`; subagents take `model` and `effort` in frontmatter and per Agent call.
The advice uses aliases so it does not age, and it is not validated against
outcomes.

## Unknowns

- The estimator against `count_tokens` on real Rationale responses — needs an
  API key.
- The hooks' behavior with a live model beyond SessionStart — needs a
  logged-in Claude Code CLI.
- Whether the reported team's 116 retired-module Records split the way the
  synthetic 12 did (retire, unbind, pinned) — needs their canon, which this
  work did not access.
