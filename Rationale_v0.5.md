# Rationale

## Causal context and trust layer for coding agents

### Founding document, product contract, and development plan

**Conceptual version:** 0.5\
**Status:** near-final design prior to implementation and experimental validation\
**Guiding principle:** compile the project's trustworthy knowledge into the minimum sufficient context a task needs; measure whether it improves real results; first demonstrate value, then stabilize the architecture and formalize the protocol.

> **Status (1.0):** this is the founding contract Rationale was built from, and
> other documents cite its sections (`Rationale_v0.5.md §N`). The problem, the
> principles, and the trust model still hold. The operating model changed in
> 1.0: `finalize_change` writes Records directly through a capture gate instead
> of leaving proposals for review, people keep authority by pinning Records and
> deciding conflicts, and Records can explain relationships between symbols.
> Read [`README.md`](README.md), [`docs/user-guide/concepts.md`](docs/user-guide/concepts.md),
> and [`docs/work-items/vnext-implementation-plan.md`](docs/work-items/vnext-implementation-plan.md)
> for what ships today.

---

# 1. Project summary


**Rationale** is an open-source, local, queryable tool that preserves the causal context of important software decisions and delivers it to coding agents before they modify the related parts of a system.

Its purpose is not only to explain what the code does or how it is connected.

Its purpose is to preserve and retrieve information such as:

* Why a section of code exists.
* What problem motivated its creation.
* What behavior it was meant to achieve.
* What decisions were made.
* What constraints must be kept.
* What risks were already discovered.
* What alternatives were discarded.
* How the change was verified to work.
* What parts of the system depend on that decision.
* What future changes could make that knowledge obsolete.
* Who stated or approved a claim.
* What authority that person had to establish it.
* In which repository revision it was evaluated.
* What quality and coverage the structural evidence used had.

Rationale will initially integrate with **Codebase Memory MCP**, using it as its structural intelligence engine.

Codebase Memory makes it possible to discover:

* Functions.
* Classes.
* Routes.
* Calls.
* Dependencies.
* Flows.
* Symbols.
* Structural changes.
* Impact on other areas of the code.
* Relationships between packages, services, and repositories when the provider has enough coverage.

Rationale will add a different layer:

* Intent.
* Causality.
* Normative decisions.
* Constraints.
* Risks.
* Evidence.
* Provenance.
* Authority.
* Epistemic trust.
* Validity.
* Per-revision consistency.
* Consequences.

The relationship between the two can be summarized like this:

> **Codebase Memory understands the current structure of the code.\
> Rationale keeps which decisions still govern that structure, why they exist, and why they should be considered trustworthy.**

Rationale does not aim to remember absolutely everything.

Its job is to retrieve only the knowledge that:

1. Is still trustworthy.
2. Has known provenance.
3. Was approved by the appropriate authority when it is normative.
4. Still applies to the current revision.
5. Is relevant to the intent and scope of the change.
6. Is backed by evidence whose coverage is known.
7. Fits within a reasonable context budget.

That is why the project's central phrases will be:

> **Git remembers what changed. Rationale remembers why it still matters.**

> **Rationale does not remember everything. It remembers what still matters.**

The main unit of value will not be a large database or a perfect graph.

It will be a **software decision preflight**:

> Before modifying code, the agent receives the constraints, decisions, and risks in force that govern that area, together with their provenance, authority, evidence, revision, and uncertainty.

Rationale must also be understood as a **project context compiler**. It does not replace the task prompt or hand over all available memory. It takes an intent when one exists, some targets, and a snapshot of the repository, and compiles a small packet with the most useful institutional knowledge for that operation.

```text
Shared project memory
        +
Current intent and symptoms
        +
Observed structure
        +
Context budget
        ↓
Task-specific context packet
```

The goal is not for the agent to receive more text. The goal is for it to receive **more relevant, trustworthy, actionable, and current context per token used**.

## 1.1 Initial validation of the problem

The problem is real: modern agents can reconstruct structure, calls, and dependencies, but they cannot know with certainty reasons that were never encoded. The Codebase Memory ecosystem itself already shows interest in ADRs, per-symbol history, and architectural drift, which validates the need; at the same time, it forces Rationale to have a clear boundary so it does not duplicate structural or historical features that the provider may add.

Rationale's differentiation will not simply be "storing more detailed ADRs".

It will be the combination of:

```text
Normative decisions
        +
Provenance and authority
        +
Validity per revision
        +
Structural anchoring
        +
Comparison against the intent
        +
Compact delivery before the change
```

## 1.2 The fundamental questions

```text
What?       Git: exact result and modified lines.
Where?      Codebase Memory: location, symbols, and connections.
When?       Git: history; Rationale: validity and evaluated revision.
Who?        Git identifies authors; Rationale identifies provenance and authority.
How?        Codebase Memory explains the current mechanics; the agent designs the implementation.
Why?        Rationale keeps the causal origin and the evidence.
What for?   Rationale keeps the intent, constraints, and invariants that must survive.
```

In the construction analogy:

* Git keeps the bricks laid and the site logbook.
* Codebase Memory keeps the current plans of pipes, cables, and connections.
* Rationale keeps the soil study, the engineer's decisions, the safety constraints, and the revision in which those conclusions were verified.

## 1.3 Landscape of related solutions

Rationale does not start from empty ground. Several families of nearby tools exist:

| Family or example | What it solves well | Gap that remains |
|---|---|---|
| Codebase Memory | Structural graph, symbols, calls, routes, impact, and local search | It must not be assumed that a structural relationship explains the normative decision or its authority |
| ADR / MADR | Explicit preservation of architectural decisions | They usually do not keep per-revision bindings, current applicability, or comparison against a concrete intent |
| PROJECTMEM and event-based memories | Events, decisions, and gates before actions | How much they model authority, structural evidence, and exact consistency with Git must be evaluated |
| Packmind and standards systems | Distribution of engineering rules and standards to agents | Rules may not keep the causal origin, the revision, or the link to real behavior |
| Hermes Agent and procedural memories | Learning an agent's habits, skills, and preferences | An agent's general memory is not the same as portable causal governance of the repository |

Hermes is probably the tool remembered as "Hermes or Hercules". Its approach can inspire habit capture and procedural learning, but Rationale must avoid becoming an agent's personal memory.

Rationale's competitive boundary will be:

```text
Not remembering how an agent works in general.
Not re-indexing the code.
Not limiting itself to storing ADRs.

Yes to keeping which approved decision governs a behavior,
why it exists, what evidence backs it, and in which revision it applies.
```

## 1.4 Product hypotheses

Problem hypothesis:

> In complex repositories, a significant part of agents' risk comes from correctly reconstructing the how, but not knowing a causal or normative constraint that is not visible in the code.

Solution hypothesis:

> A compact preflight, linked to the current structure and limited to approved knowledge, reduces regressions and repeated archaeology without adding an excessive human burden.

These hypotheses must be validated experimentally before the architecture or the protocol is declared stable.

## 1.5 Conceptual decisions of version 0.4

Version 0.4 incorporates four problems from real development, but corrects several excessive interpretations of the feedback received.

### Accepted: scope must cross packages and workspaces

A decision can originate in one package and govern others. An authorization rule implemented in the backend can affect an API contract, the components the dashboard allows to be shown, and the integration tests.

However, in a monorepo there is not necessarily "one `.rationale/` folder per package". A monorepo is still one Git repository. The real problem is a different one:

> **Rationale needs project identity, hierarchical scopes, workspace-qualified bindings, and retrieval able to cross packages without contaminating the context with irrelevant rules.**

v0.1 must support one Git repository with multiple workspaces or packages. Full federation across independent repositories will remain for later.

### Accepted with limits: strict Subject resolution

Before creating a new Subject, Rationale must look for existing concepts through IDs, aliases, bindings, scope, FTS, and, optionally, local semantic similarity.

Embeddings may produce candidates. They can never:

* Merge Subjects automatically.
* Declare that two rules are equivalent.
* Turn textual similarity into conceptual identity.
* Block the creation of a concept without a review path.

When there is a strong match and the agent proposes a new Subject, it must provide an explicit `novelty_reason`.

### Accepted with limits: passive drift detection

Rationale must detect that Git moved forward even if nobody ran `finalize_change`. The minimum guarantee does not depend on a daemon:

* Every query compares the current `HEAD` with the last processed revision.
* If they do not match, it degrades the related bindings and assessments.
* It never silently serves an old evaluation as current.

Git hooks or a local process can speed up this detection, but they are optional optimizations. Hooks can be omitted, are not cloned automatically, and can be skipped. That is why they are not the correctness boundary.

### Accepted with limits: not depending on the agent remembering a tool

MCP alone cannot universally intercept every edit or know an intent the agent never expressed. The solution will be layered:

1. **Baseline target context:** when reading, searching, or editing a target, compatible integrations may inject a minimal packet with linked critical constraints.
2. **Intent-aware preflight:** when there is an explicit intent, `prepare_change` compares that intent against active decisions and delivers richer context.
3. **Post-change audit:** when finalizing or reviewing a diff, Rationale detects possible violations even if the preflight was skipped.
4. **Policy enforcement:** CI only blocks critical, deterministic, approved rules evaluated at a coherent revision.

Automatic injection improves coverage, but it does not replace the explicit contract of `prepare_change`.

### Important correction: more context does not always mean better context

A precise description of the bug, its reproduction, symptoms, and scope usually improves the solution a lot. But adding irrelevant text or placing critical knowledge inside an enormous context can reduce the model's ability to use it.

Rationale adopts this principle:

```text
context_utility_density =
    relevance
  × reliability
  × actionability
  × freshness
  ÷ tokens
```

The goal is to maximize utility, not volume.

### Realistic goal: senior continuity, not replacement of a senior person

Rationale can preserve:

* Earlier decisions.
* Invariants.
* Historical incidents and bugs.
* Discarded alternatives.
* Structural relationships.
* Known validations.

That enables a continuity similar to a senior person's technical memory. It does not replace:

* Product judgment.
* Business prioritization.
* Negotiation between teams.
* Human authority.
* Knowledge that was never expressed or evidenced.

The right promise is to **preserve actionable institutional memory**, not to fabricate complete human experience.

## 1.6 Conceptual decisions of version 0.5

Version 0.5 does not change Rationale's central identity. It turns several aspirations from 0.4 into measurable hypotheses and adds two operational limits needed before starting to build.

### Accepted: utility density must be falsifiable

`context_utility_density` will remain a design principle, but it will not be used as a self-referential score with which Rationale declares itself successful.

It must be evaluated on the **exact Context Packet** delivered for a task and checked against:

* A ground truth prepared for the case.
* The real result of the task.
* The total tokens until it was completed correctly.
* The constraints respected or omitted.
* The falsehoods included.
* The manual context the person still had to write.

A high packet score does not make up for an incorrect solution.

### Accepted with limits: `novelty_reason` must be structured

A free-form justification such as:

```text
This Subject is different.
```

does not demonstrate conceptual novelty.

When a similar candidate exists, the proposal must explicitly contrast:

* The existing Subject.
* The difference in behavior, scope, lifecycle, authority, or invariant.
* The evidence that backs that difference.

The tool must reject generic or circular reasons. However, high similarity does not make the candidate identical either, nor does it authorize an automatic merge.

### Accepted: baseline needs a separate fast path

The high-frequency baseline must not run Rationale's full pipeline.

It must use a local, precomputed, bounded, non-blocking path to retrieve only critical constraints and consistency warnings that are already indexed.

Intent analysis, archaeology, embeddings, LLM classification, and relationship reconstruction belong to the full preflight, not to every read or search.

### Correction: latency and density are end-to-end properties

It is not enough to measure how many milliseconds an isolated query takes or how many tokens `prepare_change` returns.

Rationale can be fast locally and still add enough calls or clarifications to make the overall flow worse. It can also deliver few tokens but omit the only critical constraint.

That is why the pilot will measure simultaneously:

```text
Context Packet quality
        +
Retrieval latency
        +
Tokens and calls until a correct solution
        +
Regressions and constraints respected
        +
Manual context required
```

### Condition for moving to a stable implementation

0.5 is considered near-final in concept, not validated in results.

Before stabilizing the architecture, experiment 0.0 must produce evidence that Rationale improves at least one material combination of:

* Task success.
* Recall of critical constraints.
* Prevention of historical regressions.
* Total tokens.
* Tool calls.
* Human time preparing the prompt.
* Time to a correct solution.

If it does not, the product, the retrieval, or even the central hypothesis will have to be revised.

---

# 2. The problem

When an artificial intelligence works on a repository, it can reconstruct a large part of how it currently works.

It can discover that:

* One function calls another.
* A route modifies a table.
* A controller validates a permission.
* A class implements an interface.
* A change affects multiple symbols.
* A service depends on another.

However, reading the current code does not guarantee understanding why it ended up implemented that way.

An AI may find this code:

```ts
if (entityAssignment) {
  return resolveEntityPermissions(entityAssignment);
}

return denyAccess();
```

It can deduce that access depends on a per-entity assignment.

But it probably cannot deduce with certainty that:

* Previously, users with access to several entities received global `super_admin`.
* This granted unnecessary privileges to support accounts.
* It was decided that having access to several entities must not imply global administration.
* Some users are deliberate exceptions because they own the project.
* A user without assignments must be left without access.
* Restoring the global role would reintroduce the original problem.
* The migration was designed to be reversible.
* The change had to be idempotent.
* The decision came after observing a real risk of excessive privileges.

That knowledge usually ends up scattered across:

* Conversations with agents.
* Internal chats.
* Pull requests.
* Issues.
* Meetings.
* Temporary comments.
* Commit messages.
* Incidents.
* Tests.
* Developers' memory.

After several months, the code remains, but its causal context is lost.

This produces the problem known as **Chesterton's Fence**:

> Before removing or modifying a structure, you need to understand why it was built.

The AI can understand the fence.

What it does not necessarily understand is why someone decided to build it.

---

# 3. The real challenge


Rationale's main challenge is not storing text.

Saving explanations in YAML files or in a database is relatively simple.

The hard problems are:

* Knowing whether an explanation is true.
* Distinguishing facts, human claims, and inferences.
* Knowing who had the authority to turn a claim into a rule.
* Detecting contradictions between humans, teams, and later decisions.
* Detecting when a decision stopped applying without confusing structural change with conceptual change.
* Keeping links even when the code is refactored.
* Avoiding constant alerts.
* Not forcing developers to document every change.
* Adopting the tool in old projects.
* Avoiding flooding the agent with too much context.
* Deciding which knowledge matters for a concrete intent.
* Avoiding answers built on incompatible revisions.
* Not treating the structural provider's absences or errors as truth.
* Keeping text records from becoming prompt injection.
* Preventing secrets or sensitive information from ending up versioned.
* Guaranteeing that the agent consults Rationale when it really matters.
* Proving that the tool reduces the total cost of solving a task, not just the size of a response.
* Resolving scopes and inheritance in monorepos without injecting policies from unrelated packages.
* Detecting human commits and Git progress even when the assisted flow was skipped.
* Avoiding duplicate Subjects without handing conceptual identity over to a semantic heuristic.
* Sharing the canonical memory through Git while each machine keeps different local indexes and coverage.
* Delivering critical constraints even when the agent skips the preflight, without pretending the intent can always be inferred.
* Distinguishing sufficient context from indiscriminate accumulation of context.

There is also an unavoidable limitation:

```text
The true why cannot always be deduced from the code.
Humans do not want to document every change.
Automatic inferences can be false.
```

Rationale cannot eliminate this contradiction. It must be designed around it.

The right answer is to accept **partial, intentional coverage**:

