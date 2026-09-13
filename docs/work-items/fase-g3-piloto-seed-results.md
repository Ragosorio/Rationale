# G3.1 — seeding historical decisions in the pilot

Run date: 2026-07-26. This evidence covers the first activity of the comparative
pilot: turning already documented decisions into Rationale proposals without
approving them automatically.

## Scope and sources

Work was done on the authorized local clones:

| Project | HEAD | Codebase Memory index | Proposals created |
|---|---|---:|---:|
| Monorepo | `8588e14329a39e8f206296a94abcc1b840964de9` | 12,555 nodes / 23,910 edges | 11 |
| BoostAPI | `3102e5d2b9d65861fe9eb5756a5e34d7aeeae96c` | 8,605 nodes / 27,719 edges | 18 |

The sources were contemporary documentation and a verifiable commit, not a
reconstruction based only on memory:

- BoostAPI: decisions about assignment, workflows, auth/RBAC, WhatsApp sessions,
  payments, interactions, and three incident-driven commits.
- Monorepo: the BFF/auth audit, the BoostAPI contract, RBAC, deployment,
  sessions, the mobile OAuth plan, the catalog, the documentation ADR,
  sanitization, and commit `5287080` about message limits.

The graph coverage is useful to locate symbols and boundaries, but it is not
treated as the sole truth. Directly verifying each documentary source was the
basis of the claims; the indexes stayed `ready` and the providers reported
`successful/Complete` in both repositories.

## Validation result

| Project | YAML files | Interface used | Result |
|---|---:|---|---|
| BoostAPI | 18 | `rationale review --project-root ... </dev/null` | 18 listed; EOF skipped all |
| Monorepo | 11 | `rationale review --project-root ... </dev/null` | 11 listed; EOF skipped all |

The CLI read the 29 proposals without YAML or deserialization errors. Each one
showed its statement, rationale, Subject, severity, actor, and declared
authority. No approved Record was written: `approvals` stays empty, and the EOF
input keeps every proposal pending in `proposals/`.

## State of the repositories

The `.rationale/` structure already existed, empty, in Monorepo; BoostAPI keeps
the 18 newly created proposals. No agent installed blocks in `CLAUDE.md`,
`.mcp.json`, or other configuration files. Both clones were left dirty only by
pilot artifacts (`.rationale/` in BoostAPI and `.rationale-local/` from earlier
runs); no commits were made in those repositories.

## What it demonstrates and what it does not

It demonstrates that Rationale can receive a heterogeneous batch of real
decisions from two repositories, keep their evidence, and present them one per
screen without approving them or hiding errors.

It does not yet demonstrate context recall/precision or authorize assisted
capture. The next step of G3 is a human review of the 29 proposals (correct,
reject, or approve explicitly). After that, the read-only matrix of 20–30 targets
with ground truth will run before enabling mutations on real changes.

## Pending human gate

The actor resolved by the CLI was `user:Roo Rolando Osorio <rosorio@roo.com.gt>`
with declared authority `contributor`. That identifies who would review, but it is
not an approval. The agent must not turn these proposals into Records without the
explicit human `rationale review` session.
