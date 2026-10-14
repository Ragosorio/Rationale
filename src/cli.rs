//! Comandos de la CLI añadidos en 1.2: operaciones abiertas, `explain`,
//! retirada de alcances y reglas declaradas por una persona. La salida para
//! personas está en inglés, igual que la documentación; con `--json` cada
//! comando devuelve un contrato estable en stdout.

use crate::{
    configuration, evaluation, git_reviewer_actor, operations, parse_string_flag,
    resolve_project_root,
};
use serde_json::json;

fn fail<T>(error: impl std::fmt::Display) -> T {
    crate::fail(error)
}

/// `rationale operations [--all] [--json]` y
/// `rationale operations abandon <operation-id> --reason "…"`.
pub fn cmd_operations(args: &[String]) {
    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let config = configuration::load(&project_root).unwrap_or_else(fail);
    let local_dir = configuration::find_rationale_local(&config.project_root);
    let positionals: Vec<&String> = {
        let mut found = Vec::new();
        let mut index = 0;
        while index < args.len() {
            if matches!(args[index].as_str(), "--project-root" | "--reason") {
                index += 2;
                continue;
            }
            if !args[index].starts_with('-') {
                found.push(&args[index]);
            }
            index += 1;
        }
        found
    };
    let json_output = args.iter().any(|a| a == "--json");

    if positionals.first().map(|s| s.as_str()) == Some("abandon") {
        let Some(operation_id) = positionals.get(1) else {
            fail::<()>("usage: rationale operations abandon <operation-id> --reason \"why\"");
            return;
        };
        let reason = parse_string_flag(args, "--reason")
            .unwrap_or_else(|| fail("--reason is required: say why no change is being finalized"));
        let by = format!("user:{}", git_reviewer_actor(&config.project_root));
        let operation =
            operations::record_abandon(&local_dir, operation_id, &reason, &by).unwrap_or_else(fail);
        if json_output {
            println!(
                "{}",
                json!({"operation_id": operation.operation_id, "state": operation.state().as_str(), "abandoned": operation.abandoned})
            );
        } else {
            println!(
                "Abandoned {} ({}). No memory was written.",
                operation.operation_id, operation.target.spec
            );
        }
        return;
    }
    if let Some(other) = positionals.first() {
        fail::<()>(format!(
            "unknown operations action '{other}'; use `rationale operations` or `rationale operations abandon <id> --reason \"…\"`"
        ));
        return;
    }

    let show_all = args.iter().any(|a| a == "--all");
    let now = evaluation::parse_timestamp_millis(&evaluation::now_rfc3339_millis()).unwrap_or(0);
    let mut listed: Vec<operations::Operation> = operations::list_all(&local_dir)
        .into_iter()
        .filter(|op| show_all || op.state() == operations::OperationState::Open)
        .collect();
    listed.reverse();
    let open = listed
        .iter()
        .filter(|op| op.state() == operations::OperationState::Open)
        .count();
    if json_output {
        let items: Vec<serde_json::Value> = listed
            .iter()
            .map(|op| {
                json!({
                    "operation_id": op.operation_id,
                    "state": op.state().as_str(),
                    "created_at": op.created_at,
                    "age_hours": op.age_hours(now),
                    "target": op.target.spec,
                    "client": op.actor.client,
                    "intent": op.intent,
                    "finalized": op.finalized,
                    "abandoned": op.abandoned,
                })
            })
            .collect();
        println!("{}", json!({"open": open, "operations": items}));
        return;
    }
    if listed.is_empty() {
        println!("No open operations.");
        return;
    }
    for op in &listed {
        let age = op
            .age_hours(now)
            .map(|h| {
                if h >= operations::STALE_AFTER_HOURS {
                    format!("{h}h, stale")
                } else {
                    format!("{h}h")
                }
            })
            .unwrap_or_else(|| "age unknown".to_string());
        println!(
            "{}  {:<10} {:<12} {}  [{}]{}",
            op.operation_id,
            op.state().as_str(),
            op.actor.client,
            op.target.spec,
            age,
            op.intent
                .as_deref()
                .map(|intent| format!(" — {}", crate::activity::bounded(intent, 80)))
                .unwrap_or_default()
        );
    }
    if open > 0 {
        println!(
            "\n{open} open. An agent closes one with finalize_change (with or without candidates) or \
             finalize_change(abandon=true); a person with `rationale operations abandon <id> --reason \"…\"`."
        );
    }
}
