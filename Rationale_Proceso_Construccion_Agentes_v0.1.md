# Rationale

## Agent build process 0.1

### Operating manual for Claude Code, Codex, and other coding agents

**Version:** 0.1\
**Cutoff date:** 2026-07-24\
**Required architecture:** `Rationale_Arquitectura_Conceptual_v0.1.md`\
**Required product contract:** `Rationale_v0.5.md`

> **Status (1.0):** the working rules in this manual still apply to how
> Rationale is built. The sections on packaging (§23), the landing page (§24),
> and the initial deliverables (§27) describe steps that are already done.
> §21 predates the 1.0 model: Records are no longer reviewed through a
> proposal queue; `finalize_change` writes them through the capture gate and
> people keep authority through pins and conflicts. For current behavior, read
> [`README.md`](README.md) and [`docs/user-guide/`](docs/user-guide/).

---

# 0. Purpose

This document defines how several coding agents must build Rationale without
losing:

- Decisions.
- Evidence.
- Context.
- Architecture.
- Experiment results.
- Limitations.
- Risks.
- Reasons.
- Continuity between sessions.

The project will be built mainly with:

- Claude Code.
- OpenAI Codex.
- Other compatible agents.
- Human review of critical decisions.

Different agents may develop the tool at different times.

For that reason, no agent may assume it knows earlier conversations.

The repository must contain everything needed to continue.

---

# 1. Fundamental rule

> Nothing important may exist only in an agent's conversation.

Every important decision must end up in:

- Code.
- A test.
- A document.
- An ADR.
- A research note.
- An experiment result.
- A Rationale Record.

Whichever applies.

Nothing is documented for the sake of documenting.

What gets documented is what another agent would need so it neither repeats
mistakes nor destroys decisions.

---

# 2. Source hierarchy

Before acting, an agent must apply:

```text
1. Tests and reproducible behavior
2. Rationale_v0.5.md
3. Approved ADRs
4. Current conceptual architecture
5. Approved Rationale Records
6. Verified research notes
7. Issue or task plan
8. Code comments
9. Agent inferences
```

An inference must never silently overwrite an approved decision.

---

# 3. Session start protocol

Every agent must:

## Step 1 — Identify the repository

```bash
git rev-parse --show-toplevel
git status --short
git branch --show-current
git rev-parse HEAD
```

## Step 2 — Read the minimum documents

In this order:

```text
AGENTS.md
Rationale_v0.5.md
Rationale_Arquitectura_Conceptual_v0.1.md
Rationale_Proceso_Construccion_Agentes_v0.1.md
Relevant ADRs
Current issue/plan
```

Reading 5,000 lines for every trivial operation is not necessary.

`AGENTS.md` must contain a reading path per task.

## Step 3 — Check the environment

```bash
rationale health          # once it exists
codebase-memory-mcp --version
git --version
```

## Step 4 — Query Codebase Memory

For non-trivial tasks:

- View the architecture.
- Find targets.
- View dependencies.
- View impact.
- View coverage.
- Confirm the revision.

## Step 5 — Review existing work

- Git status.
- Diffs.
- TODOs.
- Failing tests.
- Research in progress.
- Pending ADRs.

## Step 6 — Declare scope

Before editing, the agent must state in its plan:

- What it will change.
- What it will not change.
- Which documents it needs.
- Which tests it will run.
- Which decisions could be affected.

---

# 4. Agent roles

An agent may take several roles in sequence.

It must not mix them without saying so.

## 4.1 Research Agent

Responsible for:

- Reading upstream.
- Running experiments.
- Citing files.
- Separating claim from observation.
- Recording unknowns.
- Not implementing production code prematurely.

## 4.2 Architecture Agent

Responsible for:

- Defining boundaries.
- Evaluating tradeoffs.
- Creating ADRs.
- Keeping traceability.
- Avoiding coupling.

It does not approve critical decisions on its own.

## 4.3 Implementation Agent

Responsible for:

