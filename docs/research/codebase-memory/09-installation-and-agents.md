# 09 — Installation and agents

**Source of evidence:** `install.sh` (read, not re-run — it was already installed on this machine), the `--help` of the built binary, and a check that the registration in `~/.claude.json` exists (without dumping its content — it is the user's personal configuration, not evidence to quote verbatim).

## Observed

- `install.sh` is a wrapper script: it validates that the download URL is HTTPS (or explicit loopback for local smoke tests, with redirects disabled), downloads the binary from `https://github.com/DeusData/codebase-memory-mcp/releases/latest/download`, places it in `$HOME/.local/bin` (configurable with `--dir`), and then **delegates agent configuration to the binary itself** by invoking it with `install -y --force --dir=<path>`.
- The binary exposes `install [-y|-n] [--force] [--dry-run] [--dir=<path>] [--skip-config]` as its own subcommand — the client detection/registration logic lives in the compiled binary, not in the bash script.
- **Confirmed on this machine:** `~/.claude.json` contains a registration entry for `codebase-memory-mcp` (verified by pattern matching, without dumping the whole file — it is personal configuration, potentially with other entries unrelated to this research).
- `--help` declares "automatic/conditional" support for **43 client surfaces** named explicitly (Claude Code, Codex CLI, Gemini CLI, Cursor, Windsurf, VS Code, Zed, Aider, etc.) plus a second group of "manual/UI MCP boundaries" (Qodo, Warp, JetBrains AI/ACP, Replit, GitHub cloud agents, Jules, CodeRabbit) where the integration requires manual steps.
- `--help` itself clarifies: *"Conditional/explicit targets are changed only when their documented platform, marker, or explicit existing config path is present"* — that is, it does not write configuration for a client it does not detect as present on the machine.
- The `uninstall [-y|-n] [--dry-run]` subcommand exists as the symmetric counterpart of `install`.
- `--skip-config` in `install` lets you install the binary without touching any agent's configuration — an explicit separation between "installing the binary" and "installing the integration".

## Claimed

The presence of `--dry-run` in both `install` and `uninstall` suggests an implicit promise of auditability (being able to see what would change before applying it) — `--dry-run` was not run in this session, so as not to alter a working installation used as evidence in the rest of the epic.

## Verified

- The real installation chain (`install.sh` → download → the binary's `install`) is consistent with what actually resulted in a working registration in `~/.claude.json` and an operational binary in `~/.local/bin/codebase-memory-mcp` on this same machine.

## Unknown

- The exact content of what `install` writes into each of the 43 supported clients — not audited client by client; only the Claude Code case (the client active in this session) was confirmed.
- The exact behavior of `uninstall`: whether it cleanly reverts only what `install` added, or whether it can remove pre-existing unrelated configuration — not tested (running `uninstall` would destroy the working installation used as evidence throughout the epic).
- Whether `--dry-run` really enumerates every file it would touch, with the level of detail `Rationale_Arquitectura_Conceptual_v0.1.md §24` requires ("The installer must record exactly: binary, config, hooks, agent entries, skills, cache, PATH changes").

## Risk

**Low for CBM, informative for Rationale.** No unsafe behavior was detected (forced HTTPS download, a skip-config option, dry-run available, a symmetric uninstall). The relevant risk is one of future design: replicating this surface of 43+ clients is a non-trivial engineering effort that Rationale **must not try to match in v1** (`Rationale_Arquitectura_Conceptual_v0.1.md §2`: "Perfect compatibility with every agent" is explicitly out of scope for architecture 0.1).

## Decision impact

- It confirms that the right installer pattern for Rationale (Phase K, much later) is: **binary first, agent configuration as a separate, auditable step (`--dry-run`), with a symmetric `uninstall`** — the same as CBM. This pattern is a valid design reference for the future, not a current priority.
- The principle *"changed only when their documented platform, marker, or explicit existing config path is present"* is exactly the kind of conservative detection that avoids breaking the configuration of a client that is not installed — applicable to the future `rationale install-agent` (`Rationale_Arquitectura_Conceptual_v0.1.md §24`).
- It generates no immediate decision change for Rationale's Phase A/B — this research is recorded for when Phase K (packaging/distribution) becomes relevant, much later in the roadmap.

## Reproduce

```bash
cat ~/Desktop/codebase-memory-mcp/install.sh | head -30
./build/c/codebase-memory-mcp install --help 2>&1 || ./build/c/codebase-memory-mcp --help
grep -c "codebase-memory-mcp" ~/.claude.json   # confirms the registration without dumping content
```
