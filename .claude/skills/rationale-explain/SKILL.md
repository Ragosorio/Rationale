---
description: "Explains why a target exists, from the Records that govern it, before anyone simplifies or removes it."
argument-hint: "[target]"
arguments: ["target"]
disable-model-invocation: true
---

Apply Chesterton's fence to `$target`: find out why it exists before simplifying or removing it.

Reply in the language the user writes in. Keep tool names, Record ids, field values, paths, and commands verbatim.

1. Call `explain_target(target: "$target")`.
2. Explain the governing Records: their statement, authority (`pinned` or `normal`), provenance (asserted by an agent, stated by a person, or migrated), evidence, linkage, and coverage.
3. Keep retrieved facts, inferences, and unknowns apart.
4. Do not simplify or delete the target until you have explained why it exists and which constraint could break. When nothing governs it, say that the canon records no reason; that is not proof the code is unnecessary.

When the `rationale` skill is installed, its `references/explain.md` covers this in depth.
