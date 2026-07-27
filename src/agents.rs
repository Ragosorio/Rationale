//! Agents — registra Rationale en los agentes de código presentes en el
//! proyecto y escribe las instrucciones que hacen que se invoque solo.
//!
//! Arquitectura_Conceptual_v0.1.md §24: instalar debe cubrir MCP +
//! instrucciones de invocación, y `uninstall` debe poder revertir
//! exactamente lo que instaló. Registrar el servidor MCP no basta — un
//! agente con herramientas disponibles y sin instrucciones no las llama
//! (v0.5 §4.12: "el agente puede olvidar llamar una herramienta MCP").
//!
//! Esto nunca toca `.rationale/` del proyecto (eso es el canon del
//! usuario) ni sobrescribe en silencio los archivos de instrucciones de
//! otros agentes: si ya existen, se les añade un bloque delimitado e
//! idempotente.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MARKER_BEGIN: &str =
    "<!-- rationale:begin (no editar a mano — `rationale uninstall-agent` lo revierte) -->";
const MARKER_END: &str = "<!-- rationale:end -->";
const MANIFEST_FILE: &str = "installed-agent-files.json";
const MASTER_PROMPT: &str = include_str!("../docs/prompt-master.md");

struct AgentTarget {
    name: &'static str,
    /// Binario en PATH cuya presencia indica que el agente está instalado.
    detect_binary: &'static str,
    /// Archivo de instrucciones del agente, relativo a la raíz del proyecto.
    instructions_file: &'static str,
    /// Archivo de configuración MCP por proyecto, si el agente soporta uno.
    /// `None` significa que el registro es global vía CLI (Codex).
    mcp_config_file: Option<&'static str>,
}

