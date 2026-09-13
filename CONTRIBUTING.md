# Contributing to Rationale

Thank you for wanting to improve Rationale. The project deliberately separates
the code that prepares context from the decisions that carry authority.

## Before you start

Read according to the change:

- Product and model: [`Rationale_v0.5.md`](Rationale_v0.5.md).
- Architecture: [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md).
- Process between agents: [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md).
- Factual code map: [`docs/architecture/code-map.md`](docs/architecture/code-map.md).
- Rust: [`docs/rust/`](docs/rust/).
- Decisions: [`docs/adr/`](docs/adr/).
- The agent skill: [`docs/user-guide/skills.md`](docs/user-guide/skills.md).

For changes that cross modules, storage, MCP, security, or packaging, query
Codebase Memory before editing and state its coverage and warnings in your work.

## Local development

```bash
git clone https://github.com/Ragosorio/Rationale.git
cd Rationale
export PATH="$HOME/.cargo/bin:$PATH"
cargo build
cargo test
```

Before opening a pull request:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo audit
bash scripts/check-docs.sh
```

When you touch the Control Room or the website:

```bash
npm --prefix ui ci && npm --prefix ui run build
npm --prefix site ci && npm --prefix site run check
```

Rust code changes need proportional tests. Schema changes must pass
`cargo test --test schema_validation`; storage changes must preserve the
round trip; MCP changes must keep stdout exclusively for the protocol.

## The agent skill and agent-facing text

`skills/rationale/` is the single source of the `rationale` Agent Skill. The
binary embeds it through `src/skill_bundle.rs`, and tests fail when the two
drift, when `SKILL.md` breaks the Agent Skills limits, when a reference is not
linked, or when the candidate validator stops mirroring the capture gate.

- Add a file to the skill? Add it to `FILES` in `src/skill_bundle.rs`. Remove
  one? Move its path to `RETIRED_FILES`, so existing installs can still be
  uninstalled.
- Change the validator? Run the fixtures in `skills/rationale/evals/`.
- Change what the skill asks agents to do? Update `evals/scenarios.json` or
  `evals/triggers.csv` first, then the instructions.

Everything an agent reads — the skill, the pre-made actions in
`src/prompts.rs`, `docs/prompt-master.md`, and MCP tool descriptions — is
written in English and tells the agent to reply in the user's language. Explain
why a rule exists instead of adding emphasis; current models follow reasons
better than capital letters.

## Workflow

1. Record the problem or work item and the evidence you have.
2. Decide whether an ADR is needed before touching an architectural boundary.
3. Use a descriptive branch: `feature/...`, `fix/...`, `docs/...`, or
   `release/...`.
4. Implement the smallest change, add tests, and update documentation.
5. Review adversarially: look for races, drift, false positives, cross-platform
   problems, missing tests, and operational costs.
6. Re-index Codebase Memory when it applies and record the coverage.
7. Run the Rationale loop for the change: a decision is not approved because an
   agent wrote it down.
8. Open the pull request with context, evidence, risks, and the commands you ran.

## Conventions

- Small, causal commits, for example
  `fix(review): reject stale proposal before approval`.
- Do not mix unrelated refactors.
- Do not add secrets, `.env` files, dumps, or personal data.
- Do not approve Records automatically.
- Do not modify Codebase Memory internals or read its private SQLite database.
- Do not hide partial coverage: write `Unknown`, with evidence, risk, and the
  next experiment, when that is the honest state.
- Documentation in this repository is written in English. The website keeps
  English and Spanish versions of the user documentation in `site/`.

## Pull request checklist

- [ ] Code and tests updated.
- [ ] Formatter and lint clean.
- [ ] Security check run when it applies.
- [ ] Documentation, ADR, or research artifact updated.
- [ ] Independent review done for critical changes.
- [ ] Codebase Memory re-indexed, or the reason it does not apply is stated.
- [ ] `git status` clean except for deliberately unrelated files.

## Reviewing decisions

Agents capture Records with `finalize_change`; review what they wrote in
`.rationale/records/` as part of the pull request. Pinning and unpinning rules
(`rationale pin` / `unpin`), deciding conflicts (`rationale resolve`), and a
Record's lifecycle (`rationale review-record`) are interactive human acts: MCP
prepares and captures, but never replaces that authority.

For security questions, follow [`SECURITY.md`](SECURITY.md). For usage
questions, follow [`SUPPORT.md`](SUPPORT.md).
