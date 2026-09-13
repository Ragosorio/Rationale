# Maintain the canon

Keep Records accurate as code moves. Maintenance never deletes knowledge
silently: outdated Records are superseded with a reason, and anything that
touches a pinned Record goes to a person.

## Find problems

```bash
rationale doctor --json    # read-only report
rationale doctor --check   # exits 1 when there are findings; suited to CI
```

`doctor` reports invalid severities or authorities, Records without bindings,
bindings to paths that no longer exist, Subjects nothing refers to, and pre-1.0
proposals that were never migrated. `rationale doctor --repair` fixes findings
one at a time with confirmation, and it belongs to the person.

## Fix by case

| Finding | What to do |
|---|---|
| Binding to a moved or renamed file | Confirm the rule still applies at the new location. Capture a candidate bound to the new code that names the old Record in `supersedes`. The statement must differ from the old one, or the gate discards it as `duplicate`, so state the rule precisely for its new home (for example, name the new symbol). A person can instead correct the Record with `rationale review-record <record-id>` |
| Binding to deleted code | Check whether the rule moved or went away with the code. If it moved, handle it as above. If it no longer applies, tell the user: revoking a Record is `rationale review-record <record-id>`, a person's command |
| Record without bindings | It cannot govern anything. Find the code it describes and supersede it with a bound candidate, or report it for review |
| Invalid severity or authority | A hand edit broke the file. Report it; the person repairs it with `doctor --repair` or `review-record` |
| Subject nothing refers to | Report it; removing it is the person's decision |
| Pre-1.0 proposals | Run `rationale migrate --dry-run --json` and show the result. Run `rationale migrate` only when the user agrees |
| `orphaned` relationship | Check whether the call still exists under another name or path. If it moved, capture a candidate with the new `relationships` that supersedes the old Record. If you cannot tell, report it; `orphaned` is never permission to delete |
| Two active Records contradict each other | Report both ids. When neither is pinned and the code settles which one is true, supersede the false one; otherwise ask |
| A pinned Record is outdated | Report it with evidence. Only a person can replace or unpin it |

## After maintenance

Run `rationale doctor --check` again and report which findings remain, which
were fixed through capture, and which wait for a person.
