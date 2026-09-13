# 00 — Source lock

## Observed

- Local clone in `~/Desktop/codebase-memory-mcp`, remote `origin` = `https://github.com/DeusData/codebase-memory-mcp.git`, branch `main`.
- `HEAD` = `97ce23f9827177fff3858831156e9795c6832b18`, a commit from 2026-07-23, `Merge pull request #1230 from DeusData/fix/dry-run-determinism`.
- `git describe --tags` = `v0.9.0-338-g97ce23f9` → the clone is **338 commits ahead** of the `v0.9.0` tag.
- The clone's working tree: clean (`git status --short` prints nothing).
- Binary installed at `~/.local/bin/codebase-memory-mcp`, reporting version **0.8.1** through `--version`.
- `install.sh` (present in the clone) downloads from `https://github.com/${REPO}/releases/latest/download` — that is, it installs the **latest published release**, not the repository's HEAD.
- Available tags sorted by creation date: `v0.9.0`, `v0.8.1`, `0.8.0`, `v0.8.0`, `0.7.0`.

## Claimed

`install.sh` documents itself as a "One-line installer for codebase-memory-mcp", with support for a UI variant and a custom directory. It does not explicitly guarantee that the installed binary corresponds to `main`'s HEAD.

## Verified

- The version discrepancy is reproducible: `codebase-memory-mcp --version` returns `0.8.1` while `git rev-parse HEAD` in the clone returns a commit 338 positions after the `v0.9.0` tag.
- The full source is available in the clone (`src/`, `internal/`, `pkg/`, `Makefile.cbm`, `tests/`, `flake.nix`) — see the inventory in `docs/research/codebase-memory/source-lock.yaml`.
- No locally compiled build exists in the tree (`build/`, `bin/`, `out/`, `dist/`, `.build/` are absent) — the binary in use comes exclusively from the downloaded release, never from a build of this source code.

## Unknown

- Whether `v0.9.0` itself is a published "stable" release or an internal development tag — it was not verified whether a GitHub release tagged `v0.9.0` with attached binaries exists, or whether "latest" points at `v0.8.1` because `v0.9.0` is not yet considered ready to publish.
- What changed functionally between `0.8.1` and the current `main` commit (338 commits) — the changelog and the diff were not reviewed.
- Whether the MCP server exposed by the 0.8.1 binary has the same tool contract as the code at HEAD.

## Risk

**High and directly actionable.** Any observation made in this epic using the MCP tools active in this session (`mcp__codebase-memory-mcp__*`) reflects the behavior of **0.8.1**, not the code read in the clone. If a later research document (CBM-005 to CBM-011) mixes "this is what I saw in the code" with "this is what I observed through MCP" without distinguishing the version, the conclusion is contaminated. This is exactly the kind of discrepancy `Rationale_Arquitectura_Conceptual_v0.1.md §6.2` requires distinguishing explicitly between "published binary" and "build from source".

## Decision impact

- Every later research document in this epic (`01` to `12`) must state explicitly whether its evidence comes from the **installed binary (0.8.1)** or from **reading the source at HEAD** (`97ce23f9`), and never present both as a single source.
- CBM-002 (build from source) becomes more important than planned: it is the only way to observe HEAD's real behavior instead of depending on a potentially outdated release.
- It affects ADR-0002 (MCP versus CLI transport): capability negotiation in Rationale's adapter (`ProviderCapabilities`, `Rationale_v0.5.md §21.2`) must assume that the Codebase Memory version available in a user environment may lag behind the provider's active development, reinforcing the need to negotiate capabilities instead of assuming a fixed contract.

## Reproduce

```bash
cd ~/Desktop/codebase-memory-mcp
git rev-parse HEAD
git describe --tags --always
git status --short
git remote get-url origin
codebase-memory-mcp --version
```
