# Phase H — read-only pilot and installable alpha

## Entry gate

- Phase G complete and the current dogfood release `v0.0.0-dogfood.7` published.
- Security baseline reviewed with no open P0/P1 findings.
- Installer tested on a clean machine.
- The project owner expressly authorizes the Rationale, Monorepo, and BoostAPI
  paths that may be read.

## Cases and conditions

20–30 authorized internal historical changes will be run, distributed across the
three repositories. Each case will have a pre-registered ground truth and will
compare:

1. code/Git;
2. code + documentation;
3. Codebase Memory;
4. Codebase Memory + Rationale.

The first pass is read-only. Assisted capture is enabled only after its metrics
are met; no blocking or automatic approvals are activated.

## Exit metrics

- critical constraint recall `>= 90%`;
- context precision `>= 80%`;
- harmful context `< 2%`;
- false blocks `0`;
- median packet `<= 600` tokens and P95 `<= 1000`;
- reduction of manual context `>= 50%`;
- no stale assessment presented as exact.

Every failure is kept as evidence, an issue, or a disputed Record; it is never
removed to improve the metric artificially.
