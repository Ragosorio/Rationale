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

/// `rationale retire <path> --reason "…" [--moved-to <path>] [--apply [--yes]] [--json]`
/// y `rationale retire --undo <batch-id> [--force] [--json]`.
pub fn cmd_retire(args: &[String]) {
    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let config = configuration::load(&project_root).unwrap_or_else(fail);
    let local_dir = configuration::find_rationale_local(&config.project_root);
    let json_output = args.iter().any(|a| a == "--json");

    if let Some(batch_id) = parse_string_flag(args, "--undo") {
        let force = args.iter().any(|a| a == "--force");
        let report = crate::retire::undo(&config.rationale_dir, &local_dir, &batch_id, force)
            .unwrap_or_else(fail);
        if json_output {
            println!("{}", json!(report));
        } else {
            println!(
                "Restored {} Record(s) from {}.",
                report.restored.len(),
                report.batch_id
            );
            if !report.changed_since.is_empty() {
                println!(
                    "Not restored because they changed after the batch (use --force to overwrite, or Git): {}",
                    report.changed_since.join(", ")
                );
            }
        }
        if !report.changed_since.is_empty() {
            std::process::exit(1);
        }
        return;
    }

    let Some(scope) =
        crate::first_positional_arg(args, &["--project-root", "--reason", "--moved-to"])
    else {
        fail::<()>("usage: rationale retire <path> --reason \"why\" [--moved-to <path>] [--apply]");
        return;
    };
    let reason = parse_string_flag(args, "--reason")
        .unwrap_or_else(|| fail("--reason is required: say why the scope is gone"));
    let moved_to = parse_string_flag(args, "--moved-to");
    let plan = crate::retire::plan(
        &config.rationale_dir,
        &config.project_root,
        scope,
        &reason,
        moved_to.as_deref(),
    )
    .unwrap_or_else(fail);

    if !args.iter().any(|a| a == "--apply") {
        if json_output {
            println!("{}", json!(plan));
        } else {
            print_plan(&plan);
            println!("\nNothing was written. Review the plan; a person applies it with --apply.");
        }
        return;
    }

    let changes = plan.summary.retire + plan.summary.unbind + plan.summary.rebind;
    if changes == 0 {
        if json_output {
            println!("{}", json!({"plan": plan, "batch_id": null}));
        } else {
            print_plan(&plan);
            println!("\nNothing to apply.");
        }
        return;
    }
    if !args.iter().any(|a| a == "--yes") {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            fail::<()>(
                "retire --apply changes the canon: run it in an interactive terminal, or pass --yes \
                 only after a person approved this exact plan",
            );
        }
        print_plan(&plan);
        println!("\nType 'retire {changes}' to apply these {changes} change(s), anything else to cancel:");
        let mut answer = String::new();
        let _ = std::io::stdin().read_line(&mut answer);
        if answer.trim() != format!("retire {changes}") {
            println!("Cancelled. Nothing was written.");
            return;
        }
    }
    let actor = format!("user:{}", git_reviewer_actor(&config.project_root));
    let (applied, batch) = crate::retire::apply(
        &config.rationale_dir,
        &config.project_root,
        &local_dir,
        &plan,
        &actor,
    )
    .unwrap_or_else(fail);
    if json_output {
        println!(
            "{}",
            json!({"plan": applied, "batch_id": batch.batch_id, "files": batch.files.len()})
        );
    } else {
        println!(
            "Applied {} change(s) as {}. Review them with `git diff .rationale/`; undo with \
             `rationale retire --undo {}`.",
            batch.files.len(),
            batch.batch_id,
            batch.batch_id
        );
    }
}

fn print_plan(plan: &crate::retire::RetirePlan) {
    println!(
        "Retire '{}'{} — {}",
        plan.scope,
        if plan.path_exists {
            " (the path still exists)"
        } else {
            ""
        },
        plan.reason
    );
    if plan.actions.is_empty() {
        println!("No active Record is bound to it.");
        return;
    }
    for action in &plan.actions {
        println!(
            "  {:<11} {} [{} {}, {}] — {}",
            format!("{:?}", action.action).to_lowercase(),
            action.record_id,
            action.kind,
            action.severity,
            action.authority,
            action.note
        );
    }
    let s = &plan.summary;
    println!(
        "{} to retire, {} to unbind, {} to rebind, {} need a person.",
        s.retire, s.unbind, s.rebind, s.needs_human
    );
    if !plan.exceptions_left_without_rule.is_empty() {
        println!(
            "Exceptions that would be left without their rule: {}",
            plan.exceptions_left_without_rule.join(", ")
        );
    }
}