* Capture going forward.
* Prioritize high-risk areas.
* Allow an unknown reason.
* Request confirmation only for important normative deltas.
* Retrieve old history only on demand.
* Never present partial coverage as complete knowledge.

Therefore, Rationale must not be defined only as a memory of the why.

It must be defined as:

> **A local layer of causal context, provenance, authority, and validity control that connects decisions, constraints, risks, and evidence with the real behaviors of a system, and retrieves only what is still relevant and trustworthy for a specific change.**

The goal is not to build a perfect memory.

The goal is to keep an important decision from being destroyed because the agent could only observe its current implementation.

---

# 4. Fundamental principles


## 4.1 Having no explanation is better than keeping a false explanation

An inference generated by an AI must never silently become a fact.

If an agent observes a limit of 50 items and deduces that it exists for performance, but the real reason is a commercial constraint of an external API, storing the wrong explanation would be more dangerous than admitting that the reason is unknown.

Rationale must be able to answer:

```text
Confirmed reason: unknown.

Hypothesis:
The limit could be related to performance.

Confidence: low.
Not confirmed by a person or by direct evidence.
```

It must never answer:

```text
The limit exists for performance.
```

if that was not demonstrated.

---

## 4.2 A structural change does not imply a conceptual change

A function can:

* Be renamed.
* Move to another file.
* Be split.
* Be extracted into a class.
* Become a service.
* Be rewritten in another language.

And still keep implementing exactly the same decision.

Rationale must distinguish:

```text
The code changed.
```

from:

```text
The decision stopped being applicable.
```

It must not mark a decision as obsolete only because a function's fingerprint changed.

---

## 4.3 Knowledge belongs first to a concept, not to a file

The primary identity of a decision must not be:

```yaml
path: src/auth/authorization.ts
symbol: resolveEntityRole
```

The primary identity must be the behavior it represents:

```yaml
subject:
  type: system-behavior
  id: authorization.entity-scoped-staff-access
```

Files, functions, tables, and routes will be anchors of the current implementation.

This lets the decision survive refactors and architectural migrations.

However, a conceptual ID is not magically stable either. Rationale must support explicit identity operations:

```text
alias
rename
merge
split
scope-narrowed
scope-expanded
supersede
```

The tool may propose lineage, but it must not promise perfect automatic conceptual reconstruction.

---

## 4.4 Capture must have low friction

Developers must not fill in giant forms or approve complete documents after every change.

Rationale must automatically obtain the verifiable data:

* Modified files.
* Modified symbols.
* Added dependencies.
* Removed dependencies.
* Tests run.
* Related commits.
* Affected routes.
* Schema changes.

Human participation must be limited to confirming the claims that a tool cannot know on its own:

* Why the change was made.
* What decision was taken.
* What alternative was discarded.
* What behavior must never break.
* What risk is not directly visible in the code.
* Who has the authority to approve the rule.

A useful question:

```text
I detected a possible normative constraint:
"Multi-entity access must not imply global administration."

Should it be kept as an approved rule of the system?
```

A useless question:

```text
Do you want to document this change?
```

It must always be valid to answer:

```text
Unknown reason.
Do not create a constraint.
```

---

## 4.5 The system must be adopted progressively

Rationale does not need to know the full history of a repository to be useful.

When installed in an old project, it must accept:

```text
The reason for much of the code is still unknown.

From today on, important changes will start keeping their context.

Historical knowledge will be retrieved only when needed.
```

The tool must start generating value from the first new change.

The goal is not 100% coverage.

The goal is sufficient coverage in the areas where losing context has serious consequences.

---

## 4.6 Retrieval must respect a context budget

An agent must not receive fifteen complete historical records before modifying a central function.

Rationale must build prioritized, token-limited responses.

It must deliver first:

1. Approved critical constraints.
2. Conflicts with the current intent.
3. The decision in force.
4. Directly relevant risks.
5. Quality and revision of the evidence.
6. Main structural connections.

The rest of the history must remain available through additional queries.

---

## 4.7 Provenance and authority are different dimensions

Knowing that a claim was stated by a person does not indicate that the person had the authority to turn it into policy.

Rationale must separate:

```text
Provenance: who or what produced the claim.
Authority: what capacity it had to approve it within that domain.
```

A developer can correctly describe what they believe happens and still not be the product, security, or architecture owner who can establish the rule.

A critical constraint must not be considered approved only because it has `human-confirmed`.

It must have an explicit approval policy.

---

## 4.8 Every response must belong to a coherent revision

Rationale must never build a seemingly current response using:

```text
Git HEAD: revision C
Codebase Memory index: revision B
Rationale evaluation: revision A
```

Every packet must declare:

* Git revision.
* Structural provider revision or generation.
* Records revision.
* Revision of the last applicability evaluation.
* Consistency status.

If there is no coherence, the tool must degrade or reject the response instead of serving plausible but incorrect context.

---

## 4.9 Structural evidence is fallible

Codebase Memory is a valuable provider, not an oracle.

A missing relationship can mean:

* That it does not exist.
* That the index is behind.
* That the language or framework was not resolved.
* That the code uses reflection or dynamic configuration.
* That a folder was ignored.
* That there was a provider error.

Rationale must distinguish:

```text
No relationship was found.
```

from:

```text
It was verified that the relationship does not exist.
```

The second claim will require much stronger evidence.

---

## 4.10 Stored text is data, not instruction

Records, evidence, ADRs, issues, and conversations can contain malicious or accidentally instructive content.

Rationale must treat them as non-executable data.

It must never allow a record to modify:

* The system instructions.
* The agent's permissions.
* The security policies.
* The scope of external tools.
* The obligation to protect secrets.

Critical constraints must prefer a structured declarative representation and use free text as explanation, not as a program.

---

## 4.11 Sensitive data must be minimized and classified

The why can include vulnerabilities, customer names, contracts, incidents, costs, or private information.

Rationale must apply:

* Minimization.
* External references when possible.
* Sensitivity classification.
* Visibility controls.
* Secret redaction.
* Export policies.

It must not copy complete conversations into the repository for convenience.

---

## 4.12 Conversation is the main interface, not the only mechanism

The agent can forget to call an MCP tool.

That is why Rationale must offer progressive layers:

```text
Informative: voluntary query by the agent.
Assisted: client instructions to run the preflight.
Review: analysis of the diff when finishing.
Policy: CI only for critical, deterministic, approved rules.
```

The human experience can live mainly in the IDE chat, but critical cases need a verifiable mechanism beyond the model's goodwill.

---

## 4.13 Savings must be measured end to end

Rationale consumes calls and tokens of its own.

It must not promise that every query will be cheaper.

Real savings appear when it avoids:

* Reading dozens of files.
* Repeating historical archaeology.
* Proposing an incompatible architecture.
* Reintroducing an incident.
* Running several failed attempts.

The right metric is the total cost until the task is solved, not only the size of the context packet.

## 4.14 Rationale compiles context; it does not dump memory

The user's prompt will keep describing the current task:

* What is to be achieved.
* What bug is observed.
* How it is reproduced.
* What behavior was expected.
* What temporary constraints exist.

Rationale contributes what the user should not have to repeat:

* Historical decisions.
* Business rules.
* Known risks.
* Earlier incidents.
* Contracts between packages.
* Validations that must be repeated.

The combination forms the effective context. Rationale does not turn an ambiguous prompt into a perfect specification.

## 4.15 Project identity is not identical to the folder or the repository

Rationale will use a logical project identity. In the first implementation it will normally correspond to a Git repository, but the model will distinguish:

```text
Project
└── Repository
    └── Workspace / Package / Service
        └── Component / Domain
            └── Target
```

This allows a project Subject to govern several packages, while a local record stays limited to a workspace or component.

## 4.16 Freshness is actively checked at every read boundary

A daemon can fail and a hook may not run. That is why, before returning knowledge as current, Rationale must compare the observed revision with the evaluated revision.

Passive detection is a latency improvement. The check at query time is the correctness guarantee.

## 4.17 Conceptual identity is resolved deterministically before using semantics

Creating Subjects will not be a free write by the agent. It will go through a resolver that queries:

1. Exact ID.
2. Exact and normalized aliases.
3. Shared bindings.
4. Parent domain and scope.
5. FTS text match.
6. Optional semantic similarity.

Similarity only widens the candidates. The final decision remains explicit and auditable.

## 4.18 The preflight has two modes

```text
Baseline mode:
Known targets, intent absent or incomplete.
Delivers critical constraints and directly linked risks.

Intent-aware mode:
Targets + intent + symptoms or reproduction.
Compares the proposed change against decisions and precedents.
```

This reduces the single point of failure of the forgetful agent without claiming that Rationale knows an intent that does not exist.

## 4.19 Shared memory and local state are different layers

Rationale will distinguish:

```text
Shared canonical layer
- Subjects
- Records
- Binding declarations
- Approvals
- Supersession events
- Portable evidence references

Local derived layer
- Binding resolutions
- Provider coverage
- Recomputable assessments
- FTS / embeddings / caches
- Working-tree overlays

Ephemeral session layer
- Hypotheses
- Temporary discoveries
- Current intent
- Tool traces
```

A team shares canonical knowledge through Git. Each computer rebuilds the structural representation and declares its own coverage.

---

# 5. Exact product definition


Rationale is a local, structured, queryable tool that records the consolidated reasoning behind important software changes and turns it into a **decision preflight** before future changes.

Its main unit is still a **Rationale Record**, but the system must not mix into a single mutable object everything that happened and everything that is believed about it today.

v1 will distinguish six main entities:

## 5.1 `Subject`

Represents the governed behavior or conceptual domain.

```yaml
id: authorization.entity-scoped-staff-access
type: system-behavior
title: Entity-scoped staff authorization
scope: project
aliases:
  - auth.staff-per-entity
applies_to:
  - workspace:apps/api
  - workspace:apps/dashboard
```

## 5.2 `Record`

Represents a consolidated decision, constraint, risk, or piece of operational knowledge.

```yaml
id: constraint.no-global-admin-for-staff
kind: constraint
statement: Staff users must not receive global super_admin.
rationale: Multi-entity access must not imply global administration.
severity: critical
scope:
  subjects:
    - authorization.entity-scoped-staff-access
```

A record can contain:

* Problem.
* Intent.
* Decision.
* Constraint.
* Risk.
* Alternatives.
* Non-goals.
* Consequences.
* Validations.
* Review conditions.

In v1, these elements will be structured fields inside the record. They will only become independent nodes when a real query justifies that complexity.

## 5.3 `Binding`

Relates the concept to an expected or known implementation. It must separate the portable declaration from its local resolution.

Shared canonical declaration:

```yaml
id: binding.authorization.resolve-entity-role
subject_id: authorization.entity-scoped-staff-access
provider: codebase-memory
structural_id: function:typescript:auth.resolveEntityRole
path_hint: apps/api/src/auth/authorization.ts
scope: package:npm:@boost/api
```

Derived local resolution:

```yaml
binding_id: binding.authorization.resolve-entity-role
provider_version: 0.9.x
resolved_revision: def456
provider_generation: 184
coverage: complete
status: current
```

The declaration makes it possible to share the conceptual anchor. The resolution expresses what one computer and one concrete provider version were able to verify.

## 5.4 `Evidence`

Describes verifiable or referenced evidence.

```yaml
type: migration
revision: 91ac21f
path: database/migrations/remove_staff_super_admin.sql
content_hash: sha256:...
visibility: repository
```

## 5.5 `Approval`

Describes who approved a normative claim and with what authority.

```yaml
actor: user:security-owner
authority: domain-owner
domain: authorization
approved_at: 2026-07-23
policy: codeowners
```

## 5.6 `Assessment`

Describes a mutable evaluation of the relationship between a record and the current system.

```yaml
record_id: constraint.no-global-admin-for-staff
applicability: active
linkage: current
assessed_revision: def456
provider_generation: 184
assessment_reason: implementation-still-enforces-entity-assignments
```

This separation is fundamental:

```text
Record = what the project decided.
Assessment = what Rationale can claim today about its validity and linkage.
```

The main unit is not:

* A file.
* A commit.
* A conversation.
* An AI-generated summary.
* A free-form document.
* An embedding.
* Codebase Memory output taken as absolute truth.

The system must answer:

> What did the project learn during this change, who had the authority to approve it, what evidence backs it, and which part of that learning still governs the current revision?

## 5.7 `ProjectScope` and qualified references

`ProjectScope` will be a value object, not a seventh independently persisted entity. It defines where a Subject or Record governs.

Initial hierarchy:

```text
project
workspace:<path-or-name>
package:<manager>:<name>
service:<id>
domain:<id>
target:<provider>:<structural-id>
```

Example:

```yaml
project_id: boost
repository_id: git:sha256:...

scope:
  kind: project

applies_to:
  - package:npm:@boost/api
  - package:npm:@boost/dashboard

excludes:
  - package:npm:@boost/legacy-admin
```

Conceptual rules:

* Project records can be inherited by child scopes.
* Inheritance never implies automatic inclusion in the context packet; structural relevance or a conflict with the intent must still exist.
* A local record is not promoted to project level by textual similarity.
* A binding always includes project, repository, workspace, or package when available.
* Dependencies between packages can propagate relevance, but they must keep the path that explains why the record was included.
* Federation across several repositories will use qualified references in the future, without changing the identity of current Subjects.

## 5.8 v1 product contract

Before the change, Rationale must:

1. Receive targets, revision, and budget; plus intent, symptoms, or reproduction when available.
2. Verify coherence between Git, the structural provider, and the evaluations.
3. Resolve the affected subjects.
4. Retrieve approved decisions and constraints.
5. Compare the intent against them.
6. Deliver a compact packet with explicit uncertainty.

After the change, Rationale must:

1. Receive the diff and the validations.
2. Store mechanical facts.
3. Propose only new normative deltas.
4. Request concrete confirmations.
5. Update bindings and assessments.
6. Keep supersessions and lineage without rewriting history.
7. Detect whether Git moved forward outside the flow and degrade the affected assessments.
8. Resolve existing Subjects before allowing a new one to be created.
9. Share canonical declarations through Git without necessarily sharing local indexes.

---

# 6. Questions it must answer

## 6.1 About existing code

* Why does this function exist?
* What problem gave rise to this behavior?
* Is this condition deliberate?
* What decision governs this module?
* What constraints must I preserve?
* What risk was already discovered?
* What tests protect this behavior?
* What parts of the system conceptually depend on this?
* What could break even if the code compiles?
* What information is still unknown?

## 6.2 Before making a change

* Does the intent contradict a decision in force?
* Am I trying to restore a removed behavior?
* What invariants must I preserve?
* What contracts may change?
* What risks are related to this intent?
* What validations should be repeated?
* What other components are involved?
* What knowledge is trustworthy and what part is inferred?

## 6.3 About the validity of knowledge

* Is this decision still active?
* Did the implementation move?
* Did the link to the code degrade?
* Did the behavior change or only the structure?
* What evidence is still valid?
* Is there a later decision that replaces it?
* Are there contradictory records?
* What needs human review?

## 6.4 About history

* What change introduced this constraint?
* What incident motivated the decision?
* What alternatives were discarded?
* What earlier attempts failed?
* What unexpected consequence was discovered?
* When was it last validated?

---

# 7. What Rationale is not


## 7.1 It is not another code indexer

Rationale must not reimplement everything Codebase Memory already solves.

It does not need to rebuild on its own:

* Complete ASTs.
* Call graphs.
* Symbol resolution.
* Structural search.
* Change impact.
* Relationships between services.
* General semantic code search.
* Structural history the provider already exposes with sufficient guarantees.

These capabilities will be consumed through a versioned adapter.

---

## 7.2 It is not a replacement for Git

Git will remain the source of truth on:

