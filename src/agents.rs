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
use std::process::Command;
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
    /// Cabecera que el archivo de instrucciones necesita para que el agente
    /// lo reconozca, escrita **solo al crearlo**.
    ///
    /// `CLAUDE.md` y `AGENTS.md` son markdown que el cliente lee entero, así
    /// que no necesitan ninguna: `None`. Una regla de Cursor
    /// (`.cursor/rules/*.mdc`) sí — sin su frontmatter YAML, Cursor no la
    /// aplica, y el bloque quedaba escrito en un archivo que el agente
    /// ignoraba. Va solo al crear porque un archivo que ya existe tiene su
    /// propia cabecera: insertar una segunda a mitad del archivo lo
    /// corrompería.
    file_preamble: Option<&'static str>,
    /// Directorio cuya presencia prueba que el proyecto ya usa este agente,
    /// cuando no hay un archivo concreto que lo delate.
    ///
    /// `claude-code` ya se detecta por `skills_dir` (`.claude/skills`).
    /// Cursor no tenía equivalente: se detectaba solo por `.cursor/mcp.json`
    /// o por su propia regla, así que un proyecto abierto en el IDE de Cursor
    /// —que crea `.cursor/` pero no `mcp.json`— nunca se detectaba si el
    /// usuario no tenía el CLI `cursor-agent` en el `PATH`.
    detect_dir: Option<&'static str>,
}

/// Frontmatter de una regla de Cursor. `alwaysApply: true` la incluye en todo
/// el proyecto, que es lo que un protocolo de invocación necesita; con
/// `alwaysApply` activo, `globs` no acota nada y se deja vacío.
const CURSOR_RULE_PREAMBLE: &str = "---\n\
description: Protocolo de invocación de Rationale — preserva el porqué del código.\n\
globs:\n\
alwaysApply: true\n\
---\n\n";

