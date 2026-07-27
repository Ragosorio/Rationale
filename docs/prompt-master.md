You are working in a project that uses Rationale for decision context.

Use this protocol at the start of every conversation that may change code:

1. If Codebase Memory is installed, use it first to locate the target symbol,
   its callers, and the relevant files. It tells you where the code is and
   how it connects; it does not decide why the code must remain as it is.
2. Before changing non-trivial code, call Rationale's
   `prepare_change(target, intent)` with the target you found and your actual
   intended change. Read the returned constraints, authority, evidence,
   linkage, provider coverage, and intent conflicts.
3. If the packet reports a governing constraint or a conflict with your
   intent, say so explicitly. Compare the proposed change with the Record;
   do not silently proceed and do not call an undetermined conflict a proven
   semantic contradiction. Ask for clarification when the decision is not
   yours to make.
4. If code looks unnecessarily complex, redundant, or "weird", call
   `explain_target(target)` before simplifying it. The code may be a
   Chesterton fence whose reason lives in the canon.
5. Make the smallest change consistent with the approved context. Keep
   tests, evidence, and the declared project authority in view.
6. After a non-trivial change, run the relevant tests and call
   `finalize_change(...)` so observed facts and the diff become a pending
   proposal when the capture policy requires it.
7. A proposal is not an approved Record. Never claim that a decision is
   approved until a human has completed `rationale review`.

When Codebase Memory is unavailable, continue with the coverage reported by
Rationale and state that limitation. Never invent a symbol resolution,
authority, approval, evidence, or provider result.
