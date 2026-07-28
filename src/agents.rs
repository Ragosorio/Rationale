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
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MARKER_BEGIN: &str =
    "<!-- rationale:begin (no editar a mano — `rationale uninstall-agent` lo revierte) -->";
const MARKER_END: &str = "<!-- rationale:end -->";
const MANIFEST_FILE: &str = "installed-agent-files.json";
const MASTER_PROMPT: &str = include_str!("../docs/prompt-master.md");
static OWNED_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct AgentTarget {
    name: &'static str,
    /// Binario en PATH cuya presencia indica que el agente está instalado.
    detect_binary: &'static str,
    /// Archivo de instrucciones del agente, relativo a la raíz del proyecto.
    instructions_file: &'static str,
    /// Archivo de configuración MCP por proyecto, si el agente soporta uno.
    /// `None` significa que el registro es global vía CLI (Codex).
    mcp_config_file: Option<&'static str>,
    /// Directorio de skills por proyecto, si el agente los consume.
    skills_dir: Option<&'static str>,
}

const TARGETS: &[AgentTarget] = &[
    AgentTarget {
        name: "claude-code",
        detect_binary: "claude",
        instructions_file: "CLAUDE.md",
        mcp_config_file: Some(".mcp.json"),
        skills_dir: Some(".claude/skills"),
    },
    AgentTarget {
        name: "codex",
        detect_binary: "codex",
        instructions_file: "AGENTS.md",
        mcp_config_file: None,
        skills_dir: None,
    },
    AgentTarget {
        name: "cursor",
        detect_binary: "cursor-agent",
        instructions_file: ".cursor/rules/rationale.mdc",
        mcp_config_file: Some(".cursor/mcp.json"),
        skills_dir: None,
    },
];

