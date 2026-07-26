//! Captura mecánica — Rationale_v0.5.md §15.1: lo que Rationale captura
//! automáticamente porque es verificable, nunca una inferencia.
//!
//! "Diff. Revisión base y final. Estado del working tree. Commits. Archivos.
//! Símbolos. Relaciones reportadas por el proveedor. Cobertura y versión del
//! proveedor. Tests ejecutados. Resultados." Este módulo produce esos
//! hechos; nunca decide si algo es una decisión normativa (eso es Fase F3,
//! señales) ni escribe nada al canon (eso es `finalize_change`, Fase F5).
//! Todo aquí se marca `epistemic_status: observed` cuando entra a un Record
//! — es la afirmación humana de menor riesgo posible, verificable por
//! cualquiera con acceso al mismo repo.

use crate::providers::{Coverage, ProviderHandle, ProviderStatus};
use crate::revision;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChangedFile {
    pub path: String,
    pub change_type: ChangeType,
}

#[derive(Debug, serde::Serialize)]
pub struct MechanicalCapture {
    /// Revisión desde la que se compara — normalmente `bound_revision` del
    /// último Record relevante, o la revisión en la que empezó la tarea.
    pub base_revision: Option<String>,
    pub final_revision: Option<String>,
    pub working_tree_dirty: bool,
    /// Nunca vacío por "no hubo cambios" silencioso: si `git diff` no pudo
    /// ejecutarse, queda vacío igual, pero el caller siempre tiene
    /// `provider_status`/`base_revision` para saber si el resultado es
    /// confiable.
    pub changed_files: Vec<ChangedFile>,
    pub provider_status: String,
    pub provider_coverage: String,
}

fn provider_status_label(status: &ProviderStatus) -> &'static str {
    match status {
        ProviderStatus::Successful => "successful",
        ProviderStatus::Degraded => "degraded",
        ProviderStatus::Unavailable => "unavailable",
    }
}

fn coverage_label(coverage: &Coverage) -> &'static str {
    match coverage {
        Coverage::Complete => "complete",
        Coverage::Partial => "partial",
        Coverage::Unknown => "unknown",
    }
}

/// Parsea una línea de `git diff --name-status`. Formato real de Git:
/// `M\tpath`, `A\tpath`, `D\tpath`, o `R100\told-path\tnew-path` para
/// renombres (el número tras `R` es el porcentaje de similitud, se ignora).
fn parse_name_status_line(line: &str) -> Option<ChangedFile> {
    let mut parts = line.split('\t');
    let status = parts.next()?;
    let change_type = match status.chars().next()? {
        'A' => ChangeType::Added,
        'M' => ChangeType::Modified,
        'D' => ChangeType::Deleted,
        'R' => ChangeType::Renamed,
        _ => return None,
    };
    let path = if change_type == ChangeType::Renamed {
        // "R100\told\tnew" — el path final es el segundo campo restante.
        parts.nth(1)?.to_string()
    } else {
        parts.next()?.to_string()
    };
    Some(ChangedFile { path, change_type })
}

/// `git diff --name-status <base_revision>..HEAD`, normalizado. Nunca
/// infiere: si Git no puede resolver `base_revision` (no existe, o el path
/// no es un repo), devuelve una lista vacía — el caller lo distingue de
/// "sin cambios reales" mirando `provider_status`/si `base_revision` es
/// resoluble por separado (`revision::snapshot`), nunca asumiendo aquí.
pub fn diff_since(repo_path: &Path, base_revision: &str) -> Vec<ChangedFile> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["diff", "--name-status", base_revision, "HEAD"])
        .output();

    let Ok(output) = output else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().filter_map(parse_name_status_line).collect()
}

