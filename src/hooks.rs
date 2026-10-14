//! Integración con Claude Code por hooks — `rationale hook <evento>`.
//!
//! Las instrucciones (CLAUDE.md, el skill) son contexto, no garantía: el
//! reporte que motivó esto encontró un 20% de operaciones sin cerrar. Los
//! hooks son el respaldo mecánico, discreto y desinstalable:
//!
//! - `session-start` (SessionStart, también tras `/compact` y al retomar):
//!   una línea sobre el canon, las operaciones abiertas de sesiones
//!   anteriores y los hallazgos nuevos de `doctor`. Nada si el repositorio no
//!   usa Rationale.
//! - `pre-edit` (PreToolUse sobre Edit/Write/MultiEdit/NotebookEdit): una vez
//!   por archivo y sesión, si el archivo lo gobiernan reglas fijadas, graves o
//!   excepciones y la sesión no preparó ese cambio, lo recuerda. Nunca
//!   bloquea (`policy.no-inferred-blocks`).
//! - `post-tool` (PostToolUse sobre `prepare_change`): anota qué operaciones
//!   abrió esta sesión.
//! - `stop` (Stop): si una operación que esta sesión abrió sigue abierta, pide
//!   cerrarla — finalizar con o sin candidatos, o abandonar con motivo — una
//!   sola vez por operación (`--mode remind` solo avisa a la persona).
//!
//! Un hook nunca rompe la sesión: cualquier error interno sale con código 0 y
//! sin salida. `install` y `uninstall` administran solo las entradas propias
//! en `.claude/settings.local.json` (local, fuera de Git).

