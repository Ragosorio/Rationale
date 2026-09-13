# Changelog

Notable changes are recorded here by release. The technical detail of each
change lives in the linked commits, ADRs, and work items.

## v1.1.0

Rationale ships an Agent Skill that teaches coding agents every operation, and
the text agents read is written in English while they answer in the user's
language. The CLI, the MCP tools, and the canon format are unchanged, so a
1.0 canon keeps working as is.

**The `rationale` Agent Skill.** A complete, portable skill in
`skills/rationale/`, following the Agent Skills format and the published
guidance from Anthropic and OpenAI on writing skills: a short `SKILL.md` that
routes to one playbook per operation (preflight, explain, capture, conflicts,
health, adopt, maintain), references loaded on demand, a guide to writing
Records the capture gate keeps, report templates, Codex metadata in
`agents/openai.yaml`, a candidate validator (`scripts/check_candidates.py`)
that mirrors the gate, and trigger and behavior evals for maintainers. The
binary embeds it through `src/skill_bundle.rs`, and `install-agent` writes it
to `.claude/skills/rationale/` (Claude Code) and `.agents/skills/rationale/`
(Codex) with a hash per file: edited files are kept, and a skill directory that
is a symbolic link created by another tool is left alone. It can also be
installed from GitHub with `npx skills add Ragosorio/Rationale`. Tests keep the
bundle identical to the directory, enforce the specification's limits and
one-level references, and fail when the validator drifts from the gate.

**Agent-facing text in English, replies in the user's language.** The pre-made
actions, the master prompt, the MCP tool descriptions, the retired-prompt
message, and the instruction block header and marker are written in English and
tell the agent to reply in the language the user writes in, keeping
identifiers verbatim. Instruction blocks and Cursor rules written with the
earlier Spanish marker and preamble are still recognized, replaced on
reinstall, and removed on uninstall.

**One entry point for automatic selection.** The Claude Code shortcuts
(`/rationale-preflight`, `/rationale-explain`, `/rationale-capture`,
`/rationale-conflicts`, `/rationale-health`) are now invoked by people only;
the model selects the `rationale` skill instead of choosing among near-identical
descriptions. The `/rationale-protocol` skill is retired in favor of
`/rationale` and removed by `install-agent` while it keeps its recorded hash;
the `protocol` MCP prompt remains.

**Identical bytes on every platform.** A `.gitattributes` keeps the embedded
skill, the master prompt, the installed skill copies, and shell scripts in LF
on every checkout. Without it, Git on Windows converted them to CRLF: the
Windows binary would have embedded different bytes and hashes than macOS and
Linux, and `install-agent` would have treated a teammate's committed skill
files as edited. Tests fail if an embedded file contains a carriage return.

**`finalize_change` documents `relationships`.** The MCP input schema now
describes the `relationships` a candidate can explain, which the gate already
accepted.

**Documentation.** All documentation in the repository is written in English
and updated to the current state, with a new guide to the skill
(`docs/user-guide/skills.md`). The founding documents
(`Rationale_v0.5.md`, the conceptual architecture, and the agent build process)
are translated with their section numbers intact, so existing `§` citations
still resolve. The Spanish copy of the master prompt was removed: the website's
Spanish page presents the installed English protocol and explains why it is in
English, and `scripts/check-docs.sh` rejects a translated copy.

**Website.** The landing page gains a step-by-step *Get started* section right
after the loop and a section on the `rationale` skill (operations, how to
install and invoke it, the validator, and the language rule); the Claude Code
card lists `/rationale` and the five shortcuts instead of `/rationale-protocol`.
The documentation adds a page for the skill in English and Spanish and updates
the quickstart, agents, master prompt, MCP reference, versioning,
troubleshooting, limits, architecture, and evidence pages.

**Also in this release.**

- The structural client announces the artifact's real version in
  `initialize.clientInfo.version` instead of the `0.0.0` placeholder from
  `Cargo.toml`.
- The release workflow validates the tag, formatting, Clippy, release-profile
  tests, dependency audit, documentation, Control Room, and website before
  allowing packaging.
- The security baseline and operating status describe the 1.0 series instead
  of the alpha and Phase G.
- The landing page and documentation drop generated-UI patterns: self-hosted
  IBM Plex, the favicon's R/ mark across the site, and SVG diagrams derived from
  the repository's real call graph.

## v1.0.0

