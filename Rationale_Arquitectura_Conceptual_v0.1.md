# Rationale

## Conceptual architecture 0.1

### Technical contract prior to implementation

**Architecture version:** 0.1\
**Cutoff date:** 2026-07-24\
**Status:** conceptual architecture, deliberately not final\
**Required conceptual document:** `Rationale_v0.5.md`\
**Complementary operating document:** `Rationale_Proceso_Construccion_Agentes_v0.1.md`

> **Status (1.0):** this document captured the boundaries Rationale was built
> against. The decisions it left open were settled in the ADRs
> ([`docs/adr/index.md`](docs/adr/index.md)) and the 1.0 plan
> ([`docs/work-items/vnext-implementation-plan.md`](docs/work-items/vnext-implementation-plan.md)).
> Where this document and an accepted ADR disagree, the ADR describes what was
> built. The factual map of the current code is
> [`docs/architecture/code-map.md`](docs/architecture/code-map.md).

---

# 0. Main warning

This document does not pretend that the final architecture has already been
discovered.

It defines:

- The boundaries the product needs.
- The conceptual components that must exist.
- The contracts that must be verified.
- The experiments that must be run.
- The decisions that cannot yet be made responsibly.
- The right order to analyze, build, validate, package, and distribute
  Rationale.

The final architecture must not be implemented by mechanically copying this
document.

Before fixing:

- Language.
- Runtime.
- IPC.
- Concurrency model.
- MCP SDK.
- Codebase Memory integration.
- Physical index format.
- Installers.
- Hooks.
- Daemon.
- Distribution.

the team and the agents must analyze the current version of Codebase Memory,
build small prototypes, and record the results.

The rule will be:

> The conceptual architecture defines the responsibilities.\
> Technical research defines their implementation.

If a real observation of Codebase Memory contradicts an assumption in this
document, it must not be hidden or forced.

It must be:

1. Documented.
2. Reproduced.
3. Measured.
4. Turned into an explicit decision.
5. Used to update the architecture through an ADR.

---

# 1. Relationship with the conceptual document

`Rationale_v0.5.md` remains the source of truth on:

- The problem.
- The product definition.
- Chesterton's Fence.
- Epistemic humility.
- Subjects.
- Records.
- Bindings.
- Evidence.
- Approvals.
- Assessments.
- Provenance.
- Authority.
- Applicability.
- Per-revision consistency.
- Monorepos.
- Context budget.
- Context utility density.
- Baseline mode.
- Intent-aware mode.
- Cold start.
- Progressive capture.
- Security.
- Metrics.
- Validation experiment.
- Product roadmap.

This document does not replace that definition.

The architecture must be considered invalid if it implements a system that
contradicts the conceptual contract.

In case of conflict, the priority will be:

```text
1. Real, reproducible evidence from the system
2. Rationale_v0.5.md
3. Approved ADRs
4. Current conceptual architecture
5. Implementation plans
6. Undocumented code
```

Code must never accidentally become the only explanation of an architectural
decision.

---

# 2. Goal of architecture 0.1

Architecture 0.1 must make it possible to start building a first vertical slice
of Rationale without prematurely committing to the final implementation.

It must answer:

- Which components the system needs.
- Which data is canonical.
- Which data is derived.
- How it integrates with Codebase Memory.
- How consistency is kept between Git, Codebase Memory, and Rationale.
- How context is retrieved at low cost.
- How the product is instrumented.
- How it is tested on its own repository.
- How it is tested on a real monorepo.
- How it is initially installed for development.
- What must be packaged later.
- Which decisions need research before they are written into code.

It must not yet try to solve:

- The final universal installer.
- A final graphical interface.
- A landing page.
- Remote synchronization between organizations.
- A SaaS service.
- A marketplace.
- A central database.
- Billing.
- A full extension for every IDE.
- Perfect compatibility with every agent.
- A stable public protocol.
- Perfect automatic conceptual lineage.

---

# 3. Mandatory project order

The order of work will be:

```text
1. Consolidate the concept
2. Analyze Codebase Memory
3. Analyze the local environment
4. Select language and toolchain
5. Build a minimal vertical slice
6. Instrument
7. Dogfood on Rationale
8. Run a pilot on a real monorepo
9. Correct the architecture
10. Complete the tool
11. Package for macOS, Linux, and Windows
12. Design the installation experience
13. Publish user documentation
14. Create the landing page
```

The landing page is not part of the first build.

Multi-platform packaging must not block validation of the core either.

First it must be shown that the tool:

- Retrieves the right context.
- Reduces manual context.
- Does not introduce falsehoods.
- Respects revisions.
- Integrates usefully with Codebase Memory.
- Improves real agent results.

---

# 4. Fundamental constraints

## 4.1 Local-first

The core must be able to run locally.

It must not require:

- Its own API.
- A remote server.
- A managed database.
- An embeddings service.
- A Rationale account.
- Docker.
- A cluster.
- Central telemetry.
- A paid license.

The external agents used for programming may have their own costs or
subscriptions.

Those costs must not become a runtime dependency of Rationale.

## 4.2 No mandatory embedded LLM

Rationale does not need to embed a language model in order to exist.

The model the developer already uses will be the consumer of the context.

The core must be able to:

- Read records.
- Validate schemas.
- Resolve scope.
- Evaluate authority.
- Query providers.
- Build packets.
- Apply budgets.
- Detect inconsistencies.
- Emit metrics.

without making an external call to an LLM.

A local or remote model may optionally be used to:

- Propose a summary.
- Propose Claims.
- Propose a Subject.
- Help with archaeology.
- Classify candidates.

But its results must enter as `inferred` and never as automatic authority.

## 4.3 Offline after installing dependencies