const TARGETS: &[AgentTarget] = &[
    AgentTarget {
        name: "claude-code",
        detect_binary: "claude",
        instructions_file: "CLAUDE.md",
        mcp_config_file: Some(".mcp.json"),
    },
    AgentTarget {
        name: "codex",
        detect_binary: "codex",
        instructions_file: "AGENTS.md",
        mcp_config_file: None,
    },
    AgentTarget {
        name: "cursor",
        detect_binary: "cursor-agent",
        instructions_file: ".cursor/rules/rationale.mdc",
        mcp_config_file: Some(".cursor/mcp.json"),
    },
];

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum FileAction {
    /// El archivo no existía; Rationale lo creó entero. Uninstall lo borra.
    Created,
    /// El archivo ya existía; Rationale solo añadió/actualizó una parte
    /// delimitada. Uninstall revierte solo esa parte.
    Modified,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InstalledEntry {
    agent: String,
    path: PathBuf,
    action: FileAction,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Manifest {
    entries: Vec<InstalledEntry>,
}

pub struct InstallReport {
    pub dry_run: bool,
    pub detected: Vec<String>,
    /// Líneas legibles describiendo qué se hizo (o haría, en dry-run).
    pub actions: Vec<String>,
}

/// `rationale install-agent` — Arquitectura §24.
pub fn install(
    project_root: &Path,
    rationale_local: &Path,
    binary_path: &Path,
    dry_run: bool,
) -> Result<InstallReport, String> {
    let mut manifest = load_manifest(rationale_local);
    let mut report = InstallReport {
        dry_run,
        detected: Vec::new(),
        actions: Vec::new(),
    };

    let detected_targets: Vec<&AgentTarget> = TARGETS
        .iter()
        .filter(|t| {
            find_on_path(t.detect_binary).is_some() || project_already_uses(project_root, t)
        })
        .collect();

    if detected_targets.is_empty() {
        report
            .actions
            .push("no se detectó ningún agente conocido (claude, codex, cursor-agent) en PATH ni configuración previa en el proyecto".to_string());
        return Ok(report);
    }

    for target in detected_targets {
        report.detected.push(target.name.to_string());

        let instructions_path = project_root.join(target.instructions_file);
        let (action, changed) =
            upsert_instructions_block(&instructions_path, binary_path, dry_run)?;
        if changed {
            report.actions.push(format!(
                "{}: {} bloque de instrucciones en {}",
                target.name,
                if action == FileAction::Created {
                    "creado"
                } else {
                    "actualizado"
                },
                target.instructions_file
            ));
            record_entry(&mut manifest, target.name, &instructions_path, action);
        } else {
            report.actions.push(format!(
                "{}: instrucciones ya presentes y al día en {}",
                target.name, target.instructions_file
            ));
        }

        match target.mcp_config_file {
            Some(config_rel) => {
                let config_path = project_root.join(config_rel);
                let (action, changed) = upsert_mcp_json(&config_path, binary_path, dry_run)?;
                if changed {
                    report.actions.push(format!(
                        "{}: servidor MCP registrado en {}",
                        target.name, config_rel
                    ));
                    record_entry(&mut manifest, target.name, &config_path, action);
                } else {
                    report.actions.push(format!(
                        "{}: servidor MCP ya registrado en {}",
                        target.name, config_rel
                    ));
                }
            }
            None => {
                let registered = register_codex_mcp(binary_path, dry_run)?;
                report.actions.push(format!(
                    "{}: {}",
                    target.name,
                    if registered {
                        "servidor MCP registrado globalmente (codex mcp add)"
                    } else {
                        "servidor MCP ya registrado globalmente"
                    }
                ));
            }
        }
    }

    if !dry_run {
        save_manifest(rationale_local, &manifest)?;
    }

    Ok(report)
}

/// `rationale uninstall-agent` — revierte exactamente lo que `install`
/// escribió, dejando cualquier contenido previo del usuario intacto.
pub fn uninstall(project_root: &Path, rationale_local: &Path) -> Result<Vec<String>, String> {
    let manifest = load_manifest(rationale_local);
    let mut actions = Vec::new();

    for entry in &manifest.entries {
        let path = if entry.path.is_absolute() {
            entry.path.clone()
        } else {
            project_root.join(&entry.path)
        };
        match entry.action {
            FileAction::Created => {
                if path.exists() {
                    std::fs::remove_file(&path)
                        .map_err(|e| format!("no se pudo borrar {}: {e}", path.display()))?;
                    actions.push(format!("{}: eliminado {}", entry.agent, path.display()));
                }
            }
            FileAction::Modified => {
                if path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "json")
                    .unwrap_or(false)
                {
                    remove_mcp_json_entry(&path)?;
                } else {
                    remove_instructions_block(&path)?;
                }
                actions.push(format!("{}: revertido {}", entry.agent, path.display()));
            }
        }
    }

    let manifest_path = manifest_path(rationale_local);
    if manifest_path.exists() {
        std::fs::remove_file(&manifest_path).ok();
    }

    if actions.is_empty() {
        actions.push("nada que revertir — install-agent no había registrado cambios".to_string());
    }

    Ok(actions)
}

fn project_already_uses(project_root: &Path, target: &AgentTarget) -> bool {
    project_root.join(target.instructions_file).exists()
        || target
            .mcp_config_file
            .map(|c| project_root.join(c).exists())
            .unwrap_or(false)
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var).find_map(|dir| {
        let candidate = dir.join(name);
        is_executable(&candidate).then_some(candidate)
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn instructions_block(binary_path: &Path) -> String {
    format!(
        "{MARKER_BEGIN}\n\
## Rationale — protocolo de invocación

Este proyecto usa Rationale (servidor MCP `rationale`, binario en `{bin}`)
para preservar el *por qué* del código. Sigue este protocolo:

{prompt}
{MARKER_END}\n",
        bin = binary_path.display(),
        prompt = MASTER_PROMPT.trim()
    )
}

fn upsert_instructions_block(
    path: &Path,
    binary_path: &Path,
    dry_run: bool,
) -> Result<(FileAction, bool), String> {
    let existing = std::fs::read_to_string(path).ok();
    let block = instructions_block(binary_path);

    match existing {
        None => {
            if !dry_run {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("no se pudo crear {}: {e}", parent.display()))?;
                }
                std::fs::write(path, &block)
                    .map_err(|e| format!("no se pudo escribir {}: {e}", path.display()))?;
            }
            Ok((FileAction::Created, true))
        }
        Some(content) if content.contains(MARKER_BEGIN) => {
            let current_block = extract_block(&content).unwrap_or_default();
            if current_block.trim() == block.trim() {
                Ok((FileAction::Modified, false))
            } else {
                if !dry_run {
                    let updated = replace_block(&content, &block);
                    std::fs::write(path, updated)
                        .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
                }
                Ok((FileAction::Modified, true))
            }
        }
        Some(content) => {
            if !dry_run {
                let updated = format!("{}\n\n{}", content.trim_end(), block);
                std::fs::write(path, updated)
                    .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
            }
            Ok((FileAction::Modified, true))
        }
    }
}

fn extract_block(content: &str) -> Option<String> {
    let start = content.find(MARKER_BEGIN)?;
    let end = content.find(MARKER_END)? + MARKER_END.len();
    Some(content[start..end].to_string())
}

fn replace_block(content: &str, new_block: &str) -> String {
    match (content.find(MARKER_BEGIN), content.find(MARKER_END)) {
        (Some(start), Some(end_marker_start)) => {
            let end = end_marker_start + MARKER_END.len();
            format!("{}{}{}", &content[..start], new_block, &content[end..])
        }
        _ => format!("{}\n\n{}", content.trim_end(), new_block),
    }
}

fn remove_instructions_block(path: &Path) -> Result<(), String> {
    let Some(content) = std::fs::read_to_string(path).ok() else {
        return Ok(());
    };
    match (content.find(MARKER_BEGIN), content.find(MARKER_END)) {
        (Some(start), Some(end_marker_start)) => {
            let end = end_marker_start + MARKER_END.len();
            let mut remaining = format!("{}{}", &content[..start], &content[end..]);
            while remaining.ends_with("\n\n\n") {
                remaining.pop();
            }
            let trimmed = remaining.trim();
            if trimmed.is_empty() {
                std::fs::remove_file(path)
                    .map_err(|e| format!("no se pudo borrar {}: {e}", path.display()))?;
            } else {
                std::fs::write(path, format!("{trimmed}\n"))
                    .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn upsert_mcp_json(
    path: &Path,
    binary_path: &Path,
    dry_run: bool,
) -> Result<(FileAction, bool), String> {
    let existing = std::fs::read_to_string(path).ok();
    let mut root: serde_json::Value = match &existing {
        Some(content) => serde_json::from_str(content)
            .map_err(|e| format!("{} no es JSON válido: {e}", path.display()))?,
        None => serde_json::json!({}),
    };

    let servers = root
        .as_object_mut()
        .ok_or_else(|| format!("{} no es un objeto JSON", path.display()))?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let already = servers.get("rationale").is_some();
    if already {
        return Ok((FileAction::Modified, false));
    }

    servers
        .as_object_mut()
        .ok_or_else(|| format!("{} tiene mcpServers en formato inesperado", path.display()))?
        .insert(
            "rationale".to_string(),
            serde_json::json!({
                "command": binary_path.display().to_string(),
                "args": ["serve"]
            }),
        );

    let action = if existing.is_none() {
        FileAction::Created
    } else {
        FileAction::Modified
    };

    if !dry_run {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("no se pudo crear {}: {e}", parent.display()))?;
        }
        let serialized = serde_json::to_string_pretty(&root).expect("serialize mcp config");
        std::fs::write(path, serialized + "\n")
            .map_err(|e| format!("no se pudo escribir {}: {e}", path.display()))?;
    }

    Ok((action, true))
}

fn remove_mcp_json_entry(path: &Path) -> Result<(), String> {
    let Some(content) = std::fs::read_to_string(path).ok() else {
        return Ok(());
    };
    let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Ok(());
    };
    if let Some(servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.remove("rationale");
        let servers_empty = servers.is_empty();
        if servers_empty {
            root.as_object_mut().unwrap().remove("mcpServers");
        }
    }
    let remaining_empty = root.as_object().map(|o| o.is_empty()).unwrap_or(false);
    if remaining_empty {
        std::fs::remove_file(path)
            .map_err(|e| format!("no se pudo borrar {}: {e}", path.display()))?;
    } else {
        let serialized = serde_json::to_string_pretty(&root).expect("serialize mcp config");
        std::fs::write(path, serialized + "\n")
            .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Solo la parte que no requiere un proyecto: registro global de Codex.
/// La usa `scripts/rationale-installer.sh` justo tras instalar el binario,
/// cuando todavía no existe ningún `.rationale/` — así el script deja de
/// duplicar en bash la misma lógica idempotente que ya vive aquí.
pub fn install_global_only(binary_path: &Path, dry_run: bool) -> Result<Vec<String>, String> {
    let mut actions = Vec::new();
    if find_on_path("codex").is_none() {
        actions.push("codex: no detectado en PATH, se omite el registro global".to_string());
        return Ok(actions);
    }
    let registered = register_codex_mcp(binary_path, dry_run)?;
    actions.push(format!(
        "codex: {}",
        if registered {
            "servidor MCP registrado globalmente (codex mcp add)"
        } else {
            "servidor MCP ya registrado globalmente"
        }
    ));
    Ok(actions)
}

/// Registro global de Codex — mismo patrón idempotente que
/// `scripts/rationale-installer.sh`.
fn register_codex_mcp(binary_path: &Path, dry_run: bool) -> Result<bool, String> {
    let list = std::process::Command::new("codex")
        .args(["mcp", "list"])
        .output()
        .map_err(|e| format!("no se pudo ejecutar codex: {e}"))?;
    let already = String::from_utf8_lossy(&list.stdout)
        .lines()
        .any(|line| line.split_whitespace().any(|tok| tok == "rationale"));
    if already {
        return Ok(false);
    }
    if dry_run {
        return Ok(true);
    }
    let status = std::process::Command::new("codex")
        .args(["mcp", "add", "rationale", "--"])
        .arg(binary_path)
        .arg("serve")
        .status()
        .map_err(|e| format!("no se pudo ejecutar codex mcp add: {e}"))?;
    if !status.success() {
        return Err("codex mcp add rationale falló".to_string());
    }
    Ok(true)
}

fn record_entry(manifest: &mut Manifest, agent: &str, path: &Path, action: FileAction) {
    manifest.entries.retain(|e| e.path != path);
    manifest.entries.push(InstalledEntry {
        agent: agent.to_string(),
        path: path.to_path_buf(),
        action,
    });
}

fn manifest_path(rationale_local: &Path) -> PathBuf {
    rationale_local.join(MANIFEST_FILE)
}

fn load_manifest(rationale_local: &Path) -> Manifest {
    std::fs::read_to_string(manifest_path(rationale_local))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_manifest(rationale_local: &Path, manifest: &Manifest) -> Result<(), String> {
    std::fs::create_dir_all(rationale_local)
        .map_err(|e| format!("no se pudo crear {}: {e}", rationale_local.display()))?;
    let serialized = serde_json::to_string_pretty(manifest).expect("serialize manifest");
    std::fs::write(manifest_path(rationale_local), serialized + "\n")
        .map_err(|e| format!("no se pudo escribir manifest: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "rationale-agents-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn fake_binary() -> PathBuf {
        PathBuf::from("/usr/local/bin/rationale")
    }

    #[test]
    fn instructions_block_is_idempotent() {
        let project = temp_dir("idempotent");
        let claude_md = project.join("CLAUDE.md");
        let (action1, changed1) =
            upsert_instructions_block(&claude_md, &fake_binary(), false).unwrap();
        assert_eq!(action1, FileAction::Created);
        assert!(changed1);

        let (action2, changed2) =
            upsert_instructions_block(&claude_md, &fake_binary(), false).unwrap();
        assert_eq!(action2, FileAction::Modified);
        assert!(
            !changed2,
            "segunda pasada no debe volver a escribir el mismo bloque"
        );

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn instructions_block_preserves_existing_content() {
        let project = temp_dir("preserve");
        let claude_md = project.join("CLAUDE.md");
        std::fs::write(
            &claude_md,
            "# Mis instrucciones\n\nAlgo que el usuario escribió.\n",
        )
        .unwrap();

        upsert_instructions_block(&claude_md, &fake_binary(), false).unwrap();
        let content = std::fs::read_to_string(&claude_md).unwrap();
        assert!(content.contains("Mis instrucciones"));
        assert!(content.contains(MARKER_BEGIN));

        remove_instructions_block(&claude_md).unwrap();
        let after = std::fs::read_to_string(&claude_md).unwrap();
        assert!(after.contains("Mis instrucciones"));
        assert!(!after.contains(MARKER_BEGIN));

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn instructions_block_embeds_the_canonical_master_prompt() {
        let block = instructions_block(&fake_binary());
        assert!(block.contains(MASTER_PROMPT.trim()));
        assert!(block.contains("prepare_change(target, intent)"));
        assert!(block.contains("finalize_change(...)"));
    }

    #[test]
    fn remove_instructions_block_deletes_file_it_created_entirely() {
        let project = temp_dir("delete-created");
        let claude_md = project.join("CLAUDE.md");
        upsert_instructions_block(&claude_md, &fake_binary(), false).unwrap();
        remove_instructions_block(&claude_md).unwrap();
        assert!(!claude_md.exists());
        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn mcp_json_merges_without_disturbing_other_servers() {
        let project = temp_dir("mcp-merge");
        let config = project.join(".mcp.json");
        std::fs::write(
            &config,
            r#"{"mcpServers":{"other-tool":{"command":"other","args":[]}}}"#,
        )
        .unwrap();

        let binary = PathBuf::from("/usr/local/bin/rationale");
        let (action, changed) = upsert_mcp_json(&config, &binary, false).unwrap();
        assert_eq!(action, FileAction::Modified);
        assert!(changed);

        let content = std::fs::read_to_string(&config).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(value["mcpServers"]["other-tool"].is_object());
        assert_eq!(value["mcpServers"]["rationale"]["args"][0], "serve");

        remove_mcp_json_entry(&config).unwrap();
        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert!(after["mcpServers"].get("rationale").is_none());
        assert!(after["mcpServers"]["other-tool"].is_object());

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn dry_run_touches_nothing() {
        let project = temp_dir("dry-run");
        let claude_md = project.join("CLAUDE.md");
        let (_, changed) = upsert_instructions_block(&claude_md, &fake_binary(), true).unwrap();
        assert!(changed, "dry-run debe reportar lo que haría");
        assert!(!claude_md.exists(), "dry-run no debe escribir nada");
        std::fs::remove_dir_all(project).ok();
    }
}
