//! Acciones pre-hechas de Rationale.
//!
//! Esta es la única fuente para las dos superficies que las presentan:
//! prompts MCP y skills de Claude Code. Los bodies usan placeholders de
//! skills (`$target`, `$intent`, ...); Claude Code los sustituye al invocar
//! un skill y el servidor MCP hace la misma sustitución en `prompts/get`.
//!
//! Los textos que lee un agente van en inglés y le piden responder en el
//! idioma de la persona: un solo texto sirve a cualquier usuario, y los
//! identificadores (herramientas, ids, valores de campos) quedan intactos.

pub struct Action {
    pub name: &'static str,
    pub description: &'static str,
    pub argument_hint: &'static str,
    pub arguments: &'static [&'static str],
    /// `disable-model-invocation` del skill. Todas las acciones son atajos
    /// que escribe una persona: el skill `rationale` (`skills/rationale/`)
    /// es la única entrada que el modelo elige solo. Varias descripciones
    /// casi iguales compitiendo por la misma tarea empeoran la selección, y
    /// cada descripción visible ocupa contexto en todos los turnos. En
    /// `conflicts` además es una frontera de autoridad: decidir es humano.
    pub user_only: bool,
    /// `false` cuando la acción solo existe como prompt MCP. `protocol` dejó
    /// de instalarse como skill porque `/rationale` carga el protocolo y sus
    /// playbooks; clientes sin skills siguen pidiéndolo por MCP.
    pub skill: bool,
    /// Valor de `allowed-tools` en el frontmatter del SKILL.md, si la acción
    /// inyecta un comando Bash específico. `None` para acciones que no
    /// ejecutan shell — declarar un permiso sin uso solo confundiría al
    /// usuario que revisa qué autoriza cada skill.
    pub allowed_tools: Option<&'static str>,
    pub body: &'static str,
}

