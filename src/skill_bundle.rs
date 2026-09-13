//! El skill `rationale`, empaquetado en el binario.
//!
//! `skills/rationale/` es la única fuente: el mismo directorio que instala
//! `npx skills add Ragosorio/Rationale` desde GitHub y el que `install-agent`
//! escribe en `.claude/skills/rationale/` (Claude Code) y
//! `.agents/skills/rationale/` (Codex). Sigue el estándar Agent Skills: un
//! `SKILL.md` que enruta, referencias de un solo nivel que el agente carga
//! bajo demanda, un validador determinista y plantillas.
//!
//! `evals/` no se empaqueta: sirve a quien mantiene el skill, no al agente que
//! lo usa, y no debe llenar los proyectos de los usuarios.

pub const SKILL_NAME: &str = "rationale";

pub struct BundleFile {
    /// Ruta relativa al directorio del skill, siempre con `/`.
    pub path: &'static str,
    pub content: &'static str,
}

macro_rules! bundled {
    ($path:literal) => {
        BundleFile {
            path: $path,
            content: include_str!(concat!("../skills/rationale/", $path)),
        }
    };
}

/// Archivos anidados primero y `SKILL.md` al final. `uninstall-agent` borra
/// en el orden del manifest y solo retira el directorio padre de cada archivo
/// cuando queda vacío: con este orden, borrar `SKILL.md` al final deja
/// `rationale/` vacío y también se retira.
pub const FILES: &[BundleFile] = &[
    bundled!("agents/openai.yaml"),
    bundled!("assets/candidates.template.json"),
    bundled!("assets/report-templates.md"),
    bundled!("references/adopt.md"),
    bundled!("references/anti-patterns.md"),
    bundled!("references/capture.md"),
    bundled!("references/cli.md"),
    bundled!("references/concepts.md"),
    bundled!("references/conflicts.md"),
    bundled!("references/explain.md"),
    bundled!("references/health.md"),
    bundled!("references/maintain.md"),
    bundled!("references/packet.md"),
    bundled!("references/preflight.md"),
    bundled!("references/records.md"),
    bundled!("scripts/check_candidates.py"),
    bundled!("SKILL.md"),
];

/// Archivos que el skill tuvo y ya no tiene. Un archivo que sale de `FILES`
/// entra aquí: un manifest de una versión anterior todavía lo registra, y
/// sin reconocerlo `uninstall-agent` rechazaría la entrada entera
/// (`constraint.retired-skills-remain-managed-destinations`).
pub const RETIRED_FILES: &[&str] = &[];