/// Rutas repo-relativas que `install-agent` administra — la misma lista que
/// `capture::capture` necesita excluir del diff mecánico de `finalize`, para
/// que sus propios archivos de bookkeeping nunca se aten como binding a un
/// Record del usuario. Única fuente de verdad: si `TARGETS` gana un agente
/// nuevo, esta lista lo hereda sin tocar `pipeline.rs`.
pub fn managed_paths() -> Vec<&'static str> {
    let mut paths: Vec<_> = TARGETS
        .iter()
        .flat_map(|t| std::iter::once(t.instructions_file).chain(t.mcp_config_file))
        .collect();
    paths.extend(TARGETS.iter().filter_map(|target| target.skills_dir));
    paths
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum FileAction {
    /// El archivo no existía; Rationale lo creó entero. Uninstall lo borra.
    Created,
    /// El archivo ya existía; Rationale solo añadió/actualizó una parte
    /// delimitada. Uninstall revierte solo esa parte.
    Modified,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ReversalStrategy {
    /// Manifests viejos: extirpar solo el bloque o entrada MCP administrada.
    #[default]
    ManagedPart,
    /// El archivo completo pertenece a Rationale mientras conserve su hash.
    OwnedFile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InstalledEntry {
    agent: String,
    path: PathBuf,
    action: FileAction,
    #[serde(default)]
    reversal: ReversalStrategy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content_hash: Option<String>,
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

        // Valida todas las rutas de archivos completos antes de tocar
        // instrucciones o MCP. Así un `.claude` enlazado fuera del proyecto
        // falla sin dejar una instalación parcial.
        if let Some(skills_dir) = target.skills_dir {
            for action in crate::prompts::ACTIONS {
                let path = project_root
                    .join(skills_dir)
                    .join(format!("rationale-{}", action.name))
                    .join("SKILL.md");
                validate_no_symlink_components(project_root, &path)?;
            }
        }

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

        if let Some(skills_dir) = target.skills_dir {
            for action in crate::prompts::ACTIONS {
                let relative = format!("{skills_dir}/rationale-{}/SKILL.md", action.name);
                let path = project_root.join(&relative);
                let content = skill_content(action);
                let previous_hash = manifest
                    .entries
                    .iter()
                    .find(|entry| entry.path == path)
                    .and_then(|entry| entry.content_hash.as_deref());
                let outcome = upsert_owned_file(&path, content.as_bytes(), previous_hash, dry_run)?;

                if outcome.preserved {
                    report.actions.push(format!(
                        "{}: conservado {} porque contiene cambios del usuario",
                        target.name, relative
                    ));
                } else if outcome.changed {
                    report.actions.push(format!(
                        "{}: {} skill {}",
                        target.name,
                        if outcome.action == FileAction::Created {
                            "creado"
                        } else {
                            "actualizado"
                        },
                        relative
                    ));
                } else {
                    report
                        .actions
                        .push(format!("{}: skill al día en {}", target.name, relative));
                }

                if outcome.owned && !dry_run {
                    record_owned_entry(
                        &mut manifest,
                        target.name,
                        &path,
                        outcome.action,
                        &content_hash(content.as_bytes()),
                    );
                }
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
        let path = resolve_managed_entry_path(project_root, entry)?;
        // `entry.action` (Created/Modified) solo describe qué pasó al
        // instalar — nunca decide cómo se revierte. Un archivo que
        // Rationale creó puede haber ganado contenido del usuario después
        // (otro servidor MCP en el mismo `.mcp.json`, texto propio debajo
        // del bloque en `CLAUDE.md`); borrarlo entero solo porque Rationale
        // lo creó se llevaría ese contenido con él. La extirpación segura
        // — quitar solo lo que Rationale escribió, y borrar el archivo
        // completo únicamente si no queda nada más — es la misma sin
        // importar si la acción original fue Created o Modified.
        match entry.reversal {
            ReversalStrategy::ManagedPart => {
                let is_json = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "json")
                    .unwrap_or(false);
                if is_json {
                    remove_mcp_json_entry(&path)?;
                } else {
                    remove_instructions_block(&path)?;
                }
                actions.push(format!("{}: revertido {}", entry.agent, path.display()));
            }
            ReversalStrategy::OwnedFile => {
                let expected_hash = entry.content_hash.as_deref().unwrap_or("");
                match remove_owned_file_if_unchanged(&path, expected_hash)? {
                    OwnedRemoval::Removed => {
                        actions.push(format!("{}: eliminado {}", entry.agent, path.display()))
                    }
                    OwnedRemoval::Preserved => actions.push(format!(
                        "{}: conservado {} porque fue editado",
                        entry.agent,
                        path.display()
                    )),
                    OwnedRemoval::Missing => {
                        actions.push(format!("{}: ya no existe {}", entry.agent, path.display()))
                    }
                }
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

fn expected_managed_entry(
    project_root: &Path,
    candidate: &Path,
) -> Option<(&'static str, ReversalStrategy)> {
    for target in TARGETS {
        if candidate == project_root.join(target.instructions_file) {
            return Some((target.name, ReversalStrategy::ManagedPart));
        }
        if target
            .mcp_config_file
            .is_some_and(|path| candidate == project_root.join(path))
        {
            return Some((target.name, ReversalStrategy::ManagedPart));
        }
        if let Some(skills_dir) = target.skills_dir {
            if crate::prompts::ACTIONS.iter().any(|action| {
                candidate
                    == project_root
                        .join(skills_dir)
                        .join(format!("rationale-{}", action.name))
                        .join("SKILL.md")
            }) {
                return Some((target.name, ReversalStrategy::OwnedFile));
            }
        }
    }
    None
}

fn resolve_managed_entry_path(
    project_root: &Path,
    entry: &InstalledEntry,
) -> Result<PathBuf, String> {
    let candidate = if entry.path.is_absolute() {
        entry.path.clone()
    } else {
        project_root.join(&entry.path)
    };
    let Some((expected_agent, expected_reversal)) =
        expected_managed_entry(project_root, &candidate)
    else {
        return Err(format!(
            "el manifest contiene una ruta no administrada; se rechaza para no tocar contenido \
             del usuario: {}",
            entry.path.display()
        ));
    };
    if entry.agent != expected_agent || entry.reversal != expected_reversal {
        return Err(format!(
            "el manifest contiene metadata inválida para {}; se esperaba agent={expected_agent} \
             y reversal={expected_reversal:?}",
            entry.path.display()
        ));
    }
    if entry.reversal == ReversalStrategy::OwnedFile
        && entry
            .content_hash
            .as_deref()
            .is_none_or(|hash| hash.is_empty())
    {
        return Err(format!(
            "el manifest no contiene el hash requerido para el archivo administrado completo: {}",
            entry.path.display()
        ));
    }

    validate_no_symlink_components(project_root, &candidate)?;

    Ok(candidate)
}

fn validate_no_symlink_components(project_root: &Path, candidate: &Path) -> Result<(), String> {
    let canonical_root = std::fs::canonicalize(project_root).map_err(|error| {
        format!(
            "no se pudo canonicalizar {}: {error}",
            project_root.display()
        )
    })?;
    let relative = candidate.strip_prefix(project_root).map_err(|_| {
        format!(
            "la ruta administrada escapa del proyecto: {}",
            candidate.display()
        )
    })?;
    let mut current = canonical_root.clone();
    for component in relative.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "la ruta administrada atraviesa un symlink; se rechaza para no escapar del \
                     proyecto: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(format!(
                    "no se pudo validar la ruta administrada {}: {error}",
                    current.display()
                ));
            }
        }
    }
    Ok(())
}

fn project_already_uses(project_root: &Path, target: &AgentTarget) -> bool {
    project_root.join(target.instructions_file).exists()
        || target
            .mcp_config_file
            .map(|c| project_root.join(c).exists())
            .unwrap_or(false)
        || target
            .skills_dir
            .map(|dir| project_root.join(dir).exists())
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

fn skill_content(action: &crate::prompts::Action) -> String {
    let description =
        serde_json::to_string(action.description).expect("serialize skill description");
    let argument_hint =
        serde_json::to_string(action.argument_hint).expect("serialize skill argument hint");
    let arguments = serde_json::to_string(action.arguments).expect("serialize skill arguments");
    format!(
        "---\n\
description: {description}\n\
argument-hint: {argument_hint}\n\
arguments: {arguments}\n\
disable-model-invocation: {}\n\
---\n\n\
{}\n",
        action.user_only,
        action.body.trim()
    )
}

#[derive(Debug, Clone)]
struct OwnedWriteOutcome {
    action: FileAction,
    changed: bool,
    owned: bool,
    preserved: bool,
}

fn upsert_owned_file(
    path: &Path,
    desired: &[u8],
    previous_hash: Option<&str>,
    dry_run: bool,
) -> Result<OwnedWriteOutcome, String> {
    let observed = match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "no se pudo leer {} antes de actualizarlo: {error}",
                path.display()
            ));
        }
    };
    let action = if observed.is_none() {
        FileAction::Created
    } else {
        FileAction::Modified
    };

    if dry_run {
        return Ok(classify_owned_write(
            action,
            observed.as_deref(),
            desired,
            previous_hash,
        ));
    }

    // El rename reclama la entrada de directorio antes de comprobarla. Así,
    // cualquier proceso que recree `path` después del claim obtiene una
    // entrada distinta que Rationale nunca sobrescribe ni elimina.
    let claimed = claim_owned_file(path)?;
    let current = match claimed.as_ref() {
        Some(claimed_path) => match std::fs::read(claimed_path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                if !path.exists() {
                    restore_claimed_file(path, claimed_path)?;
                }
                return Err(format!(
                    "no se pudo leer el archivo reclamado {}: {error}",
                    claimed_path.display()
                ));
            }
        },
        None => None,
    };
    let outcome = classify_owned_write(action.clone(), current.as_deref(), desired, previous_hash);

    if outcome.preserved || !outcome.changed {
        if let Some(claimed_path) = claimed {
            restore_claimed_file(path, &claimed_path)?;
        }
        return Ok(outcome);
    }

    if let Err(error) = publish_owned_file_noclobber(path, desired) {
        if let Some(claimed_path) = claimed.as_ref() {
            if !path.exists() {
                restore_claimed_file(path, claimed_path)?;
            }
        }
        return Err(error);
    }

    if let Some(claimed_path) = claimed {
        std::fs::remove_file(&claimed_path).map_err(|error| {
            format!(
                "el archivo nuevo quedó instalado, pero no se pudo retirar la copia reclamada {}: \
                 {error}",
                claimed_path.display()
            )
        })?;
    }

    Ok(outcome)
}

