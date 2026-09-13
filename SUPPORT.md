# Support

## Before asking for help

Run:

```bash
rationale health
rationale doctor
rationale --help
```

Then read [`docs/runbooks/diagnostics.md`](docs/runbooks/diagnostics.md) and
[`docs/runbooks/provider-failure.md`](docs/runbooks/provider-failure.md). In
Claude Code, `/rationale-health` combines the MCP `health` tool with
`rationale doctor`.

## What to include in an issue

- operating system and architecture;
- Rationale version (`rationale --version`, or `git describe --tags` from source);
- the command you ran;
- the output of `rationale health`, without secrets;
- whether Codebase Memory was available;
- the agent and how it was connected (installer, `install-agent`, or manual);
- a minimal reproduction and the result you expected.

Redact tokens, private URLs, customer names, and sensitive content before
posting. For vulnerabilities, use [`SECURITY.md`](SECURITY.md), not a public
issue.

## Kinds of help

- Reproducible bug: an issue with minimal steps.
- Usage question: a discussion, or an issue labeled `question`.
- Improvement: a proposal with the problem, alternatives, and cost.
- Documentation: a direct pull request when the change is self-contained.