/// Rutas de todos los archivos que Rationale administra o administró dentro
/// del skill, relativas a su directorio.
pub fn managed_file_paths() -> impl Iterator<Item = &'static str> {
    FILES
        .iter()
        .map(|file| file.path)
        .chain(RETIRED_FILES.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    fn skill_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/rationale")
    }

    fn files_on_disk(dir: &Path, root: &Path, out: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if relative == "evals" {
                continue;
            }
            if path.is_dir() {
                files_on_disk(&path, root, out);
            } else if !relative.ends_with(".DS_Store") {
                out.insert(relative);
            }
        }
    }

    fn frontmatter_field<'a>(skill: &'a str, field: &str) -> Option<&'a str> {
        let frontmatter = skill.strip_prefix("---\n")?.split("\n---\n").next()?;
        frontmatter
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{field}: ")))
    }

    fn skill_md() -> &'static str {
        FILES
            .iter()
            .find(|file| file.path == "SKILL.md")
            .unwrap()
            .content
    }

    /// Un archivo nuevo en `skills/rationale/` que nadie añadió a `FILES`
    /// llegaría a quien instala con `npx skills` pero nunca a quien usa
    /// `install-agent`: dos instalaciones del mismo skill con contenido
    /// distinto.
    #[test]
    fn the_bundle_matches_the_skill_directory() {
        let mut on_disk = BTreeSet::new();
        files_on_disk(&skill_dir(), &skill_dir(), &mut on_disk);
        let bundled: BTreeSet<String> = FILES.iter().map(|f| f.path.to_string()).collect();
        assert_eq!(
            on_disk, bundled,
            "FILES debe listar exactamente skills/rationale/ sin evals/"
        );
    }

    #[test]
    fn skill_md_is_written_last_so_uninstall_leaves_no_empty_directory() {
        assert_eq!(FILES.last().unwrap().path, "SKILL.md");
        for file in FILES {
            assert!(!file.path.contains('\\') && !file.path.contains(".."));
        }
        for retired in RETIRED_FILES {
            assert!(
                FILES.iter().all(|file| file.path != *retired),
                "'{retired}' no puede estar vigente y retirado a la vez"
            );
        }
    }

    /// Límites del estándar Agent Skills (agentskills.io/specification) y de
    /// la guía de Anthropic: `name` coincide con el directorio, `description`
    /// ≤ 1024 caracteres sin etiquetas XML, `compatibility` ≤ 500 y un
    /// `SKILL.md` de menos de 500 líneas.
    #[test]
    fn skill_md_follows_the_agent_skills_specification() {
        let skill = skill_md();
        assert_eq!(frontmatter_field(skill, "name"), Some(SKILL_NAME));
        let description = frontmatter_field(skill, "description").unwrap();
        assert!(!description.is_empty() && description.chars().count() <= 1024);
        assert!(!description.contains('<') && !description.contains('>'));
        assert!(
            description.contains("Use "),
            "la descripción dice cuándo usarlo"
        );
        let compatibility = frontmatter_field(skill, "compatibility").unwrap();
        assert!(compatibility.chars().count() <= 500);
        assert!(skill.lines().count() < 500);
    }

    /// Referencias a un solo nivel: todo archivo que el agente pueda
    /// necesitar se enlaza desde `SKILL.md`, y cada enlace existe. Un
    /// archivo sin enlace nunca se carga; un enlace roto manda al agente a
    /// buscar algo que no está.
    #[test]
    fn every_reference_is_linked_from_skill_md_and_every_link_exists() {
        let skill = skill_md();
        let links: BTreeSet<&str> = skill
            .split("](")
            .skip(1)
            .filter_map(|rest| rest.split(')').next())
            .filter(|target| !target.starts_with("http"))
            .collect();
        for link in &links {
            assert!(
                FILES.iter().any(|file| file.path == *link),
                "SKILL.md enlaza {link}, que no está en el skill"
            );
        }
        for file in FILES {
            if file.path.starts_with("references/") || file.path.starts_with("assets/") {
                assert!(
                    links.contains(file.path),
                    "{} no está enlazado desde SKILL.md",
                    file.path
                );
            }
        }
    }

    /// La guía de Anthropic pide un índice en referencias de más de 100
    /// líneas: un agente que previsualiza el archivo ve igual todo su alcance.
    #[test]
    fn long_references_start_with_contents() {
        for file in FILES.iter().filter(|f| f.path.starts_with("references/")) {
            if file.content.lines().count() > 100 {
                assert!(
                    file.content.contains("## Contents"),
                    "{} supera 100 líneas y no tiene índice",
                    file.path
                );
            }
        }
    }

    #[test]
    fn skill_md_asks_for_the_users_language() {
        assert!(skill_md().contains("Reply in the language the user writes in"));
    }

    /// El validador del skill promete reflejar el gate de captura. Si una
    /// lista o un umbral del gate cambia sin tocar el script, el agente
    /// recibiría un "ok" para candidatos que el gate descarta.
    #[test]
    fn the_candidate_validator_mirrors_the_capture_gate() {
        let script = FILES
            .iter()
            .find(|file| file.path == "scripts/check_candidates.py")
            .unwrap()
            .content;
        for marker in crate::canon::MECHANICAL_PREFIXES
            .iter()
            .chain(crate::canon::CAUSAL_MARKERS.iter())
        {
            assert!(
                script.contains(&format!("\"{marker}\"")),
                "check_candidates.py no refleja la entrada del gate {marker:?}"
            );
        }
        for kind in crate::canon::VALID_KINDS {
            assert!(script.contains(&format!("\"{kind}\"")));
        }
        assert!(script.contains(&format!(
            "MIN_TEXT_CHARS = {}",
            crate::canon::MIN_TEXT_CHARS
        )));
        assert!(script.contains(&format!(
            "MIN_TEXT_WORDS = {}",
            crate::canon::MIN_TEXT_WORDS
        )));
    }
}
