//! Configuration — localiza la raíz del proyecto y lee `.rationale/config.yaml`.
//!
//! Arquitectura_Conceptual_v0.1.md §11.2. Precedencia (documentada, no toda
//! implementada todavía en la vertical slice): project config > user config >
//! environment overrides > safe defaults. La vertical slice de Fase D solo
//! implementa project config + safe defaults; user config y env overrides
//! quedan para Fase E cuando exista una razón concreta que los requiera.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub project: ProjectSection,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProjectSection {
    pub id: Option<String>,
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub project_root: PathBuf,
    pub rationale_dir: PathBuf,
    pub project_id: String,
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

    let project_id = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(ConfigError::Io)?;
        let parsed: ProjectConfig =
            serde_yaml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        parsed
            .project
            .id
            .unwrap_or_else(|| default_project_id(&project_root))
    } else {
        default_project_id(&project_root)
    };

    Ok(ResolvedConfig {
        project_root,
        rationale_dir,
        project_id,
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
}