The main flow must be able to work offline when:

- The code is available locally.
- Codebase Memory is installed.
- The required dependencies have been downloaded.
- The agent can operate without a network or needs no new external calls.

## 4.4 Modularity

Every component must have:

- A clear responsibility.
- An explicit contract.
- Its own tests.
- Directed dependencies.
- The ability to be replaced.

Modularity does not mean creating dozens of packages prematurely.

The initial implementation must prefer a modular monolith.

## 4.5 Reasonable scalability

The first goal is not to index all the software in the world.

It must scale correctly for:

- Small repositories.
- Medium monorepos.
- Projects with multiple packages.
- A real enterprise project.
- Several local agents.
- Thousands of causal records over the long term.

The architecture must measure before promising.

## 4.6 Fail with humility

When there is not enough coverage, Rationale must say so.

It cannot turn:

```text
I did not find a relationship.
```

into:

```text
The relationship does not exist.
```

It cannot turn:

```text
The index is behind.
```

into:

```text
The decision still holds.
```

---

# 5. Main development environment

The known main environment is:

```text
Machine: MacBook Air
Chip: Apple M4
Architecture: arm64 / Apple Silicon
Memory: 16 GB RAM
Operating system: macOS
```

The exact macOS version and available storage must be recorded when the
repository is started.

The profile must not store:

- Serial number.
- Hardware UUID.
- Private identifiers.
- Unnecessary personal paths.
- Tokens.
- Credentials.

## 5.1 Inventory commands

```bash
system_profiler SPHardwareDataType
sw_vers
uname -a
uname -m
sysctl -n hw.memsize
sysctl -n hw.ncpu
df -h
git --version
clang --version
xcode-select -p
```

A reproducible script must be created:

```text
scripts/dev/collect-environment.sh
```

The script will produce:

```text
.rationale-local/environment.json
```

That folder will be in `.gitignore`.

An anonymized version may be documented in:

```text
docs/environment/reference-development-machine.md
```

## 5.2 Design implications

On a MacBook Air M4 with 16 GB:

- The architecture must avoid unnecessary resident processes.
- It must not duplicate full indexes in memory.
- Large-scale tests must have limits.
- Benchmarks must record peak memory.
- The daemon, if it exists, must be optional and lean.
- Frequent operations must use a local cache.
- The system must support Apple Silicon from the start.
- Initial development may prioritize macOS arm64.
- The implementation must not use macOS-only APIs in the core.

---

# 6. Codebase Memory as an object of research

Codebase Memory will not be treated only as a dependency.

It will also be a system to study.

The official repository must be cloned to understand:

- How it discovers projects.
- How it identifies workspaces.
- How it stores the graph.
- How it exposes MCP.
- How it runs the CLI.
- How it coordinates the daemon and watchers.
- How it represents revisions.
- How it reports coverage.
- How it resolves symbols.
- How it computes impact.
- How it handles monorepos.
- How it installs agent configurations.
- How it packages binaries.
- How it protects MCP stdout.
- How it implements deadlines.
- How it fails when it has no information.

## 6.1 Research workspace layout

Codebase Memory must not be copied into Rationale's source code as an
accidental dependency.

The recommended layout is:

```text
rationale-lab/
├── rationale/
├── upstream/
│   └── codebase-memory-mcp/
├── pilots/
│   └── work-monorepo/
└── datasets/
    └── historical-cases/
```

`upstream/codebase-memory-mcp/` will be:

- An independent clone.
- Read-only for normal work.
- Pinned to a commit.
- Updated deliberately.
- Not automatically included in Rationale releases.

The analyzed revision will be recorded in:

```text
docs/research/codebase-memory/source-lock.yaml
```

Example:

```yaml
repository: DeusData/codebase-memory-mcp
branch: main
commit: <sha>
analyzed_at: 2026-07-24
binary_version: <detected>
```

## 6.2 Two ways to use Codebase Memory during development

### Published binary

It will be used to:

- Index Rationale.
- Index the Codebase Memory clone.
- Get a stable behavioral reference.
- Keep a modified local build from contaminating the observation.
- Use its tools from Claude Code, Codex, or other agents.

### Build from source

It will be used to:

- Understand its architecture.
- Run its suite.
- Reproduce problems.
- Read implementations.
- Confirm contracts.
- Test compatibility.
- Investigate performance.
- Compare CLI against MCP.

The two kinds of results must be kept apart.

## 6.3 Research bootstrap

```bash
mkdir -p ../upstream
git clone https://github.com/DeusData/codebase-memory-mcp.git ../upstream/codebase-memory-mcp
cd ../upstream/codebase-memory-mcp
git rev-parse HEAD
scripts/build.sh
make -f Makefile.cbm test
```

The exact commands may change.

Agents must read the repository's current documentation first.

## 6.4 Codebase Memory will index itself

The initial analysis must include:

```text
A. Index codebase-memory-mcp with the official binary.
B. Query its architecture.
C. Query the MCP, store, daemon, watcher, pipeline, and CLI modules.
D. Compare the structural results against the code.
E. Record errors, omissions, or partial coverage.
```

This will make it possible to observe:

- How reliable the provider is.
- Which metadata it returns.
- Which calls are most useful.
- Which calls are expensive.
- Which limitations exist.
- Which data the adapter needs.

## 6.5 Rationale will also be indexed from day one

Before every non-trivial change, agents must be able to query:

- Current architecture.
- Symbols.
- Dependencies.
- Impact.
- Coverage.
- Uncommitted changes.

Codebase Memory will support the build even before Rationale can be used on
itself.

## 6.6 Observed facts that must be revalidated

In the revision analyzed while this document was written, Codebase Memory
declares:

- Main implementation in C.
- Static binary for macOS, Linux, and Windows.
- Local use.
- SQLite.
- Tree-sitter.
- MCP and CLI.
- Shared daemon.
- Watchers.
- Support for workspaces and cross-package relationships, with coverage limits.
- ADR management.
- Change detection.
- Derived index.
- Non-blocking hooks.
- Distribution with no required runtime.

Its current build shows separate modules for:

- Foundation.
- Store.
- Cypher.
- MCP.
- Daemon.
- Discovery.
- Pipeline.
- Semantic.
- Watcher.
- Git.
- CLI.
- UI.
- Tests and bug reproductions.

These facts help prepare questions.

They do not authorize copying its architecture without evaluation.

---


# 7. Codebase Memory analysis protocol

Before freezing the implementation architecture, the following documents must
be produced.

```text
docs/research/codebase-memory/
├── 00-source-lock.md
├── 01-build-and-test.md
├── 02-module-map.md
├── 03-mcp-contracts.md
├── 04-cli-contracts.md
├── 05-revision-and-coverage.md
├── 06-daemon-and-watcher.md
├── 07-storage-and-cache.md
├── 08-workspaces-and-monorepos.md
├── 09-installation-and-agents.md
├── 10-failure-modes.md
├── 11-performance-observations.md
└── 12-integration-recommendation.md
```

Each analysis must contain:

```text
Observed:
What it actually does.

Claimed:
What the documentation promises.

Verified:
What was reproduced.

Unknown:
What is still unclear.

Risk:
What could affect Rationale.

Decision impact:
Which architectural decision depends on this.
```

## 7.1 Mandatory questions

- What is the most stable way to invoke it from another process?
- Is client-to-server MCP better than a CLI subprocess for the first vertical
  slice?
- How does it report the indexed project?
- How does it report the indexed revision?
- How does it tell the working tree apart from HEAD?
- How does it report partial coverage?
- Can it be queried without starting the daemon?
- What latency does the CLI have compared with a persistent MCP session?
- What happens if two agents use it?
- What happens if the binary is updated during a session?
- Which data can be considered public?
- Which data belongs to unstable internals?
- What are its real limits in monorepos?
- Which contracts can be tested without reading its SQLite directly?
- What compatibility exists with Windows and Linux?
- How does it install hooks and instructions?
- How does it uninstall what it modifies?
- What can Rationale reuse as a pattern?
- What must stay completely decoupled?

## 7.2 Integration rule

Rationale must not:

- Read Codebase Memory's internal tables directly.
- Import internal headers.
- Assume private paths.
- Copy its cache.
- Share internal locks.
- Link against its binary as a library without an approved contract.
- Depend on undocumented node names without capability negotiation.

The preferred boundary will be public:

```text
MCP
or
structured CLI
```

The choice will be decided through a spike.

---

# 8. Language selection

The core language is not decided in architecture 0.1.

This is intentional.

It must not be chosen only because:

- Codebase Memory uses C.
- A popular SDK exists.
- An agent writes a certain language better.
- The prototype feels fast.
- A person prefers a language.

It must be chosen on evidence.

## 8.1 Initial candidates

The research must evaluate at least:

- Rust.
- Go.
- C.
- TypeScript/Node.js for prototyping or tooling.
- Another option only if there is a concrete reason.

Python may be used for:

- Experiments.
- Harnesses.
- Data analysis.
- Scripts.

It must not automatically become the distributed core.

## 8.2 Weighted criteria

```text
20% Memory safety and reliability
15% Distribution as a binary
15% Performance and latency
10% MCP and JSON-RPC
10% SQLite and filesystem
10% macOS/Linux/Windows compatibility
10% Maintainability with agents
5%  Compile time and development speed
5%  Interoperability with C processes
```

Each candidate must prove:

- A minimal MCP server.
- A client for Codebase Memory or a CLI wrapper.
- Reading and validating records.
- SQLite.
- File locking.
- Subprocesses.
- Cancellation.
- Deadlines.
- arm64 build.
- Cross-compilation or a CI strategy.
- Binary size.
- Startup time.
- Memory.
- Test tooling.
- Fuzzing or property tests.
- Packaging.

## 8.3 Deliverables

```text
docs/research/language/
├── candidates.md
├── benchmark-results.json
├── compatibility-matrix.md
├── spike-notes.md
└── ADR-0001-core-language.md
```

The decision must record:

- Evidence.
- Tradeoffs.
- Alternatives.
- Why they were discarded.
- Reversal risk.
- Review date.

## 8.4 Language-independent architecture

Until the ADR is approved:

- Module names will be conceptual.
- Interfaces will use pseudocode.
- No final crates, packages, or modules will be defined.
- Scripts will not assume a package manager.
- CI will have placeholders.
- The layout will avoid coupling documentation to Rust, Go, or C.

---

# 9. System overview

```text
┌──────────────────────────────────────────────────────┐
│ Coding Agent                                         │
│ Claude Code / Codex / other MCP client               │
└───────────────────────────┬──────────────────────────┘
                            │
                            │ MCP / CLI / hook surface
                            ▼
┌──────────────────────────────────────────────────────┐
│ Rationale Application Boundary                       │
│                                                      │
│ - MCP server                                         │
│ - CLI                                                │
│ - configuration                                      │
│ - health                                             │
│ - output formatting                                  │
└───────────────────────────┬──────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────┐
│ Context Coordination                                 │
│                                                      │
│ - revision coordinator                               │
│ - workspace/scope resolver                           │
│ - context compiler                                   │
│ - trust/authority/policy evaluator                   │
│ - subject resolver                                   │
│ - binding resolver                                   │
│ - capture/finalize lifecycle                         │
└───────────────┬───────────────────┬──────────────────┘
                │                   │
                ▼                   ▼
┌──────────────────────────┐  ┌────────────────────────┐
│ Structural Provider      │  │ Rationale Data         │
│ Adapter                  │  │                        │
│                          │  │ Canonical Git records  │
│ Codebase Memory          │  │ Local derived index    │
│ Future providers         │  │ Session state          │
└───────────────┬──────────┘  └────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────┐
│ Git + filesystem + tests + external evidence refs    │
└──────────────────────────────────────────────────────┘
```

