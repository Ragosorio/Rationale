//! Project and Target resolution — Arquitectura §11.3.
//!
//! Responsable de traducir un target dado por el agente (`path::symbol`) en
//! una ruta canonicalizada dentro del proyecto, rechazando cualquier intento
//! de escapar de la raíz (`Arquitectura §15.3`: "Canonicalizar. Impedir
//! traversal.").

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Target {
    pub path: PathBuf,
    pub symbol: Option<String>,
}

#[derive(Debug)]
pub enum TargetError {
    PathTraversal,
    NotFound,
    Io(std::io::Error),
}

impl std::fmt::Display for TargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetError::PathTraversal => {
                write!(f, "el target intenta salir de la raíz del proyecto")
            }
            TargetError::NotFound => write!(f, "el target no existe dentro del proyecto"),
            TargetError::Io(e) => write!(f, "error de I/O resolviendo target: {e}"),
        }
    }
}

/// Parsea `"src/auth/authorization.ts::resolveEntityRole"` en path + symbol.
/// Sin `::`, se asume que todo el string es un path sin symbol específico.
pub fn parse_target_spec(spec: &str) -> (String, Option<String>) {
    match spec.split_once("::") {
        Some((path, symbol)) => (path.to_string(), Some(symbol.to_string())),
        None => (spec.to_string(), None),
    }
}

/// Resuelve un target contra `project_root`, canonicalizando y rechazando
/// cualquier ruta que termine fuera de la raíz del proyecto.
pub fn resolve_target(project_root: &Path, spec: &str) -> Result<Target, TargetError> {
    let (path_str, symbol) = parse_target_spec(spec);
    let candidate = project_root.join(&path_str);

    let canonical_root = project_root.canonicalize().map_err(TargetError::Io)?;
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| TargetError::NotFound)?;

    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(TargetError::PathTraversal);
    }

    Ok(Target {
        path: canonical_candidate,
        symbol,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_and_symbol() {
        let (path, symbol) = parse_target_spec("src/auth/authorization.ts::resolveEntityRole");
        assert_eq!(path, "src/auth/authorization.ts");
        assert_eq!(symbol, Some("resolveEntityRole".to_string()));
    }

    #[test]
    fn parses_path_only() {
        let (path, symbol) = parse_target_spec("src/auth/authorization.ts");
        assert_eq!(path, "src/auth/authorization.ts");
        assert_eq!(symbol, None);
    }

    #[test]
    fn rejects_path_traversal() {
        let dir =
            std::env::temp_dir().join(format!("rationale-project-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("inside")).unwrap();
        std::fs::write(dir.join("inside").join("f.ts"), "// ok").unwrap();
        std::fs::write(dir.parent().unwrap().join("outside-secret.txt"), "secret").unwrap();

        let result = resolve_target(&dir, "../outside-secret.txt");
        assert!(matches!(result, Err(TargetError::PathTraversal)));

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_file(dir.parent().unwrap().join("outside-secret.txt")).ok();
    }

    #[test]
    fn resolves_valid_target() {
        let dir = std::env::temp_dir().join(format!(
            "rationale-project-test-valid-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("auth.ts"), "// ok").unwrap();

        let target = resolve_target(&dir, "src/auth.ts::resolveEntityRole").unwrap();
        assert_eq!(target.symbol, Some("resolveEntityRole".to_string()));
        assert!(target.path.ends_with("auth.ts"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