fn classify_owned_write(
    action: FileAction,
    existing: Option<&[u8]>,
    desired: &[u8],
    previous_hash: Option<&str>,
) -> OwnedWriteOutcome {
    let existing_matches_desired = existing == Some(desired);
    let safe_to_replace = existing_matches_desired
        || match (existing, previous_hash) {
            (None, _) => true,
            (Some(bytes), Some(expected)) => content_hash(bytes) == expected,
            (Some(_), None) => false,
        };

    OwnedWriteOutcome {
        action,
        changed: safe_to_replace && !existing_matches_desired,
        owned: safe_to_replace,
        preserved: !safe_to_replace,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnedRemoval {
    Removed,
    Preserved,
    Missing,
}

fn remove_owned_file_if_unchanged(
    path: &Path,
    expected_hash: &str,
) -> Result<OwnedRemoval, String> {
    let claimed_path = match claim_owned_file(path)? {
        Some(claimed_path) => claimed_path,
        None => return Ok(OwnedRemoval::Missing),
    };
    finish_owned_removal(path, &claimed_path, expected_hash)
}

fn finish_owned_removal(
    original_path: &Path,
    claimed_path: &Path,
    expected_hash: &str,
) -> Result<OwnedRemoval, String> {
    let bytes = match std::fs::read(claimed_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if !original_path.exists() {
                restore_claimed_file(original_path, claimed_path)?;
            }
            return Err(format!(
                "no se pudo leer el archivo reclamado {}: {error}",
                claimed_path.display()
            ));
        }
    };
    if expected_hash.is_empty() || content_hash(&bytes) != expected_hash {
        restore_claimed_file(original_path, claimed_path)?;
        return Ok(OwnedRemoval::Preserved);
    }

    // Se borra la identidad que se verificó, no el nombre original. Si otro
    // proceso recreó `original_path` durante la operación, queda intacto.
    std::fs::remove_file(claimed_path).map_err(|error| {
        format!(
            "no se pudo borrar el archivo reclamado {}: {error}",
            claimed_path.display()
        )
    })?;
    if let Some(parent) = original_path.parent() {
        let _ = std::fs::remove_dir(parent);
    }
    Ok(OwnedRemoval::Removed)
}

#[derive(Debug, PartialEq, Eq)]
enum ClaimAttempt {
    Claimed,
    SourceMissing,
    DestinationOccupied,
}

fn try_claim_owned_file(path: &Path, claimed_path: &Path) -> Result<ClaimAttempt, String> {
    match std::fs::symlink_metadata(claimed_path) {
        Ok(_) => return Ok(ClaimAttempt::DestinationOccupied),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "no se pudo comprobar el destino de claim {}: {error}",
                claimed_path.display()
            ));
        }
    }
    match std::fs::rename(path, claimed_path) {
        Ok(()) => Ok(ClaimAttempt::Claimed),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ClaimAttempt::SourceMissing)
        }
        Err(error) => Err(format!(
            "no se pudo reclamar atómicamente {}: {error}",
            path.display()
        )),
    }
}