/// Valores de un flag repetible (`--bind a --bind b`).
fn repeated_flag(args: &[String], flag: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter(|(_, arg)| arg.as_str() == flag)
        .filter_map(|(index, _)| args.get(index + 1).cloned())
        .collect()
}

/// `rationale rule add <kind> "<statement>" --why "…" --bind <path[::symbol]>…`
///
/// La vía para que una persona declare una regla, un invariante o una
/// excepción sin pasar por un agente: el Record queda `human_stated` con su
/// actor de Git. Exige una terminal interactiva, igual que `pin`: un agente no
/// puede firmar como persona.
pub fn cmd_rule(args: &[String]) {
    const VALUE_FLAGS: [&str; 7] = [
        "--project-root",
        "--why",
        "--bind",
        "--severity",
        "--exception-to",
        "--supersedes",
        "--id",
    ];
    let positionals: Vec<&String> = {
        let mut found = Vec::new();
        let mut index = 0;
        while index < args.len() {
            if VALUE_FLAGS.contains(&args[index].as_str()) {
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
    let usage = "usage: rationale rule add <constraint|decision|risk|exception> \"statement\" --why \"rationale\" --bind <path[::symbol]> [--bind …] [--severity <critical|high|medium|low>] [--pin] [--exception-to <id>] [--supersedes <id>] [--id <kind.slug>]";
    let (Some(action), Some(kind), Some(statement)) =
        (positionals.first(), positionals.get(1), positionals.get(2))
    else {
        fail::<()>(usage);
        return;
    };
    if action.as_str() != "add" {
        fail::<()>(usage);
        return;
    }
    let rationale = parse_string_flag(args, "--why")
        .unwrap_or_else(|| fail("--why is required: the reason is what makes a rule durable"));
    let bindings = repeated_flag(args, "--bind");
    if bindings.is_empty() {
        fail::<()>("at least one --bind is required: a rule bound to no code is never retrieved");
    }
    let project_root = resolve_project_root(args).unwrap_or_else(fail);
    let config = configuration::load(&project_root).unwrap_or_else(fail);
    crate::require_interactive_terminal("rule add");
    let local_dir = configuration::find_rationale_local(&config.project_root);
    let actor = git_reviewer_actor(&config.project_root);
    let declared = config.authority_for_actor(&actor).declared;
    let candidate = crate::canon::Candidate {
        id: parse_string_flag(args, "--id"),
        kind: kind.to_string(),
        statement: statement.to_string(),
        rationale,
        durability: Some("durable".to_string()),
        severity: Some(parse_string_flag(args, "--severity").unwrap_or_else(|| "high".to_string())),
        bindings: bindings
            .into_iter()
            .map(crate::canon::CandidateBinding::Spec)
            .collect(),
        relationships: vec![],
        supersedes: repeated_flag(args, "--supersedes"),
        risks: vec![],
        evidence: vec![],
        subject: None,
        exception_to: repeated_flag(args, "--exception-to"),
    };
    let mut provider = crate::providers::ProviderHandle::spawn();
    let outcome = crate::canon::state_rule(
        &crate::canon_context_for_cli(&config, &local_dir),
        candidate,
        args.iter().any(|a| a == "--pin"),
        &crate::canon::HumanDecision {
            actor: actor.clone(),
            declared,
            relayed_by: None,
            human_answer: None,
        },
        &mut provider,
    );
    if args.iter().any(|a| a == "--json") {
        println!("{}", json!(outcome));
    } else {
        for committed in &outcome.committed {
            println!(
                "Stated {} ({}) as {actor}. Commit .rationale/ so the team and its agents get it.",
                committed.id, committed.authority
            );
        }
        for discarded in &outcome.discarded {
            println!("Not written: {}", discarded.detail);
        }
        for conflict in &outcome.conflicts {
            println!(
                "Conflict with pinned {}: decide it with `rationale resolve {}`.",
                conflict.pinned_record_id, conflict.conflict_id
            );
        }
        for warning in &outcome.warnings {
            println!("warning: {warning}");
        }
    }
    if outcome.committed.is_empty() {
        std::process::exit(1);
    }
}
