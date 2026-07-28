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
    pub body: &'static str,
}

pub const ACTIONS: &[Action] = &[
    Action {
        name: "preflight",
        description: "Prepara contexto y conflictos de gobernanza antes de cambiar código.",
        argument_hint: "[target] [intent]",
        arguments: &["target", "intent"],
        user_only: false,
        body: r#"Haz el preflight de Rationale para `$target` con esta intención real:

`$intent`

1. Si Codebase Memory está disponible, úsalo primero para localizar el símbolo, sus callers y los archivos relevantes. Declara su cobertura y warnings; no lo trates como autoridad sobre el porqué.
2. Llama `prepare_change(target: "$target", intent: "$intent")`.
3. Antes de tocar código, resume constraints, autoridad, evidencia, linkage, cobertura del proveedor e intent conflicts.
4. Si hay un Record gobernante o un conflicto, pronúnciate explícitamente sobre si la intención lo respeta, lo contradice o sigue indeterminada. No procedas en silencio ni conviertas solapamiento léxico en contradicción semántica probada.
5. Si falta autoridad para decidir, detente y pide la decisión humana concreta."#,
    },
    Action {
        name: "explain",
        description: "Explica por qué existe un target antes de simplificarlo.",
        argument_hint: "[target]",
        arguments: &["target"],
        user_only: false,
        body: r#"Aplica la valla de Chesterton a `$target`.

1. Llama `explain_target(target: "$target")`.
2. Explica los Records gobernantes, su autoridad, evidencia, linkage y cobertura.
3. Distingue hechos recuperados, inferencias y desconocidos.
4. No simplifiques ni borres el target hasta explicar por qué existe y qué restricción podría romperse."#,
    },
    Action {
        name: "capture",
        description: "Captura hechos y una propuesta pendiente después de un cambio.",
        argument_hint: "[statement]",
        arguments: &["statement"],
        user_only: false,
        body: r#"Cierra el cambio actual con Rationale. Statement opcional del humano:

`$statement`

Contexto Git vivo inyectado por el skill:

- HEAD actual: !`git rev-parse HEAD`
- Estado: !`git status --short`
- Diff desde HEAD: !`git diff --no-ext-diff HEAD`

Si esas líneas todavía aparecen como literales `!`comando`` (por ejemplo,
porque recibiste esta acción mediante un prompt MCP), obtiene los mismos datos
con las herramientas Git disponibles antes de continuar.

1. Usa el `base_revision` real reportado por el preflight; si no existe, determina y declara la revisión base correcta en vez de inventarla.
2. Revisa el diff y las pruebas ejecutadas. Separa hechos observados de intención o inferencia.
3. Llama `finalize_change(...)` con target, base_revision, intent, statement, severity y metadatos de Subject/Record reales. Usa el statement de arriba solo si no está vacío y refleja la decisión.
4. Reporta si se escribió una propuesta pendiente o si el cambio fue mecánico. Nunca llames aprobada a una propuesta: solo `rationale review` humano puede aprobarla."#,
    },
    Action {
        name: "review",
        description: "Muestra propuestas pendientes y entrega la aprobación al humano.",
        argument_hint: "",
        arguments: &[],
        user_only: true,
        body: r#"Prepara la revisión humana de Rationale.

1. Lista los archivos YAML pendientes bajo `.rationale/proposals/` sin alterar su estado.
2. Resume qué propone cada uno y cualquier diagnóstico de YAML corrupto.
3. Indica al humano que ejecute `rationale review` en un terminal interactivo para aprobar, rechazar o saltar.
4. No ejecutes la revisión en nombre del humano, no elijas una respuesta interactiva y no afirmes aprobación antes de que exista evidencia canónica."#,
    },
    Action {
        name: "health",
        description: "Comprueba conexión MCP, proveedor y salud del canon.",
        argument_hint: "",
        arguments: &[],
        user_only: false,
        body: r#"Diagnostica la salud de Rationale.

Resultado local de `doctor` inyectado por el skill:

!`rationale doctor --check`

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
        body: include_str!("../docs/prompt-master.md"),
    },
];

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