use crate::{binding_match, configuration, doctor, lifecycle, operations, storage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_STDIN: u64 = 4 * 1024 * 1024;
const SETTINGS_FILE: &str = ".claude/settings.local.json";
const EDIT_MATCHER: &str = "Edit|Write|MultiEdit|NotebookEdit";
const PREPARE_MATCHER: &str = "mcp__.*rationale.*__prepare_change";

/// Estado de una sesión de Claude Code, local a esta máquina.
#[derive(Debug, Default, Serialize, Deserialize)]
struct SessionState {
    started_at: String,
    #[serde(default)]
    operations: Vec<String>,
    #[serde(default)]
    reminded_files: Vec<String>,
    #[serde(default)]
    reminded_operations: Vec<String>,
}

fn safe_session_id(raw: &str) -> Option<String> {
    let id: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(80)
        .collect();
    (!id.is_empty()).then_some(id)
}

fn state_path(local_dir: &Path, session_id: &str) -> PathBuf {
    local_dir
        .join("hooks")
        .join(format!("session-{session_id}.json"))
}

fn load_state(local_dir: &Path, session_id: &str) -> SessionState {
    std::fs::read_to_string(state_path(local_dir, session_id))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_state(local_dir: &Path, session_id: &str, state: &SessionState) {
    if let Ok(mut bytes) = serde_json::to_vec_pretty(state) {
        bytes.push(b'\n');
        let _ = storage::atomic_write_bytes(&state_path(local_dir, session_id), &bytes);
    }
}

/// Entrada común de todos los eventos.
struct HookInput {
    json: Value,
    session_id: Option<String>,
    project_root: PathBuf,
    local_dir: PathBuf,
}

fn read_input() -> Option<HookInput> {
    let mut text = String::new();
    std::io::stdin()
        .take(MAX_STDIN)
        .read_to_string(&mut text)
        .ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let cwd = json
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;
    let project_root = configuration::find_project_root(&cwd)?;
    let local_dir = configuration::find_rationale_local(&project_root);
    let session_id = json
        .get("session_id")
        .and_then(Value::as_str)
        .and_then(safe_session_id);
    Some(HookInput {
        json,
        session_id,
        project_root,
        local_dir,
    })
}

/// `rationale hook <evento>`. Devuelve el código de salida; siempre 0 salvo
/// un uso inválido escrito a mano.
pub fn run(args: &[String]) -> i32 {
    let Some(event) = args.first() else {
        eprintln!(
            "usage: rationale hook <session-start|pre-edit|post-tool|stop> [--mode block|remind]"
        );
        return 2;
    };
    let mode = crate::parse_string_flag(args, "--mode").unwrap_or_else(|| "block".to_string());
    if !matches!(
        event.as_str(),
        "session-start" | "pre-edit" | "post-tool" | "stop"
    ) {
        eprintln!("unknown hook event '{event}'");
        return 2;
    }
    let Some(input) = read_input() else {
        return 0;
    };
    let output = match event.as_str() {
        "session-start" => session_start(&input),
        "pre-edit" => pre_edit(&input),
        "post-tool" => {
            post_tool(&input);
            None
        }
        "stop" => stop(&input, &mode),
        _ => None,
    };
    if let Some(output) = output {
        println!("{output}");
    }
    0
}

fn additional_context(event: &str, text: String) -> Value {
    json!({"hookSpecificOutput": {"hookEventName": event, "additionalContext": text}})
}

fn describe_open(open: &operations::OpenOperation) -> String {
    match open.age_hours {
        Some(hours) => format!("{} {} ({hours}h)", open.operation_id, open.target),
        None => format!("{} {}", open.operation_id, open.target),
    }
}

fn session_start(input: &HookInput) -> Option<Value> {
    let config = configuration::load(&input.project_root).ok()?;
    let records = storage::list_records(&config.rationale_dir.join("records")).unwrap_or_default();
    let summary = crate::pipeline::canon_summary(&records);
    if let Some(session_id) = &input.session_id {
        let mut state = load_state(&input.local_dir, session_id);
        if state.started_at.is_empty() {
            state.started_at = crate::evaluation::now_rfc3339_millis();
            save_state(&input.local_dir, session_id, &state);
        }
    }
    let mut lines = vec![format!(
        "Rationale keeps this repository's decisions in .rationale/ ({} active Record(s), {} \
         pinned by people). Before a non-trivial change call prepare_change(target, intent) and \
         close it with finalize_change; explain_target before simplifying odd-looking code.",
        summary.active, summary.pinned
    )];
    let open = operations::open_summary(&input.local_dir);
    if open.count > 0 {
        let listed: Vec<String> = open.oldest.iter().map(describe_open).collect();
        lines.push(format!(
            "OPEN_OPERATIONS: {} prepared change(s) were never closed: {}{}. If that work \
             happened, finalize it; if it did not, finalize_change(operation_id, abandon=true, \
             summary=why). Never finalize work that did not happen.",
            open.count,
            listed.join("; "),
            if open.count > listed.len() {
                "; …"
            } else {
                ""
            }
        ));
    }
    let report = doctor::check(&config.rationale_dir, &config.project_root);
    let baseline = doctor::load_baseline(&config.rationale_dir).ok().flatten();
    let (checked, totals) =
        doctor::classify(&report, baseline.as_ref(), doctor::FindingSeverity::Error);
    if totals.new_failing > 0 {
        let first: Vec<String> = checked
            .iter()
            .filter(|c| c.baseline == "new" && c.severity == doctor::FindingSeverity::Error)
            .take(2)
            .map(|c| crate::activity::bounded(&c.message, 140))
            .collect();
        lines.push(format!(
            "CANON_FINDINGS: `rationale doctor` reports {} new error(s), e.g. {}. Mention them to \
             the person; do not repair the canon as a side effect of another task.",
            totals.new_failing,
            first.join(" | ")
        ));
    }
    Some(additional_context("SessionStart", lines.join("\n")))
}

fn relative_file(input: &HookInput, file: &str) -> Option<String> {
    let path = Path::new(file);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        input.project_root.join(path)
    };
    let root = input.project_root.canonicalize().ok()?;
    let canonical = absolute
        .canonicalize()
        .or_else(|_| {
            absolute
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .map(|p| p.join(absolute.file_name().unwrap_or_default()))
                .ok_or(())
        })
        .ok()?;
    let rel = canonical.strip_prefix(&root).ok()?;
    let rel = rel.to_string_lossy().replace('\\', "/");
    (!rel.is_empty() && !rel.starts_with(".rationale")).then_some(rel)
}

fn pre_edit(input: &HookInput) -> Option<Value> {
    let tool_input = input.json.get("tool_input")?;
    let file = tool_input
        .get("file_path")
        .or_else(|| tool_input.get("notebook_path"))
        .and_then(Value::as_str)?;
    let rel = relative_file(input, file)?;
    let session_id = input.session_id.clone()?;
    let mut state = load_state(&input.local_dir, &session_id);
    if state.reminded_files.contains(&rel) {
        return None;
    }
    // Ya preparado en esta sesión: el agente tiene el contexto.
    let prepared_here = state.operations.iter().any(|id| {
        operations::load(&input.local_dir, id)
            .is_some_and(|op| op.target.file_path.as_deref() == Some(rel.as_str()))
    });
    if prepared_here {
        return None;
    }
    let config = configuration::load(&input.project_root).ok()?;
    let records = storage::list_records(&config.rationale_dir.join("records")).ok()?;
    let status = lifecycle::StatusIndex::build(&records);
    let key = binding_match::TargetKey {
        rel_path: Some(rel.clone()),
        symbol: None,
        is_dir: false,
    };
    let important: Vec<&storage::Record> = binding_match::governing(&key, &records)
        .into_iter()
        .map(|m| m.record)
        .filter(|r| status.is_active(r))
        .filter(|r| {
            storage::record_authority(r) == storage::RecordAuthority::Pinned
                || r.kind == "exception"
                || (r.kind == "constraint" && matches!(r.severity.as_str(), "critical" | "high"))
        })
        .collect();
    if important.is_empty() {
        return None;
    }
    state.reminded_files.push(rel.clone());
    if state.started_at.is_empty() {
        state.started_at = crate::evaluation::now_rfc3339_millis();
    }
    save_state(&input.local_dir, &session_id, &state);
    let pinned = important
        .iter()
        .filter(|r| storage::record_authority(r) == storage::RecordAuthority::Pinned)
        .count();
    let ids: Vec<String> = important
        .iter()
        .take(6)
        .map(|r| {
            if storage::record_authority(r) == storage::RecordAuthority::Pinned {
                format!("{} (pinned)", r.id)
            } else {
                r.id.clone()
            }
        })
        .collect();
    Some(additional_context(
        "PreToolUse",
        format!(
            "Rationale: {rel} is governed by {} rule(s){}: {}{}. If you have not prepared this \
             change, call prepare_change(target=\"{rel}::<symbol>\", intent=...) before editing; \
             a pinned rule is not yours to work around.",
            important.len(),
            if pinned > 0 {
                format!(", {pinned} pinned by a person")
            } else {
                String::new()
            },
            ids.join(", "),
            if important.len() > ids.len() {
                ", …"
            } else {
                ""
            }
        ),
    ))
}

/// Ids `op_…` que aparecen en un texto (la respuesta de `prepare_change`).
fn operation_ids(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("op_") {
        let tail: String = rest[at + 3..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        if tail.len() >= 8 {
            let id = format!("op_{tail}");
            if !found.contains(&id) {
                found.push(id);
            }
        }
        rest = &rest[at + 3..];
    }
    found
}

fn post_tool(input: &HookInput) {
    let tool = input
        .json
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !tool.ends_with("__prepare_change") {
        return;
    }
    let Some(session_id) = &input.session_id else {
        return;
    };
    let response = input
        .json
        .get("tool_response")
        .map(Value::to_string)
        .unwrap_or_default();
    let ids = operation_ids(&response);
    if ids.is_empty() {
        return;
    }
    let mut state = load_state(&input.local_dir, session_id);
    if state.started_at.is_empty() {
        state.started_at = crate::evaluation::now_rfc3339_millis();
    }
    for id in ids {
        if !state.operations.contains(&id) {
            state.operations.push(id);
        }
    }
    save_state(&input.local_dir, session_id, &state);
}

fn stop(input: &HookInput, mode: &str) -> Option<Value> {
    if mode == "off" {
        return None;
    }
    // Claude Code marca así el turno que un Stop anterior ya extendió.
    if input.json.get("stop_hook_active").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let session_id = input.session_id.clone()?;
    let mut state = load_state(&input.local_dir, &session_id);
    let still_open: Vec<operations::Operation> = state
        .operations
        .iter()
        .filter(|id| !state.reminded_operations.contains(id))
        .filter_map(|id| operations::load(&input.local_dir, id))
        .filter(|op| op.state() == operations::OperationState::Open)
        .collect();
    if still_open.is_empty() {
        return None;
    }
    for op in &still_open {
        state.reminded_operations.push(op.operation_id.clone());
    }
    save_state(&input.local_dir, &session_id, &state);
    let listed: Vec<String> = still_open
        .iter()
        .map(|op| format!("{} ({})", op.operation_id, op.target.spec))
        .collect();
    let message = format!(
        "Rationale: this session prepared {} change(s) that are still open: {}. If you changed \
         the code, call finalize_change(operation_id, summary, candidates — or none if nothing \
         durable was learned). If you did not, call finalize_change(operation_id, abandon=true, \
         summary=\"why\"). Do not finalize as done a change that did not happen.",
        still_open.len(),
        listed.join(", ")
    );
    Some(if mode == "remind" {
        json!({"systemMessage": message})
    } else {
        json!({"decision": "block", "reason": message})
    })
}

// ---------------------------------------------------------------------------
// Instalación en `.claude/settings.local.json`
// ---------------------------------------------------------------------------

/// Una entrada propia: `<…>/rationale hook <evento>`, con o sin comillas y
/// con o sin `.exe`.
fn our_entry(command: &str) -> bool {
    let Some((program, _)) = command.split_once(" hook ") else {
        return false;
    };
    let program = program.trim().trim_matches('"');
    Path::new(program)
        .file_stem()
        .is_some_and(|stem| stem == "rationale")
}

/// El ejecutable instalado, con ruta absoluta: igual que el registro MCP por
/// usuario (`decision.user-scoped-agent-mcp-registration`), el archivo es
/// local a la máquina y un hook no debe depender del PATH que vea el cliente.
fn hook_command(binary: &Path, event: &str) -> String {
    let program = binary.display().to_string();
    if program.contains(' ') {
        format!("\"{program}\" hook {event}")
    } else {
        format!("{program} hook {event}")
    }
}

fn hook_group(matcher: Option<&str>, command: &str, timeout: u64) -> Value {
    let mut group = json!({"hooks": [{"type": "command", "command": command, "timeout": timeout}]});
    if let Some(matcher) = matcher {
        group["matcher"] = json!(matcher);
    }
    group
}

/// Quita del mapa `hooks` cada entrada propia; elimina grupos y eventos que
/// queden vacíos. Devuelve cuántas quitó.
fn strip_ours(settings: &mut Value) -> usize {
    let mut removed = 0;
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return 0;
    };
    for groups in hooks.values_mut() {
        let Some(groups) = groups.as_array_mut() else {
            continue;
        };
        for group in groups.iter_mut() {
            if let Some(entries) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                let before = entries.len();
                entries.retain(|entry| {
                    !entry
                        .get("command")
                        .and_then(Value::as_str)
                        .is_some_and(our_entry)
                });
                removed += before - entries.len();
            }
        }
        groups.retain(|group| {
            group
                .get("hooks")
                .and_then(Value::as_array)
                .is_none_or(|entries| !entries.is_empty())
        });
    }
    hooks.retain(|_, groups| groups.as_array().is_none_or(|g| !g.is_empty()));
    if hooks.is_empty() {
        if let Some(object) = settings.as_object_mut() {
            object.remove("hooks");
        }
    }
    removed
}

