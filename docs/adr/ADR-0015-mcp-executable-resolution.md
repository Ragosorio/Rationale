# ADR-0015: Executable resolution in per-project MCP configuration

**Status:** proposed — pending independent cross-review and human approval before `accepted`.
**Date:** 2026-07-28
**Deciders:** Claude Code (analysis and implementation); pending human approval and/or cross-review by another agent
**Supersedes / Superseded by:** none. It complements ADR-0014, which removed the binary path from `CLAUDE.md` and `AGENTS.md` and explicitly left this problem out of its scope.

## Context

`install-agent` writes Rationale's executable as an **absolute path** into the
per-project MCP configuration files:

```json
{ "mcpServers": { "rationale": {
  "command": "/Users/roor.osorio/.local/bin/rationale", "args": ["serve"] } } }
```

Those files — `.mcp.json` for Claude Code, `.cursor/mcp.json` for Cursor — are
**shared, versioned project configuration**. In both pilot repositories,
`.mcp.json` is committed and present on `origin/main` with one specific
person's `$HOME` inside. Any other member who clones gets a `command` that does
not exist on their machine.

The justification given for the absolute path was that an MCP client launched as
a graphical application may not inherit the shell's `PATH`, and so a bare
`"command": "rationale"` would fail with "command not found". That premise was
never tested against the clients Rationale actually supports. This ADR tests it.

## Decision

1. **`.mcp.json` (Claude Code) uses the logical command `"rationale"`, not an
   absolute path.** The `PATH` premise does not hold for this client: it was
   refuted empirically (see Evidence). The file is shared by design and must be
   portable.

2. **`.cursor/mcp.json` (Cursor) also uses the logical command `"rationale"`**,
   for consistency and because the file is equally shared — but **`PATH`
   inheritance in Cursor is not verified** and remains a declared risk with its
   own `Revisit trigger`, not a silent assumption.

3. **Shared MCP configuration contains nothing machine-dependent.** If a client
   ever requires a demonstrated absolute path, that configuration goes to a local,
   unversioned file, never to the shared one.

4. **A resolution failure must be diagnosable, not mysterious.**
   `rationale doctor` is where an MCP client's `command not found` should be
   translated into "the binary is not on the `PATH` your client sees; it is at
   `<path>`". A silent MCP server startup failure is indistinguishable from ten
   other causes.

5. **No wrapper, intermediate script, or `.mcp.json.example` file is
   introduced.** See Alternatives.

## Evidence

**The `PATH` premise was refuted on this very repository, at runtime.**
Rationale's `.mcp.json` declares a **bare** command, with no path:

```json
{ "command": "cargo", "args": ["run", "--quiet", "--release", "--", "serve"] }
```

Claude Code started the server with that configuration and answered a real call
to the `health` tool during the session in which this ADR was written:

```json
{"project_id":"rationale","provider_status":"successful",
 "git_revision":"0ecc5a275055ab1b7c7391cfb2a5625217614c76", ...}
```

`cargo` lives in `~/.cargo/bin/cargo`, a directory that is **not** on macOS's
default `PATH`: `~/.profile` adds it through `. "$HOME/.cargo/env"`.

**The exact scope of what this proves, and what it does not.** It proves that
Claude Code resolves a bare command against its environment's `PATH`, and that
this environment included `~/.cargo/bin`. **It does not prove that Claude Code
processes `~/.profile`**: the mechanism was almost certainly that it inherited
the environment of the process that launched it — an interactive shell, which
had loaded the profile. The distinction matters, because a client launched from
Finder or Spotlight would receive `launchd`'s environment, with no profile
extension, and there a bare command would fail.

Nor does it prove the directory that matters: Rationale's binary lives in
`~/.local/bin`, not `~/.cargo/bin`. That both are on the `PATH` observed on this
machine, added by the same profile mechanism, makes the inference reasonable but
**is not direct verification**.

Put exactly: the evidence **refutes that the absolute path is necessary** in the
launch mode that was tested, and no more than that. Validations #1 and #2 close
both gaps.

**The harm of the current alternative is measured**, not assumed: `.mcp.json`
committed with `/Users/roor.osorio/.local/bin/rationale` in Monorepo and
BoostAPI, both on `origin/main`. And reinstalling with a different binary
rewrites it: during this investigation it changed to
`/Users/roor.osorio/Desktop/Rationale/target/release/rationale`, producing churn
in a versioned file per machine *and* per binary.

