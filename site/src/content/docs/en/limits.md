---
lang: en
slug: limits
title: Known limits
description: What Rationale 1.0 deliberately does not claim — stated plainly so you can calibrate trust.
section: Verify
order: 12
---

## Meaning is still the agent's judgment

Rationale proves which Records govern a target and reports lexical intent
conflicts with a conservative polarity hint. It does not call a remote model to
decide meaning. The hint is noisy in both directions — it can flag an unrelated
rule as `opposed` and miss a direct contradiction — so every conflict is marked
as unverified overlap, and `undetermined` never becomes a blocking verdict. The
governing Records themselves are reliable; read those.

## Structure is as good as the provider

Codebase Memory is optional and fallible. Some languages resolve calls by name,
so an `observed` edge means "present in the index", not a semantic guarantee.
Relationship states are derived on every call and never delete an explanation:
`orphaned` and `unknown` ask for a look, not a cleanup.

## Capture is autonomous

Agents write Records without an approval queue. The gate removes noise and
duplicates, and provenance always says an agent asserted it, but the quality of
memory still depends on the agent following the protocol. Pin the rules that
must not move, and review `.rationale/records/` in pull requests like code.

## Skill selection is probabilistic

An agent decides whether to load the `rationale` skill from its description,
and it can miss. The protocol in `CLAUDE.md` or `AGENTS.md` is the floor every
conversation keeps; `/rationale` and `$rationale` make the skill explicit. The
skill's evals measure selection and behavior, but they are run by maintainers,
not on your machine.

## One repository at a time

A project is one Git repository. There is no multi-repository federation, no
hosted canon, and no account.

## Governance documents in progress

Some architecture decisions behind 1.0 (record lifecycle, local data exclusion,
user-scoped registration, the activity stream) are recorded as ADRs whose status
is still `proposed` pending independent review. The behavior is implemented and
tested; the formal acceptance is open.
