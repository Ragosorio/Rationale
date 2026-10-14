# Maintain the canon

Keep Records accurate as code moves. Maintenance never deletes knowledge
silently: outdated Records are superseded with a reason, retirement is planned
before it is applied, and anything that touches a pinned Record goes to a
person. Never repair the canon as a side effect of another task: report what
you found and let the person choose.

## Find problems

```bash
rationale doctor            # read-only report with severity and a fix per finding
rationale doctor --json     # the same, for tools
rationale doctor --check    # exits 1 on new errors not in .rationale/doctor-baseline.json
rationale operations        # operations prepared and never closed
```

Every finding has a severity (`error`, `warning`, `info`), a fingerprint, and a
`fix`. `--check` fails only on findings at or above `--fail-on` (default
`error`) that are not in the committed baseline; known ones are still printed.
Recording a baseline (`--update-baseline`) is the person's decision.

## Fix by case

| Finding | What to do |
|---|---|
| `retired-scope` (a folder or file was removed and active Records still bind it) | Show the person `rationale retire <path> --reason "…"` (it only plans). Applying it (`--apply`) and undoing it (`--undo <batch>`) are theirs. Pinned Records in the scope stay for them to decide |
| `binding-path-missing` (one Record, one moved or deleted path) | If the rule moved, capture a candidate bound to the new code that names the old Record in `supersedes`, with a statement precise for its new home. If it no longer applies, tell the person: revoking is `rationale review-record <record-id>` |
| `inconsistent-supersession` | Only the new Record says it replaces the old one; binaries before 1.2 still serve the old one. `rationale doctor --repair` writes the old side (the person runs it) |
| `orphaned-exception` | The rule it relaxed was replaced or revoked. Ask the person whether it still applies to the current rule; if so, capture it again with `exception_to` naming the current rule and `supersedes` naming the old exception |
| `unlinked-exception` | Find the rule it relaxes and capture it again with `exception_to` |
| `unconfirmed-symbol-missing` | A lexical binding's symbol is gone from its file: rebind it like a moved path |
| `supersession-cycle` | Report both ids; a person decides which governs |
| Record without bindings | It cannot govern anything. Supersede it with a bound candidate, or report it |
| Invalid severity or authority, unreadable Record | A hand edit broke the file. Report it; the person repairs it |
| Pre-1.0 proposals | Run `rationale migrate --dry-run --json` and show the result; migrate only when the user agrees |
| `orphaned` relationship | Check whether the call moved. If so, capture the new `relationships` and supersede the old Record; if you cannot tell, report it — `orphaned` is never permission to delete |
| An operation open for a long time | If its work happened, finalize it; if not, abandon it with the reason. Never close it as done to clear the list |
| A pinned Record is outdated | Report it with evidence. Only a person can replace or unpin it |

## After maintenance

Run `rationale doctor` again and report which findings remain, which were
fixed through capture, and which wait for a person.