First stable release. Rationale moves from a proposal system that a person
approved one by one to **autonomous causal memory with human authority**:
agents receive the context that governs code before changing it and write
durable knowledge afterwards, and people keep authority over the rules they
pin. The decision belongs to the project owner (`user:ragosorio`,
`architecture-owner`); the reasoning and evidence are in
`docs/work-items/vnext-implementation-plan.md`.

**Autonomous capture.** `finalize_change` receives `candidates` and writes them
as canonical Records in the same call. A gate discards — always with a reason —
malformed candidates, transient ones, those without a rationale or with one
that repeats the statement, mechanical noise, duplicates, and those without a
meaningful binding. Every Record declares its provenance (`agent_asserted`,
`human_stated`, `migrated`) and its authority (`normal`, `pinned`). Without
candidates, no memory is written.

**Human authority.** `rationale pin` / `unpin` pin rules; a candidate that tries
to replace a pinned Record is not written and becomes a conflict that only a
person decides with `rationale conflicts` / `resolve` or, relaying their literal
answer, with the new MCP tool `resolve_conflict`. Pinning and adopting a
replacement require an actor declared in `.rationale/config.yaml`.

**Context compiler.** `prepare_change` opens an operation (`operation_id`) and
returns the governing constraints and decisions with authority and provenance,
the relationships explained by the canon with their structural state
(`observed`, `indirect`, `orphaned`, `unknown`), a bounded structural
neighborhood, and the target's code. The token budget is a ceiling: governing
knowledge is never trimmed, and when it does not fit the packet says so in
`budget_overflow`. Padding with constraints that did not govern the target was
fixed.

**Control Room.** `rationale ui` serves a read-only interface embedded in the
binary on `127.0.0.1`: the working subgraph in 3D with a causal overlay, each
session's activity live over SSE, the memory browser, and system status. Local
activity is one NDJSON file per session with identifiers and a one-line intent
(ADR-0017); `RATIONALE_ACTIVITY=off` disables it.

**Agents.** MCP registration for Claude Code, Codex, and Cursor moves to
`serve --client <agent>` and migrates the earlier registration of the same
binary. The master prompt and the actions teach the new contract; the `review`
action is retired in favor of `conflicts` (human-only), its skill retires itself
when nobody edited it, and the `review` MCP prompt answers with its replacement.

**Structural provider.** A normalized model behind Codebase Memory, reached only
through its public MCP tools. The adapter resolves symbols inside the declared
file (it used to fall back to the first search result), recovers a stale project
mapping without blindly re-indexing, and always passes absolute paths to the
provider.

**Releases and installers.** The default channel of the installers and of
`rationale update` becomes `stable`. The release workflow builds the Control
Room before packaging and fails if it is missing, and CI gains a job for its
typecheck, tests, and build.

**Migrating from beta.** Run `rationale install-agent` (it updates registration,
protocol, and skills) and `rationale migrate` if pending proposals remain: valid
ones become `migrated` Records and noisy ones are archived with their reason.
`rationale review` remains available as legacy. An MCP call with the earlier
contract (a statement without candidates) is discarded explicitly instead of
creating a proposal.

**Documentation.** The landing page and the site documentation were redesigned
and rewritten for 1.0 in English and Spanish, with a new Control Room page; the
repository documentation and runbooks describe the new flow.

Known limits: the lexical polarity of intent conflicts is a noisy hint; Codebase
Memory resolves calls by name in some languages; several ADRs behind 1.0 are
still `proposed`, awaiting independent review. Full verification in
`docs/work-items/v1.0-release-verification.md`.

## v0.1.0-beta.3

**Per-user, convergent agent registration (ADR-0016).** Cursor showed
`rationale` as disconnected because a graphical application could not resolve
the logical command declared in `.cursor/mcp.json`. The MCP server is now
registered once per user in Claude Code, Codex, and Cursor with the installed
binary's absolute path; project files keep only instructions and skills, and
`install-agent` removes per-project entries that keep Rationale's known shape.
Installation compares command and arguments, so a Codex registration that
pointed at an old build is migrated; uninstalling removes only what points at
the uninstalled binary.

**Project identity in Codebase Memory.** When the provider does not persist
`root_path` across processes, Rationale stores in `.rationale-local/` the public
name returned by `index_repository`, without node IDs or access to its storage.

## v0.1.0-beta.2

