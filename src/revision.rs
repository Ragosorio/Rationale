//! Revision Coordinator — ADR-0006: el fingerprint de revisión se deriva
//! exclusivamente de Git, nunca del proveedor estructural.
//!
//! Evidencia: docs/research/codebase-memory/05-revision-and-coverage.md
//! demostró que `detect_changes` de Codebase Memory puede devolver cero
//! cambios ante 200 archivos realmente modificados. Este módulo nunca
//! consulta al proveedor para decidir si el código cambió.

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Consistency {
    /// La revisión Git actual coincide exactamente con `bound_revision`.
    Exact,
    /// El working tree tiene cambios no confirmados (dirty).
    WorkingTreeAhead,
    /// La revisión Git actual difiere de `bound_revision` (HEAD avanzó).
    StructuralIndexBehind,
    /// No se pudo determinar (no es un repo Git, o git no está disponible).
    Unresolved,
}

impl std::fmt::Display for Consistency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Consistency::Exact => "exact",
            Consistency::WorkingTreeAhead => "working-tree-ahead",
            Consistency::StructuralIndexBehind => "structural-index-behind",
            Consistency::Unresolved => "unresolved",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, serde::Serialize)]
pub struct GitSnapshot {
    pub head: Option<String>,
    pub working_tree_dirty: bool,
}

/// Ejecuta `git rev-parse HEAD` y `git status --short` sobre `repo_path`.
/// Nunca consulta al proveedor estructural para esto (ADR-0006).
pub fn snapshot(repo_path: &Path) -> GitSnapshot {
    let head = run_git(repo_path, &["rev-parse", "HEAD"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let dirty = run_git(repo_path, &["status", "--short"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    GitSnapshot {
        head,
        working_tree_dirty: dirty,
    }
}

/// Compara la revisión Git real contra la revisión a la que un binding fue
/// declarado válido (`bound_revision` en un Record). Nunca delega esta
/// comparación al proveedor estructural.
pub fn check_consistency(snapshot: &GitSnapshot, bound_revision: &str) -> Consistency {
    match &snapshot.head {
        None => Consistency::Unresolved,
        Some(head) => {
            if snapshot.working_tree_dirty {
                Consistency::WorkingTreeAhead
            } else if head == bound_revision {
                Consistency::Exact
            } else {
                Consistency::StructuralIndexBehind
            }
        }
    }
}

fn run_git(repo_path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_when_head_matches_bound_revision_and_clean() {
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: false,
        };
        assert_eq!(check_consistency(&snap, "abc123"), Consistency::Exact);
    }

    #[test]
    fn structural_index_behind_when_head_differs() {
        let snap = GitSnapshot {
            head: Some("def456".to_string()),
            working_tree_dirty: false,
        };
        assert_eq!(
            check_consistency(&snap, "abc123"),
            Consistency::StructuralIndexBehind
        );
    }

    #[test]
    fn working_tree_ahead_when_dirty_even_if_head_matches() {
        let snap = GitSnapshot {
            head: Some("abc123".to_string()),
            working_tree_dirty: true,
        };
        assert_eq!(
            check_consistency(&snap, "abc123"),
            Consistency::WorkingTreeAhead
        );
    }

    #[test]
    fn unresolved_when_no_head() {
        let snap = GitSnapshot {
            head: None,
            working_tree_dirty: false,
        };
        assert_eq!(check_consistency(&snap, "abc123"), Consistency::Unresolved);
    }

    #[test]
    fn snapshot_reads_real_git_repo() {
        // Usa el propio repo de Rationale como fixture — siempre existe en CI/dev.
        let repo_root = std::env::current_dir().unwrap();
        let snap = snapshot(&repo_root);
        assert!(
            snap.head.is_some(),
            "el propio repo debe tener HEAD resoluble"
        );
    }
}