---

# 10. Data layers

## 10.1 Shared canonical layer

Location:

```text
<project-root>/.rationale/
```

It will contain only portable, reviewable data.

```text
.rationale/
├── config.yaml
├── subjects/
├── records/
├── bindings/
├── approvals/
├── schemas/
└── migrations/
```

Characteristics:

- Versioned in Git.
- Reviewable in a PR.
- Readable without the tool.
- No cache.
- No mandatory embeddings.
- No machine-specific absolute paths.
- No tokens.
- No unnecessary secret data.
- With a schema version.

## 10.2 Local derived layer

Conceptual location:

```text
<user-cache>/rationale/projects/<project-id>/
```

On macOS an appropriate path must be respected.

The exact decision will be made during implementation.

It could map to:

```text
~/Library/Caches/Rationale/
```

or to a configurable XDG-compatible root.

It will contain:

- SQLite.
- Binding resolutions.
- FTS indexes.
- Scores.
- Coverage.
- Provider capabilities.
- Indexed revision.
- Packet cache.
- Local metrics.
- Hook state.
- Locks.
- Private logs.

It will be:

- Regenerable.
- Not versioned.
- Machine-specific.
- Deletable.
- Migratable.
- Restrictively permissioned.

## 10.3 Ephemeral layer

It will contain:

- Current intent.
- Targets.
- Hypotheses.
- Signals.
- Context packet.
- Tool call state.
- Record drafts.
- Provisional evaluation result.

It must have:

- A TTL.
- A session identifier.
- A base revision.
- Safe cleanup.
- No automatic promotion to approved knowledge.

---

# 11. Conceptual modules

## 11.1 Application Boundary

Responsibilities:

- Expose MCP.
- Expose the CLI.
- Validate arguments.
- Negotiate versions.
- Format responses.
- Apply external deadlines.
- Keep MCP stdout clean.
- Send logs to stderr or a file.
- Translate internal errors into explicit states.

It contains no domain logic.

## 11.2 Configuration

Responsibilities:

- Find the project root.
- Read `.rationale/config.yaml`.
- Resolve the cache root.
- Apply limits.
- Read the operating mode.
- Enable providers.
- Configure privacy.
- Configure blocking policies.
- Configure optional hooks.

It must support:

```text
project config
user config
environment overrides
safe defaults
```

The precedence must be documented.

## 11.3 Project and Workspace Discovery

Responsibilities:

- Detect the Git root.
- Detect a monorepo.
- Identify packages.
- Resolve ProjectScope.
- Normalize paths.
- Tell the workspace root apart from a package.
- Map a target to scopes.
- Avoid escaping the root.

It must understand that:

```text
repository != package
package != domain
domain != subject
```

## 11.4 Revision Coordinator

Responsibilities:

- Read Git HEAD.
- Identify the working tree.
- Get the provider revision.
- Get the assessments revision.
- Compute the consistency status.
- Prevent responses that look exact when the revisions do not match.
- Trigger the fast path or revalidation.

Conceptual input:

```json
{
  "git_head": "...",
  "working_tree_hash": "...",
  "provider_revision": "...",
  "rationale_assessment_revision": "..."
}
```

Output:

```text
exact
working-tree-ahead
provider-behind
rationale-behind
mixed
unknown
```

## 11.5 Structural Provider Adapter

Responsibilities:

- Capability negotiation.
- Resolve targets.
- Get relationships.
- Get impact.
- Get changes.
- Get coverage.
- Get the provider revision.
- Produce structural evidence.
- Apply timeouts.
- Normalize errors.

Conceptual contract:

```text
capabilities()
health()
indexed_revision()
resolve_target()
relationships()
impact()
changed_targets()
coverage()
architecture()
```

Every response must include:

```text
provider
provider_version
capability
revision
coverage
status
data
warnings
latency
```

## 11.6 Canonical Store

Responsibilities:

- Read records.
- Validate schemas.
- Write atomic changes.
- Maintain schema versions.
- Protect IDs.
- Maintain supersession.
- Resolve approvals.
- Not mix in cache.

## 11.7 Derived Index

Responsibilities:

- Index Records.
- FTS.
- Aliases.
- Scope paths.
- Binding resolutions.
- Candidate retrieval.
- Deduplication.
- Assessment cache.
- Invalidation by revision.

The derived database can never be the only copy of a decision.

## 11.8 Subject Resolver

Initial order:

```text
1. Exact ID
2. Alias
3. Explicit binding
4. Scope compatibility
5. FTS
6. Optional semantic candidates
```

It cannot:

- Create a Subject automatically by similarity.
- Merge Subjects automatically.
- Treat an embedding as identity.

When it proposes a new one and similar candidates exist, it must require
`novelty_reason`.

## 11.9 Binding Resolver

Responsibilities:

- Read portable declarations.
- Resolve them against the current provider.
- Record the revision.
- Record coverage.
- Detect stale/unresolved bindings.
- Keep derived history.
- Resolve package/workspace scope.
- Not silently edit the canonical declaration.

## 11.10 Trust, Authority and Policy Evaluator

Responsibilities:

- Distinguish observed, declared, and inferred.
- Resolve Approval.
- Verify authority per domain.
- Compute applicability.
- Identify contradictions.
- Decide attention.
- Apply blocking rules.