**`rationale update` returned an older version.** A defect observed in the real
update test of beta.1, not in any test: `update` on an `alpha.7` installation
installed `alpha.7` again.

The cause was a direct consequence of beta.1's channel change. The installers'
`preview` channel selected "the most recent release **marked prerelease**",
which worked while every version was one. `beta.1` is a full release on purpose —
so the `stable` channel can resolve it — so `preview` skipped it and fell back to
`alpha.7`.

`preview` now means "the most recent release, prerelease or not", which is what
it always should have meant: the API returns them from newest to oldest, so the
first one is enough. `stable` still uses `releases/latest`. Both channels
resolve to the same version when the most recent one is full, and `preview` gets
ahead when a newer prerelease exists.

`check-docs.sh` gains a guard that rejects selecting by the flag again, verified
by reintroducing the defect on purpose.

The beta.1 binaries were correct; the defect was only in the helper scripts. The
already-published artifacts were not replaced: they carry attestations, and
overwriting them would invalidate those. beta.2 was published instead.

## v0.1.0-beta.1

First beta. Rationale enters beta because the full flow — prepare, capture,
review, and retrieve decisions — works repeatably on a real project in use, not
because it is finished. `docs/work-items/beta-readiness.md` states precisely
what was tested and what was not.

**The `stable` channel served a dogfood build.** GitHub marks as "latest" only a
release that is not a prerelease, and `releases/latest` is what the installers'
`stable` channel resolves. The workflow marked any tag with a hyphen as
`--prerelease`, so all seven alphas stayed prereleases and "latest" stayed
anchored at `v0.0.0-dogfood.7` — older than `install-agent` and half of the
current CLI. Anyone installing through that channel received that binary. Now
only `-alpha.`, `-rc.`, and `-dogfood.` are prereleases (ADR-0010).

**Cursor was broken, not just unvalidated.** `install-agent` wrote to
`.cursor/rules/rationale.mdc` the same markdown block as in `CLAUDE.md`, without
the YAML frontmatter Cursor requires to apply a rule: the protocol ended up in a
file the agent ignored. Cursor was also not detected when the project had only
`.cursor/` without `mcp.json` and the user did not have the `cursor-agent` CLI.
Both are fixed, and `uninstall-agent` no longer leaves the frontmatter behind.
That Cursor **applies** the rule is confirmed by a person, not by CI.

**One decision per Record.** `finalize_change` bound every file in the diff,
which pushed agents to write one giant Record instead of several small ones. It
was observed in real use: a planning session produced a Record binding twelve
documents when its own content specified ten decisions. The master prompt now
instructs splitting — with criteria for when to and when not to — and
`governs_paths` lets each Record bind only what it governs. Omitting it keeps
the earlier behavior; declaring a path that is not in the diff fails instead of
fabricating an unverifiable binding.

**`/rationale-health` did not work on a clean install.** The skill did not
declare the Bash permission for its own injection, so it asked for interactive
approval; and once granted, it still reported a failure because `doctor --check`
exits 1 both for normal findings — its purpose — and for real errors. It now
declares the exact permission and tells the two cases apart by content, without
an indiscriminate `|| true`.

**Public documentation stopped drifting.** The version was copied by hand in
nineteen places with no guard; `docs/RELEASE_VERSION` is now the single source
and CI fails if any mention drifts. The Spanish master prompt was one step behind
the English one — the one compiled into the binary — and a user reading it got
different instructions from those their agent had installed. And the site was
never built in CI: `npm run check` existed and nobody ran it.

**A GUI client could not find the binary, and nothing explained why.** The MCP
configuration declared the logical command `rationale` on purpose, so the file
could be versioned without anyone's personal path. But on macOS an app opened
from the Dock inherits `launchd`'s environment, not the shell's, so
`~/.local/bin` — where the installer puts the binary — is invisible to it:
Cursor reported the server as unavailable while Codex and Claude Code in a
terminal worked. `install-agent` now detects it and prints the remedy, and
troubleshooting documents it in both languages. The project configuration does
not change.

**Known limitations of this beta**, stated instead of omitted:

- Windows passes CI end to end, including the packaging smoke test, but nobody
  has installed and used the binary on a physical Windows machine.
- Cursor: the files are generated and reverted correctly, with tests; that
  Cursor applies them requires human confirmation.
- No one else's repository: all the evidence comes from projects of the person
  developing Rationale, so nothing yet proves the instructions work without
  prior knowledge.
