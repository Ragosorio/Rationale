# Claude Code

Rationale works with Claude Code through four official mechanisms, all
installed and removed by `rationale install-agent` / `uninstall-agent`:

| Mechanism | What Rationale puts there | Loaded |
|---|---|---|
| MCP server (per user, `~/.claude.json`) | `rationale serve --client claude-code` with the installed binary's absolute path | Every session; tools are deferred until used |
| `CLAUDE.md` | The invocation protocol, in a delimited block | Every session |
| Skills (`.claude/skills/`) | The `rationale` skill and five shortcuts (`/rationale-preflight`, …) | The skill's description always; its body when the task matches |
| Hooks (`.claude/settings.local.json`) | `rationale hook <event>` for four events | Every session in this project |

Instructions are context, not a guarantee: in the project whose report shaped
Rationale 1.2, a fifth of the operations agents opened were never closed. The
hooks are the mechanical backstop. They are discreet, never block an edit, and
never fail a session.

## The hooks

| Event | Matcher | What it does |
|---|---|---|
| `SessionStart` (startup, resume, clear, compact) | — | Adds one short paragraph: the canon's size and how many rules people pinned, operations earlier sessions left open (finalize or abandon them), and new `doctor` errors (mention them; do not repair them on the side). Nothing in a repository without `.rationale/`. |
| `PreToolUse` | `Edit\|Write\|MultiEdit\|NotebookEdit` | Once per file and session, when pinned, critical, or high rules or exceptions govern the file and this session did not prepare a change to it, reminds the agent to call `prepare_change`. It never denies the edit. |
| `PostToolUse` | `mcp__.*rationale.*__prepare_change` | Records which operations this session opened. Silent. |
| `Stop` | — | When an operation this session opened is still open, asks the agent once to finalize it (with or without candidates) or abandon it with a reason — never to finalize work that did not happen. |

The entries in `.claude/settings.local.json` look like this, with the absolute
path of the binary that installed them:

```json
{
  "hooks": {
    "SessionStart": [{"hooks": [{"type": "command", "command": "/Users/you/.local/bin/rationale hook session-start", "timeout": 10}]}],
    "PreToolUse": [{"matcher": "Edit|Write|MultiEdit|NotebookEdit", "hooks": [{"type": "command", "command": "/Users/you/.local/bin/rationale hook pre-edit", "timeout": 5}]}],
    "PostToolUse": [{"matcher": "mcp__.*rationale.*__prepare_change", "hooks": [{"type": "command", "command": "/Users/you/.local/bin/rationale hook post-tool", "timeout": 5}]}],
    "Stop": [{"hooks": [{"type": "command", "command": "/Users/you/.local/bin/rationale hook stop", "timeout": 10}]}]
  }
}
```

`settings.local.json` is Claude Code's machine-local settings file, outside
Git. Rationale merges its entries with whatever else the file holds and
recognizes its own by the program they run, so reinstalling is idempotent and
uninstalling removes only Rationale's entries (and the file, when nothing else
is left). A file that is not valid JSON is never rewritten.

The hooks keep a little state per session in
`.rationale-local/hooks/session-<id>.json`: the operations it opened and the
files it already reminded about.

## Configure or remove

```bash
rationale install-agent --no-hooks             # protocol, skill and MCP, no hooks
rationale install-agent --stop-hook remind     # Stop only tells you; it does not ask the agent
rationale install-agent --stop-hook off        # no Stop reminder at all
rationale uninstall-agent                      # removes the hooks with everything else
```

You can also delete or edit the entries in `.claude/settings.local.json`
directly, or disable all hooks with Claude Code's own settings.

## Model and effort advice

`prepare_change` with `advise: true` (or `rationale prepare --advise`, or
`advice.enabled: true` in `.rationale/config.yaml`) adds `execution_advice`:

```json
{"tier": "deep", "model": "opus", "effort": "high", "context": "medium",
 "reasons": ["1 pinned rule(s) govern the target", "the intent looks opposed to 1 governing Record(s)", "sensitive domain: security"],
 "how": "Advice only, derived from this packet; the person's choice wins. ..."}
```

- **Effort and context are separate.** Effort rises with risk and ambiguity:
  pinned or grave rules, an intent that looks opposed to them, invariants that
  cross modules, exceptions that lost their rule, an incomplete index, and
  sensitive paths (authorization, payments, security, schema). Context rises
  with the volume of governing knowledge and the breadth of the target. A large
  context suggests a subagent with its own context window, not a larger model.
- **Aliases, not model ids.** Tiers map to Claude Code aliases — `haiku`,
  `sonnet`, `opus` by default — which the client resolves to the current model
  of each family, so the advice does not age with each release. Change the map
  in the project config:

  ```yaml
  advice:
    enabled: true
    models:
      fast: haiku
      balanced: sonnet
      deep: opus      # or fable, for the most demanding work
  ```

- **Rationale never applies it.** In Claude Code an agent can delegate a
  subtask with the Agent tool's `model` and `effort` parameters, or a subagent
  definition's `model` and `effort` frontmatter; switching the whole session
  (`/model`, `/effort`) is the person's choice.

The advice is a deterministic heuristic over the packet. It was not validated
against task outcomes; treat it as a checklist of why a change deserves care.

## What was verified

- Unit and integration tests drive every hook with Claude Code's documented
  JSON on stdin and check the output, idempotent installation, coexistence with
  other settings, and removal.
- With Claude Code 2.1.296 in headless mode, the `SessionStart` hook ran and
  Claude Code accepted its output, and the MCP server connected. The rest of
  that session could not run because the CLI's login had expired, so the
  PreToolUse, PostToolUse, and Stop hooks have not yet been observed with a
  live model.
