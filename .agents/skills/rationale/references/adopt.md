# Adopt Rationale in a repository

Set Rationale up in a repository and seed the canon with knowledge the team
already has. Installing software and declaring authority are the person's
decisions; you prepare everything and run the commands they ask for.

## Contents

- Install and initialize
- Declare authority
- Seed the first Records
- Hand over what needs a person

## Install and initialize

Check first with `rationale --version`. If Rationale is missing, give the user
the install commands from the project README or the documentation site, and run
an installer only when they ask you to.

From the repository root:

```bash
rationale init                      # creates .rationale/ (keeps an existing one) and configures detected agents
rationale health                    # Git revision, working tree, provider status
rationale install-agent --dry-run   # what agent configuration would change
```

`install-agent` registers the MCP server once per user and writes the protocol
block into `CLAUDE.md`, `AGENTS.md`, or the Cursor rule. Since v1.1.0 it also
installs this skill in `.claude/skills/rationale/` for Claude Code and
`.agents/skills/rationale/` for Codex. The agent has to restart before it sees
the tools.

Codebase Memory is recommended so symbol bindings and structural context work.
Without it, file bindings still work and coverage is reported as degraded.

## Declare authority

The people who may pin rules and adopt replacements are listed in
`.rationale/config.yaml`:

```yaml
authority:
  "user:<git-user-name> <git-user-email>":
    role: architecture-owner
```

Propose the block with the person's real Git identity (`git config user.name`,
`git config user.email`) and let them review and commit it. Never declare
yourself or invent a role: declared authority is what gives a pin its meaning.

## Seed the first Records

Durable knowledge usually already exists in the repository:

- ADRs and design documents (`docs/adr/`, RFCs, `DESIGN.md`);
- comments that explain why ("do not remove: …", "must run before …");
- tests named after an invariant or a past incident;
- postmortems and changelog entries that explain a guard.

Seed one area at a time, as a normal capture:

1. Pick one area (a module or subsystem) and list the rules you can source.
2. Confirm the code still behaves as each source says. Skip what you cannot
   confirm: an outdated Record misleads every future agent.
3. Call `prepare_change` on the area's main target with an intent such as
   `record existing constraints for billing retries`, to see what the canon
   already holds and avoid duplicates.
4. Write candidates following `references/records.md`, with `evidence`
   pointing at the source, for example
   `{"type": "reference", "path": "docs/adr/ADR-0007-retries.md"}`.
5. Validate them with `scripts/check_candidates.py`, then call
   `finalize_change` with the `operation_id` and a summary such as
   `Seeded billing retry constraints from ADR-0007 and the retry tests`.
6. Report what was written and discarded.

Seeded Records carry `agent_asserted` provenance, which is accurate: an agent
read the source. A person makes a rule binding by pinning it.

## Hand over what needs a person

End adoption with a short list:

- Records worth pinning, each with its id and why it should not move
  (`rationale pin <record-id>` in their terminal);
- the authority block to review and commit;
- sources you skipped because the code no longer matched them.