- Implementing approved scope.
- Creating tests.
- Respecting contracts.
- Updating nearby documentation.
- Not widening scope without recording it.

## 4.4 Review Agent

Responsible for:

- Reading the diff.
- Running tests.
- Looking for contradictions.
- Reviewing security.
- Reviewing docs.
- Reviewing performance.
- Not defending the implementation because it wrote it.

Preferably this will be:

- Another agent.
- Another session.
- Another model.
- A human.

## 4.5 Evaluation Agent

Responsible for:

- Running the harness.
- Applying the rubric.
- Not modifying outputs.
- Recording metrics.
- Comparing conditions.
- Separating analysis from judgment.

## 4.6 Documentation Agent

Responsible for:

- Consolidating results.
- Avoiding duplication.
- Keeping links working.
- Updating the changelog.
- Verifying examples.
- Not inventing behavior.

---

# 5. Minimum separation of duties

For critical changes:

```text
Research/Plan
    ↓
Implementation
    ↓
Independent Review
    ↓
Evaluation
    ↓
Human or authorized approval
```

The same agent may implement and do a first self-review.

That does not replace an independent review.

---

# 6. Workflow for a piece of work

## 6.1 Intake

Create or update:

```text
docs/work-items/<id>.md
```

It must contain:

- Problem.
- Goal.
- Non-goals.
- Base revision.
- Evidence.
- Risks.
- Plan.
- Tests.
- Docs.
- Success criterion.

## 6.2 Preflight

Before code:

- Query Codebase Memory.
- Query Rationale once it exists.
- Identify constraints.
- Identify ADRs.
- Identify modules.
- Record unknowns.

## 6.3 Research

When there is uncertainty:

- Create a spike.
- Do not contaminate production.
- Measure.
- Compare.
- Save results.

## 6.4 Implementation

- Small changes.
- Coherent commits.
- Tests alongside the change.
- No unrelated refactors.
- No unnecessary dependencies.
- No secrets.

## 6.5 Self-review

The implementer will review:

```bash
git diff --check
git diff
```

And will run:

- Formatter.
- Lint.
- Unit tests.
- Contract tests.
- Relevant integration tests.
- Relevant security checks.
- Benchmark, if applicable.

## 6.6 Independent review

Another reviewer will verify:

- Correctness.
- Scope.
- Architecture.
- Security.
- Error handling.
- Concurrency.
- Docs.
- Tests.
- Metrics.
- Backward compatibility.

## 6.7 Documentation gate

Before completing:

- Update the ADR if a decision was made.
- Update the architecture if a boundary changed.
- Update research if something was discovered.
- Update the runbook if an operation changed.
- Update examples if a contract changed.
- Update the Rationale Record once it exists.

## 6.8 Finalize

- Run the suite.
- Save the results.
- Record the revision.
- Capture evidence.
- Close the work item.
- Write a change summary.

---

# 7. Mandatory use of Codebase Memory

## 7.1 During bootstrap

Codebase Memory will be used to:

- Index its own clone.
- Index Rationale.
- Compare declared and observed architecture.
- Find modules.
- Analyze impact.
- Reduce manual reading.

## 7.2 Before changes

The agent must query structure when the change:

- Touches several modules.
- Changes contracts.
- Changes storage.
- Changes MCP.
- Changes revision handling.
- Changes providers.
- Changes security.
- Changes packaging.
- Changes a critical Subject.

## 7.3 After changes

- Reindex, or wait for a verified update.
- Check the provider revision.
- Query the target again.
- Confirm the impact.
- Record discrepancies.

## 7.4 Do not trust blindly

Results must include:

- Coverage.
- Revision.
- Provider version.
- Warnings.

The source code remains the primary evidence for exact behavior.

---

# 8. Initial Codebase Memory analysis

The first epic must produce:

```text
EPIC-CBM-ANALYSIS
```

Subtasks:

```text
CBM-001 Clone and lock revision
CBM-002 Build on MacBook Air M4
CBM-003 Run tests
CBM-004 Index itself
CBM-005 Map modules
CBM-006 Inspect MCP
CBM-007 Inspect CLI
CBM-008 Inspect revision and coverage
CBM-009 Inspect daemon and watcher
CBM-010 Inspect workspace support
CBM-011 Measure CLI vs MCP
CBM-012 Recommend adapter boundary
```

Every task will have evidence.

---

# 9. Language selection

No agent will start the definitive core before `ADR-0001`.

## 9.1 Common spike

Every candidate will implement the same function:

```text
Input:
target + intent + revision

Operations:
read one Record
open SQLite
call/mock provider
check revision
rank one constraint
emit JSON

Measurements:
startup
latency
memory
binary size
test speed
cross-platform viability
```

## 9.2 Same workload

One candidate may not use a trivial demo while another implements everything.

## 9.3 Skills and documentation

After choosing the language:

- Find the official documentation.
- Install or create skills.
- Record versions.
- Create a style guide.
- Create a testing guide.
- Create a security guide.
- Configure the formatter and linter.
- Create agent instructions.

## 9.4 Skills policy

Create a skill when:

- The workflow repeats.
- It has verifiable steps.
- It reduces errors.
- It can be maintained.

Do not create a skill to:

- Handle a one-off task.
- Replace official documentation.
- Hide unsafe commands.
- Grant global permissions.

---

# 10. Mandatory documentation

## 10.1 AGENTS.md

It must be short.

It will contain:

- What to read.
- How to run things.
- What not to do.
- Quality gates.
- Links to docs.

It will not duplicate the whole architecture.

## 10.2 ADR

Format:

```text
Context
Decision
Status
Evidence
Alternatives
Consequences
Risks
Validation
Revisit trigger
```

## 10.3 Research note

Format:

```text
Question
Environment
Source revision
Method
Observation
Result
Limitations
Impact
Artifacts
```

## 10.4 Experiment

Format:

```text
Hypothesis
Protocol
Dataset
Conditions
Metrics
Raw results
Analysis
Threats to validity
Decision
```

## 10.5 Runbook

For:

- Build.
- Test.
- Install.
- Update.
- Uninstall.
- Cache reset.
- Provider failure.
- Migration.
- Release.
- Diagnostics.

## 10.6 Code comments

Comments must explain:

- Why.
- Invariants.
- Security reasons.
- Non-obvious tradeoffs.

They must not repeat syntax.

---

# 11. Per-agent work log

Every important run must produce a local log.

```json
{
  "run_id": "...",
  "agent": "codex",
  "model": "...",
  "role": "implementation",
  "task_id": "...",
  "base_revision": "...",
  "end_revision": "...",
  "started_at": "...",
  "ended_at": "...",
  "tools": [],
  "files_read": [],
  "files_changed": [],
  "tests": [],
  "metrics": {},
  "status": "..."
}
```

Location:

```text
.rationale-local/runs/
```

Sensitive data is not versioned.

A summary may be saved in the work item.

---

# 12. Agent-assisted self-review

Self-review has five passes.

## Pass 1 — Correctness

- Does it meet the goal?
- Does it handle errors?
- Does it have tests?
- Does it break invariants?

## Pass 2 — Architecture

- Does it respect boundaries?
- Does it add a circular dependency?
- Does it couple to Codebase Memory internals?
- Does it duplicate a responsibility?

## Pass 3 — Security

- Paths.
- Secrets.
- Injection.
- Permissions.
- Untrusted data.
- Temp files.
- Subprocesses.

## Pass 4 — Performance

- Hot path.
- Allocations.
- Queries.
- Process spawn.
- Locks.
- Timeouts.
- Cache.

## Pass 5 — Documentation

- ADR.
- Examples.
- Runbook.
- Comments.
- Changelog.
- Rationale Record.

The agent must report concrete findings.

It must not write only:

```text
Looks good.
```

---

# 13. Claude Code / Codex cross-review

When possible:

```text
Agent A implements.
Agent B reviews.
Agent A addresses.
Agent B verifies.
```