pub const ACTIONS: &[Action] = &[
    Action {
        name: "preflight",
        description: "Reads the Records that govern a target and states conflicts with the intended change before editing.",
        argument_hint: "[target] [intent]",
        arguments: &["target", "intent"],
        user_only: true,
        skill: true,
        allowed_tools: None,
        body: r#"Run Rationale's preflight for `$target` with this intended change:

`$intent`

Reply in the language the user writes in. Keep tool names, Record ids, field values, paths, and commands verbatim.

1. Locate the target. If Codebase Memory is available, use it to confirm the symbol, its callers, and the files around it, and note its coverage and warnings. It tells you where code is, not why it must stay.
2. Call `prepare_change(target: "$target", intent: "$intent")` and keep the `operation_id` it returns: `finalize_change` uses it to close the same operation.
3. Before editing, summarize what governs the target: `critical_constraints` and `decisions` with their authority (`pinned` or `normal`) and provenance; the explained `relationships` with their structural state (`observed`, `indirect`, `orphaned`, `unknown`) and why they exist; `intent_conflicts`; risks; `known_unknowns`; provider coverage; and `budget_overflow` if it appears.
4. For each governing Record and intent conflict, state whether the intent respects it, contradicts it, or remains undetermined, and why. Do not proceed silently, and do not treat lexical overlap as a proven contradiction. A `pinned` Record is a rule the project fixed: when the intent contradicts one, stop and ask instead of working around it.
5. An `orphaned` or `unknown` relationship does not prove that the explanation is wrong or that the relationship is gone; report it as uncertainty.
6. When the decision is not yours to make, stop and ask the person the specific question.

When the `rationale` skill is installed, its `references/preflight.md` and `references/packet.md` cover each step and packet field in depth."#,
    },
    Action {
        name: "explain",
        description: "Explains why a target exists, from the Records that govern it, before anyone simplifies or removes it.",
        argument_hint: "[target]",
        arguments: &["target"],
        user_only: true,
        skill: true,
        allowed_tools: None,
        body: r#"Apply Chesterton's fence to `$target`: find out why it exists before simplifying or removing it.

Reply in the language the user writes in. Keep tool names, Record ids, field values, paths, and commands verbatim.

1. Call `explain_target(target: "$target")`.
2. Explain the governing Records: their statement, authority (`pinned` or `normal`), provenance (asserted by an agent, stated by a person, or migrated), evidence, linkage, and coverage.
3. Keep retrieved facts, inferences, and unknowns apart.
4. Do not simplify or delete the target until you have explained why it exists and which constraint could break. When nothing governs it, say that the canon records no reason; that is not proof the code is unnecessary.

When the `rationale` skill is installed, its `references/explain.md` covers this in depth."#,
    },
    Action {
        name: "capture",
        description: "Closes a change and writes only durable knowledge to the canon through finalize_change.",
        argument_hint: "[statement]",
        arguments: &["statement"],
        user_only: true,
        skill: true,
        allowed_tools: None,
        body: r#"Close the current change with Rationale. Optional statement from the person:

`$statement`

Live Git context injected by the skill:

- Current HEAD: !`git rev-parse HEAD`
- Status: !`git status --short`
- Diff since HEAD: !`git diff --no-ext-diff HEAD`

If those lines still appear as literal `!`command`` text (for example, because this action arrived as an MCP prompt), collect the same data with the available Git tools before continuing.

Reply in the language the user writes in. Keep tool names, Record ids, field values, paths, and commands verbatim. Write candidate statements and rationales in the language the existing Records use.

1. Review the diff and the tests that ran. Separate observed facts from intent and inference.
2. Decide which knowledge will stay true after this change: why the code is the way it is and what must be preserved. What only describes this change (what was edited, the steps, temporary state) is not memory: it goes in `summary`, not in `candidates`.
3. **One decision per Record.** Split into several candidates when the parts could be replaced or revoked separately, answer different questions, or have different lifetimes. Do not break a single decision into fragments that mean nothing apart.
4. Call `finalize_change` with the preflight's `operation_id` (if there is one), a short `summary`, and the `candidates`. Each candidate has `kind`, `statement`, a `rationale` that gives the cause (not a repeat of the statement), `durability: "durable"`, and `bindings` to the code it governs (`src/x.rs` or `src/x.rs::symbol`). Name the Records it replaces in `supersedes`. Use the statement above only when it is not empty and expresses a real decision.
5. When nothing durable was learned, call `finalize_change` without candidates: no memory is written, and that is fine.
6. Rationale writes valid candidates as canonical Records in that same call, with agent provenance; there is no approval queue. Report what was written, what was discarded and why, and which Records were superseded.
7. If the response includes `conflicts`, a candidate tried to replace a pinned rule. Stop: show the person both statements and the question, and only after their explicit answer call `resolve_conflict(conflict_id, decision, human_answer)` with their literal words. Never decide for them.

When the `rationale` skill is installed, its `references/capture.md` and `references/records.md` cover candidates in depth, and `scripts/check_candidates.py` checks them against the gate's rules before you send them."#,
    },
    Action {
        name: "conflicts",
        description: "Presents conflicts with pinned Records and hands the decision to a person.",
        argument_hint: "",
        arguments: &[],
        user_only: true,
        skill: true,
        // Mismo criterio que `health`: `rationale conflicts` imprime y sale 0
        // haya o no conflictos, así que la inyección es un comando simple.
        allowed_tools: Some("Bash(rationale conflicts:*)"),
        body: r#"Prepare the person's decision on Rationale conflicts.

Pending conflicts injected by the skill:

!`rationale conflicts`

If the line above still appears as a literal `!`command`` (for example, through an MCP prompt), get the same list before replying.

Reply in the language the user writes in. Keep conflict ids, Record ids, decisions, and commands verbatim, and quote statements exactly.

1. A conflict appears when an agent tried to replace (`supersedes`) a `pinned` Record. That assertion was not written to the canon: it waits for this decision.
2. For each conflict, show the pinned rule, the new assertion, and the question, without leaning toward an answer.
3. The decision belongs to the person: `keep_pinned` keeps the pinned rule; `adopt_new` replaces it and requires authority declared in `.rationale/config.yaml`.
4. Tell them they can decide in a terminal with `rationale resolve <conflict-id> <keep-pinned|adopt-new>`. If they prefer to decide in this conversation, call `resolve_conflict` only after their explicit answer and pass their words verbatim in `human_answer`.
5. Do not choose an option for them, do not pin or unpin Records, and do not claim a resolution before the tool confirms it."#,
    },
    Action {
        name: "health",
        description: "Checks the MCP connection, the structural provider, and the health of the canon.",
        argument_hint: "",
        arguments: &[],
        user_only: true,
        skill: true,
        // `doctor --check` existe para CI: sale 1 cuando hay hallazgos, y ese
        // contrato está testeado. Pero un skill de diagnóstico necesita lo
        // contrario — mostrar los hallazgos ES su propósito, y Claude Code
        // trata cualquier código distinto de cero como "Shell command failed"
        // y descarta el output.
        //
        // `doctor` SIN `--check` ya tiene exactamente la semántica correcta:
        // imprime el reporte y sale 0 con hallazgos, y sigue saliendo 1 ante
        // un fallo operativo real (`fail()`). No hace falta normalizar nada.
        //
        // Un intento anterior envolvió `--check` en un compuesto de shell con
        // `$?`/`$$` para traducir el código. Claude Code lo rechazó con
        // "Contains simple_expansion": su verificador de permisos no puede
        // comprobar estáticamente un comando con expansión de variables
        // contra un patrón permitido, y hace bien. La inyección debe ser un
        // comando simple.
        allowed_tools: Some("Bash(rationale doctor:*)"),
        body: r#"Diagnose Rationale's health.

Local `doctor` result injected by the skill:

!`rationale doctor`

If the line above still appears as a literal `!`command`` (for example, through an MCP prompt), run the equivalent check before replying.

Reply in the language the user writes in. Keep tool names, field values, and commands verbatim.

1. Call the MCP tool `health`.
2. Keep apart: availability of the MCP tools, Codebase Memory status and coverage, the Git revision, and the health of the canon.
3. Report exactly what works, what is degraded, and what was not checked. Do not turn a missing provider into a missing canon, and never invent coverage."#,
    },
    Action {
        name: "protocol",
        description: "Loads Rationale's full invocation protocol when the project instructions were not loaded.",
        argument_hint: "",
        arguments: &[],
        user_only: true,
        skill: false,
        allowed_tools: None,
        body: include_str!("../docs/prompt-master.md"),
    },
];

