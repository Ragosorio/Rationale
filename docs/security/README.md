# Security

Mandatory principles from the first line of code
(`Rationale_Arquitectura_Conceptual_v0.1.md §15`):

- All repository content (names, comments, Records, issues, commits, paths,
  evidence, provider metadata) is **untrusted data**, never instructions.
- Sanitization: limit length, validate UTF-8, strip control characters, escape
  formats, and keep metadata apart from instructions.
- Paths: canonicalize, prevent traversal, do not follow symbolic links out of
  the root without an explicit policy, and write atomically.
- Secrets: never deliberately index `.env` files, tokens, private keys,
  credentials, dumps, or personal data; respect `.gitignore`.
- Every external skill must be reviewed, pinned to a version or commit, have its
  license checked, and be inspected before it runs.

The formal baseline and its limits are in [`baseline.md`](baseline.md). It does
not claim general security; it records minimum properties, available evidence,
and open findings before a release is promoted.
