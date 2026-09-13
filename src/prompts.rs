//! Acciones pre-hechas de Rationale.
//!
//! Esta es la única fuente para las dos superficies que las presentan:
//! prompts MCP y skills de Claude Code. Los bodies usan placeholders de
//! skills (`$target`, `$intent`, ...); Claude Code los sustituye al invocar
//! un skill y el servidor MCP hace la misma sustitución en `prompts/get`.

pub struct Action {
    pub name: &'static str,
    pub description: &'static str,
    pub argument_hint: &'static str,
    pub arguments: &'static [&'static str],
    pub user_only: bool,
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
        description: "Prepara contexto y conflictos de gobernanza antes de cambiar código.",
        argument_hint: "[target] [intent]",
        arguments: &["target", "intent"],
        user_only: false,
        allowed_tools: None,
        body: r#"Haz el preflight de Rationale para `$target` con esta intención real:

`$intent`

1. Si Codebase Memory está disponible, úsalo primero para localizar el símbolo, sus callers y los archivos relevantes. Declara su cobertura y warnings; no lo trates como autoridad sobre el porqué.
2. Llama `prepare_change(target: "$target", intent: "$intent")` y guarda el `operation_id` que devuelve: `finalize_change` lo usa para cerrar la misma operación.
3. Antes de tocar código, resume lo que gobierna el target: `critical_constraints` y `decisions` con su autoridad (`pinned` o `normal`) y procedencia, las `relationships` explicadas con su estado estructural (`observed`, `indirect`, `orphaned`, `unknown`) y su porqué, `intent_conflicts`, riesgos, `known_unknowns`, cobertura del proveedor y `budget_overflow` si aparece.
4. Si hay un Record gobernante o un conflicto con la intención, pronúnciate explícitamente sobre si la intención lo respeta, lo contradice o sigue indeterminada. No procedas en silencio ni conviertas solapamiento léxico en contradicción semántica probada. Un Record `pinned` lo fijó el proyecto: no lo esquives.
5. Una relación `orphaned` o `unknown` no prueba que la explicación sea falsa ni que la relación haya desaparecido; repórtala como incertidumbre.
6. Si falta autoridad para decidir, detente y pide la decisión humana concreta."#,
    },
    Action {
        name: "explain",
        description: "Explica por qué existe un target antes de simplificarlo.",
        argument_hint: "[target]",
        arguments: &["target"],
        user_only: false,
        allowed_tools: None,
        body: r#"Aplica la valla de Chesterton a `$target`.

1. Llama `explain_target(target: "$target")`.
2. Explica los Records gobernantes, su autoridad (`pinned` o `normal`), su procedencia (afirmado por un agente, declarado por un humano o migrado), evidencia, linkage y cobertura.
3. Distingue hechos recuperados, inferencias y desconocidos.
4. No simplifiques ni borres el target hasta explicar por qué existe y qué restricción podría romperse."#,
    },
    Action {
        name: "capture",
        description: "Cierra un cambio y escribe en el canon solo el conocimiento durable.",
        argument_hint: "[statement]",
        arguments: &["statement"],
        user_only: false,
        allowed_tools: None,
        body: r#"Cierra el cambio actual con Rationale. Statement opcional del humano:

`$statement`

Contexto Git vivo inyectado por el skill:

- HEAD actual: !`git rev-parse HEAD`
- Estado: !`git status --short`
- Diff desde HEAD: !`git diff --no-ext-diff HEAD`

Si esas líneas todavía aparecen como literales `!`comando`` (por ejemplo,
porque recibiste esta acción mediante un prompt MCP), obtiene los mismos datos
con las herramientas Git disponibles antes de continuar.

1. Revisa el diff y las pruebas ejecutadas. Separa hechos observados de intención o inferencia.
2. Decide qué conocimiento seguirá siendo cierto después de este cambio: por qué el código es como es y qué debe mantenerse. Lo que solo describe este cambio (qué se editó, pasos, estado temporal) no es memoria: va en `summary`, no en `candidates`.
3. **Una decisión por Record.** Divide en varios candidatos cuando las partes podrían reemplazarse o revocarse por separado, responden preguntas distintas o tienen vida distinta. No fragmentes una sola decisión en trozos que por separado no dicen nada.
4. Llama `finalize_change` con el `operation_id` del preflight (si existe), un `summary` breve y los `candidates`. Cada candidato lleva `kind`, `statement`, un `rationale` que dé la causa (no repita el statement), `durability: "durable"` y `bindings` con el código que gobierna (`src/x.rs` o `src/x.rs::symbol`). Nombra en `supersedes` los Records que reemplaza. Usa el statement de arriba solo si no está vacío y refleja una decisión real.
5. Si no se aprendió nada durable, llama `finalize_change` sin candidatos: no se escribe memoria y está bien.
6. Rationale escribe los candidatos válidos como Records canónicos en esa misma llamada, con procedencia de agente; no hay cola de aprobación. Reporta qué quedó escrito, qué se descartó y por qué, y qué Records quedaron reemplazados.
7. Si la respuesta trae `conflicts`, un candidato intentó reemplazar una regla fijada. Detente: muestra al humano las dos afirmaciones y la pregunta, y solo con su respuesta explícita llama `resolve_conflict(conflict_id, decision, human_answer)` transcribiendo esa respuesta. Nunca decidas por él."#,
    },
    Action {
        name: "conflicts",
        description: "Muestra conflictos con reglas fijadas y entrega la decisión al humano.",
        argument_hint: "",
        arguments: &[],
        user_only: true,
        // Mismo criterio que `health`: `rationale conflicts` imprime y sale 0
        // haya o no conflictos, así que la inyección es un comando simple.
        allowed_tools: Some("Bash(rationale conflicts:*)"),
        body: r#"Prepara la decisión humana sobre conflictos de Rationale.

Conflictos pendientes inyectados por el skill:

!`rationale conflicts`

Si la línea anterior todavía aparece como un literal `!`comando`` (por
ejemplo, mediante un prompt MCP), obtén la misma lista antes de responder.

1. Un conflicto aparece cuando un agente intentó reemplazar (`supersedes`) un Record fijado (`pinned`). Esa afirmación no se escribió en el canon: espera esta decisión.
2. Para cada conflicto, muestra la regla fijada, la afirmación nueva y la pregunta, sin inclinar la respuesta.
3. La decisión es del humano: `keep_pinned` conserva la regla fijada; `adopt_new` la reemplaza y exige autoridad declarada en `.rationale/config.yaml`.
4. Indica que puede decidir en un terminal con `rationale resolve <conflict-id> <keep-pinned|adopt-new>`. Si prefiere decidir en esta conversación, llama `resolve_conflict` solo después de su respuesta explícita y transcríbela en `human_answer`.
5. No elijas una opción en su nombre, no fijes ni desfijes Records y no afirmes una resolución antes de que la herramienta la confirme."#,
    },
    Action {
        name: "health",
        description: "Comprueba conexión MCP, proveedor y salud del canon.",
        argument_hint: "",
        arguments: &[],
        user_only: false,
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
        body: r#"Diagnostica la salud de Rationale.

Resultado local de `doctor` inyectado por el skill:

!`rationale doctor`

Si la línea anterior todavía aparece como un literal `!`comando`` (por
ejemplo, mediante un prompt MCP), ejecuta el chequeo equivalente antes de
responder.

1. Llama la herramienta MCP `health`.
2. Distingue: disponibilidad de las herramientas MCP, estado/cobertura de Codebase Memory, revisión Git y salud del canon.
3. Reporta exactamente qué funciona, qué está degradado y qué no fue comprobado. No conviertas ausencia del proveedor en ausencia del canon ni inventes cobertura."#,
    },
    Action {
        name: "protocol",
        description: "Carga el protocolo maestro completo de Rationale.",
        argument_hint: "",
        arguments: &[],
        user_only: false,
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
    replacement: "usa 'conflicts' para decidir conflictos con reglas fijadas; las propuestas \
                  pendientes de antes de vNext se procesan con `rationale migrate`",
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
        .ok_or_else(|| "arguments debe ser un objeto".to_string())?;
    let mut resolved = Vec::new();
    for argument in action.arguments {
        let value = match supplied.get(*argument) {
            Some(value) => value
                .as_str()
                .ok_or_else(|| format!("el argumento '{argument}' debe ser texto"))?,
            None if action.name == "capture" && *argument == "statement" => "",
            None => return Err(format!("falta el argumento requerido '{argument}'")),
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
            rendered.contains("para `$intent`"),
            "el valor del target debe conservarse literalmente"
        );
        assert!(rendered.contains("`valor real`"));
    }
}