* What changed.
* Who changed it.
* When it happened.
* Which revision contains the change.
* Which lines were modified.

Rationale will add causal meaning and validity evaluation.

> Git remembers what changed.\
> Rationale remembers why it still matters.

---

## 7.3 It is not a conversation store

Rationale must not index and retrieve complete conversations.

Conversations contain:

* Temporary ideas.
* Wrong hypotheses.
* Repetitions.
* Confusion.
* Discarded paths.
* Decisions that later changed.
* Private data.
* Possible malicious instructions.

The system must extract the consolidated learning, not preserve all the noise.

---

## 7.4 It is not only an ADR system

Architecture Decision Records usually represent large decisions:

* Choice of database.
* Distributed architecture.
* Authentication strategy.
* Framework change.

Rationale must also represent more local decisions:

* Why a function validates an assignment.
* Why a specific limit exists.
* Why a migration deactivates instead of deleting.
* Why a user is an exception.
* Why a process must be idempotent.
* Why an endpoint rejects a certain state.

An ADR can be evidence or an imported source of a Rationale Record, but it does not replace:

* Provenance.
* Authority.
* Per-revision binding.
* Applicability assessment.
* Comparison against the intent.

---

## 7.5 It is not a complete organizational memory

The first version will not try to model:

* People as a complete social network.
* Complete meetings.
* Company culture.
* Customers.
* Contracts.
* Strategy.
* The whole product history.

The initial domain will be:

> Causal context needed to modify software safely.

---

## 7.6 It is not an autonomous system that invents history

Rationale can:

* Retrieve evidence.
* Propose hypotheses.
* Detect signals.
* Corroborate sources.
* Request confirmation.

It cannot guarantee that it will reconstruct a reason that was never documented.

---

## 7.7 It is not a layer that always reduces tokens

On small tasks it can add cost.

It must be activated with budgets and policies, and prove its value on tasks where it avoids extensive reading, repeated archaeology, or costly mistakes.

---

## 7.8 It is not an open protocol from the first commit

The final vision may become a protocol, but first there must be:

* A useful implementation.
* Real cases.
* More than one consumer or provider.
* Proven versioning.
* Conformance tests.

The first delivery will be a product and a versioned context model.

The open protocol will be a consequence of use, not an initial declaration.

---

## 7.9 It is not an internal fork of Codebase Memory

Rationale must not:

* Read non-public internal Codebase Memory tables.
* Import private headers.
* Be compiled inside its repository.
* Depend on IDs without a stability contract.
* Assume that an internal tool or schema will never change.

The right integration will be through a public interface, an adapter, and capability negotiation.

---

# 8. Conceptual model


The complete conceptual model is still useful for understanding the domain:

```text
Problem
   ↓
Intent
   ↓
Change
   ↓
Decisions
   ↓
Constraints
   ↓
System behaviors
   ↓
Implementation anchors
   ↓
Validations
   ↓
Consequences
   ↓
Validity, authority, and trust
```

The domain concepts will be:

* `Problem`
* `Intent`
* `Change`
* `Decision`
* `Constraint`
* `Risk`
* `Evidence`
* `Validation`
* `Subject`
* `Binding`
* `Claim`
* `Consequence`
* `Approval`
* `Assessment`

Conceptual relationships:

```text
Change       --SOLVES----------> Problem
Change       --HAS_INTENT------> Intent
Change       --MADE_DECISION---> Decision
Decision     --GOVERNS---------> Subject
Constraint   --PROTECTS--------> Subject
Risk         --THREATENS-------> Subject
Subject      --IMPLEMENTED_BY--> Binding
Claim        --SUPPORTED_BY----> Evidence
Claim        --APPROVED_BY-----> Approval
Assessment   --EVALUATES-------> Record
Validation   --VALIDATES-------> Change
Change       --SUPERSEDES------> Change
Decision     --SUPERSEDES------> Decision
Binding      --RELOCATED_TO----> Binding
Subject      --DEPENDS_ON------> Subject
Subject      --ALIASES---------> Subject
Subject      --SPLIT_INTO------> Subject
```

## 8.1 Complete conceptual model vs. v1 persisted model

The system does not need to materialize all these concepts as nodes from the start.

v1 persistence will use:

```text
Subject
Record
Binding
Evidence
Approval
Assessment
```

`Problem`, `Intent`, `Risk`, `Validation`, `Consequence`, and `Change` will remain internal structures of the `Record` until real cases show that they need independent identity.

This keeps the complete vision without prematurely paying the cost of an excessive domain graph.

---

# 9. Concept-first, code-anchored


Every record must have a stable conceptual subject.

Example:

```yaml
subject:
  id: authorization.entity-scoped-staff-access
  type: system-behavior
  title: Entity-scoped access for staff users
  aliases:
    - auth.staff-per-entity
```

It will then have multiple bindings or anchors:

```yaml
bindings:
  - type: symbol
    provider: codebase-memory
    provider_version: 0.9.x
    structural_id: function:typescript:auth.resolveEntityRole
    path_hint: src/auth/authorization.ts
    bound_revision: def456
    provider_generation: 184
    coverage: complete

  - type: database-table
    id: entity_user_roles
    bound_revision: def456

  - type: route
    id: GET /entities/:id
    bound_revision: def456

  - type: migration
    path: database/migrations/remove_staff_super_admin.sql
    revision: 91ac21f

  - type: test
    id: staff_cannot_access_unassigned_entity
    bound_revision: def456

  - type: commit
    revision: 91ac21f
```

Anchors can change.

The concept must be kept when the decision still represents the same behavior.

If an implementation is split:

```yaml
concept_lineage:
  - type: split
    from:
      - authorization.staff-access
    into:
      - authorization.staff-global-role
      - authorization.staff-entity-assignment
```

If a function is split without the concept changing:

```yaml
binding_lineage:
  - type: implementation-split
    from:
      - function:typescript:auth.resolveEntityRole
    to:
      - service:identity.resolveGlobalRole
      - service:entity.resolveAssignment
      - gateway:authorizeEntityRequest
```

If it cannot be reconstructed automatically:

```yaml
assessment:
  applicability: active
  linkage: unresolved
  assessed_revision: def456
  requires_review: true
```

The decision is not lost just because its implementation moved.

## 9.1 Conceptual identity and deduplication

Two agents can create different IDs for the same concept:

```text
authorization.entity-scoped-staff-access
auth.staff-per-entity-permissions
```

That is why an agent will not write a new Subject directly. `finalize_change` will send a proposal to the **Subject Resolver**.

Mandatory resolution order:

```text
1. Exact ID or alias.
2. Normalized name within the same project and domain.
3. Overlap of structural bindings.
4. Parent/child relationship and scope compatibility.
5. FTS search by title, description, and invariants.
6. Local semantic similarity, if enabled.
7. Candidate review and explicit decision.
```

Possible result:

```yaml
resolution:
  action: reuse | create | alias | merge_candidate | split_candidate
  selected_subject: authorization.entity-scoped-staff-access
  candidates:
    - id: auth.staff-per-entity-permissions
      signals:
        binding_overlap: 0.92
        lexical_similarity: 0.81
        semantic_similarity: 0.88
        scope_compatible: true
  novelty_reason: null
```

If the agent decides `create` in the face of a highly similar candidate, it must explain a concrete `novelty_reason`:

```text
This Subject represents the dashboard's visual contract, not the backend assignment policy; the two are related, but they have different lifecycles and authorities.
```

Allowed operations:

```text
alias
merge
split
rename
scope-narrowed
scope-expanded
supersede
```

Rules:

* Similarity does not merge automatically.
* Binding overlap does not demonstrate conceptual identity.
* A merge or split requires an auditable event.
* Aliases must be unique within the project.
* The local database can keep a queue of conceptual collisions without blocking day-to-day work.
* Every operation keeps lineage, actor, evidence, scope, and revision.

## 9.2 Structured, bypass-resistant `novelty_reason`

`novelty_reason` must not depend only on a well-written prompt. It must have an auditable form that forces the proposed concept to be compared against concrete candidates.

Example:

```yaml
novelty_reason:
  compared_against:
    - authorization.entity-scoped-staff-access

  difference_types:
    - different_behavior
    - different_scope

  contrast:
    existing: >
      Governs how staff permissions are assigned inside an entity.

    proposed: >
      Governs which owner-level capabilities can cross every entity.

  evidence:
    - type: symbol
      id: auth.resolveOwnerRole

    - type: test
      id: owner_has_global_access

  produced_by:
    type: agent
    name: codex

  review_state: unreviewed
```

Initial difference types:

```text
different_behavior
different_scope
different_lifecycle
different_authority_domain
different_invariant
existing_subject_is_superseded
existing_subject_is_too_broad
existing_subject_is_too_narrow
```

A reason is considered insufficient when it:

* Does not identify which candidates were compared.
* Repeats that the concept is new without expressing an observable difference.
* Uses generic phrases such as `is different`, `separate concern`, or `new functionality` without contrast.
* Confuses different files with different concepts.
* Confuses different wording with different identity.
* Provides no evidence when there is high similarity of bindings, invariants, or scope.
* Declares a different authority without naming the responsible domain.

The Subject Resolver must return an explicit result:

```yaml
novelty_validation:
  status: accepted | insufficient | requires_review
  candidate_threshold_triggered: true
  missing_fields:
    - contrast.existing
  generic_reason_detected: false
  decision_source: deterministic_rules
```

Product rules:

* An LLM may draft the proposal, but it may not self-approve its sufficiency through another free-form explanation.
* Basic validation must be deterministic and based on the schema, the candidates mentioned, the contrast, and the evidence.
* An insufficient reason returns the candidates and asks for a correction; it does not invent the Subject by default.
* For non-critical changes, continuing with a pending collision may be allowed so work is not blocked.
* In critical domains, creation may require human review before it acquires normative authority.
* Semantic similarity is never proof of identity nor a sufficient reason to permanently block a creation.

During the pilot the following will be measured:

* New Subjects proposed.
* Similar candidates presented.
* Percentage of `novelty_reason` values rejected as generic.
* Duplicate Subjects detected later.
* Correct reuses.
* Merges or splits that had to be reverted.
* Human time added by resolution.

## 9.3 Realistic promise

v1 promises:

> To keep explicit bindings, detect relocation candidates, and preserve approved lineage.

v1 does not promise:

> To automatically reconstruct perfect conceptual identity after any migration across repositories, languages, or architectures.

---

# 10. Trust, provenance, and authority model


Rationale must separate three distinct questions:

```text
Epistemology: how was this claim obtained?
Provenance: who or what produced it?
Authority: who could approve it as a rule?
```

## 10.1 Mechanical facts

Automatically verifiable data:

* This commit modified the file.
* This function calls another function according to the provider.
* This test was run.
* This dependency was removed.
* This route changed.
* This table was modified.

Classification:

```yaml
epistemic_status: observed
```

The observation must record provider, version, revision, and coverage.

## 10.2 Human claims

Information explicitly stated by a person:

```text
The limit of 50 exists because the API charges more from 51 on.
```

Classification:

```yaml
epistemic_status: stated
provenance:
  type: human
  actor: user:rolando
```

This does not automatically mean it is an approved policy.

## 10.3 Corroborated claims

Information backed by several independent sources.

Example:

* An issue mentions isolation between entities.
* A cross-access test is added.
* The migration removes global permissions.

Classification:

```yaml
epistemic_status: corroborated
confidence: 0.88
```

## 10.4 Inferences

Conclusions produced by an agent:

```yaml
epistemic_status: inferred
confidence: 0.54
requires_confirmation: true
```

An inference must never automatically become an approved decision.

## 10.5 Disputed and unknown claims

```yaml
epistemic_status: disputed
```

```yaml
epistemic_status: unknown
```

The `unknown` status is a valid answer and preferable to an invented explanation.

## 10.6 Authority

Minimum states:

```text
unreviewed
approved
policy
revoked
```

Possible authority roles:

```text
contributor
domain-maintainer
domain-owner
security-owner
product-owner
architecture-owner
repository-policy
```

Example:

```yaml
authority:
  status: approved
  role: security-owner
  domain: authorization
  approval_policy: codeowners
```

## 10.7 Rule for critical knowledge

A critical constraint may only block if it simultaneously satisfies:

```text
kind = constraint
severity = critical
authority ∈ {approved, policy}
applicability = active
linkage = current
revision_consistency = exact
conflict = concrete
```

It will never block because of:

* An inference.
* Vector similarity.
* A stale binding.
* Unknown structural coverage.
* An LLM-generated summary.
* The absence of a relationship in the provider.

---

# 11. Structure of a claim


```yaml
claim:
  id: claim.staff-global-access-was-unsafe

  statement: >
    Staff users received excessive global privileges.

  epistemic_status: corroborated
  confidence: 0.88

  provenance:
    produced_by:
      type: agent
      name: codex
      version: unknown

    created_at: 2026-07-23T18:20:00Z

  authority:
    status: approved
    role: security-owner
    domain: authorization

    approved_by:
      - actor: user:security-owner
        approved_at: 2026-07-23T19:00:00Z

  evidence:
    - type: source-code
      revision: 91ac21f^
      path: src/auth/authorization.ts
      verified: true
      provider: git

    - type: database-migration
      revision: 91ac21f
      path: database/migrations/remove_staff_super_admin.sql
      verified: true

    - type: human-statement
      content_hash: sha256:...
      verified: true
      visibility: restricted

  sensitivity:
    classification: internal

  scope:
    subjects:
      - authorization.entity-scoped-staff-access
```

Possible epistemic states:

```text
observed
stated
corroborated
inferred
hypothetical
disputed
unknown
```

Authority states:

```text
unreviewed
approved
policy
revoked
```

## 11.1 Free text is never executable

Every text field must be considered untrusted content.

The final packet must wrap it as data and never allow instructions contained in evidence or explanations to alter the agent's behavior.

## 11.2 Declarative representation of constraints

When possible, a constraint will have a structured form:

```yaml
constraint_expression:
  subject: authorization.staff
  predicate: must_not_have
  object: role.global_super_admin
  conditions:
    - actor_type: staff
```

And a separate human explanation:

```yaml
rationale: >
  Multi-entity access must not imply global administration.
```

The declarative representation makes validation easier and reduces ambiguity; the explanation keeps the reason.

---

# 12. Multidimensional state


Rationale must not use a single general state such as `possibly-stale`.

Nor must it multiply states until every combination becomes impossible to understand.

v1 will use four minimum dimensions.

## 12.1 Epistemic state

Indicates what we know about the claim:

```text
observed
stated
corroborated
inferred
disputed
unknown
```

## 12.2 Authority

Indicates whether the normative claim was approved:

```text
unreviewed
approved
policy
revoked
```

## 12.3 Applicability

Indicates whether the decision still governs the system:

```text
active
superseded
unknown
```

`Suspected drift` will not be a permanent applicability. It will be a signal or a pending evaluation.

## 12.4 Link state

Indicates the quality of the connection to the current implementation:

```text
current
stale
unresolved
```

The details `relocated`, `partially-linked`, `degraded`, and `orphaned` can be kept as internal reasons, but they do not need to become separate public states in v1.

## 12.5 Per-revision consistency

```text
exact
working-tree-overlay
structural-index-behind
assessment-behind
unresolved
```

## 12.6 Computed attention

Attention will not be a persisted state.

It will be computed:

```text
attention =
    severity
  × authority
  × applicability
  × target_overlap
  × intent_conflict
  × linkage_quality
  × revision_consistency
```

Example:

```yaml
state:
  epistemic: stated
  authority: approved
  applicability: active
  linkage: current
  revision_consistency: exact
```

A function may have moved and the binding may have been repaired. If the decision is still active, the tool must not raise an alert just because of the move.