**Claude Desktop is not a consumer of this file.** It uses
`claude_desktop_config.json`, not `.mcp.json`. The graphical-app scenario that
motivated the absolute path does not apply to the file Rationale writes for
Claude Code.

## Alternatives considered

- **A versioned absolute path (status quo).** Discarded: it guarantees failure
  for everyone except whoever installed, puts one person's `$HOME` in a shared
  file, and produces churn per machine and per binary. Its only claimed benefit —
  immunity to `PATH` — was refuted for Claude Code.

- **A stable wrapper (a committed `./scripts/rationale` that resolves the
  binary).** Discarded: Rationale would add an executable file to the user's
  repository to solve its own problem. It is more invasive than the problem, and
  it moves `PATH` resolution into a script with exactly the same difficulty.

- **A local, unversioned `.mcp.json` + a shared `.mcp.json.example`.** Discarded
  for now: `.mcp.json` is the mechanism Claude Code defines *as* shared project
  configuration; taking it out of Git breaks "clone and it works" for the team,
  and forces every member to run `install-agent` before having tools. With
  Decision #1 the file is already portable and the problem that motivated
  removing it disappears. Reconsider only if a client appears that requires a
  demonstrated absolute path — then the machine-dependent part goes to a local
  file; the whole shared file is not ignored.

- **Shared configuration + a local override per member.** Discarded as the base
  design: it is the right solution for a problem that, after Decision #1, no
  longer exists. Adding two files and a precedence between them for a
  hypothetical case is complexity without evidence asking for it.

- **Client-specific detection (absolute for some, logical for others).**
  Discarded: it produces two behaviors to maintain and document, and the client
  where the premise was refuted is precisely the majority one. If Cursor turns
  out to need something else, it is decided then, with evidence — the
  `Revisit trigger` covers it.

## Consequences

- A member who clones either pilot gets a working `.mcp.json` as soon as they
  have `rationale` installed, without running anything.
- `.mcp.json` stops producing diffs when the machine or binary changes. It stops
  being a file that dirties the team's working tree.
- Rationale's repository keeps `cargo run --quiet --release -- serve` in its own
  `.mcp.json`: here the server is built from source, not installed.
  `install-agent` will rewrite it to `"rationale"` if run in this repository,
  and it has to be reverted — the same friction that already exists and that
  this ADR does not solve.
- If the binary is not on the client's `PATH`, the server does not start.
  Decision #4 exists so that is diagnosable instead of silent.

## Risks

- **Cursor might not inherit the `PATH`.** It is an Electron app and its
  behavior was not verified. Mitigation: Decision #2 declares it an open risk,
  not an assumption; validation #2 closes it. If it fails, the fix is contained —
  a single `AgentTarget`.

- **A user with Rationale outside the `PATH`.** A manual installation in a
  non-standard path, or `~/.local/bin` not exported. It used to "work" because the
  absolute path covered it up; now it fails. Mitigation: Decision #4, and the
  installer already warns when the installation directory is not on the `PATH`.

