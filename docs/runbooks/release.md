# Release runbook

Releases are built directly from `main` from a `vMAJOR.MINOR.PATCH` tag; the
binary's version comes from the tag. `docs/RELEASE_VERSION` is the single source
of the public version in the documentation, and `scripts/check-docs.sh` fails
when any mention drifts from it. `alpha`, `beta`, and `dogfood.*` tags are
history from before 1.0.

## Before the tag

- A pull request or direct commit to `main` with green CI.
- `cargo fmt --check`.
- `cargo clippy --all-targets -- -D warnings`.
- `cargo test --release` (it includes the `rationale` skill bundle checks).
- `cargo audit`.
- `npm --prefix ui ci && npm --prefix ui run typecheck && npm --prefix ui test && npm --prefix ui run build`.
- `npm --prefix site ci && npm --prefix site run check`.
- `./scripts/check-docs.sh`.
- Security baseline with no open P0/P1 findings.
- Internal dogfood with its cases recorded.
- Installer matrix and clean-machine smoke test.
- `CHANGELOG.md`: move *Unreleased* under the new version, and update
  `docs/RELEASE_VERSION` and any "starting with the release after …" wording in
  the documentation.

## Tag and publish

Before tagging, confirm that the commit receiving the tag is exactly the one
that passed CI. A tag points at a commit, not a branch: if the tree is dirty or
`HEAD` moved past `origin/main`, the published artifact would not match the
verified code.

```bash
git fetch origin
git status --short                 # must be empty
git rev-parse HEAD                 # must match...
git rev-parse origin/main          # ...this one
```

Only then:

```bash
git tag -a v1.0.0 -m "Rationale 1.0.0"
git push origin v1.0.0
```

`release.yml` marks `--prerelease` only for `-alpha.`, `-rc.`, and `-dogfood.`.
A `beta` or final tag is published as a full release and can therefore become
"latest", which is what the installers' `stable` channel resolves (ADR-0010).
After publishing, check it:

```bash
gh api repos/Ragosorio/Rationale/releases/latest --jq .tag_name
```

The [`release.yml`](../../.github/workflows/release.yml) workflow builds the
Control Room (`ui/dist`) and fails if it is missing — a binary without it would
serve the `rationale ui` fallback page — builds the targets, creates archives
and ZIPs, computes SHA-256, publishes installers, and generates attestations. It
also publishes `rationale-update.sh` and `rationale-update.ps1`, which sit next
to the binary so `rationale update` can update an existing installation.
`.rationale-local/`, caches, and secrets are never uploaded.

## Rollback

If a smoke test fails or a security finding appears, the release is not
promoted. For a user who already installed it, reinstalling an earlier version
with `RATIONALE_VERSION` restores the binary without touching `.rationale/`.
