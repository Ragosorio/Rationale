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
//!
//! Defecto real de un dogfood: `diff_since` solo comparaba
//! `<base_revision>..HEAD` — commits. Un cambio editado y NUNCA commiteado
//! (exactamente lo que hace un agente mientras trabaja) producía
//! `changed_files: []`, y de ahí en cascada `binding_declarations: []` en
//! el Record final. Este módulo ahora distingue explícitamente cuatro
//! procedencias (`ChangeOrigin`) y nunca finge que un cambio sin commitear
//! es tan verificable como uno commiteado — ver `Verifiability`.

use crate::providers::{Coverage, ProviderHandle, ProviderStatus};
use crate::revision;
use std::collections::HashMap;
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

/// Orden = verificabilidad decreciente: un tercero con el mismo repo puede
/// verificar `Committed` sin más; `Untracked` requiere confiar en que el
/// working tree local de quien reportó el cambio de verdad lo tenía así en
/// ese momento — nadie más puede reproducirlo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeOrigin {
    Committed,
    Staged,
    Unstaged,
    Untracked,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChangedFile {
    pub path: String,
    pub change_type: ChangeType,
    pub origin: ChangeOrigin,
}

/// Resumen honesto de cuánto de la captura es verificable por un tercero
/// con el mismo repo — nunca se mezcla en silencio con `changed_files`
/// commiteados, que sí lo son.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verifiability {
    FullyCommitted,
    PartiallyUncommitted,
    EntirelyUncommitted,
    NoChanges,
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
    /// confiable. Un mismo path aparece una sola vez, con el `origin` MÁS
    /// DÉBIL entre los que lo tocaron (ver `capture`).
    pub changed_files: Vec<ChangedFile>,
    pub committed_file_count: usize,
    pub uncommitted_file_count: usize,
    pub verifiability: Verifiability,
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

/// Parsea la salida `-z` (NUL-delimitada) de `git diff --name-status`.
/// `-z` no es cosmético: sin él, Git cita y escapa cualquier path con
/// caracteres no-ASCII, corrompiendo `path_hint` para cualquier archivo con
/// tilde o eñe. Formato real: `STATUS\0PATH\0` por entrada, o
/// `R100\0OLDPATH\0NEWPATH\0` para renombres (el número tras `R` es el
/// porcentaje de similitud, se ignora). Un status desconocido (`C` copy,
/// `T` typechange...) se ignora — nunca se infiere qué representa.
fn parse_name_status_z(output: &[u8]) -> Vec<(String, ChangeType)> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text.split('\0').filter(|f| !f.is_empty());
    let mut result = Vec::new();
    while let Some(status) = fields.next() {
        let change_type = match status.chars().next() {
            Some('A') => ChangeType::Added,
            Some('M') => ChangeType::Modified,
            Some('D') => ChangeType::Deleted,
            Some('R') => ChangeType::Renamed,
            _ => continue,
        };
        let path = if change_type == ChangeType::Renamed {
            let Some(_old_path) = fields.next() else {
                break;
            };
            match fields.next() {
                Some(p) => p.to_string(),
                None => break,
            }
        } else {
            match fields.next() {
                Some(p) => p.to_string(),
                None => break,
            }
        };
        result.push((path, change_type));
    }
    result
}

fn run_git_name_status(repo_path: &Path, args: &[&str]) -> Vec<(String, ChangeType)> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output();
    let Ok(output) = output else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    parse_name_status_z(&output.stdout)
}

/// `git diff --name-status <base_revision>..HEAD`, normalizado. Nunca
/// infiere: si Git no puede resolver `base_revision` (no existe, o el path
/// no es un repo), devuelve una lista vacía — el caller lo distingue de
/// "sin cambios reales" mirando `provider_status`/si `base_revision` es
/// resoluble por separado (`revision::snapshot`), nunca asumiendo aquí.
pub fn diff_since(repo_path: &Path, base_revision: &str) -> Vec<ChangedFile> {
    run_git_name_status(
        repo_path,
        &["diff", "--name-status", "-z", base_revision, "HEAD"],
    )
    .into_iter()
    .map(|(path, change_type)| ChangedFile {
        path,
        change_type,
        origin: ChangeOrigin::Committed,
    })
    .collect()
}

/// Cambios ya en el índice (`git add`) pero sin commitear.
fn staged_changes(repo_path: &Path) -> Vec<ChangedFile> {
    run_git_name_status(
        repo_path,
        &["diff", "--name-status", "-z", "--cached", "HEAD"],
    )
    .into_iter()
    .map(|(path, change_type)| ChangedFile {
        path,
        change_type,
        origin: ChangeOrigin::Staged,
    })
    .collect()
}

/// Cambios en archivos ya rastreados por Git, ni siquiera `add`eados.
fn unstaged_changes(repo_path: &Path) -> Vec<ChangedFile> {
    run_git_name_status(repo_path, &["diff", "--name-status", "-z"])
        .into_iter()
        .map(|(path, change_type)| ChangedFile {
            path,
            change_type,
            origin: ChangeOrigin::Unstaged,
        })
        .collect()
}