/// Una acción que Rationale instaló en versiones anteriores y ya no ofrece.
pub struct RetiredAction {
    pub name: &'static str,
    /// Qué la reemplaza, para quien todavía la invoque por su nombre.
    pub replacement: &'static str,
}

/// Acciones retiradas. Se siguen conociendo por su nombre: un manifest
/// existente todavía registra su skill —sin reconocerlo, `uninstall-agent`
/// rechazaría la entrada como ruta no administrada— y un cliente MCP puede
/// seguir pidiendo el prompt. `install-agent` retira el skill si conserva el
/// hash que Rationale escribió; `prompts/get` responde con su reemplazo.
///
/// `review` era la cola de aprobación pre-vNext; en vNext el único punto de
/// decisión humana del flujo normal es un conflicto con una regla fijada
/// (`conflicts`). `rationale review` sigue existiendo para propuestas
/// heredadas.
pub const RETIRED_ACTIONS: &[RetiredAction] = &[RetiredAction {
    name: "review",
    replacement: "use 'conflicts' to decide conflicts with pinned Records; pending proposals \
                  from before 1.0 are processed with `rationale migrate`",
}];

pub fn retired(name: &str) -> Option<&'static RetiredAction> {
    RETIRED_ACTIONS.iter().find(|retired| retired.name == name)
}