The identities of A and B may alternate.

For critical decisions:

- One agent proposes.
- The other tries to falsify.
- The human approves or rejects.

The review must look for:

- Counterexamples.
- Race conditions.
- Stale revisions.
- False confidence.
- Hidden cost.
- Cross-platform issues.
- Missing tests.
- Documentation gaps.

---

# 14. Build metrics

Besides measuring Rationale on tasks, its construction will be measured.

```text
Lead time per work item
Rework cycles
Tests added
Regression count
Review findings
Documentation completeness
Architecture violations
Dependency growth
Build time
Test time
Binary size
Memory
Agent tool calls
Prompt context written manually
```

The goal is not to maximize commits.

It is to increase evidence per change.

---

# 15. Empirical evaluation

## 15.1 Ground truth

Every historical case will have:

- Must know.
- Useful.
- Irrelevant.
- Dangerous falsehoods.
- Expected tests.
- Expected invariant.

The ground truth must be prepared before running the conditions.

## 15.2 Conditions

```text
A. Code + Git
B. AGENTS/ADR/docs
C. Codebase Memory
D. Codebase Memory + Rationale
E. Expert prompt
```

## 15.3 Same task

The following will be controlled:

- Model.
- Temperature, when configurable.
- Base revision.
- Tools.
- Time budget.
- Prompt.
- Test harness.

## 15.4 Blind evaluation

When possible, the evaluator must not know which condition produced the result.

## 15.5 Self-evaluation limitation

Agents may:

- Collect data.
- Run tests.
- Apply rubrics.
- Generate analysis.

They cannot be the only proof.

Conclusions must rest on:

- Tests.
- Ground truth.
- Paired comparisons.
- Bootstrap confidence intervals.
- Separate review.
- Raw data.

---

# 16. Definition of Done

A work item is not complete if any applicable item is missing:

- Code.
- Tests.
- Formatter.
- Lint.
- Security check.
- Documentation.
- ADR.
- Research artifact.
- Metrics.
- Review.
- Reindex verification.
- Rationale finalize.
- Clean git status.

---

# 17. Quality gates per phase

## Bootstrap

- Docs exist.
- Links work.
- Environment captured.
- Codebase Memory installed.
- Repo indexes.

## Research

- Source pinned.
- Reproducible commands.
- Claims cited.
- Unknowns visible.
- Results committed.

## Vertical slice

- End-to-end test.
- Structured errors.
- No paid service.
- Revision shown.
- Provider coverage shown.
- Packet bounded.

## Dogfood

- Rationale installed in itself.
- Records reviewed.
- Baseline measured.
- No false block.
- Recovery documented.

## Pilot

- Dataset locked.
- Conditions run.
- Raw data preserved.
- Analysis reproducible.
- Sensitive data protected.

## Packaging

- Clean machine install.
- Update.
- Uninstall.
- Rollback.
- Checksums.
- Platform matrix.
- No orphan config.

---

# 18. Git workflow

## Branches

```text
research/<topic>
spike/<topic>
feature/<topic>
fix/<topic>
docs/<topic>
release/<version>
```

## Commits

Every commit must:

- Be coherent.
- Pass the relevant tests.
- Avoid mixing in refactors.
- Have a causal message.

Example:

```text
feat(revision): reject exact context when provider is behind
```

## Pull requests

They must include:

- Why.
- What.
- Non-goals.
- Tests.
- Metrics.
- Docs.
- Risks.
- Rollback.
- Rationale impact.

---

# 19. Dependencies

Before adding a dependency:

- Is it necessary?
- Can the standard library solve it?
- Is it active?
- Is the license compatible?
- Is it cross-platform?
- What is the binary impact?
- What is the supply-chain risk?
- Can it be pinned?
- Does it need network access?
- Is there an alternative?

It will be recorded in:

```text
docs/dependencies/<name>.md
```

---

# 20. Cost control

Development must prefer:

- Local execution.
- Local tests.
- Small fixtures.
- Controlled benchmarks.
- Cache.
- Context reuse.
- Models already available.
- AI already paid for.