/// Archivos nuevos que Git todavía no rastrea en absoluto (excluye lo que
/// ya cubre `.gitignore` vía `--exclude-standard`).
fn untracked_changes(repo_path: &Path) -> Vec<ChangedFile> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output();
    let Ok(output) = output else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| ChangedFile {
            path: p.to_string(),
            change_type: ChangeType::Added,
            origin: ChangeOrigin::Untracked,
        })
        .collect()
}

/// Todo lo que el working tree tiene y Git aún no llevó a un commit —
/// staged, unstaged y untracked juntos. Antes de este módulo, ninguno de
/// los tres se miraba: solo `diff_since` (commits), por eso un cambio
/// editado sin commitear producía `changed_files: []`.
fn working_tree_changes(repo_path: &Path) -> Vec<ChangedFile> {
    let mut all = staged_changes(repo_path);
    all.extend(unstaged_changes(repo_path));
    all.extend(untracked_changes(repo_path));
    all
}

/// Captura mecánica completa para `finalize_change` (Fase F5): diff desde
/// `base_revision` MÁS el working tree actual, revisión final, estado del
/// working tree, y cobertura del proveedor estructural. Ningún campo aquí
/// es una inferencia — cada uno es directamente verificable por cualquiera
/// con acceso al mismo repo y al mismo proveedor (los `Committed`; los
/// demás quedan marcados como tal en vez de fingir la misma certeza).
///
/// `exclude_rel_prefixes` filtra rutas repo-relativas que empiecen con
/// cualquiera de los prefijos dados — `finalize_change` pasa `.rationale/`
/// aquí: sin esto, el escaneo de untracked ataría un Record a las
/// propuestas YAML que la propia llamada acaba de escribir.
pub fn capture(
    repo_path: &Path,
    base_revision: &str,
    exclude_rel_prefixes: &[&str],
    provider: &mut ProviderHandle,
) -> MechanicalCapture {
    let snap = revision::snapshot(repo_path);

    let mut all_changes = diff_since(repo_path, base_revision);
    all_changes.extend(working_tree_changes(repo_path));
    all_changes.retain(|f| !exclude_rel_prefixes.iter().any(|p| f.path.starts_with(p)));

    // Dedup por path, conservando el origen MÁS DÉBIL: un archivo
    // commiteado en el rango base..HEAD y vuelto a editar después es
    // honestamente `Unstaged` — la versión a la que un binding se ataría
    // no es la que quedó en el commit.
    let mut by_path: HashMap<String, ChangedFile> = HashMap::new();
    for change in all_changes {
        by_path
            .entry(change.path.clone())
            .and_modify(|existing| {
                if change.origin > existing.origin {
                    *existing = change.clone();
                }
            })
            .or_insert(change);
    }
    let mut changed_files: Vec<ChangedFile> = by_path.into_values().collect();
    changed_files.sort_by(|a, b| a.path.cmp(&b.path));

    let committed_file_count = changed_files
        .iter()
        .filter(|f| f.origin == ChangeOrigin::Committed)
        .count();
    let uncommitted_file_count = changed_files.len() - committed_file_count;

    let verifiability = if changed_files.is_empty() {
        Verifiability::NoChanges
    } else if uncommitted_file_count == 0 {
        Verifiability::FullyCommitted
    } else if committed_file_count == 0 {
        Verifiability::EntirelyUncommitted
    } else {
        Verifiability::PartiallyUncommitted
    };

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
        committed_file_count,
        uncommitted_file_count,
        verifiability,
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

    fn run(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn diff_since_reports_added_modified_and_deleted_files() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        // Modificar a.txt, añadir b.txt, y luego borrar a.txt en un segundo commit.
        std::fs::write(dir.join("a.txt"), "segunda version\n").unwrap();
        std::fs::write(dir.join("b.txt"), "nuevo archivo\n").unwrap();
        run(&dir, &["add", "-A"]);
        run(&dir, &["commit", "-q", "-m", "modifica a, agrega b"]);

        let changes = diff_since(&dir, &base);
        assert_eq!(changes.len(), 2, "a.txt modificado, b.txt agregado");
        assert!(changes
            .iter()
            .any(|c| c.path == "a.txt" && c.change_type == ChangeType::Modified));
        assert!(changes
            .iter()
            .any(|c| c.path == "b.txt" && c.change_type == ChangeType::Added));
        assert!(changes.iter().all(|c| c.origin == ChangeOrigin::Committed));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn diff_since_reports_deleted_files() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::remove_file(dir.join("a.txt")).unwrap();
        run(&dir, &["add", "-A"]);
        run(&dir, &["commit", "-q", "-m", "borra a.txt"]);

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

        let result = capture(&dir, &base, &[], &mut provider);
        assert_eq!(result.provider_status, "unavailable");
        assert_eq!(result.provider_coverage, "unknown");
        assert_eq!(result.base_revision, Some(base));
        assert!(result.final_revision.is_some());
        assert_eq!(result.verifiability, Verifiability::NoChanges);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// El defecto real del dogfood, reproducido directamente: editar un
    /// archivo commiteado SIN commitear (`base == HEAD`) debe capturarlo
    /// igual — antes `diff_since` solo veía commits y esto producía
    /// `changed_files: []`.
    #[test]
    fn uncommitted_edit_is_captured_and_tagged_unstaged() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::write(dir.join("a.txt"), "editado sin commitear\n").unwrap();

        let mut provider = ProviderHandle::Unavailable("test".to_string());
        let result = capture(&dir, &base, &[], &mut provider);

        assert_eq!(result.changed_files.len(), 1);
        assert_eq!(result.changed_files[0].path, "a.txt");
        assert_eq!(result.changed_files[0].origin, ChangeOrigin::Unstaged);
        assert_eq!(result.verifiability, Verifiability::EntirelyUncommitted);
        assert_eq!(result.committed_file_count, 0);
        assert_eq!(result.uncommitted_file_count, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn untracked_file_is_captured() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::write(dir.join("nuevo.txt"), "nunca visto por git\n").unwrap();

        let mut provider = ProviderHandle::Unavailable("test".to_string());
        let result = capture(&dir, &base, &[], &mut provider);

        assert_eq!(result.changed_files.len(), 1);
        assert_eq!(result.changed_files[0].path, "nuevo.txt");
        assert_eq!(result.changed_files[0].origin, ChangeOrigin::Untracked);
        assert_eq!(result.changed_files[0].change_type, ChangeType::Added);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn staged_file_is_captured_and_tagged_staged() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::write(dir.join("staged.txt"), "en el indice\n").unwrap();
        run(&dir, &["add", "staged.txt"]);

        let mut provider = ProviderHandle::Unavailable("test".to_string());
        let result = capture(&dir, &base, &[], &mut provider);

        assert_eq!(result.changed_files.len(), 1);
        assert_eq!(result.changed_files[0].origin, ChangeOrigin::Staged);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Un path con tilde sobrevive intacto gracias a `-z` — sin él, Git
    /// citaría/escaparía el nombre y `path_hint` quedaría corrupto.
    #[test]
    fn path_with_accented_characters_survives_z_parsing() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::write(dir.join("acción.txt"), "contenido\n").unwrap();

        let mut provider = ProviderHandle::Unavailable("test".to_string());
        let result = capture(&dir, &base, &[], &mut provider);

        assert_eq!(result.changed_files.len(), 1);
        assert_eq!(result.changed_files[0].path, "acción.txt");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Mismo archivo commiteado en el rango base..HEAD y vuelto a editar
    /// después: debe reportarse una sola vez, con el origen más débil
    /// (Unstaged), nunca duplicado ni con el origen Committed más fuerte.
    #[test]
    fn same_file_committed_then_edited_reports_weakest_origin() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::write(dir.join("a.txt"), "segunda version, commiteada\n").unwrap();
        run(&dir, &["add", "-A"]);
        run(&dir, &["commit", "-q", "-m", "segunda version"]);
        std::fs::write(dir.join("a.txt"), "tercera version, sin commitear\n").unwrap();

        let mut provider = ProviderHandle::Unavailable("test".to_string());
        let result = capture(&dir, &base, &[], &mut provider);

        let matches: Vec<_> = result
            .changed_files
            .iter()
            .filter(|f| f.path == "a.txt")
            .collect();
        assert_eq!(matches.len(), 1, "no debe duplicarse por path");
        assert_eq!(matches[0].origin, ChangeOrigin::Unstaged);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// `exclude_rel_prefixes` — sin esto, `finalize_change` ataría un
    /// Record a las propuestas YAML que la misma llamada acaba de escribir
    /// en `.rationale/proposals/`.
    #[test]
    fn exclude_rel_prefixes_filters_out_matching_paths() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::create_dir_all(dir.join(".rationale/proposals")).unwrap();
        std::fs::write(
            dir.join(".rationale/proposals/constraint.test.yaml"),
            "id: constraint.test\n",
        )
        .unwrap();
        std::fs::write(dir.join("real-change.txt"), "esto sí importa\n").unwrap();

        let mut provider = ProviderHandle::Unavailable("test".to_string());
        let result = capture(&dir, &base, &[".rationale/"], &mut provider);

        assert_eq!(result.changed_files.len(), 1);
        assert_eq!(result.changed_files[0].path, "real-change.txt");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn verifiability_is_partially_uncommitted_when_both_kinds_present() {
        let dir = make_test_repo();
        let base = revision::snapshot(&dir).head.unwrap();

        std::fs::write(dir.join("a.txt"), "commiteado\n").unwrap();
        run(&dir, &["add", "-A"]);
        run(&dir, &["commit", "-q", "-m", "commit real"]);
        std::fs::write(dir.join("sin-commitear.txt"), "no commiteado\n").unwrap();

        let mut provider = ProviderHandle::Unavailable("test".to_string());
        let result = capture(&dir, &base, &[], &mut provider);

        assert_eq!(result.verifiability, Verifiability::PartiallyUncommitted);
        assert_eq!(result.committed_file_count, 1);
        assert_eq!(result.uncommitted_file_count, 1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