It may only block when:

```text
critical
AND approved-or-policy
AND active
AND exact-current-linkage
AND deterministic-contradiction
```

## 11.11 Context Compiler

Responsibilities:

- Interpret the target.
- Interpret the intent, if any.
- Retrieve candidates.
- Filter by scope.
- Filter by applicability.
- Prioritize constraints.
- Deduplicate.
- Respect the token budget.
- Emit uncertainty.
- Record why it included each item.

It must not generate an essay.

It must generate an operational packet.

## 11.12 Capture and Finalization

Responsibilities:

- Capture mechanical facts.
- Receive signals.
- Compare the diff.
- Propose Records.
- Propose Subjects.
- Request minimal confirmation.
- Save evidence.
- Update bindings.
- Not approve its own inferences.

## 11.13 Lifecycle Accelerators

Includes:

- Git hooks.
- File watcher.
- Daemon.
- Agent hooks.
- Post-commit detection.

They are optional.

Correctness must primarily depend on the revision gate, not on a hook always
working.

## 11.14 Evaluation and Telemetry

Responsibilities:

- Record latency.
- Record tokens when available.
- Record packet size.
- Record items.
- Record tool calls.
- Record test results.
- Record the experimental condition.
- Export data for analysis.
- Not send data automatically.

---

# 12. Initial public interfaces

The initial public surface must be small.

## `prepare_change`

Input:

- Targets.
- Optional intent.
- Optional symptoms.
- Base revision.
- Budget.
- Scope hints.

Output:

- Critical constraints.
- Conflicts.
- Decisions.
- Risks.
- Required validations.
- Affected areas.
- Coverage.
- Revision consistency.
- Trust.
- Additional history count.

## `explain_target`

Explains:

- Subject.
- Active decisions.
- Reason.
- Evidence.
- Constraints.
- Uncertainty.
- Revision.

## `finalize_change`

Input:

- Base.
- Head or working tree.
- Tests.
- Signals.
- Optional human confirmations.

Output:

- Mechanical evidence.
- Proposed records.
- Proposed binding updates.
- Required reviews.
- Stored canonical changes.

## `review_record`

Allows:

- Approving.
- Disputing.
- Correcting.
- Superseding.
- Changing authority.
- Adding evidence.

## `trace_rationale`

Walks:

```text
target
→ binding
→ subject
→ record
→ decision
→ approval
→ evidence
→ validation
```

## `health`

Reports:

- Project.
- Git revision.
- Provider.
- Provider revision.
- Cache.
- Coverage.
- Schema.
- Pending migrations.
- Stale assessments.
- Hook status.
- Latency summary.

Additional administrative operations may exist in the CLI without inflating
MCP.

---

# 13. Main flows

## 13.1 Baseline fast path

```text
Agent reads/searches target
        ↓
Client hook or explicit baseline request
        ↓
Resolve project and target cheaply
        ↓
Check revision fingerprint
        ↓
Read precomputed critical bindings
        ↓
Return compact context or no-op
```

Constraints:

- No embeddings.
- No archaeology.
- No LLM.
- No full reindex.
- No long calls.
- Fail open.
- Do not block reading.

Initial targets:

```text
P50 warm ≤ 50 ms
P95 warm ≤ 150 ms
Hard deadline ≤ 250 ms
```

These are pilot targets, not public guarantees.

## 13.2 Intent-aware preflight

```text
Agent expresses task
        ↓
prepare_change
        ↓
Revision coordination
        ↓
Structural provider query
        ↓
Subject + scope resolution
        ↓
Policy evaluation
        ↓
Ranking and budget
        ↓
Context packet
```

It may run more expensive analysis.

It must remain bounded.

Provisional target:

```text
P95 warm, excluding indexing ≤ 2 s
```

It must be measured before it is promised.

## 13.3 Finalize

```text
Implementation complete
        ↓
Collect diff and tests
        ↓
Compare against preflight
        ↓
Extract mechanical evidence
        ↓
Generate proposals
        ↓
Resolve duplicate subjects
        ↓
Ask minimal confirmation
        ↓
Write canonical files atomically
        ↓
Reindex derived state
```

## 13.4 Commit outside the flow

```text
Human commits without finalize
        ↓
Git revision advances
        ↓
Optional hook marks dirty
        ↓
Next query runs revision gate
        ↓
Assessments become behind
        ↓
Rationale degrades confidence
        ↓
Selective revalidation
```

It must not pretend to be up to date.

## 13.5 Provider unavailable

```text
Codebase Memory unavailable
        ↓
Use exact canonical bindings only
        ↓
Report provider unavailable
        ↓
Do not claim structural completeness
        ↓
Do not issue new deterministic block based on absent structure
```

---

# 14. Monorepos

The initial canonical root will be a single one:

```text
<monorepo-root>/.rationale/
```

Records will not be duplicated per package.

Every Subject and Binding may declare:

```yaml
scope:
  root: .
  includes:
    - apps/dashboard/**
    - services/api/**
    - packages/auth/**
  excludes:
    - examples/**
```

Cross-package retrieval requires a relevance path.

Example:

```text
backend authorization decision
→ API contract
→ frontend permission rendering
```

Being in the same repository is not enough.

The context compiler must apply:

- Package overlap.
- Domain relationship.
- Explicit Subject dependency.
- Structural provider relationship.
- Intent.
- Severity.
- Budget.

## 14.1 Provider limitation

The architecture will not assume that Codebase Memory resolves every workspace
perfectly.

Every cross-package relationship must include coverage and the provider
revision.

## 14.2 Real pilot

The work monorepo will be the main validation environment after dogfooding.

Before using it:

- Shared results will be anonymized.
- Sensitive code will not be copied into public datasets.
- Records with company information will carry a sensitivity.
- The evaluation may store hashes and metrics instead of content.

---

# 15. Security

## 15.1 Repository content is data

All repository text must be treated as untrusted data.

This includes:

- Names.
- Comments.
- Records.
- Issues.
- Commits.
- Paths.
- Evidence.
- Provider metadata.

Arbitrary content must never be concatenated as system instructions.

## 15.2 Sanitization

- Limit length.
- Validate UTF-8.
- Strip control characters.
- Escape formats.
- Separate metadata from instructions.
- Label untrusted content.
- Apply the schema.

## 15.3 Paths

- Canonicalize.
- Prevent traversal.
- Do not follow symlinks outside the root without a policy.
- Protect writes.
- Use temporary files and atomic rename.
- Owner-only permissions on sensitive cache.

## 15.4 Secrets

Rationale must not deliberately index:

- `.env`.
- Tokens.
- Private keys.
- Credentials.
- Dumps.
- Personal data.

It must respect:

- `.gitignore`.
- Additional configuration.
- Sensitivity.
- Redaction.

## 15.5 External skills

Every external skill must be:

- Reviewed.
- Pinned to a version or commit.
- License-checked.
- Inspected before running.
- Recorded.
- Denied global permissions by default.

---

# 16. Observability

The system must produce local structured logs.

```json
{
  "timestamp": "...",
  "event": "prepare_change.completed",
  "project_id": "...",
  "revision": "...",
  "provider_revision": "...",
  "latency_ms": 84,
  "packet_tokens": 412,
  "candidate_count": 18,
  "selected_count": 4,
  "coverage": "partial",
  "status": "ok"
}
```

By default it must not include:

- Code.
- The full prompt.
- Secrets.
- Sensitive text.
- Personal identity.

Levels:

```text
error
warn
info
debug
trace
```

Evaluation events will have a separate format.

---

# 17. Performance and resources

## 17.1 Principle

Rationale must not duplicate Codebase Memory's structural work.

Its own load must focus on:

- Reading.
- Validation.
- FTS.
- Small joins.
- Ranking.
- Policy evaluation.
- Serialization.

## 17.2 Provisional budgets

On the 16 GB MacBook Air M4:

```text
Baseline warm P95: ≤ 150 ms
Baseline hard deadline: ≤ 250 ms
Intent-aware warm P95: ≤ 2 s, excluding indexing
Steady resident memory target: ≤ 300 MB
Canonical store: proportional to Records, normally small
Derived cache: configurable and regenerable
```

These values are hypotheses.

They will be reviewed after the pilot.

## 17.3 Backpressure

- Concurrency limits.
- Cancellation.
- Timeouts.
- Query budgets.
- Queue bounds.
- Cache cap.
- No unbounded subprocess spawning.
- Per-project lock for writes.
- Concurrent reads when safe.

---

# 18. Costs

## 18.1 Mandatory core costs

Goal:

```text
Mandatory infrastructure cost: $0
```

The core must use:

- The local machine.
- Git.
- The filesystem.
- SQLite or another embedded dependency.
- Local Codebase Memory.
- Compatible open-source dependencies.

## 18.2 Uncontrolled external costs

These may exist:

- Claude Code subscription.
- Codex or OpenAI usage.
- API tokens.
- CI beyond the free tier.
- Artifact storage.
- Code signing.
- Notarization.
- Windows certificate.
- Domain.
- Landing page hosting.

They will not be dependencies of the local MVP.

## 18.3 Provisional dependency inventory

While the language is not selected, dependencies will be grouped by function.

### Required dependencies for research and initial development

| Dependency | Purpose | Rationale runtime | Mandatory cost |
|---|---|---:|---:|
| Git | Revision, history, and collaboration | Yes | $0 |
| Codebase Memory | Initial structural provider | Yes, for the initial integration | $0 |
| Xcode Command Line Tools on macOS | Compilers and base tools | Not necessarily | $0 |
| C compiler and C++ compiler | Build and study Codebase Memory | Not necessarily | $0 |
| zlib | Current Codebase Memory build | Not necessarily | $0 |
| Shell and POSIX tools | Bootstrap scripts | Development | $0 |
| Compatible MCP agent | Consume Rationale during development | External | Variable/existing |

### Likely core dependencies, not yet selected

| Capability | Expected dependency type | Constraint |
|---|---|---|
| MCP / JSON-RPC | SDK or small implementation | Must be maintainable and local |
| Derived persistence | Embedded SQLite | No server |
| Serialization | YAML, JSON, or both | Schema versioned |
| Schema validation | Local library | Deterministic errors |
| Hashing | Standard implementation | No remote service |
| File locking | Portable API | macOS, Linux, and Windows |
| Logging | Structured and local | No mandatory telemetry |
| CLI | Library or standard | Simple installation |
| Testing | Language framework | Unit, contract, property, and fuzz |
| Compression | Optional | Only if evidence justifies it |

### Evaluation dependencies

They may be used only as tooling:

- Python for statistical analysis.
- Scripts for bootstrap confidence intervals.
- NDJSON parsers.
- Charting tools.
- Local fixtures and datasets.

They will not necessarily be part of the distributed binary.

### Optional dependencies

- Ollama or another local model.
- GitHub CLI.
- Local UI.
- IDE-specific integrations.
- Signing and notarization.
- Remote CI.

No option may accidentally become a core requirement.

## 18.4 Dependency policy

Every dependency must record:

- License.
- Version.
- Size.
- Risk.
- Reason.
- Alternatives.
- Known CVEs.
- Whether it can be vendored.
- Whether it requires a runtime.
- Whether it adds external calls.

A machine-readable inventory will be kept:

```text
docs/dependencies/inventory.yaml
```

Every dependency update must:

1. Pass tests.
2. Record the change.
3. Check the license.
4. Check advisories.
5. Measure impact when it affects hot paths.
6. Be revertible.

---

# 19. Testing

## 19.1 Pyramid

```text
Unit
Contract
Integration
Property
Fuzz
Security
Performance
End-to-end
Evaluation
Cross-platform
```

## 19.2 Mandatory tests

- Schema validation.
- Atomic writes.
- Migrations.
- Subject resolution.
- novelty_reason.
- Scope filtering.
- Revision consistency.
- Provider timeout.
- Provider unavailable.
- Partial coverage.
- Token budget.
- Deduplication.
- Critical blocking predicate.
- Prompt injection sanitization.
- Path traversal.
- Concurrent reads.
- Write locks.
- Cache rebuild.
- Monorepo cross-package relevance.
- Baseline deadline.
- Context packet determinism.

## 19.3 Contract tests with Codebase Memory

Our own fixtures will be created.

They will not depend only on the upstream repository.

```text
tests/fixtures/codebase-memory/
├── simple-repo/
├── monorepo/
├── moved-symbol/
├── partial-coverage/
├── stale-index/
└── provider-unavailable/
```

## 19.4 Golden packets

For fixed inputs:

- The packet must be stable.
- The order must be deterministic.
- The budget must be respected.
- Uncertainty must be preserved.

---

# 20. Product evaluation

The architecture will include instrumentation from the first vertical slice.

It will not be added at the end.

## 20.1 Unit

The unit is:

```text
task execution
```

It includes:

- Model/agent.
- Condition.
- Initial prompt.
- Context packet.
- Tool calls.
- Revision.
- Result.
- Tests.
- Tokens.
- Latency.
- Human intervention.

## 20.2 Conditions

```text
A. Code + Git
B. Traditional documentation
C. Codebase Memory
D. Codebase Memory + Rationale
E. Expert prompt
```

## 20.3 Self-instrumentation

The agents building the project may record:

- Tools invoked.
- Files read.
- Context received.
- Errors.
- Attempts.
- Tests.
- Duration.
- Tokens, if the client exposes them.

If tokens are not available, proxies will be recorded:

- Characters.
- Words.
- Bytes.
- Tool result size.
- Number of messages.

## 20.4 Epistemic limit

The same agent that implemented a function cannot be the only entity that
scores its quality.

At least a combination of the following is required:

- Deterministic tests.
- A separate evaluator.
- Another run.
- Another model.
- Human review.
- A blind rubric.
- Predefined ground truth.

## 20.5 Success

The architecture works if it makes it possible to measure:

- Critical constraint recall.
- Context precision.
- Harmful context rate.
- Context utility density.
- Total tokens to successful completion.
- Manual context reduction.
- Bug reintroduction.
- Baseline latency.
- False blocks.
- Coverage and revision consistency.

---

# 21. Proposed repository layout

Until the language is decided:

```text
rationale/
├── README.md
├── LICENSE
├── AGENTS.md
├── Rationale_v0.5.md
├── .rationale/
├── docs/
│   ├── architecture/
│   ├── adr/
│   ├── research/
│   │   ├── codebase-memory/
│   │   └── language/
│   ├── experiments/
│   ├── environment/
│   ├── runbooks/
│   ├── security/
│   └── product/
├── schemas/
├── src/
│   ├── application/
│   ├── configuration/
│   ├── project/
│   ├── revision/
│   ├── providers/
│   ├── storage/
│   ├── subjects/
│   ├── bindings/
│   ├── policy/
│   ├── retrieval/
│   ├── capture/
│   └── evaluation/
├── tests/
│   ├── unit/
│   ├── contract/
│   ├── integration/
│   ├── security/
│   ├── performance/
│   ├── end-to-end/
│   └── evaluation/
├── fixtures/
├── scripts/
│   ├── dev/
│   ├── ci/
│   ├── research/
│   └── release/
├── tools/
└── .rationale-local/      # ignored
```

The concrete layout will adapt to the approved language.

---

# 22. Mandatory initial ADRs

```text
ADR-0001 Core language and toolchain
ADR-0002 Codebase Memory transport
ADR-0003 Canonical serialization
ADR-0004 Derived database
ADR-0005 Cache root and project identity
ADR-0006 Revision fingerprint
ADR-0007 MCP SDK and protocol version
ADR-0008 Concurrency and locking
ADR-0009 Baseline integration surfaces
ADR-0010 Packaging strategy
ADR-0011 Licensing and dependency policy
ADR-0012 Telemetry and privacy
```

No ADR may say only:

```text
We chose X because it is fast.
```

It must contain evidence.

---

# 23. Implementation phases

## Phase A — Repository bootstrap

- Create the repo.
- Copy the documents.
- Create AGENTS.md.
- Create the docs structure.
- Create templates.
- Configure Git.
- Configure Codebase Memory.
- Capture the environment.
- Do not choose the language yet.

## Phase B — Upstream analysis

- Clone Codebase Memory.
- Build.
- Tests.
- Index itself.
- Index Rationale.
- Document contracts.
- Measure CLI vs MCP.
- Analyze monorepos.
- Produce a recommendation.

## Phase C — Language spike

- Implement prototypes.
- Measure.
- ADR-0001.
- Create the toolchain.

## Phase D — Vertical slice

It must:

```text
init
read canonical record
resolve exact target
query Codebase Memory
check revisions
return one compact constraint
```

It must not include the whole vision.

## Phase E — Local store and compiler

- Subjects.
- Records.
- Bindings.
- Approvals.
- Assessments.
- FTS.
- Budget.
- Packets.

## Phase F — Capture