The following must not be introduced:

- Managed database.
- Remote vector database.
- SaaS telemetry.
- Queue service.
- Permanent infrastructure.

without a later decision.

---

# 21. Dogfooding

Rationale must be used on its own repository once these exist:

```text
init
prepare_change
explain_target
finalize_change
health
```

Suggested first Subjects:

```text
architecture.provider-boundary
architecture.revision-consistency
policy.no-inferred-blocks
policy.local-first
storage.canonical-vs-derived
retrieval.context-budget
evaluation.no-self-certification
```

Its Records must be reviewed.

They will not be self-approved.

---

# 22. Pilot in the work monorepo

The real pilot must start only after:

- A stable vertical slice.
- A security review.
- Local-only verification.
- Sensitivity support.
- No automatic writes outside `.rationale/`.
- A backup.
- Uninstall.

The first mode will be:

```text
read-only
```

Then:

```text
assisted capture
```

Automatic blocking will not be enabled at the start.

---

# 23. Packaging

Packaging will happen after value is demonstrated.

Order:

```text
1. Development build for macOS arm64
2. Reproducible release build
3. macOS package
4. Linux amd64
5. Linux arm64
6. Windows amd64
7. Install/update/uninstall
8. Sign/checksum
9. Release automation
```

The installer must be auditable.

It will not make hidden changes.

---

# 24. Landing page

The landing page will be a distribution phase.

It is not part of the core.

Before creating it, the following must exist:

- A downloadable release.
- A tested quick start.
- Real screenshots.
- Honest benchmarks.
- A security statement.
- A license.
- A changelog.
- Troubleshooting.
- Real platform support.

Unverified metrics will not be used.

---

# 25. What an agent must do when it finds a contradiction

1. Stop the affected assumption.
2. Record the evidence.
3. Create an issue or work item.
4. Reproduce it.
5. Identify the affected documents.
6. Propose an ADR.
7. Not hide the contradiction.
8. Not silently change the concept and the code.

---

# 26. What an agent must do when it does not know

It must write:

```text
Unknown:
I could not verify X.

Evidence:
...

Risk:
...

Next experiment:
...
```

It must not fill the gap with a convincing explanation.

---

# 27. Initial deliverables

## Conceptual week/iteration 1

- Repository bootstrap.
- AGENTS.md.
- Environment script.
- Codebase Memory clone.
- Source lock.
- Build note.
- Test note.

## Iteration 2

- Module map.
- MCP analysis.
- CLI analysis.
- Revision analysis.
- Monorepo analysis.

## Iteration 3

- Language spikes.
- Benchmarks.
- ADR-0001.
- Toolchain.

## Iteration 4

- Vertical slice.
- Contract tests.
- Instrumentation.
- Golden packet.

These are not promised dates.

They are the dependency order.

---

# 28. Start checklist for any agent

```text
[ ] I read AGENTS.md
[ ] I identified the base revision
[ ] I checked git status
[ ] I read the work item
[ ] I queried Codebase Memory
[ ] I checked coverage
[ ] I reviewed the ADRs
[ ] I declared non-goals
[ ] I defined tests
[ ] I defined docs
```

# 29. Closing checklist

```text
[ ] Diff reviewed
[ ] Tests pass
[ ] No secrets
[ ] Docs updated
[ ] ADR updated
[ ] Metrics saved
[ ] Independent review
[ ] Codebase Memory updated
[ ] Revision consistency verified
[ ] Rationale finalize run
[ ] Work item closed
```

---

# 30. Final rule

> Agents are not only building code.\
> They are building the system and the memory the next agent needs to build it
> better.

The project will have applied this process correctly when a new session of
Claude Code, Codex, or another agent can:

1. Clone the repository.
2. Read the instructions.
3. Index it.
4. Understand the architecture.
5. Find the decisions.
6. Run the tests.
7. Continue a task.
8. Justify its changes.
9. Leave the project easier to understand than before.