fn claim_owned_file(path: &Path) -> Result<Option<PathBuf>, String> {
    for _ in 0..32 {
        let claimed_path = unique_sibling_path(path, "claimed")?;
        match try_claim_owned_file(path, &claimed_path)? {
            ClaimAttempt::Claimed => return Ok(Some(claimed_path)),
            ClaimAttempt::SourceMissing => return Ok(None),
            ClaimAttempt::DestinationOccupied => continue,
        }
    }
    Err(format!(
        "no se encontró un nombre de claim libre para {}",
        path.display()
    ))
}

fn restore_claimed_file(original_path: &Path, claimed_path: &Path) -> Result<(), String> {
    // `hard_link` es nuestro publish no-clobber portable: falla si otro
    // proceso ya recreó el destino y deja la copia reclamada recuperable.
    std::fs::hard_link(claimed_path, original_path).map_err(|error| {
        format!(
            "no se restauró {} porque el destino reapareció o no admite publicación segura; la \
             copia se conserva en {}: {error}",
            original_path.display(),
            claimed_path.display()
        )
    })?;
    std::fs::remove_file(claimed_path).map_err(|error| {
        format!(
            "{} fue restaurado, pero no se pudo retirar la copia reclamada {}: {error}",
            original_path.display(),
            claimed_path.display()
        )
    })
}

fn publish_owned_file_noclobber(path: &Path, desired: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} no tiene directorio padre", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("no se pudo crear {}: {error}", parent.display()))?;
    let temp_path = unique_sibling_path(path, "new")?;
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "no se pudo crear el temporal seguro {}: {error}",
                    temp_path.display()
                )
            })?;
        file.write_all(desired).map_err(|error| {
            format!(
                "no se pudo escribir el temporal seguro {}: {error}",
                temp_path.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "no se pudo sincronizar el temporal seguro {}: {error}",
                temp_path.display()
            )
        })?;
    }

    let publish_result = std::fs::hard_link(&temp_path, path);
    if let Err(error) = publish_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "no se publicó {} porque el destino reapareció o no admite publicación segura; no se \
             sobrescribió ningún contenido: {error}",
            path.display()
        ));
    }
    std::fs::remove_file(&temp_path).map_err(|error| {
        format!(
            "{} quedó publicado, pero no se pudo retirar el temporal {}: {error}",
            path.display(),
            temp_path.display()
        )
    })
}

fn unique_sibling_path(path: &Path, role: &str) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} no tiene directorio padre", path.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let unique = OWNED_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(parent.join(format!(
        ".{name}.rationale-{role}-{}-{timestamp}-{unique}",
        std::process::id()
    )))
}

fn content_hash(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
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
                crate::storage::atomic_write_bytes(path, block.as_bytes())
                    .map_err(|e| format!("no se pudo escribir {}: {e}", path.display()))?;
            }
            Ok((FileAction::Created, true))
        }
        Some(content) if content.contains(MARKER_BEGIN) => {
            let current_block = extract_block(&content)?.unwrap_or_default();
            if current_block.trim() == block.trim() {
                Ok((FileAction::Modified, false))
            } else {
                if !dry_run {
                    let updated = replace_block(&content, &block)?;
                    crate::storage::atomic_write_bytes(path, updated.as_bytes())
                        .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
                }
                Ok((FileAction::Modified, true))
            }
        }
        Some(content) => {
            if !dry_run {
                let updated = format!("{}\n\n{}", content.trim_end(), block);
                crate::storage::atomic_write_bytes(path, updated.as_bytes())
                    .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
            }
            Ok((FileAction::Modified, true))
        }
    }
}

/// Ubica el bloque delimitado, validando que `rationale:end` aparezca
/// después de que termina `rationale:begin`. Sin esta validación, un
/// archivo con los marcadores invertidos o duplicados en el orden
/// equivocado (un merge conflict basta para producirlo) hacía panicar
/// `content[start..end]` con `start > end`, o — en las dos funciones que no
/// indexaban directamente — recortaba y reordenaba contenido del usuario en
/// silencio, que es peor que el panic. Ahora ambos casos devuelven un error
/// legible y ninguna función toca el archivo.
fn locate_block(content: &str) -> Result<Option<(usize, usize)>, String> {
    let Some(start) = content.find(MARKER_BEGIN) else {
        return Ok(None);
    };
    let begin_end = start + MARKER_BEGIN.len();
    match content[begin_end..].find(MARKER_END) {
        Some(offset) => Ok(Some((start, begin_end + offset + MARKER_END.len()))),
        None => Err(
            "el archivo tiene 'rationale:begin' sin un 'rationale:end' correspondiente después \
             (marcadores invertidos o incompletos, posiblemente por un merge conflict) — no se \
             modifica para no corromper contenido del usuario"
                .to_string(),
        ),
    }
}