---

# 13. Change classification

Before invalidating or alerting, Rationale must classify the change.

## 13.1 Cosmetic change

Examples:

* Formatting.
* Comments.
* Local variable renames.
* Reordering.
* Style change.

Action:

```text
Do not alert.
Do not modify applicability.
```

## 13.2 Structural refactor

Examples:

* Extract Method.
* Moving a function.
* Splitting a class.
* Changing files.
* Turning a function into a service.

Action:

```text
Try to reconnect the anchors.
Update linkage.
Do not assume a conceptual change.
```

## 13.3 Local behavior change

Examples:

* Modifying a condition.
* A new error case.
* A validation adjustment.
* A fallback change.

Action:

```text
Review related constraints.
Show a notice only if there is enough risk.
```

## 13.4 Contract change

Examples:

* Inputs or outputs.
* HTTP responses.
* Persistence.
* Events.
* Authorization.
* External integrations.

Action:

```text
Mark related decisions for analysis.
Recommend validations.
```

## 13.5 Conceptual change

Examples:

```text
Before: explicit per-entity access.
Now: inherited global access.
```

Action:

```text
Compare against active decisions.
Warn about the conflict.
Block only if there is a confirmed critical constraint.
```

---

# 14. Preventing alert fatigue


Rationale must keep every change from producing warnings.

Principles:

* Refactors must be processed silently when bindings can be repaired.
* Stale links must not always be shown.
* Repeated warnings must be grouped.
* Alerts must have priority.
* Only approved critical constraints may produce blocks.
* An inference must never block a change.
* A structural change must not be presented as a conceptual violation.
* Alerts must relate to the intent or to the current diff.
* Revision inconsistency must be shown before any conclusion.
* Incomplete coverage must not become negative certainty.

A good warning would be:

```text
The proposed intent may restore global access for support users.

This contradicts an approved critical constraint:
Multi-entity access must be resolved through per-entity assignments.

Authority: security-owner.
Applicability: active.
Structural revision: exact.
Evidence: migration, tests, and approved decision.
```

A bad warning would be:

```text
The file authorization.ts changed.
Fifteen memories may be outdated.
```

## 14.1 Single blocking condition

A change can only be blocked when:

1. A structured constraint exists.
2. Its severity is critical.
3. It was approved by the appropriate authority or declared by repository policy.
4. It is still active.
5. The binding corresponds to the current revision.
6. The contradiction is concrete and sufficiently deterministic.
7. The output explains how to resolve or review the conflict.

If any of these conditions fails, the response will be informative or advisory, never blocking.

## 14.2 Interruption budget

Besides the token budget, the repository can define an interruption budget:

```yaml
alert_policy:
  max_advisories_per_change: 3
  group_repeated_records: true
  suppress_structural_refactors: true
  blocking_requires_exact_revision: true
```

---

# 15. Low-friction capture


## 15.1 What Rationale captures automatically

* Diff.
* Base revision and final revision.
* Working tree state.
* Commits.
* Files.
* Symbols.
* Relationships reported by the provider.
* Provider coverage and version.
* Tests run.
* Results.
* Schema changes.
* Dependencies.
* Routes.
* Linked issues and PRs.
* Agent identity.
* Creation time.

## 15.2 What the agent can propose

* Apparent problem.
* Intent.
* Decisions.
* Risks.
* Alternatives.
* Consequences.
* Possible supersessions.
* Possible relocated bindings.

These proposals must start as inferences or candidates.

## 15.3 What a person must confirm

Only the most important normative claims:

* Main reason.
* Decision.
* Critical constraint.
* Deliberate exception.
* Risk not visible in the code.
* Authority or approval policy.
* Conceptual supersession.

Example confirmation:

```text
I detected these two new normative claims:

1. Multi-entity access must not imply global administration.
2. Owner users are deliberate exceptions.

Do you approve them for the authorization domain?
```

The developer must not be asked to review all the YAML.

## 15.4 High-value capture signals

Rationale will not ask about every change.

It will trigger assisted capture when it detects signals such as:

* Authorization.
* Payments.
* Billing.
* Security.
* Destructive migrations.
* Schema changes.
* Deliberate exceptions.
* Irreversible processes.
* External integrations.
* Incident fixes.
* Normative language in a PR or conversation: `must`, `never`, `because`, `avoid`, `do not`.
* Discarded alternatives with relevant consequences.

## 15.5 Preventing automatic confirmation

To reduce the "accept everything" syndrome:

* A confirmation must show a single claim per important decision.
* It must include the practical effect of approving it.
* It must allow correcting the text before approving.
* It must not preselect approval for critical constraints.
* It must record how much time passed between proposal and confirmation as a quality signal, without assuming bad faith.
* It may require a second approval in security or money domains.

## 15.6 Capture from founding documents

Rationale can import ADRs, Markdown, or existing documentation.

The import produces:

```text
stated or inferred claims.
unreviewed authority.
Candidate bindings.
```

It never automatically turns a complete document into approved critical constraints.

## 15.7 Detecting changes outside the flow

The first guarantee will be cheap and synchronous:

```text
current_git_revision != last_processed_revision
        ↓
identify changed paths and targets
        ↓
mark related assessments stale or unknown
        ↓
serve qualified context or request revalidation
```

Optionally, an integration can run this step from:

* `post-commit`.
* Agent lifecycle hooks.
* A file watcher or local daemon.
* IDE session start.
* PR review or CI.

These integrations must never silently block the commit or assume that they managed to revalidate the meaning. Their initial job is to **detect progress and degrade confidence**, not to invent a new evaluation.

## 15.8 Shared capture without full team adoption

Not every developer needs to run Rationale for the repository to keep approved Records.

A collaborator without the tool can modify code normally. When another machine with Rationale queries the project, it:

1. Will detect that Git moved forward.
2. Will compare the diff since the last evaluated revision.
3. Will mark related bindings as stale.
4. Will avoid presenting earlier assessments as current.
5. May propose revalidation or a new normative delta.

Usefulness grows when more members capture decisions, but correctness cannot assume universal adoption.

---

# 16. Capture levels


## Level 0 — Git only

For:

* Formatting.
* Renames.
* Minor dependencies.
* Mechanical changes.

No record is created.

## Level 1 — Intent

Stores:

* Goal.
* Modified areas.
* Validation.
* Base/final revision.

## Level 2 — Decision

Adds:

* Decision.
* Alternatives.
* Reason.
* Non-goals.
* Provenance.

## Level 3 — Operational knowledge

Adds:

* Risks.
* Constraints.
* Rollback.
* Consequences.
* Incidents.
* Sensitivity.

## Level 4 — Critical invariant

Knowledge no agent may ignore:

```text
A payment cannot be processed twice.
Staff cannot receive global super_admin.
One entity cannot see another entity's data.
A migration cannot delete the audit trail.
```

Additional requirements:

* Approved or policy authority.
* Explicit scope.
* Evidence.
* Current binding.
* Declarative rule when possible.
* Evaluated revision.
* Supersession policy.

The system can recommend a level based on:

* Authorization.
* Payments.
* Security.
* Migrations.
* Infrastructure.
* Schema changes.
* Number of files.
* Reversibility.
* Incidents.
* Structural impact.
* Data sensitivity.

## Level 5 — Repository policy

Reserved for deterministic rules approved as repository policy.

Examples:

* No migration may delete the audit table.
* Public endpoints must enforce rate limiting.
* Payment changes require two approvals.

This level can integrate with CI, but it must not contain ambiguous rules that depend on the LLM's free interpretation.

---

# 17. Cold start and legacy projects


Rationale must not try to automatically reconstruct the whole history of an old monolith.

It must use a progressive strategy.

## 17.1 Forward capture

From installation on:

* New important changes are recorded.
* New decisions are linked.
* New constraints become available.
* Revisions and coverage are recorded from the start.

## 17.2 On-demand archaeology

When an old area without context is modified:

```bash
rationale investigate src/auth/authorization.ts
```

The system can analyze:

* Git blame.
* Commits.
* Pull requests.
* Issues.
* ADRs.
* Tests.
* Migrations.
* Comments.
* Earlier versions.
* Documentation.
* Per-symbol history if the provider exposes it.

Result:

```text
Confirmed reason: unknown.

Evidence found:
- The condition appeared in commit 83af12.
- The commit references AUTH-184.
- The issue mentions cross-entity access.
- An isolation test was added in the same change.

Corroborated hypothesis:
This condition probably protects entity isolation.

Confidence: medium-high.
Authority: not reviewed.
Requires human confirmation: yes.
```

## 17.3 Limits of archaeology

The tool must declare:

* Whether the clone is shallow.
* Up to which revision history exists.
* Which sources were not available.
* Whether PRs or issues could not be queried.
* Whether the symbol changed range.
* Whether the evidence is temporarily incomplete.

Absence of evidence is not evidence of absence.

## 17.4 Prioritization

Not the whole repository needs causal context.

Initial priority:

* Authorization.
* Payments.
* Billing.
* Security.
* Migrations.
* Data.
* External integrations.
* Synchronization.
* Irreversible processes.

## 17.5 Coverage

Rationale can show:

```text
Authorization      High coverage
Payments           Partial coverage
Billing            Partial coverage
Notifications      No coverage
UI components      Coverage unnecessary
```

It must distinguish between:

```text
No coverage
Unknown coverage
Provider gap
Coverage unnecessary
```

The goal is not to reach 100%.

The goal is to cover the areas where losing context has serious consequences.

---

# 18. Budgeted retrieval


Every query will have an explicit budget.

```json
{
  "mode": "intent-aware",
  "max_tokens": 900,
  "max_critical_constraints": 5,
  "max_decisions": 3,
  "max_risks": 3,
  "include_history": false,
  "require_exact_revision": true,
  "scope": "auto"
}
```

## 18.0 Context compiler inputs

The quality of the packet depends on combining three different sources:

```yaml
task_context:
  intent: Fix duplicate invoice generation
  symptoms:
    - Two invoices appear after retrying a timed-out request
  reproduction:
    - Trigger checkout
    - Interrupt response after payment succeeds
    - Retry checkout
  expected_behavior: A payment creates at most one invoice

target_context:
  paths:
    - apps/api/src/billing/checkout.ts
  symbols:
    - billing.finalizeCheckout

context_budget:
  max_tokens: 900
```

The user or agent keeps providing the immediate reality of the task. Rationale retrieves the durable reality of the project.

## 18.1 Priority order

### Level 0 — Health and consistency

```text
Git revision: def456
Structural revision: def456
Assessment revision: def456
Consistency: exact
Coverage: complete for requested targets
```

If this level fails, the output must say so before any conclusion.

### Level 1 — Approved critical constraints

```text
CRITICAL

- Staff users must never receive global super_admin.
- Cross-entity access requires an explicit assignment.
```

### Level 2 — Conflicts with the intent

```text
Your proposed change may recreate a previously removed authorization path.
```

### Level 3 — Main reason

```text
This behavior was introduced after staff accounts received excessive global privileges.
```

### Level 4 — Relevant risks

```text
Users without valid entity assignments may lose access.
```

### Level 5 — Structure

```text
Affected:
- resolveEntityRole
- authorizeRequest
- entity_user_roles
- 3 tests
```

### Level 6 — Expandable history

```text
4 additional historical records available.
Use trace_rationale for details.
```

## 18.2 Progressive disclosure

The initial response must be enough to act safely, not to tell the whole story.

The agent can expand:

* Evidence.
* Discarded alternatives.
* Supersession history.
* PRs and issues.
* Secondary bindings.

## 18.3 Adaptive activation

For trivial changes, Rationale can return a minimal packet:

```text
No approved constraints found for this target.
Structural revision is current.
No additional context required.
```

The cost of the preflight must adapt to the risk.

---

# 19. Relevance selection


Finding records linked to the same symbol is not enough.

Rationale must evaluate:

```text
relevance =
    exact_binding_overlap
  + conceptual_scope_overlap
  + intent_conflict
  + constraint_severity
  + authority_strength
  + evidence_quality
  + behavioral_impact
  + current_applicability
  + revision_consistency
  - redundancy
  - historical_distance
  - provider_uncertainty
```

The intent changes the answer.

## Rename

```text
Intent: Rename resolveEntityRole.
```

Answer:

```text
The symbol is connected to an active authorization decision.
The proposed rename does not appear to change its behavior.
Binding repair can be performed silently after the change.
```

## Conceptual change

```text
Intent: Allow support users to access all entities automatically.
```

Answer:

```text
Warning: this intent conflicts with a critical approved constraint.

Multi-entity access must not imply global administration.
```

## 19.1 Deterministic retrieval before semantics

Recommended order:

1. Exact binding.
2. Structural neighborhood.
3. Conceptual scope.
4. Critical constraints.
5. Applicability and authority.
6. FTS text search.
7. Embeddings as a future fallback.

Critical constraints must not be retrieved only because their text is semantically similar to the query.

## 19.2 Negative deduction forbidden

The response must not infer:

```text
There is no decision.
```

only because it found no binding.

It must answer:

```text
No linked decision was found within the available coverage.
```

## 19.3 Workspace-aware retrieval

In a monorepo, ranking must consider:

```text
workspace_overlap
package_dependency_path
contract_relationship
subject_scope_inheritance
explicit_applies_to
explicit_excludes
```

Example:

```text
Task target: apps/dashboard/src/users/RoleBadge.tsx

Included backend record:
constraint.no-global-admin-for-staff

Reason for inclusion:
RoleBadge renders the authorization contract exported by @boost/auth-contracts,
which is governed by the project-level staff authorization Subject.
```

Not every backend rule will be included just because it is in the same monorepo. Every cross inclusion must be able to explain its relevance path.

---

# 20. Codebase Memory integration


The main architecture will be:

```text
┌─────────────────────────────────────┐
│            Coding agent             │
└──────────────────┬──────────────────┘
                   │
                   │ prepare_change
                   ▼
┌─────────────────────────────────────┐
│            Rationale MCP            │
│                                     │
│ MCP façade                          │
│ Policy / Trust evaluator            │
│ Retrieval engine                    │
│ Lifecycle service                   │
│ Revision coordinator                │
└─────────────┬─────────────┬─────────┘
              │             │
              ▼             ▼
┌───────────────────┐  ┌────────────────────┐
│ CBM adapter       │  │ Git / Record Store │
│                   │  │                    │
│ Capabilities      │  │ Records            │
│ Coverage          │  │ Evidence           │
│ Revision          │  │ Approvals          │
│ Relationships     │  │ Assessments        │
└─────────────┬─────┘  └────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│ Codebase Memory daemon / MCP / CLI  │
└─────────────────────────────────────┘
```

Rationale must query Codebase Memory internally.

The agent should not need to coordinate both tools manually.

Flow:

```text
Agent → Rationale → Codebase Memory
```

## 20.1 Responsibility boundary

### Codebase Memory

* Current structure.
* Symbols.
* Calls.
* Dependencies.
* Routes.
* Structural impact.
* Search and relationships its coverage allows.
* Structural history when a suitable public API exists.

### Rationale

* Normative decisions.
* Constraints.
* Intent.
* Provenance.
* Authority.
* Causal evidence.
* Applicability assessment.
* Comparison against the intent.
* Context budget.
* Blocking policy.

## 20.2 Codebase Memory is already moving toward ADRs and history

The integration must assume that Codebase Memory may add new capabilities such as ADRs, per-symbol history, or drift.

Rationale must not compete by duplicating those surfaces.

It must consume them as evidence and keep its differentiator:

> Authority, applicability, per-revision consistency, and intent preflight.

## 20.3 Fallible provider

Every provider query must return or record:

