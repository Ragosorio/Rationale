# Security

## Scope

Rationale is local-first, but it processes decisions, paths, evidence, and
provider metadata that can be sensitive. Do not include secrets, tokens, keys,
`.env` files, dumps, or personal data in issues, example Records, or pull
requests.

## Reporting a vulnerability

Do not disclose an unfixed vulnerability in a public issue. Use the
repository's GitHub Security Advisories or the private channel the maintainer
provides. If that channel is not enabled yet, open a neutral issue asking for
private contact, without exploitable details.

Include, when it is safe to do so:

- the affected version, commit, or release;
- platform and minimal configuration;
- reproduction steps without real data;
- impact and exploitation conditions;
- any known temporary mitigation.

Do not test against third-party projects or extract real data during your
investigation.

## Expected properties

- Repository text is treated as data, not as instructions.
- Paths are canonicalized and traversal is rejected.
- Canonical writes are atomic.
- Human review requires explicit confirmation and a declared actor.
- MCP has no approval or lifecycle-mutation operations.
- `.rationale/` is never deleted when the binary is uninstalled.
- Release artifacts carry checksums and attestations.

## Skills

An Agent Skill is instructions plus code that an agent may run with your
permissions. Review a skill before installing it, including its scripts. The
`rationale` skill in `skills/rationale/` contains one script,
`scripts/check_candidates.py`: it reads JSON input and the repository's
`.rationale/records/`, uses only the Python standard library (PyYAML when it is
already present), writes nothing, and makes no network calls.
`rationale install-agent` writes the skill only inside the project, records a
hash for every file, and never overwrites a file you edited.

The full technical baseline is in [`docs/security/baseline.md`](docs/security/baseline.md).