fn extract_block(content: &str) -> Result<Option<String>, String> {
    Ok(locate_block(content)?.map(|(start, end)| content[start..end].to_string()))
}

fn replace_block(content: &str, new_block: &str) -> Result<String, String> {
    match locate_block(content)? {
        Some((start, end)) => Ok(format!(
            "{}{}{}",
            &content[..start],
            new_block,
            &content[end..]
        )),
        None => Ok(format!("{}\n\n{}", content.trim_end(), new_block)),
    }
}

fn remove_instructions_block(path: &Path) -> Result<(), String> {
    let Some(content) = std::fs::read_to_string(path).ok() else {
        return Ok(());
    };
    let Some((start, end)) = locate_block(&content)? else {
        return Ok(());
    };
    let mut remaining = format!("{}{}", &content[..start], &content[end..]);
    while remaining.ends_with("\n\n\n") {
        remaining.pop();
    }
    let trimmed = remaining.trim();
    if trimmed.is_empty() {
        std::fs::remove_file(path)
            .map_err(|e| format!("no se pudo borrar {}: {e}", path.display()))?;
    } else {
        crate::storage::atomic_write_bytes(path, format!("{trimmed}\n").as_bytes())
            .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
    }
    Ok(())
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

    let desired = serde_json::json!({
        "command": binary_path.display().to_string(),
        "args": ["serve"]
    });
    // No basta con que la clave "rationale" exista: si el binario se movió
    // (otro RATIONALE_INSTALL_DIR, global→local), la entrada vieja apunta a
    // una ruta que ya no existe y el servidor MCP falla en silencio — el
    // único síntoma es que el agente deja de ver las herramientas de
    // Rationale. Comparar el valor, no solo la presencia de la clave.
    if servers.get("rationale") == Some(&desired) {
        return Ok((FileAction::Modified, false));
    }

    servers
        .as_object_mut()
        .ok_or_else(|| format!("{} tiene mcpServers en formato inesperado", path.display()))?
        .insert("rationale".to_string(), desired);

    let action = if existing.is_none() {
        FileAction::Created
    } else {
        FileAction::Modified
    };

    if !dry_run {
        let serialized = serde_json::to_string_pretty(&root).expect("serialize mcp config");
        crate::storage::atomic_write_bytes(path, (serialized + "\n").as_bytes())
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
        crate::storage::atomic_write_bytes(path, (serialized + "\n").as_bytes())
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
        reversal: ReversalStrategy::ManagedPart,
        content_hash: None,
    });
}