fn read_settings(path: &Path) -> Result<Option<Value>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(Some(json!({}))),
        Ok(text) => serde_json::from_str(&text).map(Some).map_err(|e| {
            format!(
                "{} is not valid JSON ({e}); fix it by hand — Rationale never rewrites a file it cannot read",
                path.display()
            )
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

/// Escribe (o actualiza) las entradas propias. Idempotente: quita las
/// anteriores y escribe las actuales, conservando todo lo demás del archivo.
pub fn install(
    project_root: &Path,
    binary: &Path,
    stop_mode: &str,
    dry_run: bool,
) -> Result<String, String> {
    let path = project_root.join(SETTINGS_FILE);
    let existing = read_settings(&path)?;
    let mut settings = existing.clone().unwrap_or_else(|| json!({}));
    if !settings.is_object() {
        return Err(format!("{} is not a JSON object", path.display()));
    }
    strip_ours(&mut settings);
    let hooks = settings
        .as_object_mut()
        .expect("checked above")
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let Some(hooks) = hooks.as_object_mut() else {
        return Err(format!("{}: `hooks` is not an object", path.display()));
    };
    let stop_command = if stop_mode == "block" {
        hook_command(binary, "stop")
    } else {
        hook_command(binary, &format!("stop --mode {stop_mode}"))
    };
    for (event, group) in [
        (
            "SessionStart",
            hook_group(None, &hook_command(binary, "session-start"), 10),
        ),
        (
            "PreToolUse",
            hook_group(Some(EDIT_MATCHER), &hook_command(binary, "pre-edit"), 5),
        ),
        (
            "PostToolUse",
            hook_group(Some(PREPARE_MATCHER), &hook_command(binary, "post-tool"), 5),
        ),
        ("Stop", hook_group(None, &stop_command, 10)),
    ] {
        let groups = hooks.entry(event).or_insert_with(|| json!([]));
        if let Some(groups) = groups.as_array_mut() {
            groups.push(group);
        }
    }
    let unchanged = existing.as_ref() == Some(&settings);
    if unchanged {
        return Ok(format!(
            "claude-code: hooks already current in {SETTINGS_FILE}"
        ));
    }
    if !dry_run {
        let mut bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        storage::atomic_write_bytes(&path, &bytes)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(format!(
        "claude-code: hooks (session-start, pre-edit, post-tool, stop{}) in {SETTINGS_FILE}, a \
         local file outside Git; `rationale uninstall-agent` removes them",
        if stop_mode == "block" {
            String::new()
        } else {
            format!(" --mode {stop_mode}")
        }
    ))
}

/// Quita solo las entradas propias. Si el archivo queda vacío y no tenía nada
/// más, se borra.
pub fn uninstall(project_root: &Path) -> Result<Option<String>, String> {
    let path = project_root.join(SETTINGS_FILE);
    let Some(mut settings) = read_settings(&path)? else {
        return Ok(None);
    };
    let removed = strip_ours(&mut settings);
    if removed == 0 {
        return Ok(None);
    }
    if settings.as_object().is_some_and(|o| o.is_empty()) {
        std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    } else {
        let mut bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        storage::atomic_write_bytes(&path, &bytes)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(Some(format!(
        "claude-code: {removed} Rationale hook(s) removed from {SETTINGS_FILE}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rationale-hooks-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        dir
    }

    #[test]
    fn install_keeps_foreign_settings_and_uninstall_removes_only_ours() {
        let project = dir();
        let path = project.join(SETTINGS_FILE);
        let foreign = json!({
            "permissions": {"allow": ["Bash(npm test)"]},
            "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "./notify.sh"}]}]}
        });
        std::fs::write(&path, foreign.to_string()).unwrap();
        let binary = Path::new("/home/me/.local/bin/rationale");
        install(&project, binary, "block", false).unwrap();
        install(&project, binary, "block", false).unwrap();
        let settings: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(settings["permissions"], foreign["permissions"]);
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2, "idempotent: one foreign group, one ours");
        assert_eq!(settings["hooks"]["PreToolUse"][0]["matcher"], EDIT_MATCHER);

        uninstall(&project).unwrap();
        let settings: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(settings, foreign);
        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn a_file_we_created_is_removed_and_invalid_json_is_never_rewritten() {
        let project = dir();
        let path = project.join(SETTINGS_FILE);
        install(
            &project,
            Path::new("/opt/my tools/rationale"),
            "remind",
            false,
        )
        .unwrap();
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains(r#"\"/opt/my tools/rationale\" hook stop --mode remind"#));
        uninstall(&project).unwrap();
        assert!(!path.exists());

        std::fs::write(&path, "{ not json").unwrap();
        assert!(install(&project, Path::new("rationale"), "block", false).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn our_entries_are_recognized_in_every_form_and_others_are_not() {
        for command in [
            "rationale hook stop",
            "/home/me/.local/bin/rationale hook pre-edit",
            "\"/opt/my tools/rationale\" hook stop --mode remind",
            "C:/tools/rationale.exe hook session-start",
        ] {
            assert!(our_entry(command), "{command}");
        }
        for command in [
            "./notify.sh",
            "other hook stop",
            "rationale serve",
            "my-rationale hook x",
        ] {
            assert!(!our_entry(command), "{command}");
        }
    }

    #[test]
    fn operation_ids_are_found_in_a_tool_response() {
        let text = r#"{"content":[{"type":"text","text":"{\"operation_id\":\"op_1a13b2da508c04adb87\"}"}]}"#;
        assert_eq!(
            operation_ids(text),
            vec!["op_1a13b2da508c04adb87".to_string()]
        );
        assert!(operation_ids("op_x op_12").is_empty());
    }
}