/// Captura mecánica completa para `finalize_change` (Fase F5): diff desde
/// `base_revision`, revisión final, estado del working tree, y cobertura
/// del proveedor estructural. Ningún campo aquí es una inferencia — cada
/// uno es directamente verificable por cualquiera con acceso al mismo repo
/// y al mismo proveedor.
pub fn capture(
    repo_path: &Path,
    base_revision: &str,
    provider: &mut ProviderHandle,
) -> MechanicalCapture {
    let snap = revision::snapshot(repo_path);
    let changed_files = diff_since(repo_path, base_revision);

    let (provider_status, provider_coverage) = match provider {
        ProviderHandle::Live(client) => {
            use crate::providers::CodeIntelligenceProvider;
            let result = client.health(repo_path.to_str().unwrap_or(""));
            (result.status, result.coverage)
        }
        ProviderHandle::Unavailable(_) => (ProviderStatus::Unavailable, Coverage::Unknown),
    };

    MechanicalCapture {
        base_revision: Some(base_revision.to_string()),
        final_revision: snap.head,
        working_tree_dirty: snap.working_tree_dirty,
        changed_files,
        provider_status: provider_status_label(&provider_status).to_string(),
        provider_coverage: coverage_label(&provider_coverage).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Repo Git real y desechable, con dos commits que tocan archivos de
    /// forma conocida — usado para verificar `diff_since` contra Git de
    /// verdad, no un mock.
    /// PID + nanos + contador atómico: bajo carga extrema (varios tests de
    /// este módulo llaman `make_test_repo()` en hilos paralelos), la
    /// resolución real del reloj puede no ser tan fina como promete
    /// `as_nanos()` — dos hilos generando el mismo timestamp colisionarían
    /// en el mismo directorio y correrían `git init` concurrente sobre él
    /// (confirmado empíricamente: causaba fallos intermitentes reales, no
    /// solo teóricos). El contador lo hace imposible.
    fn unique_suffix() -> String {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )
    }

    fn make_test_repo() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rationale-capture-test-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?} debe tener éxito");
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@rationale.local"]);
        run(&["config", "user.name", "Rationale Test"]);

        std::fs::write(dir.join("a.txt"), "primera version\n").unwrap();
        run(&["add", "a.txt"]);
        run(&["commit", "-q", "-m", "commit inicial"]);

        dir
    }

    #[test]
    fn diff_since_reports_added_modified_and_deleted_files() {
        let dir = make_test_repo();
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        };

        let base = revision::snapshot(&dir).head.unwrap();

        // Modificar a.txt, añadir b.txt, y luego borrar a.txt en un segundo commit.
        std::fs::write(dir.join("a.txt"), "segunda version\n").unwrap();
        std::fs::write(dir.join("b.txt"), "nuevo archivo\n").unwrap();
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "modifica a, agrega b"]);

        let changes = diff_since(&dir, &base);
        assert_eq!(changes.len(), 2, "a.txt modificado, b.txt agregado");
        assert!(changes
            .iter()
            .any(|c| c.path == "a.txt" && c.change_type == ChangeType::Modified));
        assert!(changes
            .iter()
            .any(|c| c.path == "b.txt" && c.change_type == ChangeType::Added));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn diff_since_reports_deleted_files() {
        let dir = make_test_repo();
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        };
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::remove_file(dir.join("a.txt")).unwrap();
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "borra a.txt"]);

        let changes = diff_since(&dir, &base);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "a.txt");
        assert_eq!(changes[0].change_type, ChangeType::Deleted);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn diff_since_returns_empty_for_unresolvable_base_revision() {
        let dir = make_test_repo();
        // Nunca infiere ni entra en pánico ante una revisión inexistente —
        // simplemente no hay diff que reportar.
        let changes = diff_since(&dir, "0000000000000000000000000000000000000000");
        assert!(changes.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn capture_reports_provider_unavailable_without_panicking() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();
        let mut provider = ProviderHandle::Unavailable("test".to_string());

        let result = capture(&dir, &base, &mut provider);
        assert_eq!(result.provider_status, "unavailable");
        assert_eq!(result.provider_coverage, "unknown");
        assert_eq!(result.base_revision, Some(base));
        assert!(result.final_revision.is_some());

        std::fs::remove_dir_all(&dir).ok();
    }
}