fn record_owned_entry(
    manifest: &mut Manifest,
    agent: &str,
    path: &Path,
    action: FileAction,
    hash: &str,
) {
    manifest.entries.retain(|entry| entry.path != path);
    manifest.entries.push(InstalledEntry {
        agent: agent.to_string(),
        path: path.to_path_buf(),
        action,
        reversal: ReversalStrategy::OwnedFile,
        content_hash: Some(hash.to_string()),
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
    let serialized = serde_json::to_string_pretty(manifest).expect("serialize manifest");
    crate::storage::atomic_write_bytes(
        &manifest_path(rationale_local),
        (serialized + "\n").as_bytes(),
    )
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

    /// Defecto real: un `rationale:end` que aparece antes de su
    /// `rationale:begin` correspondiente (un merge conflict basta para
    /// producirlo) hacía panicar `content[start..end]` con `start > end` en
    /// `extract_block`, y corrompía contenido del usuario en silencio en
    /// `replace_block`/`remove_instructions_block` (ninguna de las dos
    /// panicaba, pero reordenaban texto sin avisar). Ahora las tres deben
    /// fallar con un error legible y dejar el archivo intacto.
    #[test]
    fn inverted_markers_error_cleanly_instead_of_panicking_or_corrupting() {
        let project = temp_dir("inverted-markers");
        let claude_md = project.join("CLAUDE.md");
        let corrupted = format!(
            "some user text\n{MARKER_END}\nleftover from a merge conflict\n{MARKER_BEGIN}\nmore text\n"
        );
        std::fs::write(&claude_md, &corrupted).unwrap();

        let result = upsert_instructions_block(&claude_md, &fake_binary(), false);
        assert!(
            result.is_err(),
            "debe rechazar marcadores invertidos, no panicar ni escribir"
        );
        assert_eq!(
            std::fs::read_to_string(&claude_md).unwrap(),
            corrupted,
            "el archivo no debe tocarse cuando los marcadores están invertidos"
        );

        let remove_result = remove_instructions_block(&claude_md);
        assert!(remove_result.is_err());
        assert_eq!(std::fs::read_to_string(&claude_md).unwrap(), corrupted);

        std::fs::remove_dir_all(project).ok();
    }

    /// Defecto real: `uninstall()` borraba el archivo entero para toda
    /// entrada `FileAction::Created`, sin comprobar si el usuario había
    /// añadido algo después de que Rationale lo creara. Un `.mcp.json` que
    /// Rationale creó con solo `rationale`, al que el usuario le agrega
    /// después otro servidor MCP, perdía ese servidor al desinstalar.
    /// `CLAUDE.md` con texto propio agregado bajo el bloque tenía el mismo
    /// problema. La extirpación segura (quitar solo el bloque/la entrada de
    /// Rationale, borrar el archivo completo solo si no queda nada más)
    /// debe aplicar sin importar si la acción original fue Created.
    #[test]
    fn uninstall_preserves_user_content_added_after_rationale_created_the_file() {
        let project = temp_dir("uninstall-preserves-created");
        let rationale_local = project.join(".rationale-local");

        let claude_md = project.join("CLAUDE.md");
        let (action, _) = upsert_instructions_block(&claude_md, &fake_binary(), false).unwrap();
        assert_eq!(action, FileAction::Created);
        let existing = std::fs::read_to_string(&claude_md).unwrap();
        std::fs::write(
            &claude_md,
            format!("{existing}\n## Notas propias\n\nEsto lo escribí yo después.\n"),
        )
        .unwrap();

        let mcp_json = project.join(".mcp.json");
        let (json_action, _) = upsert_mcp_json(&mcp_json, &fake_binary(), false).unwrap();
        assert_eq!(json_action, FileAction::Created);
        let mut root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        root["mcpServers"]["other-tool"] = serde_json::json!({"command": "other", "args": []});
        std::fs::write(
            &mcp_json,
            serde_json::to_string_pretty(&root).unwrap() + "\n",
        )
        .unwrap();

        let mut manifest = Manifest::default();
        record_entry(
            &mut manifest,
            "claude-code",
            &claude_md,
            FileAction::Created,
        );
        record_entry(&mut manifest, "claude-code", &mcp_json, FileAction::Created);
        save_manifest(&rationale_local, &manifest).unwrap();

        uninstall(&project, &rationale_local).unwrap();

        let claude_after = std::fs::read_to_string(&claude_md)
            .expect("CLAUDE.md no debe borrarse: tiene contenido del usuario");
        assert!(claude_after.contains("Notas propias"));
        assert!(!claude_after.contains(MARKER_BEGIN));

        let mcp_after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap())
                .expect(".mcp.json no debe borrarse: tiene otro servidor MCP");
        assert!(mcp_after["mcpServers"]["other-tool"].is_object());
        assert!(mcp_after["mcpServers"].get("rationale").is_none());

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

    /// Defecto real: `upsert_mcp_json` cortaba en cuanto la clave
    /// "rationale" existía, sin comparar su valor. Si el binario se mueve
    /// (otro `RATIONALE_INSTALL_DIR`, global→local), `.mcp.json` sigue
    /// apuntando a la ruta vieja y el servidor MCP falla en silencio — el
    /// único síntoma es que el agente deja de ver las herramientas.
    #[test]
    fn mcp_json_converges_when_the_binary_path_changes() {
        let project = temp_dir("mcp-converge");
        let config = project.join(".mcp.json");
        std::fs::write(
            &config,
            r#"{"mcpServers":{"rationale":{"command":"/old/path/rationale","args":["serve"]}}}"#,
        )
        .unwrap();

        let new_binary = PathBuf::from("/new/path/rationale");
        let (action, changed) = upsert_mcp_json(&config, &new_binary, false).unwrap();
        assert_eq!(action, FileAction::Modified);
        assert!(
            changed,
            "una ruta de binario distinta debe contar como cambio"
        );

        let content = std::fs::read_to_string(&config).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(
            value["mcpServers"]["rationale"]["command"],
            "/new/path/rationale"
        );

        // Repetir con la misma ruta ya actualizada: debe seguir siendo
        // idempotente, sin re-escribir.
        let (action2, changed2) = upsert_mcp_json(&config, &new_binary, false).unwrap();
        assert_eq!(action2, FileAction::Modified);
        assert!(!changed2);

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

    #[test]
    fn only_claude_code_declares_a_skills_directory() {
        assert_eq!(TARGETS[0].name, "claude-code");
        assert_eq!(TARGETS[0].skills_dir, Some(".claude/skills"));
        assert!(TARGETS
            .iter()
            .filter(|target| target.name != "claude-code")
            .all(|target| target.skills_dir.is_none()));
    }

    #[test]
    fn generated_skill_frontmatter_and_body_come_from_the_action() {
        let action = crate::prompts::action("preflight").unwrap();
        let content = skill_content(action);
        assert!(content.starts_with("---\n"));
        assert!(content.contains("argument-hint: \"[target] [intent]\""));
        assert!(content.contains("arguments: [\"target\",\"intent\"]"));
        assert!(content.contains("disable-model-invocation: false"));
        assert!(content.contains(action.description));
        assert!(content.contains(action.body.trim()));

        let review = skill_content(crate::prompts::action("review").unwrap());
        assert!(review.contains("disable-model-invocation: true"));
    }

    #[test]
    fn uninstall_removes_an_intact_owned_skill_and_preserves_an_edited_one() {
        let project = temp_dir("owned-skill-uninstall");
        let rationale_local = project.join(".rationale-local");
        let intact = project.join(".claude/skills/rationale-health/SKILL.md");
        let edited = project.join(".claude/skills/rationale-review/SKILL.md");
        let intact_content = skill_content(crate::prompts::action("health").unwrap());
        let edited_content = skill_content(crate::prompts::action("review").unwrap());
        crate::storage::atomic_write_bytes(&intact, intact_content.as_bytes()).unwrap();
        crate::storage::atomic_write_bytes(&edited, edited_content.as_bytes()).unwrap();

        let mut manifest = Manifest::default();
        record_owned_entry(
            &mut manifest,
            "claude-code",
            &intact,
            FileAction::Created,
            &content_hash(intact_content.as_bytes()),
        );
        record_owned_entry(
            &mut manifest,
            "claude-code",
            &edited,
            FileAction::Created,
            &content_hash(edited_content.as_bytes()),
        );
        save_manifest(&rationale_local, &manifest).unwrap();
        std::fs::write(
            &edited,
            format!("{edited_content}\n# edición del usuario\n"),
        )
        .unwrap();

        let actions = uninstall(&project, &rationale_local).unwrap();
        assert!(!intact.exists(), "el skill intacto debe borrarse");
        assert!(edited.exists(), "el skill editado debe conservarse");
        assert!(actions
            .iter()
            .any(|action| action.contains("conservado") && action.contains("rationale-review")));

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn claimed_removal_never_deletes_a_recreated_destination() {
        let project = temp_dir("owned-skill-claim-race");
        let skill = project.join(".claude/skills/rationale-health/SKILL.md");
        let original = b"contenido administrado";
        let replacement = b"contenido concurrente del usuario";
        crate::storage::atomic_write_bytes(&skill, original).unwrap();

        let claimed = claim_owned_file(&skill).unwrap().unwrap();
        publish_owned_file_noclobber(&skill, replacement).unwrap();
        let outcome = finish_owned_removal(&skill, &claimed, &content_hash(original)).unwrap();

        assert_eq!(outcome, OwnedRemoval::Removed);
        assert_eq!(std::fs::read(&skill).unwrap(), replacement);
        assert!(
            !claimed.exists(),
            "solo debe eliminarse la identidad reclamada y verificada"
        );

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn claim_skips_an_abandoned_destination_instead_of_overwriting_it() {
        let project = temp_dir("owned-skill-stale-claim");
        let skill = project.join(".claude/skills/rationale-health/SKILL.md");
        let stale_claim = project.join(".claude/skills/rationale-health/.stale-claim");
        crate::storage::atomic_write_bytes(&skill, b"contenido actual").unwrap();
        crate::storage::atomic_write_bytes(&stale_claim, b"claim abandonado").unwrap();

        let attempt = try_claim_owned_file(&skill, &stale_claim).unwrap();

        assert_eq!(attempt, ClaimAttempt::DestinationOccupied);
        assert_eq!(std::fs::read(&skill).unwrap(), b"contenido actual");
        assert_eq!(
            std::fs::read(&stale_claim).unwrap(),
            b"claim abandonado",
            "un claim viejo nunca debe ser reemplazado"
        );

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn no_clobber_publish_preserves_a_destination_that_reappeared() {
        let project = temp_dir("owned-skill-publish-race");
        let skill = project.join(".claude/skills/rationale-health/SKILL.md");
        crate::storage::atomic_write_bytes(&skill, b"contenido concurrente").unwrap();

        let error = publish_owned_file_noclobber(&skill, b"contenido rationale").unwrap_err();
        assert!(error.contains("no se publicó"));
        assert_eq!(std::fs::read(&skill).unwrap(), b"contenido concurrente");

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn edited_claim_is_restored_without_overwrite() {
        let project = temp_dir("owned-skill-restore");
        let skill = project.join(".claude/skills/rationale-review/SKILL.md");
        let edited = "edición del usuario".as_bytes();
        let managed = "versión administrada".as_bytes();
        crate::storage::atomic_write_bytes(&skill, edited).unwrap();

        let outcome = remove_owned_file_if_unchanged(&skill, &content_hash(managed)).unwrap();

        assert_eq!(outcome, OwnedRemoval::Preserved);
        assert_eq!(std::fs::read(&skill).unwrap(), edited);
        assert!(
            std::fs::read_dir(skill.parent().unwrap())
                .unwrap()
                .all(|entry| !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("claimed")),
            "la restauración normal no debe dejar cuarentena"
        );

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn old_manifest_entries_default_to_managed_part_reversal() {
        let serialized = r#"{
          "entries": [{
            "agent": "claude-code",
            "path": "CLAUDE.md",
            "action": "created"
          }]
        }"#;
        let manifest: Manifest = serde_json::from_str(serialized).unwrap();
        assert!(matches!(
            manifest.entries[0].reversal,
            ReversalStrategy::ManagedPart
        ));
        assert!(manifest.entries[0].content_hash.is_none());
    }

    #[test]
    fn uninstall_rejects_manifest_paths_outside_the_managed_set() {
        let project = temp_dir("malicious-manifest-path");
        let rationale_local = project.join(".rationale-local");
        let victim = project
            .parent()
            .unwrap()
            .join(format!("rationale-uninstall-victim-{}", std::process::id()));
        std::fs::write(&victim, "contenido del usuario").unwrap();

        let mut manifest = Manifest::default();
        record_owned_entry(
            &mut manifest,
            "claude-code",
            &victim,
            FileAction::Created,
            &content_hash(b"contenido del usuario"),
        );
        save_manifest(&rationale_local, &manifest).unwrap();

        let error = uninstall(&project, &rationale_local).unwrap_err();
        assert!(error.contains("ruta no administrada"));
        assert_eq!(
            std::fs::read_to_string(&victim).unwrap(),
            "contenido del usuario"
        );

        std::fs::remove_file(victim).ok();
        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn uninstall_rejects_owned_file_reversal_for_managed_part_files() {
        for relative_path in ["CLAUDE.md", ".mcp.json"] {
            let project = temp_dir("malicious-manifest-reversal");
            let rationale_local = project.join(".rationale-local");
            let victim = project.join(relative_path);
            let content = b"contenido completo del usuario";
            crate::storage::atomic_write_bytes(&victim, content).unwrap();
            let manifest = Manifest {
                entries: vec![InstalledEntry {
                    agent: "claude-code".to_string(),
                    path: victim.clone(),
                    action: FileAction::Created,
                    reversal: ReversalStrategy::OwnedFile,
                    content_hash: Some(content_hash(content)),
                }],
            };
            save_manifest(&rationale_local, &manifest).unwrap();

            let error = uninstall(&project, &rationale_local).unwrap_err();

            assert!(error.contains("metadata inválida"));
            assert_eq!(
                std::fs::read(&victim).unwrap(),
                content,
                "{relative_path} debe conservarse completo"
            );
            std::fs::remove_dir_all(project).ok();
        }
    }

    #[test]
    fn uninstall_requires_a_hash_for_owned_skill_files() {
        let project = temp_dir("manifest-owned-file-without-hash");
        let rationale_local = project.join(".rationale-local");
        let skill = project.join(".claude/skills/rationale-health/SKILL.md");
        crate::storage::atomic_write_bytes(&skill, b"contenido del usuario").unwrap();
        let manifest = Manifest {
            entries: vec![InstalledEntry {
                agent: "claude-code".to_string(),
                path: skill.clone(),
                action: FileAction::Created,
                reversal: ReversalStrategy::OwnedFile,
                content_hash: None,
            }],
        };
        save_manifest(&rationale_local, &manifest).unwrap();

        let error = uninstall(&project, &rationale_local).unwrap_err();

        assert!(error.contains("hash requerido"));
        assert_eq!(std::fs::read(&skill).unwrap(), b"contenido del usuario");
        std::fs::remove_dir_all(project).ok();
    }

    #[cfg(unix)]
    #[test]
    fn uninstall_rejects_a_managed_path_through_a_symlink() {
        use std::os::unix::fs::symlink;

        let project = temp_dir("malicious-manifest-symlink");
        let rationale_local = project.join(".rationale-local");
        let outside = temp_dir("symlink-victim");
        let victim = outside.join("SKILL.md");
        std::fs::write(&victim, "contenido del usuario").unwrap();
        std::fs::create_dir_all(project.join(".claude/skills")).unwrap();
        symlink(&outside, project.join(".claude/skills/rationale-health")).unwrap();
        let managed = project.join(".claude/skills/rationale-health/SKILL.md");

        let mut manifest = Manifest::default();
        record_owned_entry(
            &mut manifest,
            "claude-code",
            &managed,
            FileAction::Created,
            &content_hash(b"contenido del usuario"),
        );
        save_manifest(&rationale_local, &manifest).unwrap();

        let error = uninstall(&project, &rationale_local).unwrap_err();
        assert!(error.contains("symlink"));
        assert_eq!(
            std::fs::read_to_string(&victim).unwrap(),
            "contenido del usuario"
        );

        std::fs::remove_dir_all(project).ok();
        std::fs::remove_dir_all(outside).ok();
    }

    #[cfg(unix)]
    #[test]
    fn install_validation_rejects_a_skills_directory_symlink() {
        use std::os::unix::fs::symlink;

        let project = temp_dir("install-symlink-project");
        let outside = temp_dir("install-symlink-outside");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        symlink(&outside, project.join(".claude/skills")).unwrap();
        let skill = project.join(".claude/skills/rationale-health/SKILL.md");

        let error = validate_no_symlink_components(&project, &skill).unwrap_err();
        assert!(error.contains("symlink"));
        assert!(
            std::fs::read_dir(&outside).unwrap().next().is_none(),
            "la validación no debe escribir fuera del proyecto"
        );

        std::fs::remove_dir_all(project).ok();
        std::fs::remove_dir_all(outside).ok();
    }
}