- Signals.
- Diff.
- Tests.
- finalize_change.
- Confirmation.

## Phase G — Dogfood

Rationale will be installed in Rationale.

Its own Records will be used to build it.

The tool will not be able to automatically approve its foundational decisions.

## Phase H — Monorepo pilot

- Install it in a real project.
- Select 20–30 historical changes.
- Run the conditions.
- Measure.
- Correct.

## Phase I — Architecture 0.2

After evidence:

- Update modules.
- Close decisions.
- Remove unnecessary components.
- Stabilize internal APIs.

## Phase J — Packaging

Only after validation:

- macOS arm64.
- macOS amd64, if kept.
- Linux amd64.
- Linux arm64.
- Windows amd64.
- Checksums.
- Install.
- Update.
- Uninstall.
- Rollback.

## Phase K — Distribution experience

- User documentation.
- Quick start.
- Troubleshooting.
- Security.
- Landing page.
- Releases.

---

# 24. Conceptual installation

## Development

```text
clone rationale
install toolchain
install Codebase Memory
index project
run health
run tests
start agent
```

## Consumer project

The desired final experience:

```text
install rationale
cd project
rationale init
rationale provider add codebase-memory
rationale install-agent
rationale health
```

The commands are conceptual.

They will not be implemented until the CLI is defined.

## Modified files

The installer must record exactly:

- Binary.
- Config.
- Hooks.
- Agent entries.
- Skills.
- Cache.
- PATH changes.

Uninstall must be able to revert what it installed.

---

# 25. Traceability to Rationale 0.5

| Conceptual requirement | Component |
|---|---|
| Causal context | Canonical Store + Context Compiler |
| Epistemic humility | Trust Evaluator |
| Authority | Approval + Policy Evaluator |
| Revision | Revision Coordinator |
| Concept-first | Subject Resolver |
| Code-anchored | Binding Resolver |
| Monorepo | Workspace Discovery + Scope |
| Context budget | Context Compiler |
| Baseline | Fast path |
| Intent-aware | prepare_change |
| Cold start | Capture + partial coverage |
| Legacy archaeology | Future provider/evidence workflow |
| No false block | Blocking predicate |
| Prompt injection | Security boundary |
| Shared project memory | `.rationale/` |
| Local machine state | Derived Index |
| Agent forgetfulness | Hooks + baseline + explicit preflight |
| Human commits | Revision gate |
| Deduplication | Subject Resolver + novelty_reason |
| Token savings | Evaluation instrumentation |
| Senior continuity | Records + evaluation |
| Progressive adoption | Optional levels and forward capture |
| Provider fallibility | Coverage + capability negotiation |
| No mandatory cloud | Local-first |
| Cross-platform | Packaging phase |
| Team collaboration | Git canonical layer |
| Metrics | Evaluation module |

---

# 26. Exit criteria for architecture 0.1

Architecture 0.1 may be considered ready to implement when:

- Conceptual document 0.5 is versioned.
- The Codebase Memory repository has been cloned and pinned.
- The upstream build works on the MacBook Air M4.
- Its relevant tests have been run.
- Codebase Memory has indexed its own repo.
- Codebase Memory has indexed Rationale.
- MCP vs CLI has been documented.
- Revision and coverage have been documented.
- Monorepo limits have been documented.
- The language spikes are complete.
- ADR-0001 has been approved.
- A planned vertical slice exists.
- An instrumentation schema exists.
- A security baseline exists.
- An agent process exists.
- There is no mandatory paid dependency.

---

# 27. What no agent may do

- Start the landing page.
- Choose the language without an ADR.
- Copy Codebase Memory internals.
- Read its private SQLite as a contract.
- Introduce a SaaS.
- Add mandatory remote embeddings.
- Create twenty services.
- Create a daemon before measuring the need.
- Block changes based on inferences.
- Automatically approve Records.
- Hide partial coverage.
- Declare success using only the opinion of the same agent.
- Skip documentation.
- Change the architecture without an ADR.
- Package before validating the core.
- Optimize before instrumenting.

---

# 28. Open questions

- Internal MCP client or CLI subprocess?
- One process per session or a shared daemon?
- What exact revision does Codebase Memory offer?
- How does it represent the working tree?
- Which capability is missing?
- Rust, Go, C, or something else?
- YAML, JSON, or a combination?
- Plain SQLite or an abstraction?
- How is the project ID computed?
- How are several agents coordinated?
- Which hooks does each client support?
- Which token metrics are accessible?
- How are releases signed?
- Which part of the cache can be shared?
- How is Windows tested before packaging?
- Which Records are sensitive?
- What minimal packet is convincing in the pilot?

These questions are part of the architecture.

They are not a sign that work is missing.

They are the work list that avoids pretending certainty.

---

# 29. Definition of architecture 0.1

> Rationale will initially be a local, modular, auditable system that exposes
> MCP and a CLI, keeps a canonical memory versioned in Git, derives a
> regenerable local index, coordinates revisions between Git and structural
> providers, and compiles decisions, constraints, and risks into small context
> packets. Codebase Memory will be its first structural provider, integrated
> only through verified public contracts. The language, the internal transport,
> the daemon, and the packaging will be decided after reproducible research.
> The tool will be instrumented from its first vertical slice and validated
> first on itself and then on a real monorepo before being distributed for
> macOS, Linux, and Windows.

---

# 30. Conclusion

Architecture 0.1 does not try to impress with complexity.

It tries to protect the project from poorly grounded early decisions.

The first technical responsibility is to understand the system Rationale will
depend on.

The second is to build the minimal path between:

```text
target
→ structure
→ decision
→ constraint
→ useful context
```

The third is to measure whether that path actually helps.

Only then is it time to turn it into a distributable product.