- `.rationale/migrations/` is still an empty affordance: only one schema version
  exists, and `doctor` already detects an unknown one as a visible gate.

## Between v0.1.0-alpha.7 and v0.1.0-beta.1

These changes were recorded as unreleased at the time and shipped with beta.1.

- **`windows-latest` and CI without installed agents found four defects the
  developer's machine could never see.** `cargo test --locked` was red on all
  three platforms (`main` at `eb35bfb5`), for four different causes:
  - **Invalid JSON on Windows.** `cmd_init` and `cmd_health` interpolated the
    project path into a hand-written JSON literal; `\` is not a valid escape, so
    the one-line contract came out corrupted. Both commands now generate the
    JSON with `serde_json`.
  - **Non-portable path comparison on Windows.** `/other/project/CLAUDE.md`
    **is not** `is_absolute()` on Windows — it lacks the drive letter — so the
    migration of legacy manifests treated it as already relative and skipped
    it, leaving an external entry that later aborted `uninstall-agent`. The
    migration now decides by the destination the path points at, not by its
    shape, comparing portable components (`/` and `\` as separators on any
    platform) instead of `is_absolute()` or substrings.
  - **`agents::install` tests depended on the developer's PATH.** `install()`
    detects an agent when its binary is on `PATH` or the project already uses
    its configuration. On the developer's Mac, `claude` and `codex` were on
    `PATH`, so detection always happened and `install` always wrote the
    manifest; on CI runners none of the three binaries exist, `install`
    returned early, and three tests that assumed a written manifest failed with
    `NotFound`. A fourth test passed for the wrong reason: the guard against
    arbitrary paths was never exercised because `install` did nothing. The
    tests now seed the detection condition explicitly (`AGENTS.md`,
    `.cursor/mcp.json`, the skills directory) instead of depending on what the
    person running them has installed.
  - **`install-agent` aborted entirely when Codex was detected without an
    invocable binary.** Seeding the detection above exposed a real production
    defect, not just a test one: when `codex` was detected through project
    configuration (a legacy `AGENTS.md`) without the `codex` binary on this
    machine, `install-agent` still tried to run `codex mcp list` for the global
    registration and aborted the whole installation — including the other
    agents detected in the same pass — with a process error. Now, without an
    invocable binary, the global MCP registration is skipped with a notice; the
    other agents install normally.

  The four defects are independent of one another. The two `tests/cli.rs` tests
  that already failed on Windows before this session were fixed in the same
  effort so the alpha.8 matrix could go fully green.

- **Dogfood fixed a false idempotency in `init` and added pre-made actions.**
  If `.rationale/` already existed, `cmd_init` emitted `already-initialized`
  and returned before `agents::install`; `update` only registers Codex globally,
  so a repository initialized before installing Claude Code was permanently left
  without `.mcp.json`, a `CLAUDE.md` block, or a manifest. The defect was
  observed in Monorepo and affected the four pilot repositories. Now `init`
  keeps the one-line JSON contract, honors both skip mechanisms, and converges
  the agent configuration when reinitializing too. Rationale exposes six actions
  from a single source: MCP prompts (`preflight`, `explain`, `capture`,
  `review`, `health`, `protocol`) and `/rationale-*` Claude Code skills. The
  skills are written atomically and recorded with a SHA-256 hash, and
  `uninstall-agent` deletes only the intact ones; a file the user edited is
  kept. The destructive operation no longer checks a path and then deletes it:
  it claims the identity through an atomic rename and publishes replacements
  without overwriting destinations that reappear, closing the pathname TOCTOU
  race documented in ADR-0008.

- **Final landing page for the alpha → beta validation.** English and Spanish
  are now static Astro routes (`/` and `/es/`) instead of text mutations in
  JavaScript. Documentation opens the localized manual, Install keeps its
  anchor, the hero links to a Quick Start that tells real Claude Code slash
  commands apart from written requests for Codex, and navigations use native MPA
  View Transitions with a normal fallback and reduced motion.

- **`windows-latest` entered real CI for the first time and found two real
  defects ubuntu/macOS could never detect.** `cache::cache_root` used `$HOME`
  directly — which does not exist on Windows — when ADR-0005 already documented
  `%LOCALAPPDATA%\rationale\projects\...` as the candidate to implement "when
  Phase J needs to solve Windows for real"; that moment was this one. It was
  implemented exactly as the ADR named it, with no new decision. The provider
  timeout test (`provider_timeout_reports_unavailable_and_kills_process`) uses a
  bash mock (`dd`, byte-exact framing) that Windows cannot run as a native
  binary — it is skipped explicitly on Windows, with the reason documented in
  the test itself: it is a limitation of the test fixture, not of the production
  code under test (`spawn_with` always launches a real binary, never a script,
  on any platform).

- **Fixes `prepare_change` silently discarding `intent`.** The MCP server
  required an explicit `mode: "intent-aware"` in addition to `intent` to enable
  conflict detection; without that flag, `intent` was ignored with no
  diagnostic. `Rationale_v0.5.md §4.18` defines the mode by the presence of an
  intent, not by a separate flag, and the documented master prompt
  (`docs/prompt-master.md`) teaches only `prepare_change(target, intent)` —
  never `mode`. Any agent following the official protocol reproduced exactly the
  symptom of the real bug that motivated the project: a contradictory intent
  went through without Rationale flagging it. An explicit `mode: "baseline"`
  still forces plain retrieval without detection, as an override.

- **Phase A — install and update without data loss.** Windows: `package.ps1`
  packed the ZIP with the files at the root while `rationale-installer.ps1`
  looked for them in a subdirectory (CI never caught it: it only built ubuntu +
  macOS); it is now fixed and `windows-latest` is added to CI with a real smoke
  test of the archive layout. A merge conflict that inverted the
  `rationale:begin`/`rationale:end` markers made `install-agent` panic; it now
  fails with a readable error without touching the file. `uninstall-agent`
  deleted the whole file for every entry Rationale had created, even if the user
  later added their own content (another MCP server in the same `.mcp.json`,
  text below the `CLAUDE.md` block); it now removes only what Rationale wrote.
  `.mcp.json` was not updated when the binary moved. The writes in `agents.rs`
  (`CLAUDE.md`, `AGENTS.md`, `.mcp.json`, the manifest) become atomic, the same
  pattern as the YAML canon.

- **Phase B — silent correctness in the governance chain.** `finalize_change`
  bound only from the mechanical diff, never from the declared target — if the
  real change was already committed before `base_revision`, the resulting Record
  could never govern its own target (the same symptom as the original dogfood
  bug, a different cause). The declared target is now also bound when it
  resolves to a real file. In addition, `finalize_change` captured
  `AGENTS.md`/`CLAUDE.md`/`.mcp.json` as if they were part of the user's change
  when `init`/`install-agent` left them untracked (confirmed in real dogfood) —
  they are now excluded, like `.rationale/`. `schema_version` was written but
  never read: `rationale doctor` now detects unknown versions. `approved_at` was
  never written in an `Approval` — an approval kept only the "who", never the
  "when". A proposal claimed by `rationale review` whose process died before
  promoting or rejecting it was orphaned forever in
  `.rationale/proposals/.in-review/`, with no recovery path even though the
  comments promised it "stays recoverable"; `rationale doctor --repair` now
  returns it to `proposals/`.

- **Phase C — a migration path for the legacy canon.** `doctor` detected
  `RecordWithoutBindings` but refused to repair it ("inventing a binding would
  be worse than none") — correct as a principle, but with no way out for the
  canon that Phase 1's broken producer had already written in four repositories.
  `rationale doctor --repair` now asks a human for the path (and an optional
  symbol) and writes the binding marked `declared_by: human` — never
  indistinguishable from one a structural provider confirmed — with its own
  lifecycle event. It rescues the dogfood evidence instead of discarding it.

- **Phase D — lifecycle observability.** `rationale review-record` now prints
  `approvals[]` (actor, authority, status, `approved_at`) and the full
  `lifecycle.events[]` history before the action menu — previously you had to
  read the YAML by hand to audit who approved a decision and when.
  `kind: "exception"` had been in the `record.schema.json` enum from the start
  but was unreachable: `finalize_change` had no `kind` parameter and the
  producer always hardcoded `"constraint"`; it is now an optional, validated
  parameter, with `"constraint"` as the implicit default (the earlier behavior).
  `review-record --project-root <path> <id>` bound `record_id` to the flag's
  VALUE instead of the real id when the flag came first — it only worked in the
  documented order by accident; positional parsing now knows which flags take a
  value.

- **Phase E — the coverage needed to claim beta with evidence.** Four areas had
  no test: that `uninstall-agent` really keeps `.rationale/` (both installers
  *printed* it, neither tested it); the exit code of `doctor --check` and the
  real shape of `doctor --json`; and a `project_root` different from
  `repo_path` (the canon in one repository, the code in another) — wired from the
  start but never exercised with two real Git repositories. All four now pass.

## v0.1.0-alpha.7 — 2026-07-27

- Fixes the default channel of `rationale-installer.sh/.ps1` and
  `rationale-update.sh/.ps1`: they defaulted to `RATIONALE_CHANNEL=stable`,
  which resolves the version through GitHub's `GET /releases/latest`. That
  endpoint excludes prereleases by design, and every alpha (including alpha.6)
  is marked as a prerelease — so "stable" silently resolved to
  `v0.0.0-dogfood.7`, a release older than `rationale-update.sh`. The installer
  failed with a real 404 when requesting that file from that old release. The
  default channel becomes `preview` while the project is pre-1.0, as
  `docs/work-items/alpha-release-mcp-cleanroom-hardening.md` already
  established. Verified end to end against the real GitHub assets.

## v0.1.0-alpha.6 — 2026-07-27

- Complete governance chain: file and symbol bindings, materialized Subjects,
  tolerant severity, capture of uncommitted changes, conflict resolution, and
  `rationale doctor` for the legacy canon.
- Chestie appears in human review, `health`, preparation, and the installers,
  with a dynamic speech bubble and sober output for CI, pipes, and
  `--no-mascot`.
- The master prompt lives in [`docs/prompt-master.md`](docs/prompt-master.md)
  and is the source `install-agent` also consumes.
- The Astro documentation site is available at `/docs/*` and `/es/docs/*`, with
  a bilingual master prompt, navigation, table of contents, and operational
  content.

## v0.1.0-alpha.5 — 2026-07-27

No functional changes over alpha.4 — a release generated by the process of
merging the release branch's pull requests; the real content arrived in alpha.6.

## v0.1.0-alpha.4 — 2026-07-26

- Fixes the MCP server's version report so the tools expose the binary's real
  release.
- Keeps the robust preview update pipeline from alpha.2 and alpha.3.

## v0.1.0-alpha.3 — 2026-07-26

- Avoids `SIGPIPE` failures in the preview update helper by no longer closing
  the releases stream prematurely.

## v0.1.0-alpha.2 — 2026-07-26

- Hardens the preview update and the selection of prerelease releases in the
  Unix and PowerShell helpers.

## v0.1.0-alpha.1 — 2026-07-26

- Publishes the first packaged alpha with binaries, checksums, installers, and
  the update helper.
- Fixes the MCP server transport: Rationale speaks line-delimited JSON at its
  stdio boundary, while keeping `Content-Length` only as a client toward
  Codebase Memory.
- Makes `--help`, `--version`, and invalid options non-mutating; previously,
  `rationale init --help` could create `.rationale/` and modify agent
  instruction files.
- Adds `install-agent`, `uninstall-agent`, MCP integration, and invocation
  instructions with Chestie.

### History before the alpha

The seven releases `v0.0.0-dogfood.1`–`v0.0.0-dogfood.7` shared the framing bug
that motivated [8ec97ea](https://github.com/Ragosorio/Rationale/commit/8ec97ea):
the server expected `Content-Length`, although MCP's stdio transport uses one
JSON object per line. The process could start and look healthy while the
handshake never completed. The fix was to separate both codecs; it was not a
Codebase Memory failure or a latency problem.

The `SIGPIPE` incident came later and was different: alpha.1's hardening of the
preview pipeline introduced it, and alpha.3 fixed it. The landing page pointing
at `releases/latest` was also a separate problem: GitHub served `dogfood.7`,
older than `install-agent`, to the safe `--help` and to the update helper.

## v0.0.0-dogfood.7 — 2026-07-26

- A local MVP installable from a GitHub Release.
- CLI for `init`, `health`, `prepare`, `review`, and `review-record`.
- MCP server with `health`, `prepare_change`, `explain_target`, and
  `finalize_change`.
- Auditable Record lifecycle and authority declared per project.
- Multi-platform artifacts, SHA-256 checksums, and installers.

It was the last dogfood iteration before the packaged alpha. It must not be used
as a current public reference: the release evidence at the time was
`v0.1.0-alpha.6`.