```yaml
provider:
  name: codebase-memory
  version: 0.9.x
  indexed_revision: def456
  generation: 184
  coverage: complete
  status: successful
  capabilities:
    - resolve_target
    - relationships
    - impact
```

If the query fails or its coverage is incomplete, Rationale must degrade its conclusion.

## 20.4 No internal coupling

Rationale will not read Codebase Memory's internal SQLite database directly or depend on private details.

It will use:

* Public MCP.
* The documented CLI when necessary.
* A versioned adapter.
* Capability negotiation.
* Contract tests.

## 20.5 Latency

Latency must not be solved with premature distributed architecture.

Measures:

* Local processes.
* Per-revision cache.
* A single coordinated query from Rationale.
* Small results.
* Timeouts.
* Avoiding multiple tool calls by the agent.
* Invalidation by provider generation.

The agent must make one main call; Rationale coordinates the dependencies internally.

### 20.5.1 Two execution paths

Rationale must separate two latency profiles.

#### Baseline fast path

Used on high-frequency surfaces such as reading, searching, starting an edit, or navigation.

It must:

* Read only local storage.
* Use bindings and scopes that are already resolved.
* Query an index per revision or generation.
* Deliver only critical constraints and consistency warnings.
* Avoid embeddings.
* Avoid LLM calls.
* Avoid archaeology and deep history.
* Avoid rebuilding the graph.
* Avoid multiple chained MCP calls when the cache is valid.
* Finish with no output when there is no high-priority context.

Conceptual cache key:

```text
project_id
+ git_revision_or_worktree_generation
+ target_identity
+ provider_generation
+ policy_generation
+ baseline_budget
```

The concrete key belongs to the architecture document, but per-revision consistency is not optional.

#### Intent-aware full path

Used when there is an intent, symptoms, reproduction, expected result, or a change that needs deep analysis.

It can:

* Query structural impact.
* Resolve cross-workspace relationships.
* Compare the intent against decisions.
* Evaluate conflicts and applicability.
* Search with FTS.
* Use local embeddings as a candidate fallback.
* Query history or additional evidence.
* Build a richer packet within the requested budget.

This path runs at change boundaries, not before every read operation.

### 20.5.2 Experimental latency budgets

The following values are initial targets for the pilot, not final public guarantees:

```text
Warm baseline:
P50 <= 50 ms
P95 <= 150 ms
hard deadline <= 250 ms

Cold baseline:
measure separately; never hide it inside the warm distribution

Intent-aware preflight:
measure by complexity, cache state, and provider
```

If the baseline exceeds its deadline:

```text
fail open
do not block the agent's operation
possibly return no context
record local timeout telemetry
do not present partial results as complete
```

`fail open` does not mean declaring that no constraints exist. It means the original operation continues and the missing context is recorded as an observable degradation.

### 20.5.3 What must be measured

For every run:

* Total latency.
* Index open time.
* Lookup time.
* Cache hit or miss.
* Cold or warm start.
* Number of provider calls.
* Timeout.
* Packet size.
* Revision and generation used.
* Context delivered, omitted, or degraded.

Latency must be analyzed together with end-to-end cost. Saving 100 milliseconds in the preflight does not justify adding several minutes to solving the task, and adding 300 milliseconds may be acceptable if it avoids a critical regression; the exact policy will depend on the surface and severity.

## 20.6 Concrete findings from the Codebase Memory repository

The earlier review of the repository confirmed several points relevant to the design:

* The core is implemented mainly as a local binary in C.
* It exposes MCP, a CLI, and a local daemon.
* It uses local storage and broad structural capabilities.
* It already includes an ADR-related surface.
* There are proposals for multiple ADRs, per-symbol history, and architectural drift.
* There are reported cases of false relationships, empty traces, gaps between packages, silently empty results, and resource problems in certain repositories or languages.

This does not invalidate Codebase Memory. It confirms two decisions:

1. Rationale must build on it instead of duplicating it.
2. Rationale must record coverage, version, revision, and warnings instead of turning any structural output into absolute mechanical truth.

It also means the adapter must be designed after reviewing the real public APIs and capabilities of the target version, not only from an imagined interface.

## 20.7 Workspace and hook implications observed in Codebase Memory

Codebase Memory already recognizes package structure through manifests and keeps specific proposals for workspace identity. Its recent history also shows that cross-package resolution can vary by platform, layout, internal limits, or provider version.

Consequence for Rationale:

* It must consume workspace and package identity when the provider offers it.
* It must declare `provider_gap` when the cross relationship cannot be verified.
* It must not interpret zero edges as the absence of a relationship.
* It must accept manual or contractual bindings as a fallback in the pilot.

Codebase Memory also implements a non-blocking augmentation hook pattern, with deadlines, sanitization, and silent exit on errors. That pattern validates that an automatic context layer can exist, but it also shows that a hook integration must be best-effort and observable.

Rationale will take these principles, not an internal dependency on that implementation:

```text
non-blocking by default
bounded latency
untrusted metadata as data
observable timeout/no-op
query-time correctness check
```

---

# 21. Structural provider interface


```rust
#[async_trait]
pub trait CodeIntelligenceProvider {
    async fn capabilities(
        &self,
    ) -> Result<ProviderCapabilities>;

    async fn health(
        &self,
        repository: &RepositoryRef,
    ) -> Result<ProviderHealth>;

    async fn resolve_target(
        &self,
        target: &UnresolvedTarget,
        revision: &Revision,
    ) -> Result<ResolvedTarget>;

    async fn get_relationships(
        &self,
        target: &ResolvedTarget,
        budget: &RelationshipBudget,
    ) -> Result<ProviderResult<Vec<CodeRelationship>>>;

    async fn get_impact(
        &self,
        target: &ResolvedTarget,
        revision: &Revision,
    ) -> Result<ProviderResult<ImpactReport>>;

    async fn changed_targets(
        &self,
        base: &Revision,
        head: &Revision,
    ) -> Result<ProviderResult<Vec<TargetChange>>>;

    async fn classify_change(
        &self,
        change: &TargetChange,
    ) -> Result<ProviderResult<ChangeClassification>>;

    async fn find_lineage_candidates(
        &self,
        target: &ResolvedTarget,
        base: &Revision,
        head: &Revision,
    ) -> Result<ProviderResult<Vec<TargetLineageCandidate>>>;
}
```

First implementation:

```rust
pub struct CodebaseMemoryProvider {
    client: McpClient,
}
```

## 21.1 Result with explicit quality

```rust
pub struct ProviderResult<T> {
    pub data: T,
    pub provider_version: String,
    pub indexed_revision: Revision,
    pub generation: String,
    pub coverage: Coverage,
    pub warnings: Vec<ProviderWarning>,
}
```

## 21.2 Capability negotiation

Rationale will not assume that every version offers:

* Per-symbol history.
* Cross-repo.
* Drift.
* Lineage.
* Framework resolution.

The adapter must be able to answer:

```text
supported
unsupported
degraded
unknown
```

## 21.3 Future adapters

* Tree-sitter.
* LSP.
* SCIP.
* LSIF.
* Sourcegraph.
* GitHub.
* In-house engines.

These adapters eventually justify an open protocol, but they are not an MVP requirement.

---

# 22. Main modules


The conceptual architecture keeps the original responsibilities, but the initial implementation must not turn every "engine" into an independent service or crate.

## 22.1 Capture module

Responsible for collecting:

* Diff.
* Commits.
* Symbols.
* Tests.
* Issues.
* PRs.
* Signals.
* Claims.
* Schema changes.
* Revisions and coverage.

It does not decide what is true.

## 22.2 Trust and policy module

Responsible for:

* Classifying claims.
* Verifying provenance.
* Evaluating authority.
* Verifying evidence.
* Detecting contradictions.
* Preventing inferences from becoming facts.
* Controlling what can block changes.
* Applying sensitivity rules.

## 22.3 Concept and linkage module

Responsible for:

* Creating conceptual subjects.
* Resolving bindings.
* Keeping lineage.
* Proposing reconnections.
* Associating tests, tables, routes, and services.
* Separating conceptual identity from implementation.
* Managing aliases, merges, and splits.

## 22.4 Drift and lifecycle module

Responsible for distinguishing:

* Cosmetic change.
* Refactor.
* Move.
* Contract change.
* Conceptual change.
* Possible violation.
* Possible supersession.

It cannot automatically invalidate a normative decision based only on an AI heuristic.

## 22.5 Retrieval module

Responsible for:

* Interpreting the intent.
* Selecting relevant records.
* Ordering them.
* Removing redundancy.
* Applying the budget.
* Building the context packet.
* Allowing progressive expansion.

## 22.6 Revision coordinator

Responsible for:

* Comparing Git HEAD, the working tree, the structural index, and assessments.
* Creating coherent snapshots.
* Rejecting inconsistent responses.
* Invalidating caches by revision or generation.
* Distinguishing `exact`, `overlay`, `behind`, and `unresolved`.

## 22.7 Initial implementation

These modules will live inside a modular monolith.

They will not be microservices.

There will not be an independent process per engine.

The goal will be to reduce operational complexity and validate the product before separating components.

---

# 23. Complete cycle


## 23.1 Before the change

Input:

```json
{
  "targets": [
    {
      "path": "src/auth/authorization.ts",
      "symbol": "resolveEntityRole"
    }
  ],
  "intent": "Allow support users to access multiple entities",
  "git_revision": "def456",
  "context_budget": {
    "max_tokens": 900,
    "require_exact_revision": true
  }
}
```

Process:

1. Resolve repository and revision.
2. Check the working tree state.
3. Query Codebase Memory health, version, revision, and coverage.
4. Resolve the symbols.
5. Get relationships within the budget.
6. Resolve related subjects.
7. Retrieve decisions and constraints.
8. Evaluate epistemic state, authority, applicability, and linkage.
9. Compare the intent.
10. Classify risks.
11. Apply the budget.
12. Build the response with a consistency snapshot.

Output:

```json
{
  "snapshot": {
    "repository_id": "boost",
    "git_revision": "def456",
    "structural_provider": "codebase-memory",
    "structural_revision": "def456",
    "structural_generation": "184",
    "rationale_revision": "a82cf1",
    "assessment_revision": "def456",
    "consistency": "exact"
  },
  "critical_constraints": [
    {
      "statement": "Multi-entity access must not imply global administration.",
      "authority": "approved",
      "applicability": "active"
    }
  ],
  "decision_conflicts": [
    "The proposed intent may recreate a previously removed privilege path."
  ],
  "why": "Staff previously received excessive global privileges.",
  "known_risks": [
    "Users without entity assignments may lose access."
  ],
  "affected_targets": [
    "auth.resolveEntityRole",
    "auth.authorizeRequest",
    "table:entity_user_roles"
  ],
  "coverage": {
    "status": "complete",
    "warnings": []
  },
  "additional_history_available": 3
}
```

## 23.2 During the change

Temporary signals:

```json
{
  "type": "hypothesis",
  "statement": "Multi-entity users may require a global role",
  "status": "unverified"
}
```

```json
{
  "type": "discovery",
  "statement": "Multiple entity assignments are already supported",
  "evidence": {
    "symbol": "auth.resolveEntityRole",
    "revision": "def456"
  }
}
```

```json
{
  "type": "decision_candidate",
  "statement": "Keep authorization entity-scoped",
  "confirmation": "pending"
}
```

Signals do not automatically become permanent records.

## 23.3 When finishing

Rationale obtains:

* Goal.
* Base and final revision.
* Diff.
* Commits.
* Symbols.
* Tests.
* Results.
* Signals.
* Errors.
* Candidate decisions.

Then it:

1. Verifies that the diff matches the preflight, or declares the divergence.
2. Stores mechanical facts.
3. Generates normative delta proposals.
4. Classifies inferences.
5. Requests minimal confirmation.
6. Creates or updates records.
7. Updates bindings.
8. Creates assessments for the final revision.
9. Records explicit supersessions.
10. Redacts or restricts sensitive evidence.

## 23.4 On future changes

When another agent modifies the area, it:

1. Resolves the current implementation.
2. Checks revision and coverage.
3. Finds the subject.
4. Retrieves approved active decisions.
5. Evaluates linkage and applicability.
6. Selects relevant knowledge.
7. Builds a compact packet.

## 23.5 Changes outside the flow

If the code changes without `prepare_change` or `finalize_change`, Rationale will detect it at the next query boundary at the latest, even if no hook worked.

```text
The linked implementation changed after the last evaluation.

Decision state: active, not revalidated.
Linkage: stale.
Last processed revision: abc123.
Current revision: def456.
Detection source: query-time revision gate.
```

Process:

1. Compare `HEAD`, the working tree, and the last processed revision.
2. Get modified targets through Git and the provider when available.
3. Mark only related assessments as `stale` or `unknown`.
4. Keep the historical Record intact.
5. Serve baseline constraints with a warning when it is safe.
6. Run advisory revalidation or request review when the change may be conceptual.

A hook or daemon can run the first three steps earlier, but it is not required to detect the inconsistency.

Rationale will not automatically declare that the decision stopped applying.

---

# 24. MCP tools


The v1 public surface must be small to reduce selection errors and unnecessary calls.

## `prepare_change`

Main tool.

Input:

* Goal or targets.
* Optional intent.
* Optional symptoms, reproduction, and expected result.
* Scope or workspace.
* Revision.
* Budget.
* Mode: `baseline` or `intent-aware`.

Output:

* Consistency snapshot.
* Constraints.
* Conflicts.
* Decisions.
* Risks.
* Relationships.
* Authority.
* Trust.
* Validity.
* Coverage.

---

## `explain_target`

Answers:

* Why a target exists.
* What decisions govern it.
* What part is known, inferred, or unknown.
* What evidence and authority exist.

---

## `finalize_change`

Consolidates the work.

* Records mechanical facts.
* Proposes new decisions.
* Updates bindings and assessments.
* Requests minimal confirmations.

---

## `review_record`

Allows:

* Approving.
* Correcting.
* Disputing.
* Revoking.
* Superseding.
* Assigning authority and scope.

---

## `trace_rationale`

Walks:

```text
Code
→ subject
→ record
→ problem
→ decision
→ evidence
→ approval
→ assessment
→ history
```

---

## `health`

Checks:

* Git revision.
* Working tree.
* Structural provider.
* Index and generation.
* Coverage.
* Stale bindings.
* Outdated assessments.
* Schema errors.

---

## 24.1 Automatic context surfaces

The public tool surface stays small, but compatible clients can internally invoke a baseline preflight on events such as:

* Symbol search.
* File reads.
* Starting an edit.
* Diff generation.
* Task completion.

The baseline packet must be extremely small:

```text
Target is governed by 1 critical approved constraint.
Staff users must never receive global super_admin.
Assessment is stale since revision abc123; verify before behavioral changes.
```

When there is an explicit intent, the client must prefer `prepare_change` in intent-aware mode.

The automatic surface must apply these rules:

* Do not run the full Subject Resolver.
* Do not create Records or Subjects.
* Do not modify authority or applicability.
* Do not call an LLM.
* Do not compute embeddings.
* Do not block on timeout or missing cache.
* Do not repeat the same constraint on every read in a session.
* Deduplicate by target, constraint, revision, and session window.
* Allow explicit expansion when the agent needs evidence or history.

The exact ability to intercept events depends on each IDE or agent and is not part of the universal MCP guarantee.

## 24.2 Internal or administrative operations

The following capabilities still exist, but they do not need to be exposed to the agent as independent public tools:

```text
get_constraints
capture_signal
investigate_history
check_drift
repair_links
revalidate_record
supersede_record
find_conflicts
```

They will be implemented as:

* Internal operations of `prepare_change` or `finalize_change`.
* Administrative CLI subcommands.
* Library functions.