- **A client could receive a `PATH` without the installation directory.** A real,
  **open** residual risk. Validation #1 closes it only for Claude Code on the
  machine and launch mode tested; since the mechanism by which that client got
  its `PATH` was not determined, the result cannot be extrapolated to other
  clients, machines, or startup modes. Cursor remains untested (validation #2).

  **It is accepted knowingly, not by oversight**, because the comparison is
  asymmetric: the absolute path fails for *every* member who is not the one who
  installed — with certainty, already measured in two pilots — while the logical
  command fails only in launch modes where the `PATH` is not inherited, and it
  fails diagnosably (Decision #4). Trading a certain, silent failure for a
  conditional, diagnosable one is an improvement even if the second is not zero.
  Validations #1 and #2 bound how much that "only" is worth.

- **Silent regression when reinstalling in Rationale's repository.** See
  Consequences. An accepted, documented risk; the real fix would be for
  `install-agent` to detect that the project *is* Rationale, which does not
  justify production code today.

## Validation

Pending implementation. Required before `accepted`:

**The binary under test**, so the evidence is tied to a concrete executable and
not to "some installation":

```
command -v rationale  → /Users/roor.osorio/.local/bin/rationale
rationale --version   → rationale v0.1.0-alpha.7
shasum -a 256         → 1933981a5dafdf020fcb4f2060c5bf0acd61678b2cd953ab4a643517b4f063fe
```

**A confounder already removed.** It was verified that this binary speaks MCP
correctly when invoked directly: `initialize` returns
`{"name":"rationale","version":"v0.1.0-alpha.7"}` and `tools/call health`
answers, on the test bench `~/Desktop/rationale-path-test`, using the stdio
transport of one JSON object per line (`src/mcp/framing.rs` — *not*
`Content-Length`, which is Codebase Memory's codec). The `main` binary does the
same. Therefore a failure in the client tests isolates `PATH` resolution and
nothing else.

1. **Direct check with `~/.local/bin` in Claude Code — ✅ PASSED
   (2026-07-28).** Test bench `~/Desktop/rationale-path-test` with a `.mcp.json`
   containing `"command": "rationale"`, after closing and reopening the client.

   The check was not made by calling `health` from the chat — the client did not
   complete the generation, for reasons unrelated to Rationale — but **by
   inspecting the process the client had launched**, which is more direct
   evidence:

   ```
   PID 52676   rationale serve          ← bare command, as in .mcp.json
   cwd         ~/Desktop/rationale-path-test
   txt         /Users/roor.osorio/.local/bin/rationale
   PATH        …:/Users/roor.osorio/.local/bin:…
   ```

   The `txt` descriptor is the executable the kernel loaded: it proves the client
   resolved `rationale` **to `~/.local/bin`**, the directory the Evidence did not
   cover. It closes that gap.

   **What was observed exactly about the `PATH`:** the process received a `PATH`
   that contained `~/.local/bin` (it appeared twice). **The mechanism by which
   the client obtained that `PATH` was not determined.** It could be resolution of
   the user's profile, inheritance from an ancestor process's environment, or the
   client's own configuration; nothing measured distinguishes among those
   hypotheses, and the duplication alone proves none of them.

   **The scope of what this validation closes:** the `PATH` resolution risk is
   closed **for Claude Code, on this machine, and in this launch mode**, and
   nothing more. It is not closed for Cursor (validation #2, pending), for other
   launch modes of this same client, or for untested MCP clients. The risk
   declared in §Risks remains open for all of them.

2. **Runtime resolution in Cursor — ⏳ `pending validation`.** Not run:
   `cursor-agent` is not on the test machine's `PATH`. It is declared pending,
   **not** verified, and ADR-0015 cannot move to `accepted` without it or without
   an explicit decision to accept the risk for Cursor.

   A test bench with `.cursor/mcp.json` written
   **by hand** with the same bare command, after closing and reopening Cursor.

   **The exact scope of this test:** it shows only that Cursor resolves and runs
   the logical command from `~/.local/bin`. It does **not** show that
   `install-agent` detects Cursor or generates that file — on the test machine
   `cursor-agent` is not on the `PATH`, so detection was not exercised. That
   second property is covered by
   `no_target_writes_an_absolute_command_into_shared_mcp_config`, which walks
   `TARGETS` including Cursor, and by `only_claude_code_declares_a_skills_directory`.
   Recording the manual test as if it had also verified detection would be
   exactly the kind of generalization that invalidated ADR-0012.

   If no working Cursor is available, this validation stays an explicit
   **`pending validation`** — never verified.
3. **A regression test** that fails if `upsert_mcp_json` writes an absolute path
   into any `mcp_config_file` of `TARGETS`.
4. **Installing the same project at two different paths** (copied), to confirm
   the resulting `.mcp.json` is byte-identical — today it is not.

**Validation cannot be an inspection of the generated file.** A JSON that "looks
right" does not prove the client starts the server; only a real call to an MCP
tool proves it.

## Revisit trigger

Reopen if: (a) validation #2 shows that Cursor does not resolve commands through
`PATH`; (b) a client launched as a graphical app that consumes a versioned file
is added to `TARGETS`; or (c) a real "command not found" report appears when
starting the MCP server in a standard installation.

## Validation update — 2026-07-29

**Validation #2 failed and triggered the Revisit trigger.** Cursor loaded
`.cursor/mcp.json` with `"command": "rationale"`, but showed the server as
disconnected and did not expose its tools. The local CLI did respond. In a
graphical application's environment, `~/.local/bin` was not available through
the observed `PATH`.

Therefore this ADR's Decision #2 did not survive the dogfood. ADR-0016 proposes
replacing the per-project MCP configuration with per-user global registration and
an absolute path. Since both ADRs remain `proposed`, this document is not marked
`superseded` until there is independent review and human approval.