const TARGETS: &[AgentTarget] = &[
    AgentTarget {
        name: "claude-code",
        detect_binary: "claude",
        instructions_file: "CLAUDE.md",
        mcp_config_file: Some(".mcp.json"),
        skills_dir: Some(".claude/skills"),
        file_preamble: None,
        detect_dir: None,
    },
    AgentTarget {
        name: "codex",
        detect_binary: "codex",
        instructions_file: "AGENTS.md",
        mcp_config_file: None,
        skills_dir: None,
        file_preamble: None,
        detect_dir: None,
    },
    AgentTarget {
        name: "cursor",
        detect_binary: "cursor-agent",
        instructions_file: ".cursor/rules/rationale.mdc",
        mcp_config_file: Some(".cursor/mcp.json"),
        skills_dir: None,
        file_preamble: Some(CURSOR_RULE_PREAMBLE),
        detect_dir: Some(".cursor"),
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

/// Cómo adquirió Rationale la administración de un archivo, **la primera vez**.
///
/// Es un hecho histórico, no el estado de la ejecución actual: una entrada que
/// nació `Created` sigue siendo `Created` en toda reinstalación posterior, y
/// una que nació `Modified` sigue siendo `Modified`. Reinstalar puede
/// actualizar el hash, la estrategia de reversión y normalizar la ruta —
/// nunca el `action` histórico. La migración desde rutas absolutas también lo
/// preserva.
///
/// Antes se sobrescribía con el estado observado en cada pasada, así que la
/// segunda ejecución volteaba todas las entradas de `Created` a `Modified`
/// mientras el reporte decía «ya al día». Se detectó en el piloto Monorepo;
/// ver `docs/work-items/manifest-action-not-convergent.md`.
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
    refresh_skills: bool,
    dry_run: bool,
) -> Result<InstallReport, String> {
    let mut manifest = load_manifest(rationale_local);
    let mut report = InstallReport {
        dry_run,
        detected: Vec::new(),
        actions: Vec::new(),
    };

    // Antes de cualquier escritura bajo `.rationale-local/` (ADR-0014
    // §Decision 3): el manifest se guarda al final de esta función, y
    // protegerlo después de crearlo es exactamente el orden que dejó tres
    // archivos versionados en dos repos piloto.
    if ensure_local_data_excluded(project_root, dry_run)? {
        report
            .actions
            .push(".rationale-local/ excluido localmente en .git/info/exclude".to_string());
    }
    if let Some(warning) = tracked_local_data_warning(project_root) {
        report.actions.push(warning);
    }

    // `install-agent` es la vía oficial de actualización, así que también
    // repara el estado administrativo que una versión anterior dejó — no solo
    // los bloques. Sin esto, un proyecto de alpha.7 que se haya movido queda
    // permanentemente sin poder desinstalarse.
    let migrated = migrate_legacy_absolute_entries(&mut manifest, project_root);
    if migrated > 0 {
        report.actions.push(format!(
            "manifest: {migrated} entrada(s) heredadas normalizadas a rutas relativas"
        ));
    }

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
            upsert_instructions_block(&instructions_path, target.file_preamble, dry_run)?;
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
            record_entry(
                &mut manifest,
                project_root,
                target.name,
                &instructions_path,
                action,
            );
        } else {
            report.actions.push(format!(
                "{}: instrucciones ya presentes y al día en {}",
                target.name, target.instructions_file
            ));
        }

        match target.mcp_config_file {
            Some(config_rel) => {
                let config_path = project_root.join(config_rel);
                let (action, changed) = upsert_mcp_json(&config_path, dry_run)?;
                if changed {
                    report.actions.push(format!(
                        "{}: servidor MCP registrado en {}",
                        target.name, config_rel
                    ));
                    record_entry(
                        &mut manifest,
                        project_root,
                        target.name,
                        &config_path,
                        action,
                    );
                } else {
                    report.actions.push(format!(
                        "{}: servidor MCP ya registrado en {}",
                        target.name, config_rel
                    ));
                }
            }
            None if find_on_path(target.detect_binary).is_none() => {
                // Detectado por configuración del proyecto (p. ej. `AGENTS.md`
                // heredado), no por PATH: no hay binario que invocar. Fallar
                // aquí abortaría la instalación completa de los demás agentes
                // por un target que ni siquiera está instalado en esta máquina.
                report.actions.push(format!(
                    "{}: no se encontró el binario en PATH, se omite el registro global de MCP",
                    target.name
                ));
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
                // El manifest guarda rutas RELATIVAS al proyecto desde la
                // migración de rutas portables; `path` es absoluta. Comparar
                // las dos con `==` no casaba nunca, así que `previous_hash`
                // salía siempre `None` y todo skill se conservaba como si el
                // usuario lo hubiera editado — incluso con el hash correcto
                // registrado. Esto hacía indistribuible cualquier corrección
                // de skill. `existing_entry_index` ya canoniza los dos lados.
                let previous_hash = existing_entry_index(&manifest, project_root, &path)
                    .and_then(|index| manifest.entries[index].content_hash.as_deref());
                let outcome = upsert_owned_file(
                    &path,
                    content.as_bytes(),
                    previous_hash,
                    refresh_skills,
                    dry_run,
                )?;

                if outcome.preserved {
                    report.actions.push(match outcome.preserve_reason {
                        Some(PreserveReason::UserEdited) => format!(
                            "{}: conservado {} porque contiene cambios del usuario",
                            target.name, relative
                        ),
                        // Sin entrada en el manifest no hay prueba de nada. Se
                        // conserva por prudencia, pero se dice la verdad y se
                        // nombra la salida.
                        _ => format!(
                            "{}: conservado {} — procedencia desconocida (sin registro en el \
                             manifest local). Si Rationale lo escribió, \
                             `install-agent --refresh-skills` lo regenera",
                            target.name, relative
                        ),
                    });
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
                        project_root,
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
                    remove_instructions_block(
                        &path,
                        preamble_for_instructions_path(project_root, &path),
                    )?;
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
        || target
            .detect_dir
            .map(|dir| project_root.join(dir).is_dir())
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

/// El bloque **no** menciona la ruta del binario (ADR-0014 §Decision 7).
///
/// `CLAUDE.md` y `AGENTS.md` son documentación compartida y versionada: la
/// instalación de una persona concreta no pertenece ahí, y en los dos repos
/// piloto llegó comiteada a `origin/main`. Quien lea estas instrucciones es un
/// agente que invoca herramientas MCP por nombre, no rutas del filesystem —
/// nunca necesitó el path. La resolución del ejecutable es un problema
/// distinto y vive solo en la configuración MCP (ADR-0015).
fn instructions_block() -> String {
    format!(
        "{MARKER_BEGIN}\n\
## Rationale — protocolo de invocación

Este proyecto usa Rationale (servidor MCP `rationale`) para preservar el
*por qué* del código. Sigue este protocolo:

{prompt}
{MARKER_END}\n",
        prompt = MASTER_PROMPT.trim()
    )
}

fn skill_content(action: &crate::prompts::Action) -> String {
    let description =
        serde_json::to_string(action.description).expect("serialize skill description");
    let argument_hint =
        serde_json::to_string(action.argument_hint).expect("serialize skill argument hint");
    let arguments = serde_json::to_string(action.arguments).expect("serialize skill arguments");
    // Solo las acciones que inyectan un comando Bash concreto declaran
    // `allowed-tools` — sin esto, Claude Code pide aprobación interactiva
    // la primera vez que el skill corre, incluso en una instalación limpia.
    let allowed_tools_line = match action.allowed_tools {
        Some(pattern) => format!("allowed-tools: {pattern}\n"),
        None => String::new(),
    };
    format!(
        "---\n\
description: {description}\n\
argument-hint: {argument_hint}\n\
arguments: {arguments}\n\
disable-model-invocation: {}\n\
{allowed_tools_line}\
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
    /// Presente solo cuando `preserved` es `true`.
    preserve_reason: Option<PreserveReason>,
}

fn upsert_owned_file(
    path: &Path,
    desired: &[u8],
    previous_hash: Option<&str>,
    adopt_unknown: bool,
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
            adopt_unknown,
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
    let outcome = classify_owned_write(
        action.clone(),
        current.as_deref(),
        desired,
        previous_hash,
        adopt_unknown,
    );

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

/// Por qué un archivo owned no se reemplazó. Las dos razones se veían iguales
/// desde fuera y no lo son:
///
/// - `UserEdited`: su hash no coincide con el que el manifest registró. Eso
///   **prueba** que alguien lo cambió después de que Rationale lo escribiera.
/// - `ProvenanceUnknown`: no hay entrada en el manifest, así que Rationale no
///   sabe si lo escribió él o el usuario. Decir "contiene cambios del usuario"
///   aquí es afirmar algo que no se comprobó.
///
/// La distinción no es cosmética. `.rationale-local/` es local-only y nunca se
/// versiona (ADR-0014), mientras los skills sí se comitean: cualquiera que
/// clone el proyecto cae en `ProvenanceUnknown` para los seis, y con el
/// mensaje anterior se le decía que él los había editado mientras se le
/// negaban las correcciones en silencio.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PreserveReason {
    UserEdited,
    ProvenanceUnknown,
}

fn classify_owned_write(
    action: FileAction,
    existing: Option<&[u8]>,
    desired: &[u8],
    previous_hash: Option<&str>,
    adopt_unknown: bool,
) -> OwnedWriteOutcome {
    let existing_matches_desired = existing == Some(desired);
    let mut preserve_reason = None;
    let safe_to_replace = existing_matches_desired
        || match (existing, previous_hash) {
            (None, _) => true,
            (Some(bytes), Some(expected)) => {
                if content_hash(bytes) == expected {
                    true
                } else {
                    preserve_reason = Some(PreserveReason::UserEdited);
                    false
                }
            }
            // Sin manifest no hay prueba en ninguna dirección. Por defecto se
            // conserva —nunca destruir trabajo del usuario ante la duda— pero
            // `--refresh-skills` permite al humano resolver la duda de forma
            // explícita, sin tocar el caso `UserEdited`, que sí está probado.
            (Some(_), None) => {
                if adopt_unknown {
                    true
                } else {
                    preserve_reason = Some(PreserveReason::ProvenanceUnknown);
                    false
                }
            }
        };

    OwnedWriteOutcome {
        action,
        changed: safe_to_replace && !existing_matches_desired,
        owned: safe_to_replace,
        preserved: !safe_to_replace,
        preserve_reason,
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
    preamble: Option<&str>,
    dry_run: bool,
) -> Result<(FileAction, bool), String> {
    let existing = std::fs::read_to_string(path).ok();
    let block = instructions_block();

    match existing {
        None => {
            // El preámbulo solo participa aquí: es la cabecera que el archivo
            // necesita para existir como tal (el frontmatter de una regla de
            // Cursor). En las ramas de abajo el archivo ya existe y tiene la
            // suya, si su formato la exige.
            let contents = match preamble {
                Some(preamble) => format!("{preamble}{block}"),
                None => block.clone(),
            };
            if !dry_run {
                crate::storage::atomic_write_bytes(path, contents.as_bytes())
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
        Some((start, end)) => {
            // `new_block` ya termina en `\n`. Si lo único que seguía al bloque
            // era el salto final del archivo, concatenarlo dejaba `\n\n` al
            // final: `git diff --check` lo reporta como "new blank line at
            // EOF" y en este repo eso es una puerta de CI. Se conserva
            // cualquier contenido real posterior; solo se descarta el relleno
            // en blanco del final.
            let tail = &content[end..];
            let tail = if tail.trim().is_empty() { "" } else { tail };
            Ok(format!("{}{}{}", &content[..start], new_block, tail))
        }
        None => Ok(format!("{}\n\n{}", content.trim_end(), new_block)),
    }
}

/// Cabecera que Rationale escribiría en este archivo de instrucciones, si
/// alguna. Se busca por destino administrado, no por extensión: la lista de
/// `TARGETS` es la única fuente.
fn preamble_for_instructions_path(project_root: &Path, path: &Path) -> Option<&'static str> {
    TARGETS
        .iter()
        .find(|target| project_root.join(target.instructions_file) == path)
        .and_then(|target| target.file_preamble)
}

fn remove_instructions_block(path: &Path, preamble: Option<&str>) -> Result<(), String> {
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
    // La cabecera también la escribió Rationale: si es todo lo que queda, el
    // archivo no tiene nada del usuario y dejarlo ahí sería filtrar 100 bytes
    // de Rationale en un proyecto que pidió desinstalar. Si en cambio el
    // usuario añadió contenido propio, el archivo se conserva **con** su
    // cabecera: en un `.mdc` quitarla invalidaría la regla que el usuario
    // quiere mantener.
    let only_our_preamble = preamble
        .map(|preamble| !trimmed.is_empty() && trimmed == preamble.trim())
        .unwrap_or(false);
    if trimmed.is_empty() || only_our_preamble {
        std::fs::remove_file(path)
            .map_err(|e| format!("no se pudo borrar {}: {e}", path.display()))?;
    } else {
        crate::storage::atomic_write_bytes(path, format!("{trimmed}\n").as_bytes())
            .map_err(|e| format!("no se pudo actualizar {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Comando lógico, nunca una ruta absoluta (ADR-0015 §Decision 1).
///
/// La ruta absoluta se justificaba con que un cliente MCP podría no heredar el
/// `PATH`. Este mismo repositorio arranca su servidor con un `"command":
/// "cargo"` pelado, y `cargo` vive en `~/.cargo/bin`, que no está en el `PATH`
/// por defecto de macOS: la ruta absoluta no es necesaria cuando el cliente se
/// lanza desde un shell. Sigue sin verificarse el lanzamiento desde GUI, y ese
/// riesgo residual está declarado en el ADR. Lo que decide es la asimetría: la
/// ruta absoluta falla para *todo* integrante que no sea quien instaló, y estos
/// archivos son configuración compartida y versionada.
const MCP_COMMAND: &str = "rationale";

fn upsert_mcp_json(path: &Path, dry_run: bool) -> Result<(FileAction, bool), String> {
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
        "command": MCP_COMMAND,
        "args": ["serve"]
    });
    // No basta con que la clave "rationale" exista: una entrada escrita por una
    // versión anterior lleva la ruta absoluta del binario de quien instaló, y
    // en la máquina de cualquier otro apunta a algo que no existe — el servidor
    // MCP falla en silencio y el único síntoma es que el agente deja de ver las
    // herramientas de Rationale. Comparar el valor, no solo la presencia de la
    // clave, es lo que convierte a `install-agent` en la vía de migración.
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

/// Ruta local-only que Rationale nunca debe dejar versionada en el proyecto
/// del usuario (ADR-0014 §Decision 1). El patrón es de directorio: cualquier
/// emisor nuevo bajo `.rationale-local/` queda cubierto sin decisión adicional.
const LOCAL_DATA_EXCLUDE_PATTERN: &str = ".rationale-local/";

/// Directorio Git *común* del proyecto, o `None` si no hay repositorio.
///
/// No se asume `<root>/.git/info/exclude` (ADR-0014 §Decision 4): en un
/// submódulo o un worktree, `.git` es un archivo con un puntero `gitdir:`, y
/// en un worktree el `info/exclude` que aplica vive en el directorio común
/// compartido, no en el del worktree. `--git-common-dir` resuelve ambos casos
/// y es la única fuente de verdad — parsear el puntero a mano reimplementaría
/// mal lo que Git ya sabe hacer.
fn git_common_dir(project_root: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = PathBuf::from(trimmed);
    // `--git-common-dir` devuelve una ruta relativa al `-C` cuando puede.
    Some(if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    })
}

/// Instala la exclusión local de `.rationale-local/` en `info/exclude`.
///
/// `info/exclude` y no `.gitignore` (ADR-0014 §Decision 2): `.gitignore` es un
/// archivo compartido y versionado del proyecto ajeno, y Rationale no tiene
/// por qué imponerle un cambio de repo al equipo para proteger sus propios
/// artefactos. `info/exclude` es local al clon y logra lo mismo.
///
/// Devuelve `Ok(true)` si escribió la entrada, `Ok(false)` si ya estaba o si
/// no hay repositorio Git. Fuera de un repo no falla ni bloquea (§Decision 5):
/// sin gitdir no hay nada que excluir.
fn ensure_local_data_excluded(project_root: &Path, dry_run: bool) -> Result<bool, String> {
    let Some(git_dir) = git_common_dir(project_root) else {
        return Ok(false);
    };
    let exclude_path = git_dir.join("info").join("exclude");

    let existing = std::fs::read_to_string(&exclude_path).unwrap_or_default();
    if existing
        .lines()
        .any(|line| line.trim() == LOCAL_DATA_EXCLUDE_PATTERN)
    {
        return Ok(false);
    }
    if dry_run {
        return Ok(true);
    }

    if let Some(parent) = exclude_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("no se pudo crear {}: {e}", parent.display()))?;
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&format!(
        "\n# Rationale — datos local-only, nunca versionados (ADR-0014)\n{LOCAL_DATA_EXCLUDE_PATTERN}\n"
    ));
    crate::storage::atomic_write_bytes(&exclude_path, updated.as_bytes())
        .map_err(|e| format!("no se pudo escribir {}: {e}", exclude_path.display()))?;
    Ok(true)
}

/// Advierte si `.rationale-local/` ya está seguido por Git.
///
/// Solo advierte: modificar el índice de un proyecto ajeno no es decisión del
/// instalador (ADR-0014 §Decision 6). `info/exclude` no despista a Git sobre
/// un archivo que ya está en el índice, así que sin esta advertencia el
/// usuario creería que quedó protegido cuando no lo está.
fn tracked_local_data_warning(project_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["ls-files", "--", ".rationale-local"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let listing = String::from_utf8(output.stdout).ok()?;
    let count = listing.lines().filter(|l| !l.trim().is_empty()).count();
    if count == 0 {
        return None;
    }
    Some(format!(
        "aviso: .rationale-local/ contiene {count} archivo(s) seguidos por Git — son datos \
         local-only (telemetría y manifest de instalación) que no deberían versionarse. \
         Rationale no modificará el índice automáticamente. Para dejar de versionarlos:\n    \
         git rm -r --cached .rationale-local"
    ))
}

/// Ruta a guardar en el manifest: relativa a la raíz del proyecto siempre que
/// sea posible (ADR-0014 §Decision 7). Guardar absolutas dejaba el `$HOME` de
/// quien instaló dentro de un archivo que ya se filtró a dos remotos.
/// `resolve_managed_entry_path` acepta ambas formas, así que los manifests
/// escritos por versiones anteriores siguen siendo legibles.
fn manifest_relative_path(project_root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(project_root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Normaliza a relativas las entradas heredadas cuyo destino es reconocible
/// (ADR-0014 §Decision 9).
///
/// Un manifest escrito antes de ADR-0014 guarda rutas absolutas. Mientras el
/// proyecto no se mueva son inofensivas, pero si se movió o copió apuntan
/// fuera del `project_root` y `resolve_managed_entry_path` las rechaza —
/// correctamente— abortando `uninstall-agent` entero, entradas legítimas
/// incluidas.
///
/// La normalización **no relaja la guarda**: solo reescribe una entrada cuando
/// su ruta termina en un destino administrado conocido (`CLAUDE.md`,
/// `AGENTS.md`, `.mcp.json`, la regla de Cursor, o un `SKILL.md` bajo el
/// directorio de skills). Una ruta arbitraria —`~/Documents/notas.md`— no
/// coincide con ninguno y se conserva intacta para que la guarda la siga
/// rechazando. El destino resultante siempre queda dentro del `project_root`,
/// porque es relativo por construcción.
///
/// Radio de acción declarado: si el manifest heredado apuntaba al `CLAUDE.md`
/// de *otro* proyecto, tras normalizar apunta al de éste. No es una escalada —
/// `uninstall` solo extirpa el bloque delimitado de Rationale, y si este
/// proyecto está instalado ese archivo ya tenía su propia entrada.
fn migrate_legacy_absolute_entries(manifest: &mut Manifest, project_root: &Path) -> usize {
    let mut migrated = 0;
    for entry in &mut manifest.entries {
        // Ya es exactamente un destino administrado relativo: nada que migrar,
        // y saltarlo evita reescribirlo en cada pasada.
        if recognized_managed_suffix(&entry.path).as_deref() == Some(entry.path.as_path()) {
            continue;
        }
        // Ni `is_absolute()` ni `starts_with` deciden esto. En Windows,
        // `/otro/proyecto/CLAUDE.md` **no** es absoluta —le falta la letra de
        // unidad— así que la versión anterior la trataba como ya relativa y la
        // saltaba, dejando una entrada externa que abortaba `uninstall-agent`.
        // Lo que importa no es la forma de la ruta sino a qué destino apunta.
        let normalized = entry
            .path
            .strip_prefix(project_root)
            .ok()
            .map(Path::to_path_buf)
            .or_else(|| recognized_managed_suffix(&entry.path));

        if let Some(new_path) = normalized {
            if new_path != entry.path {
                entry.path = new_path;
                migrated += 1;
            }
        }
    }
    migrated
}

/// Componentes de una ruta tratando `/` y `\` como separadores **en cualquier
/// plataforma**.
///
/// `Path::components` no basta: en Unix `\` no es separador, así que una ruta
/// escrita en Windows se leería como un único componente, y en Windows una
/// ruta escrita en Unix no se reconocería como absoluta. Canonizar así los
/// **dos** operandos —no solo uno— es lo que hace la comparación simétrica.
fn portable_components(path: &Path) -> Vec<String> {
    path.to_string_lossy()
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_string)
        .collect()
}

/// Todos los destinos que `install-agent` administra, como rutas relativas.
/// Fuente única: `TARGETS` + `prompts::ACTIONS`, para que un agente o una
/// acción nuevos queden cubiertos sin tocar esta lista.
fn managed_destinations() -> Vec<String> {
    let mut destinations = Vec::new();
    for target in TARGETS {
        destinations.push(target.instructions_file.to_string());
        if let Some(config) = target.mcp_config_file {
            destinations.push(config.to_string());
        }
        if let Some(skills_dir) = target.skills_dir {
            for action in crate::prompts::ACTIONS {
                destinations.push(format!("{skills_dir}/rationale-{}/SKILL.md", action.name));
            }
        }
    }
    destinations
}

/// Destino administrado en el que termina esta ruta, si alguno.
///
/// Compara por componentes, no por subcadena: `…/mi-CLAUDE.md` no debe
/// reconocerse como `CLAUDE.md`. La guarda contra rutas arbitrarias depende de
/// que esto sea estricto.
fn recognized_managed_suffix(path: &Path) -> Option<PathBuf> {
    let actual = portable_components(path);
    for candidate in managed_destinations() {
        let wanted = portable_components(Path::new(&candidate));
        if wanted.is_empty() || actual.len() < wanted.len() {
            continue;
        }
        if actual[actual.len() - wanted.len()..] == wanted[..] {
            // Reconstruido desde componentes: separadores nativos.
            return Some(wanted.iter().collect());
        }
    }
    None
}

/// Posición de la entrada de este archivo, comparando en forma absoluta.
///
/// Un manifest escrito antes de ADR-0014 guarda rutas absolutas y uno nuevo las
/// guarda relativas: comparar los campos crudos duplicaría cada entrada al
/// reinstalar sobre una instalación vieja.
fn existing_entry_index(
    manifest: &Manifest,
    project_root: &Path,
    absolute: &Path,
) -> Option<usize> {
    // Ambos operandos se canonizan a componentes portables, y se acepta tanto
    // la forma relativa como la absoluta: un manifest heredado puede tener
    // cualquiera de las dos, y `is_absolute()` no es fiable entre plataformas.
    let relative = portable_components(&manifest_relative_path(project_root, absolute));
    let full = portable_components(absolute);
    manifest.entries.iter().position(|entry| {
        let existing = portable_components(&entry.path);
        existing == relative || existing == full
    })
}

/// Escribe la entrada de un archivo administrado, **en su sitio** si ya
/// existía.
///
/// Actualizar en sitio y no `remove` + `push` es lo que hace convergente al
/// manifest. Con el patrón anterior, una segunda ejecución re-registraba solo
/// algunas entradas —los skills, que se re-escriben siempre; no las
/// instrucciones ni el MCP, que solo se registran si cambiaron— y esas pocas
/// saltaban al final, desplazando a las demás. El archivo cambiaba byte a byte
/// aunque ningún dato lo hiciera.
///
/// `action` es un hecho histórico y se conserva de la entrada previa; `hash`,
/// `reversal` y la ruta normalizada sí se actualizan (ver `FileAction`).
fn upsert_entry(
    manifest: &mut Manifest,
    project_root: &Path,
    agent: &str,
    path: &Path,
    observed_action: FileAction,
    reversal: ReversalStrategy,
    content_hash: Option<String>,
) {
    let index = existing_entry_index(manifest, project_root, path);
    let action = match index {
        Some(i) => manifest.entries[i].action.clone(),
        None => observed_action,
    };
    let entry = InstalledEntry {
        agent: agent.to_string(),
        path: manifest_relative_path(project_root, path),
        action,
        reversal,
        content_hash,
    };
    match index {
        Some(i) => manifest.entries[i] = entry,
        None => manifest.entries.push(entry),
    }
}

fn record_entry(
    manifest: &mut Manifest,
    project_root: &Path,
    agent: &str,
    path: &Path,
    action: FileAction,
) {
    upsert_entry(
        manifest,
        project_root,
        agent,
        path,
        action,
        ReversalStrategy::ManagedPart,
        None,
    );
}

fn record_owned_entry(
    manifest: &mut Manifest,
    project_root: &Path,
    agent: &str,
    path: &Path,
    action: FileAction,
    hash: &str,
) {
    upsert_entry(
        manifest,
        project_root,
        agent,
        path,
        action,
        ReversalStrategy::OwnedFile,
        Some(hash.to_string()),
    );
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
        let (action1, changed1) = upsert_instructions_block(&claude_md, None, false).unwrap();
        assert_eq!(action1, FileAction::Created);
        assert!(changed1);

        let (action2, changed2) = upsert_instructions_block(&claude_md, None, false).unwrap();
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

        upsert_instructions_block(&claude_md, None, false).unwrap();
        let content = std::fs::read_to_string(&claude_md).unwrap();
        assert!(content.contains("Mis instrucciones"));
        assert!(content.contains(MARKER_BEGIN));

        remove_instructions_block(&claude_md, None).unwrap();
        let after = std::fs::read_to_string(&claude_md).unwrap();
        assert!(after.contains("Mis instrucciones"));
        assert!(!after.contains(MARKER_BEGIN));

        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn instructions_block_embeds_the_canonical_master_prompt() {
        let block = instructions_block();
        assert!(block.contains(MASTER_PROMPT.trim()));
        assert!(block.contains("prepare_change(target, intent)"));
        assert!(block.contains("finalize_change(...)"));
    }

    #[test]
    fn remove_instructions_block_deletes_file_it_created_entirely() {
        let project = temp_dir("delete-created");
        let claude_md = project.join("CLAUDE.md");
        upsert_instructions_block(&claude_md, None, false).unwrap();
        remove_instructions_block(&claude_md, None).unwrap();
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

        let result = upsert_instructions_block(&claude_md, None, false);
        assert!(
            result.is_err(),
            "debe rechazar marcadores invertidos, no panicar ni escribir"
        );
        assert_eq!(
            std::fs::read_to_string(&claude_md).unwrap(),
            corrupted,
            "el archivo no debe tocarse cuando los marcadores están invertidos"
        );

        let remove_result = remove_instructions_block(&claude_md, None);
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
        let (action, _) = upsert_instructions_block(&claude_md, None, false).unwrap();
        assert_eq!(action, FileAction::Created);
        let existing = std::fs::read_to_string(&claude_md).unwrap();
        std::fs::write(
            &claude_md,
            format!("{existing}\n## Notas propias\n\nEsto lo escribí yo después.\n"),
        )
        .unwrap();

        let mcp_json = project.join(".mcp.json");
        let (json_action, _) = upsert_mcp_json(&mcp_json, false).unwrap();
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
            &project,
            "claude-code",
            &claude_md,
            FileAction::Created,
        );
        record_entry(
            &mut manifest,
            &project,
            "claude-code",
            &mcp_json,
            FileAction::Created,
        );
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

        let (action, changed) = upsert_mcp_json(&config, false).unwrap();
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

    /// Defecto real: `upsert_mcp_json` cortaba en cuanto la clave "rationale"
    /// existía, sin comparar su valor — así que un `.mcp.json` escrito por una
    /// versión anterior se quedaba con la ruta absoluta del binario de quien
    /// instaló, y en cualquier otra máquina el servidor MCP fallaba en
    /// silencio. Comparar el valor es lo que convierte a `install-agent` en la
    /// vía de migración de `alpha.7` a ADR-0015.
    #[test]
    fn mcp_json_migrates_a_legacy_absolute_command_to_the_logical_one() {
        let project = temp_dir("mcp-converge");
        let config = project.join(".mcp.json");
        std::fs::write(
            &config,
            r#"{"mcpServers":{"rationale":{"command":"/Users/quien-instalo/.local/bin/rationale","args":["serve"]}}}"#,
        )
        .unwrap();

        let (action, changed) = upsert_mcp_json(&config, false).unwrap();
        assert_eq!(action, FileAction::Modified);
        assert!(changed, "una entrada heredada debe contar como cambio");

        let content = std::fs::read_to_string(&config).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(value["mcpServers"]["rationale"]["command"], MCP_COMMAND);

        // Idempotente una vez migrada.
        let (action2, changed2) = upsert_mcp_json(&config, false).unwrap();
        assert_eq!(action2, FileAction::Modified);
        assert!(!changed2);

        std::fs::remove_dir_all(project).ok();
    }

    /// ADR-0015 §Validation 3: ningún `mcp_config_file` de `TARGETS` puede
    /// acabar con una ruta absoluta. Son archivos compartidos y versionados.
    #[test]
    fn no_target_writes_an_absolute_command_into_shared_mcp_config() {
        for target in TARGETS {
            let Some(config_rel) = target.mcp_config_file else {
                continue;
            };
            let project = temp_dir(&format!("mcp-portable-{}", target.name));
            let config = project.join(config_rel);
            upsert_mcp_json(&config, false).unwrap();

            let value: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
            let command = value["mcpServers"]["rationale"]["command"]
                .as_str()
                .unwrap()
                .to_string();
            assert!(
                !command.starts_with('/') && !command.contains(":\\"),
                "{config_rel} es configuración compartida y no puede llevar una ruta \
                 absoluta; llevaba: {command}"
            );
            std::fs::remove_dir_all(project).ok();
        }
    }

    #[test]
    fn dry_run_touches_nothing() {
        let project = temp_dir("dry-run");
        let claude_md = project.join("CLAUDE.md");
        let (_, changed) = upsert_instructions_block(&claude_md, None, true).unwrap();
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

    /// Solo Cursor exige una cabecera en su archivo de instrucciones.
    /// `CLAUDE.md` y `AGENTS.md` son markdown que el cliente lee entero:
    /// darles frontmatter sería ruido visible para el usuario.
    #[test]
    fn only_cursor_declares_a_file_preamble() {
        for target in TARGETS {
            match target.name {
                "cursor" => assert_eq!(
                    target.file_preamble,
                    Some(CURSOR_RULE_PREAMBLE),
                    "la regla de Cursor necesita su frontmatter o Cursor la ignora"
                ),
                other => assert!(
                    target.file_preamble.is_none(),
                    "{other} no necesita cabecera y no debe recibir una"
                ),
            }
        }
    }

    /// El defecto: `install-agent` escribía en `.cursor/rules/rationale.mdc`
    /// el mismo bloque markdown que en `CLAUDE.md`, sin el frontmatter YAML
    /// que Cursor exige para aplicar una regla — el bloque acababa en un
    /// archivo que el agente ignoraba.
    #[test]
    fn cursor_rule_is_created_with_valid_frontmatter_and_converges() {
        let project = temp_dir("cursor-mdc");
        let cursor_target = TARGETS
            .iter()
            .find(|t| t.name == "cursor")
            .expect("el target cursor debe existir");
        let rule = project.join(cursor_target.instructions_file);

        let (action, changed) =
            upsert_instructions_block(&rule, cursor_target.file_preamble, false).unwrap();
        assert_eq!(action, FileAction::Created);
        assert!(changed);

        let written = std::fs::read_to_string(&rule).unwrap();
        assert!(
            written.starts_with("---\n"),
            "una regla .mdc debe empezar por su frontmatter:\n{written}"
        );
        for key in ["description:", "globs:", "alwaysApply: true"] {
            assert!(
                written.contains(key),
                "falta {key} en el frontmatter:\n{written}"
            );
        }
        // El frontmatter va ANTES del bloque: si el orden se invirtiera,
        // Cursor no vería el frontmatter en la primera línea.
        assert!(
            written.find("---\n").unwrap() < written.find(MARKER_BEGIN).unwrap(),
            "el frontmatter debe preceder al bloque administrado"
        );
        assert!(written.contains(MARKER_END));

        // Segunda pasada: converge byte a byte y no duplica el frontmatter.
        let (action2, changed2) =
            upsert_instructions_block(&rule, cursor_target.file_preamble, false).unwrap();
        assert_eq!(action2, FileAction::Modified);
        assert!(!changed2, "la segunda pasada no debe cambiar nada");
        let second = std::fs::read_to_string(&rule).unwrap();
        assert_eq!(written, second, "el archivo debe ser byte-idéntico");
        assert_eq!(
            second.matches("alwaysApply").count(),
            1,
            "el frontmatter no debe duplicarse:\n{second}"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// `git diff --check` es una puerta de CI en este repo y el propio
    /// escritor de bloques la rompía: `instructions_block()` ya termina en
    /// `\n`, así que al reemplazar un bloque que ocupaba el archivo entero se
    /// le sumaba el salto final y quedaba una línea en blanco al final.
    #[test]
    fn replacing_a_block_leaves_exactly_one_trailing_newline() {
        let project = temp_dir("trailing-newline");
        let claude_md = project.join("CLAUDE.md");

        upsert_instructions_block(&claude_md, None, false).unwrap();
        // Simular un bloque de una versión anterior para forzar el reemplazo.
        let stale = std::fs::read_to_string(&claude_md)
            .unwrap()
            .replace("protocolo de invocación", "protocolo viejo");
        std::fs::write(&claude_md, stale).unwrap();

        upsert_instructions_block(&claude_md, None, false).unwrap();

        let written = std::fs::read_to_string(&claude_md).unwrap();
        assert!(
            written.ends_with("-->\n"),
            "debe terminar justo tras el marcador final:\n{:?}",
            &written[written.len().saturating_sub(30)..]
        );
        assert!(
            !written.ends_with("\n\n"),
            "ninguna línea en blanco al final: git diff --check la rechaza"
        );

        // Y el contenido del usuario que siga al bloque se conserva.
        std::fs::write(
            &claude_md,
            format!("{}\n## Sección del usuario\n", written.trim_end()),
        )
        .unwrap();
        let stale2 = std::fs::read_to_string(&claude_md)
            .unwrap()
            .replace("protocolo de invocación", "otra vez viejo");
        std::fs::write(&claude_md, stale2).unwrap();
        upsert_instructions_block(&claude_md, None, false).unwrap();
        let with_user = std::fs::read_to_string(&claude_md).unwrap();
        assert!(
            with_user.contains("## Sección del usuario"),
            "el contenido posterior al bloque no se descarta:\n{with_user}"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// El defecto que hacía indistribuible cualquier corrección de skill: el
    /// manifest guarda rutas relativas desde la migración de rutas portables,
    /// pero la búsqueda del hash previo comparaba contra la ruta absoluta con
    /// `==`. No casaba nunca, así que `previous_hash` salía siempre `None` y
    /// todo skill se conservaba como "editado por el usuario" — incluso con el
    /// hash correcto registrado en el manifest.
    #[test]
    fn a_skill_whose_recorded_hash_matches_is_updated_not_preserved() {
        let repo = git_repo("skill-hash-lookup");
        let local = repo.join(".rationale-local");

        // Primera instalación: escribe los skills y registra sus hashes.
        install(&repo, &local, &fake_binary(), false, false).unwrap();
        let skill = repo.join(".claude/skills/rationale-health/SKILL.md");

        // Simular una generación anterior: contenido distinto del que el
        // binario actual escribiría, pero con su hash registrado en el
        // manifest — exactamente lo que deja una versión previa.
        let older = "---\ndescription: \"vieja\"\n---\n\nCuerpo de una versión anterior.\n";
        std::fs::write(&skill, older).unwrap();
        let mut manifest = load_manifest(&local);
        let index = existing_entry_index(&manifest, &repo, &skill)
            .expect("la primera instalación debe haber registrado el skill");
        manifest.entries[index].content_hash = Some(content_hash(older.as_bytes()));
        save_manifest(&local, &manifest).unwrap();

        let report = install(&repo, &local, &fake_binary(), false, false).unwrap();

        let updated = std::fs::read_to_string(&skill).unwrap();
        assert_eq!(
            updated,
            skill_content(crate::prompts::action("health").unwrap()),
            "un skill cuyo hash registrado coincide debe actualizarse:\n{:?}",
            report.actions
        );
        assert!(
            !report
                .actions
                .iter()
                .any(|a| a.contains("rationale-health") && a.contains("conservado")),
            "no debe reportarse como conservado: {:?}",
            report.actions
        );

        std::fs::remove_dir_all(repo).ok();
    }

    /// Sin entrada en el manifest, Rationale no sabe quién escribió el archivo.
    /// Se conserva por prudencia, pero el mensaje no puede afirmar que el
    /// usuario lo editó — y `--refresh-skills` debe resolver la duda.
    #[test]
    fn unknown_provenance_is_reported_honestly_and_refreshable() {
        let repo = git_repo("skill-unknown-provenance");
        let local = repo.join(".rationale-local");

        install(&repo, &local, &fake_binary(), false, false).unwrap();
        let skill = repo.join(".claude/skills/rationale-health/SKILL.md");
        std::fs::write(&skill, "contenido de otra generación\n").unwrap();

        // El manifest local no se versiona (ADR-0014): quien clone el proyecto
        // tiene los skills pero no el manifest. Se simula borrándolo.
        std::fs::remove_file(manifest_path(&local)).unwrap();

        let report = install(&repo, &local, &fake_binary(), false, false).unwrap();
        let message = report
            .actions
            .iter()
            .find(|a| a.contains("rationale-health"))
            .expect("debe reportarse algo sobre el skill");
        assert!(
            message.contains("procedencia desconocida"),
            "el mensaje debe decir la verdad, no inventar una edición: {message}"
        );
        assert!(
            !message.contains("cambios del usuario"),
            "no puede afirmar una edición que no comprobó: {message}"
        );
        assert!(
            message.contains("--refresh-skills"),
            "debe nombrar la salida: {message}"
        );

        // Con el flag explícito, se regenera.
        std::fs::remove_file(manifest_path(&local)).ok();
        install(&repo, &local, &fake_binary(), true, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(&skill).unwrap(),
            skill_content(crate::prompts::action("health").unwrap()),
            "--refresh-skills debe regenerar el skill de procedencia desconocida"
        );

        std::fs::remove_dir_all(repo).ok();
    }

    /// Pero `--refresh-skills` NO debe pisar una edición probada: si el
    /// manifest registra un hash y el archivo no coincide, alguien lo cambió.
    #[test]
    fn refresh_skills_never_overwrites_a_proven_user_edit() {
        let repo = git_repo("skill-refresh-respects-edits");
        let local = repo.join(".rationale-local");

        install(&repo, &local, &fake_binary(), false, false).unwrap();
        let skill = repo.join(".claude/skills/rationale-health/SKILL.md");
        let mine = "---\ndescription: \"mío\"\n---\n\nEsto lo escribí yo.\n";
        std::fs::write(&skill, mine).unwrap();

        let report = install(&repo, &local, &fake_binary(), true, false).unwrap();

        assert_eq!(
            std::fs::read_to_string(&skill).unwrap(),
            mine,
            "una edición probada sobrevive incluso a --refresh-skills"
        );
        assert!(
            report
                .actions
                .iter()
                .any(|a| a.contains("rationale-health") && a.contains("cambios del usuario")),
            "y se reporta como edición del usuario: {:?}",
            report.actions
        );

        std::fs::remove_dir_all(repo).ok();
    }

    /// La cabecera que Rationale escribe también es contenido de Rationale:
    /// si es todo lo que queda tras extirpar el bloque, el archivo se borra.
    /// Sin esto, `uninstall-agent` dejaba un `.mdc` huérfano con el
    /// frontmatter de Rationale dentro — un defecto que introdujo el propio
    /// preámbulo y que solo apareció al probar el ciclo completo.
    #[test]
    fn uninstall_removes_a_rule_file_that_holds_only_our_preamble() {
        let project = temp_dir("cursor-mdc-uninstall");
        let rule = project.join(".cursor/rules/rationale.mdc");

        upsert_instructions_block(&rule, Some(CURSOR_RULE_PREAMBLE), false).unwrap();
        assert!(rule.exists());

        remove_instructions_block(&rule, Some(CURSOR_RULE_PREAMBLE)).unwrap();
        assert!(
            !rule.exists(),
            "no debe quedar un archivo con solo el frontmatter de Rationale"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// Pero si el usuario añadió contenido propio, el archivo se conserva
    /// **con** su cabecera: en un `.mdc`, quitarla invalidaría la regla que
    /// el usuario quiere mantener.
    #[test]
    fn uninstall_preserves_a_rule_file_that_has_user_content() {
        let project = temp_dir("cursor-mdc-uninstall-user");
        let rule = project.join(".cursor/rules/rationale.mdc");

        upsert_instructions_block(&rule, Some(CURSOR_RULE_PREAMBLE), false).unwrap();
        let with_user = format!(
            "{}\n\n## Regla propia del usuario\n",
            std::fs::read_to_string(&rule).unwrap().trim_end()
        );
        std::fs::write(&rule, with_user).unwrap();

        remove_instructions_block(&rule, Some(CURSOR_RULE_PREAMBLE)).unwrap();

        let left = std::fs::read_to_string(&rule)
            .expect("el archivo debe sobrevivir porque tiene contenido del usuario");
        assert!(left.contains("## Regla propia del usuario"));
        assert!(
            left.starts_with("---\n") && left.contains("alwaysApply: true"),
            "la regla debe seguir siendo válida para Cursor:\n{left}"
        );
        assert!(
            !left.contains(MARKER_BEGIN) && !left.contains(MARKER_END),
            "el bloque administrado sí debe desaparecer:\n{left}"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// Un `.mdc` que ya existe tiene su propio frontmatter. Insertar otro a
    /// mitad del archivo lo corrompería, así que el preámbulo solo participa
    /// al crear.
    #[test]
    fn an_existing_rule_file_never_receives_a_second_preamble() {
        let project = temp_dir("cursor-mdc-existing");
        let rule = project.join(".cursor/rules/rationale.mdc");
        std::fs::create_dir_all(rule.parent().unwrap()).unwrap();
        let user_content =
            "---\ndescription: regla del usuario\nalwaysApply: false\n---\n\nTexto propio.\n";
        std::fs::write(&rule, user_content).unwrap();

        let (action, changed) =
            upsert_instructions_block(&rule, Some(CURSOR_RULE_PREAMBLE), false).unwrap();
        assert_eq!(action, FileAction::Modified);
        assert!(changed);

        let written = std::fs::read_to_string(&rule).unwrap();
        assert!(
            written.starts_with("---\ndescription: regla del usuario"),
            "el frontmatter del usuario debe conservarse intacto y primero:\n{written}"
        );
        assert!(
            written.contains("Texto propio."),
            "el contenido del usuario se conserva"
        );
        assert_eq!(
            written.matches("alwaysApply").count(),
            1,
            "no se añade un segundo frontmatter:\n{written}"
        );
        assert!(written.contains(MARKER_BEGIN) && written.contains(MARKER_END));

        std::fs::remove_dir_all(project).ok();
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
        assert!(
            !content.contains("allowed-tools:"),
            "preflight no ejecuta ningún comando Bash — no debe declarar un permiso sin uso"
        );

        let review = skill_content(crate::prompts::action("review").unwrap());
        assert!(review.contains("disable-model-invocation: true"));
    }

    /// Sin esto, Claude Code pedía aprobación interactiva la primera vez que
    /// `/rationale-health` corría en una instalación limpia — el skill
    /// promete ejecutar `rationale doctor --check` pero no declaraba el
    /// permiso Bash exacto que esa inyección necesita.
    #[test]
    fn health_skill_declares_the_exact_bash_permission_it_needs() {
        let health = skill_content(crate::prompts::action("health").unwrap());
        assert!(
            health.contains("allowed-tools: Bash(rationale doctor --check:*)"),
            "el skill de health debe declarar el permiso exacto de su propia inyección:\n{health}"
        );

        for other in ["preflight", "explain", "capture", "review", "protocol"] {
            let content = skill_content(crate::prompts::action(other).unwrap());
            assert!(
                !content.contains("allowed-tools:"),
                "{other} no inyecta un comando Bash propio — no debe declarar allowed-tools"
            );
        }
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
            &project,
            "claude-code",
            &intact,
            FileAction::Created,
            &content_hash(intact_content.as_bytes()),
        );
        record_owned_entry(
            &mut manifest,
            &project,
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
            &project,
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
            &project,
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

    // --- ADR-0014: exclusión de datos local-only en proyectos consumidores ---
    //
    // La validación de ADR-0012 fue inspección manual dentro del repo de
    // Rationale, y por eso no vio que `.rationale-local/` quedaba versionado
    // en los consumidores. Estos tests son la sustitución: repos Git reales,
    // temporales, comprobando lo que Git de verdad reporta.

    fn git(repo: &Path, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .expect("git debe estar disponible para estos tests")
    }

    /// Siembra la condición de detección de los tres agentes en el proyecto.
    ///
    /// `install` detecta un agente si su binario está en el `PATH` **o** si el
    /// proyecto ya tiene su configuración. Sin sembrar, el resultado dependía
    /// de si la máquina tenía `claude`, `codex` o `cursor-agent` instalados:
    /// en la del desarrollador sí, en los runners de CI no, y `install`
    /// retornaba temprano sin escribir nada. Tres tests fallaban en las tres
    /// plataformas de CI y otro pasaba por la razón equivocada.
    ///
    /// Se siembra lo mínimo que dispara la detección sin predeterminar lo que
    /// los tests comprueban: el **directorio** de skills para claude-code —no
    /// `CLAUDE.md` ni `.mcp.json`, cuyo `action` varios tests afirman—, y la
    /// configuración de los otros dos.
    fn seed_agent_detection(project: &Path) {
        std::fs::create_dir_all(project.join(".claude/skills")).unwrap();
        std::fs::write(project.join("AGENTS.md"), "# Del proyecto\n").unwrap();
        std::fs::create_dir_all(project.join(".cursor")).unwrap();
        std::fs::write(project.join(".cursor/mcp.json"), "{}\n").unwrap();
    }

    fn git_repo(label: &str) -> PathBuf {
        let repo = temp_dir(label);
        git(&repo, &["init", "-q"]);
        git(&repo, &["config", "user.email", "test@example.com"]);
        git(&repo, &["config", "user.name", "Test"]);
        seed_agent_detection(&repo);
        repo
    }

    /// Ningún test puede depender de qué agentes tenga instalados la máquina.
    #[test]
    fn seeded_fixture_detects_every_agent_without_any_binary_on_path() {
        let repo = git_repo("seed-detection");
        let local = repo.join(".rationale-local");
        let report = install(&repo, &local, &fake_binary(), false, false).unwrap();

        for target in TARGETS {
            assert!(
                report.detected.iter().any(|d| d == target.name),
                "{} debe detectarse por configuración del proyecto, no por PATH: {:?}",
                target.name,
                report.detected
            );
        }
        assert!(
            manifest_path(&local).exists(),
            "si hay agentes detectados, install debe llegar a escribir el manifest"
        );
        std::fs::remove_dir_all(repo).ok();
    }

    /// Restaura `PATH` al salir del scope, incluso si el test panica —
    /// mutar una variable de entorno es intrínsecamente global al proceso,
    /// así que sin esto un panic dejaría el resto de la suite corriendo con
    /// un `PATH` equivocado.
    struct PathOverride(Option<std::ffi::OsString>);

    impl PathOverride {
        /// Sin ningún directorio que pueda contener `claude`, `codex` o
        /// `cursor-agent` — pero con `/usr/bin` y `/bin`, para que `git`
        /// (que `install` invoca internamente) siga resolviendo.
        fn without_any_agent_binary() -> Self {
            let original = std::env::var_os("PATH");
            std::env::set_var("PATH", "/usr/bin:/bin");
            PathOverride(original)
        }
    }

    impl Drop for PathOverride {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => std::env::set_var("PATH", value),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    /// El defecto que sembrar la detección expuso: `codex` detectado por un
    /// `AGENTS.md` heredado, sin el binario en `PATH`, abortaba la
    /// instalación completa — incluidos los demás agentes — en vez de
    /// omitirse con un aviso.
    #[test]
    fn codex_detected_without_a_binary_is_skipped_without_aborting_other_agents() {
        let repo = git_repo("codex-no-binary");
        let local = repo.join(".rationale-local");

        let report = {
            let _path = PathOverride::without_any_agent_binary();
            install(&repo, &local, &fake_binary(), false, false).expect(
                "la ausencia del binario de codex no puede abortar la instalación de los demás agentes",
            )
        };

        assert!(
            report.detected.contains(&"codex".to_string()),
            "codex se detecta por AGENTS.md aunque el binario no exista: {:?}",
            report.detected
        );
        assert!(
            report.detected.contains(&"claude-code".to_string())
                && report.detected.contains(&"cursor".to_string()),
            "los demás agentes deben seguir instalándose: {:?}",
            report.detected
        );
        assert!(
            report
                .actions
                .iter()
                .any(|a| a.contains("codex") && a.contains("no se encontró el binario en PATH")),
            "el reporte debe avisar que se omitió el registro global de codex: {:?}",
            report.actions
        );
        assert!(
            manifest_path(&local).exists(),
            "el manifest debe escribirse aunque codex se haya omitido"
        );
        std::fs::remove_dir_all(repo).ok();
    }

    /// El test que ADR-0012 necesitaba y no tuvo: qué dice Git, no qué creemos.
    #[test]
    fn install_leaves_no_local_data_visible_to_git() {
        let repo = git_repo("exclude-regression");
        let local = repo.join(".rationale-local");

        install(&repo, &local, &fake_binary(), false, false).unwrap();

        let status = git(&repo, &["status", "--porcelain", "--untracked-files=all"]);
        let listing = String::from_utf8(status.stdout).unwrap();
        let leaked: Vec<&str> = listing
            .lines()
            .filter(|line| line.contains(".rationale-local"))
            .collect();
        assert!(
            leaked.is_empty(),
            "ninguna ruta bajo .rationale-local/ puede aparecer ante Git; apareció: {leaked:?}"
        );

        std::fs::remove_dir_all(repo).ok();
    }

    #[test]
    fn exclude_entry_is_idempotent_and_preserves_existing_rules() {
        let repo = git_repo("exclude-idempotent");
        let exclude = repo.join(".git/info/exclude");
        std::fs::create_dir_all(exclude.parent().unwrap()).unwrap();
        std::fs::write(&exclude, "# regla previa del usuario\n*.tmp\n").unwrap();

        assert!(ensure_local_data_excluded(&repo, false).unwrap());
        assert!(
            !ensure_local_data_excluded(&repo, false).unwrap(),
            "la segunda pasada no debe volver a escribir la entrada"
        );

        let content = std::fs::read_to_string(&exclude).unwrap();
        assert_eq!(
            content
                .lines()
                .filter(|l| l.trim() == LOCAL_DATA_EXCLUDE_PATTERN)
                .count(),
            1,
            "la entrada no puede duplicarse"
        );
        assert!(
            content.contains("*.tmp"),
            "las reglas previas del usuario se conservan"
        );

        std::fs::remove_dir_all(repo).ok();
    }

    /// `.git` como *archivo* con puntero `gitdir:` — el caso que se rompería
    /// si se asumiera `<root>/.git/info/exclude` (ADR-0014 §Decision 4).
    #[test]
    fn exclude_resolves_the_real_gitdir_in_a_worktree() {
        let repo = git_repo("exclude-worktree");
        std::fs::write(repo.join("seed.txt"), "seed\n").unwrap();
        git(&repo, &["add", "seed.txt"]);
        git(&repo, &["commit", "-q", "-m", "seed"]);

        let worktree = repo.with_extension("wt");
        git(
            &repo,
            &["worktree", "add", "-q", worktree.to_str().unwrap(), "-d"],
        );
        assert!(
            worktree.join(".git").is_file(),
            "en un worktree, .git es un archivo con un puntero gitdir:"
        );

        assert!(ensure_local_data_excluded(&worktree, false).unwrap());

        // La exclusión debe aterrizar en el directorio común, no dentro del
        // worktree, para aplicar a todos los worktrees del repo.
        let common = repo.join(".git/info/exclude");
        let content = std::fs::read_to_string(&common)
            .expect("info/exclude debe escribirse en el git dir común");
        assert!(content.contains(LOCAL_DATA_EXCLUDE_PATTERN));

        git(
            &repo,
            &["worktree", "remove", "--force", worktree.to_str().unwrap()],
        );
        std::fs::remove_dir_all(&repo).ok();
        std::fs::remove_dir_all(&worktree).ok();
    }

    #[test]
    fn install_warns_when_local_data_is_already_tracked() {
        let repo = git_repo("exclude-warns-tracked");
        let local = repo.join(".rationale-local");
        std::fs::create_dir_all(local.join("runs")).unwrap();
        std::fs::write(local.join("runs/vertical-slice.ndjson"), "{}\n").unwrap();
        // `-f` porque el usuario pudo haberlo versionado antes de que
        // existiera cualquier exclusión — que es exactamente lo que pasó en
        // Monorepo y BoostAPI.
        git(&repo, &["add", "-f", ".rationale-local"]);

        let report = install(&repo, &local, &fake_binary(), false, false).unwrap();
        let warning = report
            .actions
            .iter()
            .find(|line| line.starts_with("aviso: .rationale-local/"))
            .expect("debe advertir cuando ya hay archivos seguidos");
        assert!(
            warning.contains("git rm -r --cached .rationale-local"),
            "la advertencia debe nombrar el comando exacto: {warning}"
        );

        // Y no debe tocar el índice: el archivo sigue seguido.
        let tracked =
            String::from_utf8(git(&repo, &["ls-files", "--", ".rationale-local"]).stdout).unwrap();
        assert!(
            tracked.contains("vertical-slice.ndjson"),
            "Rationale no puede modificar el índice del usuario"
        );

        std::fs::remove_dir_all(repo).ok();
    }

    #[test]
    fn install_outside_a_git_repository_does_not_fail() {
        let project = temp_dir("exclude-no-git");
        seed_agent_detection(&project);
        let local = project.join(".rationale-local");
        assert!(
            !ensure_local_data_excluded(&project, false).unwrap(),
            "sin gitdir no hay nada que excluir"
        );
        install(&project, &local, &fake_binary(), false, false)
            .expect("la ausencia de repo Git no puede bloquear la instalación");
        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn manifest_stores_project_relative_paths() {
        let repo = git_repo("manifest-relative");
        let local = repo.join(".rationale-local");
        install(&repo, &local, &fake_binary(), false, false).unwrap();

        let raw = std::fs::read_to_string(manifest_path(&local)).unwrap();
        assert!(
            !raw.contains(repo.to_str().unwrap()),
            "el manifest no puede contener la ruta absoluta del proyecto:\n{raw}"
        );
        let manifest: Manifest = serde_json::from_str(&raw).unwrap();
        assert!(
            manifest.entries.iter().all(|e| e.path.is_relative()),
            "toda entrada nueva debe ser relativa"
        );

        std::fs::remove_dir_all(repo).ok();
    }

    /// Compatibilidad hacia atrás: los pilotos ya instalados tienen manifests
    /// con rutas absolutas. Si `uninstall-agent` dejara de leerlos, el arreglo
    /// nuevo rompería la limpieza de las instalaciones viejas.
    #[test]
    fn uninstall_still_reads_a_legacy_absolute_path_manifest() {
        let project = temp_dir("legacy-manifest");
        let local = project.join(".rationale-local");
        let claude_md = project.join("CLAUDE.md");
        upsert_instructions_block(&claude_md, None, false).unwrap();

        // Manifest en el formato anterior a ADR-0014: ruta absoluta.
        let legacy = Manifest {
            entries: vec![InstalledEntry {
                agent: "claude-code".to_string(),
                path: claude_md.clone(),
                action: FileAction::Created,
                reversal: ReversalStrategy::ManagedPart,
                content_hash: None,
            }],
        };
        save_manifest(&local, &legacy).unwrap();

        uninstall(&project, &local).expect("un manifest heredado debe seguir siendo legible");
        assert!(
            !claude_md.exists(),
            "el bloque era todo el archivo, así que uninstall debe borrarlo"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// El escenario que abortaba `uninstall-agent`: proyecto de alpha.7
    /// movido o copiado, manifest apuntando a la ubicación anterior.
    #[test]
    fn install_migrates_a_moved_projects_legacy_manifest_and_uninstall_then_works() {
        let project = temp_dir("legacy-moved");
        seed_agent_detection(&project);
        let local = project.join(".rationale-local");
        let claude_md = project.join("CLAUDE.md");
        upsert_instructions_block(&claude_md, None, false).unwrap();

        // Manifest de alpha.7 escrito cuando el proyecto vivía en otra ruta.
        save_manifest(
            &local,
            &Manifest {
                entries: vec![InstalledEntry {
                    agent: "claude-code".to_string(),
                    path: PathBuf::from("/Users/otra-persona/Desktop/Proyecto/CLAUDE.md"),
                    action: FileAction::Created,
                    reversal: ReversalStrategy::ManagedPart,
                    content_hash: None,
                }],
            },
        )
        .unwrap();

        // Antes de migrar, la guarda rechaza y aborta todo.
        assert!(
            uninstall(&project, &local).is_err(),
            "sin migración, una entrada externa cancela la desinstalación entera"
        );

        install(&project, &local, &fake_binary(), false, false).unwrap();

        let raw = std::fs::read_to_string(manifest_path(&local)).unwrap();
        assert!(
            !raw.contains("otra-persona"),
            "install-agent debe normalizar la entrada heredada:\n{raw}"
        );
        uninstall(&project, &local).expect("tras migrar, la desinstalación debe completarse");

        std::fs::remove_dir_all(project).ok();
    }

    /// Prueba adversarial de la Decision #9. La migración reinterpreta
    /// evidencia histórica de *otro* proyecto y la traslada a éste; hay que
    /// demostrar que eso nunca se convierte en permiso para borrar contenido
    /// local. Un manifest trasladado afirma `created` sobre archivos que aquí
    /// son del usuario y que Rationale nunca tocó.
    #[test]
    fn a_relocated_manifest_never_deletes_preexisting_user_files() {
        let project = temp_dir("legacy-adversarial");
        let local = project.join(".rationale-local");

        // Archivos del usuario, sin nada de Rationale dentro.
        let claude_md = project.join("CLAUDE.md");
        let user_instructions = "# Mis instrucciones\n\nNunca las escribió Rationale.\n";
        std::fs::write(&claude_md, user_instructions).unwrap();

        let skill = project.join(".claude/skills/rationale-health/SKILL.md");
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        let user_skill = "---\ndescription: mi versión\n---\n\nEsto lo escribí yo.\n";
        std::fs::write(&skill, user_skill).unwrap();

        // Manifest de otro project-root que afirma haber creado ambos, con un
        // hash que corresponde al skill *generado*, no al del usuario.
        let generated = skill_content(crate::prompts::action("health").unwrap());
        save_manifest(
            &local,
            &Manifest {
                entries: vec![
                    InstalledEntry {
                        agent: "claude-code".to_string(),
                        path: PathBuf::from("/otro/proyecto/CLAUDE.md"),
                        action: FileAction::Created,
                        reversal: ReversalStrategy::ManagedPart,
                        content_hash: None,
                    },
                    InstalledEntry {
                        agent: "claude-code".to_string(),
                        path: PathBuf::from(
                            "/otro/proyecto/.claude/skills/rationale-health/SKILL.md",
                        ),
                        action: FileAction::Created,
                        reversal: ReversalStrategy::OwnedFile,
                        content_hash: Some(content_hash(generated.as_bytes())),
                    },
                ],
            },
        )
        .unwrap();

        let mut manifest = load_manifest(&local);
        assert_eq!(
            migrate_legacy_absolute_entries(&mut manifest, &project),
            2,
            "ambos destinos son administrados y se normalizan"
        );
        save_manifest(&local, &manifest).unwrap();

        uninstall(&project, &local).unwrap();

        assert_eq!(
            std::fs::read_to_string(&claude_md).unwrap(),
            user_instructions,
            "un CLAUDE.md sin bloque de Rationale no puede tocarse, aunque el \
             manifest trasladado afirme `created`"
        );
        assert_eq!(
            std::fs::read_to_string(&skill).unwrap(),
            user_skill,
            "un skill cuyo hash no coincide es del usuario: no puede borrarse"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// La cara complementaria: sobre contenido que Rationale sí administra, la
    /// desinstalación tras migrar debe quitar exactamente lo suyo y nada más.
    #[test]
    fn after_migrating_uninstall_removes_only_rationale_managed_content() {
        let project = temp_dir("legacy-adversarial-managed");
        let local = project.join(".rationale-local");

        let claude_md = project.join("CLAUDE.md");
        std::fs::write(&claude_md, "# Encabezado propio\n\n").unwrap();
        upsert_instructions_block(&claude_md, None, false).unwrap();
        let with_block = std::fs::read_to_string(&claude_md).unwrap();
        std::fs::write(&claude_md, format!("{with_block}\n## Cola propia\n")).unwrap();

        let skill = project.join(".claude/skills/rationale-health/SKILL.md");
        let generated = skill_content(crate::prompts::action("health").unwrap());
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        crate::storage::atomic_write_bytes(&skill, generated.as_bytes()).unwrap();

        save_manifest(
            &local,
            &Manifest {
                entries: vec![
                    InstalledEntry {
                        agent: "claude-code".to_string(),
                        path: PathBuf::from("/otro/proyecto/CLAUDE.md"),
                        action: FileAction::Modified,
                        reversal: ReversalStrategy::ManagedPart,
                        content_hash: None,
                    },
                    InstalledEntry {
                        agent: "claude-code".to_string(),
                        path: PathBuf::from(
                            "/otro/proyecto/.claude/skills/rationale-health/SKILL.md",
                        ),
                        action: FileAction::Created,
                        reversal: ReversalStrategy::OwnedFile,
                        content_hash: Some(content_hash(generated.as_bytes())),
                    },
                ],
            },
        )
        .unwrap();

        let mut manifest = load_manifest(&local);
        migrate_legacy_absolute_entries(&mut manifest, &project);
        save_manifest(&local, &manifest).unwrap();
        uninstall(&project, &local).unwrap();

        let after = std::fs::read_to_string(&claude_md).unwrap();
        assert!(after.contains("Encabezado propio") && after.contains("Cola propia"));
        assert!(
            !after.contains(MARKER_BEGIN),
            "el bloque administrado sí debe desaparecer"
        );
        assert!(
            !skill.exists(),
            "un skill intacto sí pertenece a Rationale y se borra"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// La migración normaliza destinos administrados, no cualquier ruta: una
    /// arbitraria debe seguir siendo rechazada por la guarda.
    #[test]
    fn migration_never_normalizes_an_arbitrary_path() {
        let project = temp_dir("legacy-arbitrary");
        let mut manifest = Manifest {
            entries: vec![InstalledEntry {
                agent: "claude-code".to_string(),
                path: PathBuf::from("/Users/alguien/Documents/notas.md"),
                action: FileAction::Created,
                reversal: ReversalStrategy::ManagedPart,
                content_hash: None,
            }],
        };

        assert_eq!(
            migrate_legacy_absolute_entries(&mut manifest, &project),
            0,
            "una ruta que no es un destino administrado no se normaliza"
        );
        assert_eq!(
            manifest.entries[0].path,
            PathBuf::from("/Users/alguien/Documents/notas.md"),
            "se conserva intacta para que la guarda la siga rechazando"
        );

        std::fs::remove_dir_all(project).ok();
    }

    /// El reconocimiento debe ser simétrico entre plataformas: una ruta con
    /// separadores Windows y otra con separadores Unix apuntan al mismo
    /// destino y deben reconocerse igual, se ejecute donde se ejecute.
    #[test]
    fn managed_suffix_is_recognized_with_either_separator() {
        let unix = Path::new("/otro/proyecto/.claude/skills/rationale-health/SKILL.md");
        let windows =
            Path::new(r"C:\Users\alguien\Proyecto\.claude\skills\rationale-health\SKILL.md");
        let mixed =
            Path::new(r"C:\Users\alguien\Proyecto/.claude\skills/rationale-health/SKILL.md");

        let expected: PathBuf = [".claude", "skills", "rationale-health", "SKILL.md"]
            .iter()
            .collect();
        for path in [unix, windows, mixed] {
            assert_eq!(
                recognized_managed_suffix(path),
                Some(expected.clone()),
                "no se reconoció el destino administrado en {}",
                path.display()
            );
        }

        // Y lo mismo para un archivo de instrucciones en la raíz.
        for path in [
            Path::new("/otro/proyecto/CLAUDE.md"),
            Path::new(r"C:\Users\alguien\Proyecto\CLAUDE.md"),
        ] {
            assert_eq!(
                recognized_managed_suffix(path),
                Some(PathBuf::from("CLAUDE.md")),
                "no se reconoció CLAUDE.md en {}",
                path.display()
            );
        }
    }

    /// La comparación es por componentes, no por subcadena: si fuera textual,
    /// `mi-CLAUDE.md` terminaría en «CLAUDE.md» y la guarda se caería.
    #[test]
    fn managed_suffix_never_matches_a_partial_component() {
        for path in [
            Path::new("/otro/proyecto/mi-CLAUDE.md"),
            Path::new(r"C:\Proyecto\notasCLAUDE.md"),
            Path::new("/otro/proyecto/CLAUDE.md.bak"),
            Path::new("/otro/proyecto/.claude/skills/rationale-health/SKILL.md.orig"),
        ] {
            assert_eq!(
                recognized_managed_suffix(path),
                None,
                "{} no es un destino administrado y no puede reconocerse",
                path.display()
            );
        }
    }

    /// Una ruta estilo Unix **no es** `is_absolute()` en Windows: le falta la
    /// letra de unidad. La versión anterior la saltaba como «ya relativa» y
    /// dejaba una entrada externa que abortaba `uninstall-agent`.
    #[test]
    fn migration_normalizes_a_unix_style_path_on_any_platform() {
        let project = temp_dir("migrate-unix-style");
        let mut manifest = Manifest {
            entries: vec![InstalledEntry {
                agent: "claude-code".to_string(),
                path: PathBuf::from("/otro/proyecto/CLAUDE.md"),
                action: FileAction::Created,
                reversal: ReversalStrategy::ManagedPart,
                content_hash: None,
            }],
        };

        assert_eq!(migrate_legacy_absolute_entries(&mut manifest, &project), 1);
        assert_eq!(manifest.entries[0].path, PathBuf::from("CLAUDE.md"));
        assert_eq!(
            manifest.entries[0].action,
            FileAction::Created,
            "migrar preserva el hecho histórico"
        );

        // Segunda pasada: ya está normalizada, no vuelve a contarse.
        assert_eq!(
            migrate_legacy_absolute_entries(&mut manifest, &project),
            0,
            "una entrada ya normalizada no se re-migra"
        );
        std::fs::remove_dir_all(project).ok();
    }

    #[test]
    fn migration_recognizes_every_managed_destination() {
        for target in TARGETS {
            assert_eq!(
                recognized_managed_suffix(&PathBuf::from(format!(
                    "/viejo/proyecto/{}",
                    target.instructions_file
                ))),
                Some(PathBuf::from(target.instructions_file)),
                "{} debe reconocerse",
                target.instructions_file
            );
            if let Some(skills_dir) = target.skills_dir {
                let rel = format!("{skills_dir}/rationale-health/SKILL.md");
                assert_eq!(
                    recognized_managed_suffix(&PathBuf::from(format!("/viejo/proyecto/{rel}"))),
                    Some(PathBuf::from(rel)),
                    "los skills administrados deben reconocerse"
                );
            }
        }
    }

    // --- Convergencia del manifest en la segunda ejecución ---
    //
    // `action` es un hecho histórico: cómo adquirió Rationale la
    // administración del archivo la primera vez. Antes se sobrescribía con el
    // estado observado en cada pasada, y la segunda ejecución volteaba todas
    // las entradas mientras el reporte decía «ya al día».

    fn manifest_bytes(local: &Path) -> String {
        std::fs::read_to_string(manifest_path(local)).unwrap()
    }

    /// (1) Instalación nueva: la segunda pasada no toca el manifest.
    #[test]
    fn fresh_install_manifest_is_byte_identical_on_the_second_run() {
        let repo = git_repo("converge-fresh");
        let local = repo.join(".rationale-local");

        install(&repo, &local, &fake_binary(), false, false).unwrap();
        let first = manifest_bytes(&local);
        install(&repo, &local, &fake_binary(), false, false).unwrap();

        assert_eq!(
            first,
            manifest_bytes(&local),
            "la segunda ejecución no puede cambiar el manifest"
        );
        std::fs::remove_dir_all(repo).ok();
    }

    /// (2) Manifest de alpha.7: la primera pasada migra, la segunda no mueve
    /// nada. Es el escenario exacto observado en Monorepo.
    #[test]
    fn legacy_manifest_migrates_once_then_converges() {
        let repo = git_repo("converge-legacy");
        let local = repo.join(".rationale-local");
        let claude_md = repo.join("CLAUDE.md");
        std::fs::write(&claude_md, "# Del usuario\n").unwrap();

        save_manifest(
            &local,
            &Manifest {
                entries: vec![InstalledEntry {
                    agent: "claude-code".to_string(),
                    path: repo.join("CLAUDE.md"),
                    action: FileAction::Created,
                    reversal: ReversalStrategy::ManagedPart,
                    content_hash: None,
                }],
            },
        )
        .unwrap();

        install(&repo, &local, &fake_binary(), false, false).unwrap();
        let migrated = manifest_bytes(&local);
        assert!(
            !migrated.contains(repo.to_str().unwrap()),
            "la primera pasada migra las rutas absolutas"
        );

        install(&repo, &local, &fake_binary(), false, false).unwrap();
        assert_eq!(
            migrated,
            manifest_bytes(&local),
            "la segunda pasada debe ser byte-idéntica tras migrar"
        );
        std::fs::remove_dir_all(repo).ok();
    }

    /// (3) Una entrada histórica `created` permanece `created`, aunque el
    /// archivo ya exista en las pasadas siguientes.
    #[test]
    fn a_historically_created_entry_stays_created() {
        let repo = git_repo("converge-created");
        let local = repo.join(".rationale-local");

        // Primera instalación: CLAUDE.md no existe, se crea.
        install(&repo, &local, &fake_binary(), false, false).unwrap();
        let entry_action = |local: &Path, path: &str| {
            let m: Manifest = serde_json::from_str(&manifest_bytes(local)).unwrap();
            m.entries
                .iter()
                .find(|e| e.path == Path::new(path))
                .unwrap_or_else(|| panic!("falta la entrada {path}"))
                .action
                .clone()
        };
        assert_eq!(entry_action(&local, "CLAUDE.md"), FileAction::Created);
        assert_eq!(
            entry_action(&local, ".claude/skills/rationale-health/SKILL.md"),
            FileAction::Created
        );

        // Segunda y tercera: los archivos ya existen, pero el hecho histórico
        // no cambia.
        install(&repo, &local, &fake_binary(), false, false).unwrap();
        install(&repo, &local, &fake_binary(), false, false).unwrap();
        assert_eq!(
            entry_action(&local, "CLAUDE.md"),
            FileAction::Created,
            "reinstalar no reescribe cómo se adquirió el archivo"
        );
        assert_eq!(
            entry_action(&local, ".claude/skills/rationale-health/SKILL.md"),
            FileAction::Created
        );
        std::fs::remove_dir_all(repo).ok();
    }

    /// (4) Una entrada histórica `modified` permanece `modified`, aunque la
    /// pasada actual observe otra cosa.
    #[test]
    fn a_historically_modified_entry_stays_modified() {
        let repo = git_repo("converge-modified");
        let local = repo.join(".rationale-local");
        let claude_md = repo.join("CLAUDE.md");
        std::fs::write(&claude_md, "# Ya existía cuando Rationale llegó\n").unwrap();

        install(&repo, &local, &fake_binary(), false, false).unwrap();
        let action_of = |local: &Path| {
            let m: Manifest = serde_json::from_str(&manifest_bytes(local)).unwrap();
            m.entries
                .iter()
                .find(|e| e.path == Path::new("CLAUDE.md"))
                .unwrap()
                .action
                .clone()
        };
        assert_eq!(action_of(&local), FileAction::Modified);

        // Borrar el archivo y reinstalar: ahora Rationale lo *crea*, pero la
        // entrada histórica dice que originalmente lo adquirió modificando.
        std::fs::remove_file(&claude_md).unwrap();
        install(&repo, &local, &fake_binary(), false, false).unwrap();
        assert_eq!(
            action_of(&local),
            FileAction::Modified,
            "el hecho histórico no se sobrescribe con lo observado ahora"
        );
        std::fs::remove_dir_all(repo).ok();
    }

    /// (5) La desinstalación tras migrar sigue conservando lo del usuario.
    /// Complementa `a_relocated_manifest_never_deletes_preexisting_user_files`
    /// sobre el camino de `install` completo, no solo la migración aislada.
    #[test]
    fn uninstall_after_convergence_still_preserves_user_content() {
        let repo = git_repo("converge-uninstall");
        let local = repo.join(".rationale-local");
        let claude_md = repo.join("CLAUDE.md");
        std::fs::write(&claude_md, "# Encabezado propio\n").unwrap();

        save_manifest(
            &local,
            &Manifest {
                entries: vec![InstalledEntry {
                    agent: "claude-code".to_string(),
                    path: PathBuf::from("/otro/proyecto/CLAUDE.md"),
                    action: FileAction::Created,
                    reversal: ReversalStrategy::ManagedPart,
                    content_hash: None,
                }],
            },
        )
        .unwrap();

        install(&repo, &local, &fake_binary(), false, false).unwrap();
        install(&repo, &local, &fake_binary(), false, false).unwrap();
        uninstall(&repo, &local).unwrap();

        let after = std::fs::read_to_string(&claude_md)
            .expect("el archivo tenía contenido del usuario: no puede borrarse");
        assert!(after.contains("Encabezado propio"));
        assert!(!after.contains(MARKER_BEGIN), "el bloque sí se extirpa");
        std::fs::remove_dir_all(repo).ok();
    }

    /// (6) Las rutas arbitrarias siguen rechazadas: preservar `action` no
    /// relaja ninguna guarda.
    #[test]
    fn convergence_does_not_weaken_the_arbitrary_path_guard() {
        let repo = git_repo("converge-guard");
        let local = repo.join(".rationale-local");

        save_manifest(
            &local,
            &Manifest {
                entries: vec![InstalledEntry {
                    agent: "claude-code".to_string(),
                    path: PathBuf::from("/Users/alguien/Documents/notas.md"),
                    action: FileAction::Created,
                    reversal: ReversalStrategy::ManagedPart,
                    content_hash: None,
                }],
            },
        )
        .unwrap();

        let report = install(&repo, &local, &fake_binary(), false, false).unwrap();

        // Sin esto el test pasaba por la razón equivocada: si `install`
        // retornaba temprano por no detectar agentes, el manifest quedaba
        // intacto y las dos afirmaciones de abajo se cumplían sin haber
        // ejercitado nada. Primero hay que demostrar que la instalación
        // ocurrió de verdad.
        assert!(
            !report.detected.is_empty(),
            "el fixture debe detectar agentes; si no, nada de lo que sigue prueba algo"
        );
        let manifest: Manifest = serde_json::from_str(&manifest_bytes(&local)).unwrap();
        assert!(
            manifest
                .entries
                .iter()
                .any(|e| e.path == Path::new("CLAUDE.md")),
            "install debe haber registrado sus propios destinos administrados"
        );

        assert!(
            manifest
                .entries
                .iter()
                .any(|e| e.path == Path::new("/Users/alguien/Documents/notas.md")),
            "y aun así una ruta arbitraria no se normaliza: se conserva intacta"
        );
        assert!(
            uninstall(&repo, &local).is_err(),
            "la guarda debe seguir rechazándola"
        );
        std::fs::remove_dir_all(repo).ok();
    }

    #[test]
    fn shared_instructions_never_embed_an_install_path() {
        let block = instructions_block();
        assert!(
            !block.contains("/Users/") && !block.contains("/home/") && !block.contains(":\\"),
            "CLAUDE.md y AGENTS.md son documentación compartida: no pueden llevar la \
             instalación de una persona"
        );
        assert!(block.contains("servidor MCP `rationale`"));
    }
}