This keeps all the original capabilities without forcing the model to coordinate twelve different tools.

---

# 25. Initial CLI


```bash
rationale init

rationale health

rationale add \
  --subject authorization.entity-scoped-staff-access \
  --target src/auth/authorization.ts::resolveEntityRole

rationale why \
  src/auth/authorization.ts::resolveEntityRole

rationale prepare \
  src/auth/authorization.ts::resolveEntityRole \
  --intent "Allow support users to access every entity" \
  --revision HEAD

rationale finalize \
  --base abc123 \
  --head def456

rationale review \
  constraint.no-global-admin-for-staff

rationale trace \
  constraint.no-global-admin-for-staff

rationale investigate \
  src/auth/authorization.ts

rationale drift

rationale repair-links

rationale revalidate \
  constraint.no-global-admin-for-staff

rationale supersede \
  decision.old-auth-model \
  --by decision.new-auth-model
```

The CLI will not be the main everyday interface.

It will be used for:

* Bootstrap.
* Diagnostics.
* CI.
* Administration.
* Automation.
* Recovery when the MCP client is not available.

Everyday human interaction must be possible from the IDE chat.

---

# 26. Storage


Portable source for a repository, including a monorepo:

```text
.rationale/
├── project.yaml
├── records/
├── subjects/
├── approvals/
├── evidence/
├── scopes/
├── schemas/
└── config.yaml
```

A full folder per package is not required. Scopes and `applies_to` express reach within the repository. A package may contain auxiliary files or local references only when there is a concrete reason, but the canonical source stays at the project root.

Local index:

```text
~/.cache/rationale/
└── projects/
    └── <project-id>/
        ├── repositories.db
        ├── scopes.db
        ├── index.db
        ├── bindings.db
        ├── assessments.db
        ├── retrieval.db
        └── state.json
```

Two computers can have the same canonical memory and different bindings or coverage reports because of versions, platform, working tree, or structural index state. Every response must declare that local reality.

## 26.1 Versioned files

* YAML or JSON.
* Reviewable in PRs.
* Shareable.
* Portable.
* Independent of the index.
* One file per record to reduce conflicts.
* Stable IDs.
* No embeddings or caches.

## 26.2 Derived index

* SQLite.
* Regenerable.
* Optimized.
* Not necessarily versioned.
* Invalidated by revision, schema, or provider generation.

## 26.3 Persistence and collaboration layers

| Layer | Content | Shared | Regenerable |
|---|---|---:|---:|
| Canonical | Subjects, Records, Binding declarations, Approvals, supersessions | Yes, through Git | No |
| Derived | Binding resolutions, assessments, FTS, caches | Not by default | Yes |
| Ephemeral | Intent, hypotheses, session signals | No | Yes / disposable |

Reviews of Records and Approvals can happen through PRs. Not having Rationale on a computer does not prevent modifying the repository, but it does reduce the assisted capture available in that session.

## 26.4 Stable declarations and mutable assessments

Stable declaration:

```yaml
record:
  id: constraint.no-global-admin-for-staff
  statement: Staff users must not receive global super_admin.
  created_at: 2026-07-23
  provenance: ...
  approvals: ...
```

Derived assessment:

```yaml
assessment:
  record_id: constraint.no-global-admin-for-staff
  applicability: active
  linkage: current
  assessed_revision: def456
  provider_generation: 184
```

The first represents the historical decision.

The second can be recomputed without rewriting the original meaning.

## 26.5 Visibility and sensitivity

```yaml
visibility: repository | local | restricted
sensitivity: public | internal | confidential | security
```

Policies:

* Secret scanning before versioning.
* Sensitive patches are not stored by default.
* Complete conversations are not copied.
* External references are preferred over duplication.
* Export and MCP respect visibility.

## 26.6 External references

```yaml
evidence:
  type: external-reference
  source: jira
  id: AUTH-184
  content_hash: sha256:...
  visibility: restricted
```

This keeps provenance without publishing the full content.

---

# 27. Updated record example


```yaml
schema_version: rationale/0.4

id: constraint.no-global-admin-for-staff
project_id: boost
kind: constraint
title: Staff accounts must not receive global super-admin
severity: critical
scope: project
applies_to:
  - package:npm:@boost/api
  - package:npm:@boost/dashboard

subject:
  id: authorization.entity-scoped-staff-access
  type: system-behavior
  title: Entity-scoped staff authorization
  aliases:
    - auth.staff-per-entity

statement: >
  Staff users must never receive global super_admin.

constraint_expression:
  subject: authorization.staff
  predicate: must_not_have
  object: role.global_super_admin

rationale: >
  Access to multiple entities must not imply global administration.

problem:
  statement: >
    Staff users who required access to multiple entities were assigned
    the global super_admin role.

  symptoms:
    - Staff could access unrelated entities.
    - Multi-entity access implied system-wide privileges.
    - Authorization intent was not represented by assignments.

intent:
  primary: >
    Move staff authorization to explicit entity-scoped assignments.

  non_goals:
    - Replace the entire authorization architecture.
    - Remove global access from project owners.
    - Delete historical authorization records.

decisions:
  - id: decision.staff-access-is-entity-scoped
    statement: >
      Staff access must be assigned independently for each entity.

exceptions:
  - id: exception.project-owner-global-access
    statement: >
      Users 1, 2 and 3 retain global super_admin.

risks:
  - id: risk.staff-without-assignment
    statement: >
      Staff without entity assignments may lose access.
    epistemic_status: corroborated

provenance:
  created_by:
    type: human
    actor: user:rolando
  created_at: 2026-07-23T18:20:00Z

approvals:
  - actor: user:security-owner
    authority: security-owner
    domain: authorization
    status: approved
    approved_at: 2026-07-23T19:00:00Z

binding_declarations:
  - id: binding.authorization.resolve-entity-role
    type: symbol
    provider: codebase-memory
    structural_id: function:typescript:auth.resolveEntityRole
    path_hint: apps/api/src/auth/authorization.ts
    scope: package:npm:@boost/api

  - id: binding.authorization.entity-user-roles
    type: database-table
    target_id: entity_user_roles
    scope: package:npm:@boost/api

  - id: binding.authorization.remove-staff-super-admin
    type: migration
    path_hint: apps/api/database/migrations/remove_staff_super_admin.sql
    introduced_revision: 91ac21f

  - id: binding.authorization.staff-access-test
    type: test
    target_id: staff_cannot_access_unassigned_entity
    scope: package:npm:@boost/api

validation:
  - type: integration-test
    statement: Owners retain global access.
    result: passed
    revision: def456

  - type: migration-reexecution
    statement: The migration is idempotent.
    result: passed
    revision: def456

claims:
  - id: claim.staff-previously-received-global-access
    statement: >
      Staff previously received global access.
    epistemic_status: observed
    evidence:
      - type: source-code
        revision: 91ac21f^
      - type: migration
        path: database/migrations/remove_staff_super_admin.sql
        revision: 91ac21f

applicability_policy:
  superseded_by: null
  review_conditions:
    - Staff access becomes globally inherited.
    - Entity assignments stop governing authorization.
    - A new authorization model is explicitly approved.

binding_policy:
  structural_refactors: repair-silently
  behavioral_changes: revalidate
  conceptual_conflicts: advisory-unless-exact

context_policy:
  priority: critical
  always_include:
    - constraint.no-global-admin-for-staff
  max_default_tokens: 250

sensitivity:
  classification: internal
  visibility: repository
```

Derived assessment for the current revision:

```yaml
schema_version: rationale-assessment/0.4

record_id: constraint.no-global-admin-for-staff
repository_id: boost

snapshot:
  git_revision: def456
  structural_provider: codebase-memory
  structural_revision: def456
  structural_generation: 184
  rationale_revision: a82cf1
  consistency: exact

binding_resolutions:
  - binding_id: binding.authorization.resolve-entity-role
    resolved_target: function:typescript:auth.resolveEntityRole
    resolved_revision: def456
    provider_version: 0.9.x
    provider_generation: 184
    coverage: complete
    status: current

state:
  epistemic: stated
  authority: approved
  applicability: active
  linkage: current

assessment_reason: >
  Current bindings and tests still implement entity-scoped authorization.

assessed_at: 2026-07-24T10:00:00Z
```

---

# 28. Repository architecture


The initial implementation must be a modular monolith in Rust.

```text
rationale/
├── crates/
│   ├── rationale-core/
│   │   ├── records/
│   │   ├── subjects/
│   │   ├── trust/
│   │   ├── policy/
│   │   └── lifecycle/
│   │
│   ├── rationale-storage/
│   │   ├── portable/
│   │   ├── sqlite/
│   │   └── schemas/
│   │
│   ├── rationale-providers/
│   │   ├── codebase_memory/
│   │   └── git/
│   │
│   └── rationale-app/
│       ├── mcp/
│       ├── cli/
│       ├── retrieval/
│       ├── capture/
│       └── revision/
│
├── schemas/
│   ├── record.schema.json
│   ├── subject.schema.json
│   ├── approval.schema.json
│   ├── evidence.schema.json
│   ├── assessment.schema.json
│   └── context-packet.schema.json
│
├── specification/
│   ├── context-model.md
│   ├── trust-and-authority.md
│   ├── revision-consistency.md
│   ├── provider-contract.md
│   ├── security-model.md
│   ├── retrieval-budget.md
│   └── lifecycle.md
│
└── examples/
    ├── authorization/
    ├── payments/
    ├── migrations/
    └── legacy-investigation/
```

## 28.0 Limit of this document

This section expresses conceptual constraints; it does not yet close implementation decisions such as:

* The exact runtime of the optional daemon.
* The IPC format.
* The file watching strategy.
* The concrete integration with each IDE.
* The propagation algorithm across workspaces.
* The final physical SQLite schema.

Those decisions will belong to the specific architecture document. v0.4 only requires that the future architecture respect scopes, coherent revision, shared/local layers, and mechanisms that do not depend on hooks to be correct.

## 28.1 Language

Rust is recommended because it offers:

* A distributable local binary.
* Strong typing for states and schemas.
* Memory safety.
* Good support for SQLite, Git, and MCP.
* Controlled concurrency.
* Cross-platform compatibility.

Using C is not necessary even though Codebase Memory is written mainly in C.

The right boundary is the protocol and the adapter, not sharing a language or a process.

## 28.2 Why not eleven crates

The original architecture split every engine into a crate.

That may be useful later, but it adds:

* Premature internal APIs.
* Compile time.
* Dependency complexity.
* Domain fragmentation.
* Difficulty changing the model during the MVP.

v1 will keep logical separation without excessive physical separation.

## 28.3 Embeddings

v1 will not depend on its own embeddings.

It will first use:

1. Exact bindings.
2. The provider's structural graph.
3. Conceptual scope.
4. Local FTS.

Embeddings will be evaluated later as a retrieval fallback and as a signal for identity candidates. They will never be the only basis for retrieving critical policies, merging Subjects, or deciding scope.

---

# 29. Version plan and MVP

The conceptual version of this document and the implementation versions describe different things. `Document 0.5` means the conceptual contract was revised five times; `product 0.1`, `0.2`, or `0.5` represent future milestones of the software.


## Version 0.0 — Validation experiment

Goal:

> Demonstrate that Rationale clearly outperforms a traditional ADR and Codebase Memory without Rationale on real tasks.

It must include:

* 20 to 30 historical changes.
* Two or three repositories, including at least one real monorepo with several packages.
* The work monorepo selected as a controlled pilot, using only information authorized for the test.
* Cases involving authorization, migrations, payments, contracts between packages, refactors, and regressions.
* Controlled manual records.
* Evaluation with and without preflight.
* Per-case ground truth prepared before evaluating the packets.
* Paired comparison between code/Git, traditional documentation, Codebase Memory, and Codebase Memory + Rationale.
* An optional condition with a prompt written by a person with domain experience.
* Several runs per condition when cost allows.
* Blind evaluation of the Context Packets and results.
* Recording of tokens, tool calls, files opened, latency, attempts, tests, and final result.
* Explicit measurement of the manual context written by the person.
* Local instrumentation that does not send code, prompts, or work information without authorization.
* Analysis of failures and of cases where Rationale made the result worse.

It does not yet require a distributable product.

The goal of 0.0 is not to optimize a final implementation. It is to answer:

```text
Does structured causal context materially change the quality,
cost, or safety of real tasks compared with simpler alternatives?
```

If the answer is negative or marginal, it must not be dressed up with the density metric. The retrieval, the capture, the product model, or the need for the tool will have to be revised.

---

## Version 0.1 — Trusted context preflight

Goal:

> Prevent or warn about a dangerous change by delivering a relevant, approved, consistent, and compact decision.

It must include:

* Codebase Memory integration.
* Health and exact revision.
* Subjects.
* Manual Records.
* Structural bindings.
* Evidence.
* Approvals.
* Assessments.
* `prepare_change`.
* `explain_target`.
* Context budget.
* No aggressive automatic alerts.
* Conceptual and retrieval support for workspaces/packages within a single Git repository.
* A revision gate on every query to detect commits outside the flow.
* Baseline mode when there is no explicit intent.
* Shared canonical memory and a regenerable local cache.
* No embeddings of its own.
* No CI blocking.

It must not yet include:

* Full import of legacy projects.
* Perfect lineage reconstruction.
* Slack integrations.
* Automatic inference of reasons.
* Distributed synchronization.
* A stable open protocol.

---

## Version 0.2 — Assisted capture

Add:

* Capture from Git.
* Modified symbols.
* Tests run.
* Decision proposals.
* Selective human confirmation.
* `finalize_change`.
* Mechanical evidence.
* Mandatory Subject Resolver before creating concepts.
* Duplicate detection and `novelty_reason`.
* Per-domain authority.
* Sensitivity and redaction.
* Capture of cross-package changes in the monorepo.

---

## Version 0.3 — Drift and legacy

Add:

* Change classification.
* Possible conceptual drift.
* On-demand archaeology.
* Git blame.
* Retrieval from PRs and issues.
* Advanced binding repair.
* Lineage through splits and moves.
* Declaration of shallow history and gaps.
* Optional hooks or daemon to detect drift earlier.
* Post-change audit when the preflight was skipped.

---

## Version 0.4 — Team workflows

Add:

* PR integration.
* Record review.
* Per-repository rules.
* CODEOWNERS or authority policies.
* Coverage per area.
* Conflict reports.
* Suggested validations.
* Agent and IDE hooks where they exist.
* Final diff review.
* Collaborative review of Subjects and conceptual collisions.
* Experimental import or references across repositories.

---

## Version 0.5 — Critical policies

Add, as opt-in:

* Deterministic rules.
* CI for approved critical constraints.
* Two approvals in configured domains.
* Audit of policy changes.
* Temporary exceptions.

---

## Version 1.0 — Stable context model

Add:

* Stable schema.
* SDK.
* Conformance tests.
* Portable context packets.
* Multi-repository.
* Proven external adapters.
* Formal provenance and authority model.
* Compatibility policy.

The name "Open protocol" must only be used when there is real interoperability with more than one implementation or consumer.

---

# 30. Success metrics


## Utility

* How many times a relevant constraint was retrieved.
* How many dangerous changes were warned about.
* How many queries avoided extensive manual reading.
* How many historical regressions were avoided.

## Precision

* Percentage of alerts considered useful.
* Percentage of inferences confirmed.
* Number of false blocks.
* Number of disputed records.
* Percentage of conflicts that were really normative.

## Noise

* Alerts per change.
* Ignored alerts.
* Refactors processed silently.
* Redundant records removed from the packet.
* Confirmations requested per change.

## Context

* Average tokens per `prepare_change`.
* 95th percentile of tokens.
* Number of candidate records.
* Number of records delivered.
* Reduction compared with retrieving the whole history.

