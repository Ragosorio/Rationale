# Evals for the `rationale` skill

These files test whether the skill triggers when it should, stays out of the way
when it should not, and makes agents behave better than they do without it. They
are for people who maintain the skill. Agents using the skill never need them,
and `rationale install-agent` does not install them.

## Layout

| File | What it checks |
|---|---|
| `triggers.csv` | Routing: explicit, implicit, contextual, boundary, and negative prompts in English and Spanish |
| `scenarios.json` | Behavior: expected and forbidden actions in realistic situations |
| `fixtures/` | Deterministic inputs for `scripts/check_candidates.py` |

## Principles

- **Define success before editing the skill.** Add or change a case first, then
  change the instructions.
- **Compare against a baseline.** Run every scenario with and without the
  skill. A version that does not beat the baseline gets rewritten, even if it
  reads well.
- **Prefer deterministic checks.** Grade tool calls, arguments, and exit codes
  with code. Use a rubric only for judgment, such as whether a verdict was
  stated clearly.
- **Turn every real failure into a case.** When an agent misuses Rationale in
  real work, add the prompt and the expected behavior before fixing the skill.
- **Test each host you ship to.** Claude Code and Codex select skills
  differently.

## Setup

Run evals in a disposable repository with Rationale initialized, a few Records
(at least one `pinned`), and the MCP server registered:

```bash
rationale init
rationale install-agent
```

Each scenario's `setup` field describes the canon state it needs.

## Trigger evals

For each row of `triggers.csv`, start a fresh session, send `prompt`, and record
whether the agent loaded the skill (read `rationale/SKILL.md` or invoked the
`rationale` skill). Compare the result with `should_trigger`.

```bash
# Claude Code: stream events, then look for the skill being loaded
claude -p "<prompt>" --output-format stream-json --verbose > run.jsonl

# Codex: JSONL events on stdout
codex exec --json "<prompt>" > run.jsonl
```

Report precision and recall separately for each `kind` and `language`.
`boundary` rows must trigger and also pass the matching scenario: a request to
pin must load the skill and still leave `rationale pin` to the person.

## Behavior scenarios

Each entry in `scenarios.json` has a `query`, the `setup` it needs,
`expected_behavior`, and `forbidden_behavior`. A run passes only when every
expected behavior happened and no forbidden behavior did. Grade tool usage from
the event trace (for example, `prepare_change` came before the first file
edit).

Run each scenario at least three times per host, with and without the skill, and
record pass rate, tokens, and tool calls. Keep the results next to the skill
revision they measured.

## Validator fixtures

From the root of the Rationale repository:

```bash
python3 skills/rationale/scripts/check_candidates.py skills/rationale/evals/fixtures/candidates-valid.json    # exit 0
python3 skills/rationale/scripts/check_candidates.py skills/rationale/evals/fixtures/candidates-invalid.json  # exit 2
```

The invalid fixture holds one candidate per blocking rule, so the report shows
every gate reason the validator mirrors.