pub fn action(name: &str) -> Option<&'static Action> {
    ACTIONS.iter().find(|action| action.name == name)
}

pub fn render(action: &Action, arguments: &serde_json::Value) -> Result<String, String> {
    let supplied = arguments
        .as_object()
        .ok_or_else(|| "arguments must be an object".to_string())?;
    let mut resolved = Vec::new();
    for argument in action.arguments {
        let value = match supplied.get(*argument) {
            Some(value) => value
                .as_str()
                .ok_or_else(|| format!("argument '{argument}' must be a string"))?,
            None if action.name == "capture" && *argument == "statement" => "",
            None => return Err(format!("missing required argument '{argument}'")),
        };
        resolved.push((format!("${argument}"), value));
    }

    let mut rendered = String::with_capacity(action.body.len());
    let mut remaining = action.body;
    while !remaining.is_empty() {
        let next = resolved
            .iter()
            .filter_map(|(token, value)| remaining.find(token).map(|index| (index, token, *value)))
            .min_by_key(|(index, _, _)| *index);
        let Some((index, token, value)) = next else {
            rendered.push_str(remaining);
            break;
        };
        rendered.push_str(&remaining[..index]);
        rendered.push_str(value);
        remaining = &remaining[index + token.len()..];
    }

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_names_are_unique() {
        let mut names: Vec<_> = ACTIONS.iter().map(|action| action.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ACTIONS.len());
    }

    #[test]
    fn retired_actions_are_never_offered_again() {
        for retired in RETIRED_ACTIONS {
            assert!(
                action(retired.name).is_none(),
                "'{}' está retirada: reofrecerla reinstalaría un skill que install-agent borra",
                retired.name
            );
            assert!(!retired.replacement.trim().is_empty());
        }
    }

    /// Un solo texto en inglés sirve a cualquier persona solo si le pide al
    /// agente responder en su idioma. Sin la instrucción, un agente que lee
    /// inglés tiende a contestar en inglés a quien escribe en español.
    #[test]
    fn every_action_asks_the_agent_to_reply_in_the_users_language() {
        for action in ACTIONS {
            assert!(
                action.body.contains("in the language the user writes in"),
                "'{}' debe pedir responder en el idioma de la persona",
                action.name
            );
        }
    }

    /// El skill `rationale` es la única entrada que el modelo invoca solo;
    /// las acciones son atajos humanos. `conflicts` además es una frontera
    /// de autoridad y nunca puede volver a ser invocable por el modelo.
    #[test]
    fn actions_are_shortcuts_people_type() {
        assert!(ACTIONS.iter().all(|action| action.user_only));
        assert!(action("conflicts").unwrap().user_only);
    }

    #[test]
    fn protocol_is_an_mcp_prompt_without_a_skill() {
        let protocol = action("protocol").unwrap();
        assert!(!protocol.skill);
        assert!(ACTIONS
            .iter()
            .filter(|action| action.name != "protocol")
            .all(|action| action.skill));
    }

    #[test]
    fn render_substitutes_named_arguments_and_allows_optional_statement() {
        let preflight = action("preflight").unwrap();
        let rendered = render(
            preflight,
            &serde_json::json!({"target": "src/main.rs", "intent": "corregir init"}),
        )
        .unwrap();
        assert!(rendered.contains("src/main.rs"));
        assert!(rendered.contains("corregir init"));
        assert!(!rendered.contains("$target"));
        assert!(!rendered.contains("$intent"));

        let capture = render(action("capture").unwrap(), &serde_json::json!({})).unwrap();
        assert!(!capture.contains("$statement"));
    }

    #[test]
    fn render_does_not_reprocess_placeholders_inside_argument_values() {
        let rendered = render(
            action("preflight").unwrap(),
            &serde_json::json!({"target": "$intent", "intent": "valor real"}),
        )
        .unwrap();
        assert!(
            rendered.contains("preflight for `$intent`"),
            "el valor del target debe conservarse literalmente"
        );
        assert!(rendered.contains("`valor real`"));
    }
}
