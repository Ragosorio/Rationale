//! Configuration — localiza la raíz del proyecto y lee `.rationale/config.yaml`.
//!
//! Arquitectura_Conceptual_v0.1.md §11.2. Precedencia (documentada, no toda
//! implementada todavía en la vertical slice): project config > user config >
//! environment overrides > safe defaults. La vertical slice de Fase D solo
//! implementa project config + safe defaults; user config y env overrides
//! quedan para Fase E cuando exista una razón concreta que los requiera.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::storage::AuthorityRole;

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub project: ProjectSection,
    #[serde(default)]
    pub authority: BTreeMap<String, AuthorityDeclaration>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProjectSection {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthorityDeclaration {
    pub role: AuthorityRole,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedAuthority {
    pub role: AuthorityRole,
    pub domain: Option<String>,
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub project_root: PathBuf,
    pub rationale_dir: PathBuf,
    pub project_id: String,
    authority: BTreeMap<String, AuthorityDeclaration>,
}

impl ResolvedConfig {
    /// La autoridad pertenece al proyecto, no al actor que pulsa approve.
    /// Un actor no declarado recibe el rol mínimo y honesto.
    pub fn authority_for_actor(&self, actor: &str) -> ResolvedAuthority {
        self.authority
            .get(actor)
            .map(|decl| ResolvedAuthority {
                role: decl.role,
                domain: decl.domain.clone(),
            })
            .unwrap_or(ResolvedAuthority {
                role: AuthorityRole::Contributor,
                domain: None,
            })
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NoRationaleDirFound,
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoRationaleDirFound => write!(
                f,
                "no se encontró .rationale/ en el directorio actual ni en ningún ancestro"
            ),
            ConfigError::Io(e) => write!(f, "error de I/O leyendo configuración: {e}"),
            ConfigError::Parse(e) => write!(f, "config.yaml inválido: {e}"),
        }
    }
}

/// Busca `.rationale/` subiendo por los directorios ancestros de `start`,
/// igual que Git busca `.git/` (Arquitectura §11.3: "Detectar raíz Git").
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".rationale").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

pub fn load(start: &Path) -> Result<ResolvedConfig, ConfigError> {
    let project_root = find_project_root(start).ok_or(ConfigError::NoRationaleDirFound)?;
    let rationale_dir = project_root.join(".rationale");
    let config_path = rationale_dir.join("config.yaml");

    let parsed = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(ConfigError::Io)?;
        yaml_serde::from_str::<ProjectConfig>(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?
    } else {
        ProjectConfig {
            project: ProjectSection::default(),
            authority: BTreeMap::new(),
        }
    };
    let project_id = parsed
        .project
        .id
        .unwrap_or_else(|| default_project_id(&project_root));
    let authority = parsed.authority;

    Ok(ResolvedConfig {
        project_root,
        rationale_dir,
        project_id,
        authority,
    })
}

fn default_project_id(project_root: &Path) -> String {
    project_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown-project".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_project_root_locates_ancestor_with_rationale_dir() {
        let dir = std::env::temp_dir().join(format!("rationale-cfg-test-{}", std::process::id()));
        let nested = dir.join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(dir.join(".rationale")).unwrap();

        let found = find_project_root(&nested).unwrap();
        assert_eq!(found, dir);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_project_root_returns_none_when_absent() {
        let dir = std::env::temp_dir().join(format!("rationale-cfg-absent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(find_project_root(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn authority_is_project_declared_with_contributor_default() {
        let config = load(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let declared = config.authority_for_actor("user:ragosorio <ragosorio777@gmail.com>");
        assert_eq!(declared.role, AuthorityRole::ArchitectureOwner);
        let undeclared = config.authority_for_actor("user:not-declared");
        assert_eq!(undeclared.role, AuthorityRole::Contributor);
    }
}