## End-to-end cost

* Total tokens until the task is solved.
* Number of files opened.
* Number of tool calls.
* Time to solution.
* Failed attempts.
* Later corrections.

## Friction

* Human time required per record.
* Average number of confirmations.
* Percentage of changes with no human interaction.
* Percentage of abandoned records.
* Time between proposal and approval.

## Validity

* Bindings repaired automatically.
* Records without a binding.
* Decisions correctly superseded.
* Time between a conceptual change and its review.
* Outdated assessments.

## Authority

* Critical constraints without approval.
* Approvals per domain.
* Conflicts between authorities.
* Policies correctly revoked.

## Consistency

* Responses served with an exact revision.
* Queries degraded by a lagging index.
* Cases where a stale response was avoided.
* Errors caused by provider incompatibility.

## Security

* Records rejected by schema.
* Prompt injection attempts neutralized.
* Secrets detected before versioning.
* Restricted evidence excluded from the packet.

## Monorepo and continuity

* Cross constraints correctly retrieved between packages.
* Percentage of cross-workspace inclusions with an explainable relevance path.
* Irrelevant context introduced by scope inheritance.
* Commits outside the flow detected on the next query.
* Duplicate Subjects prevented or sent to review.
* Time to rebuild the local index from the shared canonical layer.

## Context quality

* Useful context density per token.
* Critical information omitted.
* Records included without changing the agent's decision.
* Comparison between baseline and intent-aware.
* Reduction of repetitive manual prompt lines.

## Initial pilot targets

* At least 90% of critical constraints retrieved.
* Zero false blocks during the pilot.
* More than 80% of warnings rated as useful.
* Less than 90 human seconds to approve an important record.
* One or two confirmations per change at most.
* Less than 600 tokens at the median.
* Less than 1,000 tokens at the 95th percentile.
* Never serve a result as current when the index is behind.

## 30.1 Empirical evaluation protocol

Rationale's evaluation must be reproducible, auditable, and able to refute the product hypothesis.

The main unit of evaluation will be the **exact Context Packet delivered to an agent for a concrete task**, not the total number of Records or the perceived quality of the whole document.

### 30.1.1 Experimental unit

Every case must fix:

```yaml
case:
  id: auth-remove-global-admin
  repository_id: pilot-monorepo
  base_revision: abc123
  allowed_sources:
    - source-code
    - git-history
    - approved-rationale-records

  task:
    statement: Allow support users to access several entities.
    symptoms: null
    expected_result: null

  target_scope:
    - apps/api
    - packages/authorization

  evaluation_policy:
    context_budget_tokens: 900
    max_tool_calls: null
    timeout_seconds: null
```

The task must start from the same revision and comparable conditions for all variants.

### 30.1.2 Case ground truth

Before evaluating the packets, a reference sheet will be prepared:

```yaml
ground_truth:
  must_know:
    - id: gt.no-global-admin-for-staff
      statement: Multi-entity access must not imply global administration.
      importance: critical

    - id: gt.owner-exceptions
      statement: Project owners are deliberate exceptions.
      importance: critical

    - id: gt.entity-assignments
      statement: Access must be represented through entity assignments.
      importance: high

  useful:
    - id: gt.idempotent-migration
      statement: The migration was designed to be idempotent.
      importance: medium

  irrelevant:
    - statement: The authorization service was renamed months earlier.

  dangerous_falsehoods:
    - statement: Support users require a global role for multi-entity access.
```

The ground truth can be built from:

* The diff of the historical solution.
* The original issue or requirement.
* The pull request and its comments.
* Commits.
* Tests and migrations.
* Incidents.
* Documentation available at the time.
* Review by a person with domain knowledge.

It must record uncertainty and disagreement. If two experts do not agree, the case is not artificially forced into a single truth; it is marked as disputed or excluded from metrics that require certainty.

The ground truth must not leak to the agent outside the corresponding experimental condition.

### 30.1.3 Operational formula for Context Utility Density

For a packet `P` with items `i`:

```text
utility(i) =
    relevance(i)
  × reliability(i)
  × actionability(i)
  × applicability(i)
  × importance(i)
  × uniqueness(i)
```

```text
context_utility_density(P) =
    1000 × sum(utility(i)) / max(tokens(P), 1)
```

The result is interpreted as **weighted utility per thousand tokens**.

The multiplication is deliberately strict: a highly relevant but false, obsolete, or non-actionable item must not keep a high score. During the pilot the components will also be stored separately to check that the formula does not hide the reason for a result.

This formula is not a universal law. Its weights and scales are an initial hypothesis that must be subjected to sensitivity analysis.

### 30.1.4 Scoring scales

#### Relevance

Question:

> Was this item necessary or directly useful to solve the task?

```text
1.00  necessary for a safe or correct solution
0.75  very useful and materially reduces the search
0.50  useful, but not essential
0.25  weak or contextual relationship
0.00  irrelevant
```

Relevance is evaluated against the case and its ground truth, not only through semantic similarity.

#### Reliability

Question:

> Is the content correct and backed for this case?

Initial guideline values:

```text
1.00  mechanically observed and verified for the revision
1.00  approved policy confirmed as correct
0.90  approved and backed human claim
0.75  corroborated by independent sources
0.40  reasonable inference
0.15  hypothesis
0.00  false, contradicted, or fabricated
```

Provenance does not guarantee correctness. An approved human claim can receive a lower score if the case shows that it was wrong or stopped applying.

#### Actionability

Question:

> Does this item change or improve a concrete action of the agent?

```text
1.00  determines a necessary constraint, solution, or validation
0.75  considerably narrows the solution space
0.50  guides a useful investigation
0.25  provides general understanding without changing the action
0.00  does not affect any decision
```

#### Applicability or freshness

It does not mean chronological age.

Question:

> Does it still govern the evaluated revision?

```text
1.00  confirmed for the current revision
0.75  probably active with partially degraded linkage
0.50  unknown applicability
0.25  signs of supersession or drift
0.00  superseded, invalid, or out of scope
```

#### Importance

```text
1.00  critical constraint or severe risk
0.80  important normative decision
0.60  operational risk
0.40  useful historical reason
0.20  auxiliary context
```

Importance must come from the ground truth or a rubric prepared before observing which condition produced the packet.

#### Uniqueness

Penalizes repetition and redundant paraphrasing:

```text
1.00  provides new information
0.50  partially overlaps with another item
0.10  almost entirely redundant
0.00  exact duplicate
```

### 30.1.5 Context Packet segmentation

To score consistently, the packet is broken down into minimal semantic units:

* One constraint.
* One decision.
* One risk.
* One consistency warning.
* One causal claim.
* One suggested validation.
* One actionable structural relationship.

A single claim must not be split into many sentences to inflate the utility sum.

The instrumentation will keep:

```yaml
context_item_evaluation:
  packet_id: packet-123
  item_id: constraint.no-global-admin-for-staff
  token_count: 17
  relevance: 1.0
  reliability: 0.9
  actionability: 1.0
  applicability: 1.0
  importance: 1.0
  uniqueness: 1.0
  evaluator_notes: Prevents the historical regression directly.
```

### 30.1.6 Comparison conditions

At a minimum, the same case will be tested under:

#### Condition A — Code and Git

Normal access to the repository and its basic tools.

#### Condition B — Traditional documentation

Code, Git, `AGENTS.md`, ADRs, and available documentation.

#### Condition C — Codebase Memory

Code, Git, and structural context retrieved through Codebase Memory.

#### Condition D — Codebase Memory + Rationale

The complete proposed experience.

#### Condition E — Expert human prompt, optional

A person with domain knowledge manually provides the context they would normally explain to the agent.

This condition is not a trivial competitor. It serves as an approximation of how much valuable institutional knowledge Rationale manages to keep and how much still depends on a specific person.

The conditions must use:

* The same model and version.
* The same initial revision.
* The same task.
* Equivalent configurations.
* Comparable budgets.
* Memory reset or isolation between runs.

When a variable cannot be matched exactly, it must be recorded as a limitation.

### 30.1.7 Layer 1: context quality

#### Critical Constraint Recall

```text
critical_constraint_recall =
    critical constraints delivered
    / critical constraints required
```

This metric takes priority over average density. Omitting a single critical rule can invalidate a seemingly efficient packet.

#### Context Precision

```text
context_precision =
    useful delivered items
    / total delivered items
```

#### Harmful Context Rate

```text
harmful_context_rate =
    false, contradictory or inapplicable items
    / total delivered items
```

#### Weighted Context Recall

```text
weighted_context_recall =
    sum(importance of correctly delivered ground-truth items)
    / sum(importance of all required ground-truth items)
```

#### Redundancy Rate

```text
redundancy_rate =
    redundant context tokens
    / total context tokens
```

The following will also be recorded:

* Critical information omitted.
* True but useless context.
* Evidence not available.
* Items whose assessment was stale.
* Tokens consumed by trust and authority metadata.

### 30.1.8 Layer 2: task outcome

Packet quality is not enough. Every run must be evaluated by:

* Correct or incorrect solution.
* Constraints respected.
* Historical bug reintroduced.
* Existing tests passed.
* Appropriate new tests.
* Attempts needed.
* Files opened.
* Tool calls.
* Total input and output tokens.
* Time to an acceptable solution.
* Later corrections.
* Human interventions.

Central cost metric:

```text
total_tokens_to_successful_completion
```

Not only:

```text
tokens_returned_by_prepare_change
```

When a run does not reach a correct solution within the set limit, it must be recorded as censored or failed; its low token consumption cannot be counted as a win.

### 30.1.9 Manual context reduction

The product promise includes reducing how much institutional knowledge the person must repeat.

```text
manual_context_reduction_tokens =
    manual context tokens without Rationale
    - manual context tokens with Rationale
```

```text
manual_fact_reduction =
    project facts manually supplied without Rationale
    - project facts manually supplied with Rationale
```

The following will also be measured:

* Time preparing the prompt.
* Number of clarifications.
* Number of times the person had to point to a file or module.
* Number of constraints they had to repeat.
* Manual context that Rationale supplied correctly.
* Context that Rationale could not know and the person still had to provide.

The goal is not to eliminate the prompt. The person keeps expressing what they want to achieve, new symptoms, priorities, and unrecorded constraints.

### 30.1.10 Continuity with respect to senior knowledge

For suitable cases, an experienced person in the domain will prepare an independent list of:

* Precautions.
* Constraints.
* Components they would investigate.
* Tests they would require.
* Historical mistakes they would avoid.

Then the following will be measured:

```text
senior_context_recall =
    supported senior-context items retrieved by Rationale
    / supported senior-context items identified by the reviewer
```

```text
unsupported_advice_rate =
    Rationale recommendations without supporting evidence
    / total Rationale recommendations
```

This does not claim that the list captures a senior person's whole mind. It evaluates how much verifiable, transferable knowledge the project manages to preserve.

### 30.1.11 Minimum instrumentation

Every run must produce a structured local record:

```yaml
experiment_run:
  run_id: run-001
  case_id: auth-remove-global-admin
  condition: rationale
  model:
    provider: configured-provider
    model_id: exact-version
    temperature: 0

  snapshot:
    base_revision: abc123
    provider_version: x.y.z
    provider_generation: 441
    rationale_schema: rationale/0.5

  context:
    packet_id: packet-123
    mode: intent-aware
    tokens: 542
    latency_ms: 184
    cache_state: warm
    degraded: false

  execution:
    started_at: ...
    completed_at: ...
    input_tokens_total: 10340
    output_tokens_total: 2240
    tool_calls: 12
    files_read: 7
    attempts: 1

  outcome:
    task_success: true
    constraints_respected: true
    historical_regression: false
    tests_passed: true
    evaluator_score: null
```

The exact implementation of the harness belongs to the experiment document, but the conceptual contract requires capturing this data.

### 30.1.12 Pilot privacy

For a work repository:

* Only authorized data and revisions will be used.
* Telemetry will be local by default.
* No code, prompts, diffs, Records, or results will be sent to additional services without authorization.
* Reports may use IDs and aggregated metrics.
* Sensitive cases may keep only hashes, categories, and results.
* A future public dataset must be built with open repositories or equivalent synthetic cases.

### 30.1.13 Blind evaluation and bias reduction

When possible:

* Packets will be presented without indicating which condition produced them.
* Evaluators will use a common rubric.
* Two evaluators will score a sample.
* Disagreements and inter-rater agreement will be measured.
* The ground truth will be closed before observing aggregated results.
* Cases will not be selected only because they favor Rationale.
* Negative results will also be kept.

The person who built a Rationale Record should not be the only evaluator of its usefulness.

### 30.1.14 Statistical analysis

With 20 to 30 cases, the pilot will not universally prove that Rationale works for every kind of repository.

It must use:

* Paired comparisons on the same tasks.
* Medians and percentiles, not only averages.
* Bootstrap confidence intervals when appropriate.
* Distributions by type of change.
* Separate results for monorepos and simple repositories.
* Separate results for baseline and intent-aware.
* Sensitivity analysis of the CUD weights.
* Reporting of practical effect sizes.

It must not use a statistically noisy difference as a definitive commercial claim.

### 30.1.15 Initial success criteria

Pilot targets:

```text
critical_constraint_recall >= 90%
context_precision >= 80%
harmful_context_rate < 2%
ideal harmful_context_rate = 0%
false_blocking_rate = 0%
median_context_packet <= 600 tokens
p95_context_packet <= 1000 tokens
manual_prompt_context_reduction >= 50%
```

In addition:

* `total_tokens_to_successful_completion` must improve materially against one or more control conditions, or justify any increase through a clear improvement in safety or success.
* The task success rate must practically exceed simple alternatives in cases where relevant causal context exists.
* Baseline latency must respect the defined experimental budget.
* A stale assessment must not be served as exact.
* The benefits must persist in the pilot monorepo and not only in small artificial cases.

These thresholds may be adjusted before the pilot starts, but not after observing results just to declare victory.

### 30.1.16 Falsification rules

Rationale does not pass the pilot when any of these conditions occurs:

* It retrieves a lot of correct context but does not improve decisions or results.
* It reduces packet tokens but increases total tokens or tool calls without material benefit.
* It frequently omits critical constraints.
* It introduces falsehoods or superseded rules.
* It requires as much human maintenance work as the context it aims to save.
* It only works when its creator manually prepares every perfect case.
* Traditional documentation offers equivalent results with much less complexity.
* The baseline adds perceptible latency without providing context that is used.

A negative result does not necessarily invalidate the problem. It may indicate that the capture, retrieval, integration, or product model must be simplified.

### 30.1.17 Final interpretation

Rationale may only be declared superior to ADRs, `AGENTS.md`, or Codebase Memory alone if its packets:

1. Retrieve more correct critical knowledge.
2. Deliver less noise and fewer falsehoods.
3. Produce better or safer solutions.
4. Reduce the total cost or the manual context required.
5. Keep an acceptable latency for the surface where they are used.

Utility density is an explanation of **why** a packet was efficient. The task outcome determines **whether it really was**.

---

# 31. Project risks


## Risk: false explanations

Mitigation:

* Clearly marked inferences.
* Mandatory evidence.
* Selective confirmation.
* Never block with inferred knowledge.
* Allow the `unknown` state.

## Risk: wrong authority

Mitigation:

* Separate provenance from authority.
* Per-domain approval policy.
* CODEOWNERS or explicit configuration.
* Revocation and audit.
* Double approval in critical domains.

## Risk: alert fatigue

Mitigation:

* Reduced multidimensional state.
* Change classification.
* Silent repair.
* Intent-based alerts.
* Only critical blocks.
* Interruption budget.

## Risk: confirmation fatigue

Mitigation:

* Confirm only normative deltas.
* One concrete claim per interaction.
* Do not pre-approve critical constraints.
* Allow editing and rejection.
* Do not create a record if the reason is unknown.

## Risk: too much friction

Mitigation:

* Automatic capture.
* Minimal confirmations.
* Allow an unknown reason.
* Capture levels.
* Do not document trivial changes.

## Risk: legacy repositories

Mitigation:

* Forward capture.
* On-demand archaeology.
* Prioritization.
* Accepted partial coverage.
* Declaration of shallow history and gaps.

## Risk: symbol fragility

Mitigation:

* Conceptual identity.
* Multiple bindings.
* Lineage.
* Git.
* Tests.
* Tables.
* Routes.
* Linkage separate from applicability.

## Risk: conceptual duplication

Mitigation:

* Aliases.
* Merge candidates.
* Explicit split.
* No automatic merging.
* Identity history.

## Risk: context saturation

Mitigation:

* Context budget.
* Ranking.
* Deduplication.
* Progressive disclosure.
* Priority-based summaries.

## Risk: revision inconsistency

Mitigation:

* Revision coordinator.
* Mandatory snapshot.
* Reject or degrade results.
* Per-revision and per-generation cache.
* Health tool.

## Risk: Codebase Memory errors or gaps

Mitigation:

* Record version, coverage, and warnings.
* Do not assume absence means nonexistence.
* Fall back to Git or source when appropriate.
* Adapter contract tests.
* Capability negotiation.

## Risk: prompt injection in records

Mitigation:

* Treat content as non-executable data.
* Strict schema.
* Declarative fields.
* Packet sanitization.
* Separation of evidence and instructions.
* Free-text limits.

## Risk: secrets and sensitive information

Mitigation:

* Sensitivity classification.
* Visibility.
* Secret scanning.
* External references.
* Redaction.
* Do not cache sensitive patches by default.

## Risk: the agent does not call Rationale

Mitigation:

* Baseline target context in compatible integrations.
* Intent-aware `prepare_change`.
* Client instructions.
* Optional, non-blocking hooks.
* Revision gate independent of the hook.
* Diff review when finishing.
* CI only for deterministic critical policies.
* Health check before operating in configured domains.

## Risk: not saving tokens

Mitigation:

* Adaptive activation.
* Small packets.
* End-to-end measurement.
* Comparison against ADRs and direct reading.
* Do not use Rationale for trivial changes.

## Risk: Codebase Memory absorbs the function

Mitigation:

* Differentiate on authority, provenance, applicability, and preflight.
* Consume the provider's ADRs and history as evidence.
* Keep a portable model.
* Support future providers.

## Risk: scope leakage in monorepos

Mitigation:

* Explicit hierarchical scopes.
* `applies_to` and `excludes`.
* Relevance path on every cross inclusion.
* Irrelevant context metrics.
* Tests with frontend, backend, and shared packages.

## Risk: depending on hooks for correctness

Mitigation:

* Revision gate on every query.
* Hooks and daemon only as accelerators.
* Observable state when an integration did not run.
* CI as later verification, not a substitute for local freshness.

## Risk: wrong semantic deduplication

Mitigation:

* Deterministic resolution first.
* Embeddings only as candidates.
* `novelty_reason`.
* Auditable merge and split.
* Never block on isolated similarity.

## Risk: different assessments across computers

Mitigation:

* Canonical layer separate from the derived one.
* Snapshot with local version and coverage.
* Regenerable assessments.
* Do not version conclusions that depend on an incomplete index as global facts.

## Risk: abundant but barely useful context

Mitigation:

* Context utility density.
* Small baseline.
* Intent-aware retrieval.
* Progressive disclosure.
* Evals for omission and noise, not just token counts.

## Risk: promising to replace senior experience

Mitigation:

* Define the product as institutional continuity.
* Keep human authority.
* Show unknowns.
* Do not infer business priorities.
* Evaluate reduction of archaeology and regressions, not an abstract "senior level".

## Risk: optimizing for the metric

A system could artificially increase `context_utility_density` by delivering minimal packets that omit hard information, splitting items to inflate the sum, or calibrating weights after observing results.

Mitigation:

* Ground truth prepared in advance.
* Separate recall and harmful context metrics.
* Defined semantic segmentation.
* Sensitivity analysis.
* Freeze thresholds before the pilot.
* Give priority to the real task outcome.

## Risk: ground truth bias

The person who knows the historical solution may include only the knowledge Rationale already models, or confuse the final solution with the only valid solution.

Mitigation:

* Multiple sources.
* Independent review.
* Disputed states.
* Blind evaluation.
* Keep valid alternative solutions.
* Exclude cases where a reasonable reference cannot be established.

## Risk: generic `novelty_reason`

An agent can learn to pass the check by writing empty justifications or fabricating differences.

Mitigation:

* Structured schema.
* Mandatory compared candidates.
* Explicit contrast.
* Evidence when there is high similarity.
* Basic deterministic validation.
* Human review in critical domains.
* Measure duplicates discovered later.

## Risk: baseline latency

A useful but slow injection can interrupt frequent searches and reads until developers disable the tool.

Mitigation:

* Separate local fast path.
* Per-revision and per-generation cache.
* No LLM or embeddings.
* Strict deadline.
* Fail open.
* Per-session deduplication.
* Local telemetry for P50, P95, cold start, and timeouts.

## Risk: contamination between experimental conditions

The agent or evaluator can remember information from an earlier run and favor later conditions.

Mitigation:

* Isolated sessions.
* Random or counterbalanced order.
* Memory reset.
* Blind evaluation.
* Explicitly identify any unavoidable contamination.

## Risk: over-architecture

Mitigation:

* Experiment 0.0 before the full product.
* Six persisted entities.
* Six public tools.
* Four crates.
* Modular monolith.
* No embeddings of its own initially.

---

# 32. Definitive success criterion


Rationale will be successful if the following happens:

1. A new agent opens a repository without knowing earlier conversations.
2. It tries to modify an authorization system.
3. Rationale checks that Git, Codebase Memory, and assessments correspond to a coherent revision.
4. It identifies the related conceptual behavior.
5. It retrieves a decision approved by the appropriate authority.
6. It shows a critical constraint.
7. It explains why it exists and what evidence backs it.
8. It detects that the proposed intent may reintroduce an earlier problem.
9. It delivers the context in fewer than roughly a thousand tokens.
10. It does not show fifteen irrelevant historical records.
11. It does not raise a warning for a simple rename.
12. It does not block because of an inference or an incomplete index.
13. The agent can continue with a better-informed solution.
14. The total cost of solving the task improves compared with not using Rationale.
15. A relevant backend constraint can be retrieved when modifying a connected frontend package, showing the relevance path.
16. A direct human commit makes the corresponding assessment stale on the next query, even without a hook.
17. An agent cannot silently create a near-duplicate Subject without reusing it or justifying its novelty.
18. A new machine can rebuild the local context from the versioned canonical files.
19. If the agent does not declare an intent, it receives a small baseline without Rationale inventing the goal.
20. The tool reduces repetitive manual context without replacing the task's specific symptoms and requirements.

The ideal experience would be:

```text
You are modifying entity authorization.

Snapshot:
Git, structural index and rationale assessment match revision def456.

Critical approved constraint:
Staff users must never receive global super_admin.

Why:
This rule was introduced after multi-entity staff accounts received
system-wide privileges.

Authority:
Approved by the authorization security owner.

Your intent may conflict with this decision.

Known safe direction:
Grant explicit assignments for each entity.

Evidence:
Migration, integration tests and approved decision record.

Additional history:
2 records available.
```

## 32.0 Cross-workspace success case

```text
Task:
Update the dashboard role badge for support users.

Target:
apps/dashboard/src/users/RoleBadge.tsx

Cross-workspace context:
This UI consumes @boost/auth-contracts, which is governed by the project-level
entity-scoped staff authorization decision.

Critical constraint:
Do not represent support users as global administrators.

Why:
Multi-entity staff previously received system-wide privileges.

Relevant backend targets:
apps/api/src/auth/resolveEntityRole.ts
packages/auth-contracts/src/roles.ts

Context path:
RoleBadge → @boost/auth-contracts → authorization subject → approved constraint
```

The packet does not include the whole backend memory. It includes the single cross decision that changes how the task must be solved.

## 32.1 Minimum comparison to justify the product

Rationale must clearly outperform:

1. An agent without memory.
2. Traditional `AGENTS.md` or ADRs.
3. Codebase Memory with ADRs or history, without Rationale.
4. Manual reading of commits and PRs.

If it does not offer a clear improvement in safety, precision, friction, or total cost, the complete architecture is not justified.

---

# 33. Public definition


## Short description

**Rationale is an open-source project-context compiler and provenance layer for AI coding agents. It turns shared decisions, constraints, risks, and structural bindings into the smallest reliable context packet needed for a specific code change.**

## Spanish description

The product copy used for Spanish-speaking audiences:

**Rationale es un compilador open source de contexto del proyecto y una capa de procedencia para agentes de programación. Convierte decisiones, restricciones, riesgos y bindings estructurales compartidos en el paquete de contexto confiable más pequeño que necesita un cambio específico.**

## Relationship with Codebase Memory

**Codebase Memory understands what is connected. Rationale tells the agent which decisions still govern those connections—and why they are trusted.**

## Relationship with Git

**Git records what changed. Rationale preserves why the change still matters and whether the decision remains applicable.**

## Central phrases

> **Git remembers what changed. Rationale remembers why it still matters.**

> **Rationale does not remember everything. It remembers what still matters.**

> **Rationale compiles what the project learned into what the agent needs now.**

> **No explanation is better than a false explanation.**

## Technical definition

> **Rationale is a context compiler and software decision preflight with scopes, provenance, authority, structural bindings, and per-revision consistency.**

---

# 34. Long-term vision


Rationale will be the first concrete product within a broader vision:

```text
Project Cognition Protocol
└── Context Provenance Model
    └── Rationale Context Model
        └── Rationale CLI + MCP
```

## Rationale

Initial product focused on software changes.

## Rationale Context Model

Portable model for:

* Subjects.
* Records.
* Bindings.
* Evidence.
* Approvals.
* Assessments.
* Revision snapshots.

## Context Provenance Model

A more general model that defines:

* Origin.
* Transformations.
* Authority.
* Trust.
* Validity.
* Evidence.
* Invalidation.
* Sensitivity.

## Project Cognition Protocol

Future vision for preserving broader operational knowledge:

* Decisions.
* Incidents.
* Experiments.
* Migrations.
* Assumptions.
* Consequences.
* Lineage.

## Condition for calling it a protocol

It will not be declared stable until it has:

* At least two independent consumers or providers.
* Conformance tests.
* Versioning and migrations.
* Real multi-repository cases.
* A compatibility policy.

The implementation must start with the specific problem.

Not with the ambition of modeling everything.

## 34.1 Decision log for version 0.4

| Proposal evaluated | Decision | Reason |
|---|---|---|
| Global Subjects and local bindings for monorepos | Adopted with hierarchical scopes | The problem is scope and inheritance, not creating a separate database per package |
| Mandatory embeddings before creating a Subject | Partially adopted | They will be used as a candidate signal; deterministic resolution takes priority |
| Justification when not reusing a similar concept | Adopted | `novelty_reason` makes creation auditable |
| Mandatory daemon or `post-commit` | Rejected as a guarantee | It can be skipped; the query-time revision gate is the right boundary |
| Optional hook or daemon to detect earlier | Adopted | It shortens the stale window without blocking the flow |
| IDE always intercepts the intent | Rejected as a universal promise | Capabilities vary and the intent may not be expressed |
| Automatic per-target baseline | Adopted | It protects critical constraints even without a complete intent |
| Explicit intent-aware preflight | Kept | It is needed to compare the proposed change against decisions |
| "The more context, the better" | Reformulated | More focused context helps; noise and position can hurt |
| Vision similar to a senior person | Reformulated | Institutional continuity, yes; replacing senior judgment, no |
| Shared memory even though CBM is local | Adopted | Canonical records live in Git; indexes and assessments are rebuilt locally |

This adjudication must be preserved so future iterations do not reintroduce assumptions that were already discarded.

## 34.2 Decision log for version 0.5

| Proposal evaluated | Decision | Reason |
|---|---|---|
| Measure CUD only through subjective perception | Rejected | Utility must be compared against ground truth and real results |
| Utility per thousand tokens | Adopted as a diagnostic metric | It makes the efficiency of packets of different sizes comparable |
| CUD as the only success metric | Rejected | It can hide critical omissions, falsehoods, or failed tasks |
| Multiply relevance, reliability, actionability, applicability, importance, and uniqueness | Adopted as an initial hypothesis | It strictly penalizes weak items, but requires sensitivity analysis and validation |
| Per-case ground truth | Adopted | It makes it possible to evaluate recall, precision, and falsehoods reproducibly |
| Compare only against a run without memory | Rejected | It must include traditional documentation, Codebase Memory, and, when possible, expert context |
| Measure only `prepare_change` tokens | Rejected | The valid cost is end to end until a correct solution |
| Measure reduction of the human prompt | Adopted | It represents a central product promise |
| Declare a "senior vision" by stylistic similarity | Rejected | Only recall of verifiable knowledge identified by an experienced person will be measured |
| Free-text `novelty_reason` | Rejected as the only defense | Structured contrast, candidates, and evidence are required |
| Baseline running the full pipeline | Rejected | High-frequency surfaces need a precomputed fast path |
| Blocking baseline | Rejected | It must have a deadline, fail open, and observable degradation |
| Publish thresholds after seeing results | Rejected | Criteria must be frozen before the pilot to avoid metric gaming |
| Ignore results where Rationale makes things worse | Rejected | Failures are necessary to validate or correct the hypothesis |

This version is considered near-final for starting the experiment, not evidence that the product already works.

---

# 35. Conclusion


Rationale must not be a database full of historical explanations.

Nor must it become a tool that interrupts developers every time a function changes.

Its value will lie in finding the balance between:

* Memory and forgetting.
* Automation and confirmation.
* Structure and concept.
* History and relevance.
* Warning and noise.
* Trust and humility.
* Provenance and authority.
* Historical declarations and current assessments.
* Utility and operational cost.
* Shared memory and local state.
* Sufficient context and noise.
* Global reach and package scopes.
* Best-effort automation and verifiable guarantees.
* Packet density and real task outcome.
* Context savings and end-to-end cost.
* Attractive hypotheses and evidence able to refute them.

The definitive definition of the project is:

> **Rationale is a local causal context compiler and a provenance, authority, and validity layer for coding agents. As shared canonical memory, it keeps why important changes were made, which decisions and constraints govern the system's behaviors, who could approve them, and what evidence backs them. On each computer it uses structural engines such as Codebase Memory to resolve scopes and local binding resolutions at the current revision, and it compiles only the trustworthy, relevant, and actionable context a task needs.**

The best version of the product is not the one that remembers the most. Nor is it the one with the highest internal score. It is the one that shows its context helps complete real tasks more safely, with less human repetition and a reasonable total cost.


It is the one that knows:

* What it does not know.
* What was inferred.
* What was approved.
* What evidence may be incomplete.
* Which revision it is observing.
* When it must stay silent.
* When a decision is critical enough to stop a change.

Rationale also does not seek to replace the task-specific prompt or turn the agent into a complete senior person. It seeks to keep the technical continuity that usually disappears when a conversation ends or a person leaves the project.

Rationale does not seek to make an artificial intelligence remember everything that happened in a project.

It seeks to ensure it never destroys an important decision just because nobody managed to explain why it existed.

And, at the same time, it seeks to ensure it never preserves a false explanation just because it sounded convincing.

---

