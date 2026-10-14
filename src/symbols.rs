//! Verificación léxica de símbolos contra el árbol de trabajo.
//!
//! No es un parser ni un índice: responde una sola pregunta, de forma
//! determinista y barata — «¿este archivo, tal como está en disco, define un
//! identificador con este nombre?». Sirve para tres cosas que el índice del
//! proveedor estructural no puede garantizar cuando va atrasado:
//!
//! 1. Atar un Record a un símbolo creado en la misma rama (`finalize_change`)
//!    sin inventar una identidad estructural: el binding queda `symbol` +
//!    `provisional`, nunca `structural_id`.
//! 2. Distinguir «nada gobierna este símbolo» de «el índice no lo conoce».
//! 3. Descartar nodos del índice que ya no existen en el árbol de trabajo
//!    (archivos borrados, otro checkout indexado en la misma ruta).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Palabras que introducen una definición en los lenguajes habituales.
const DEFINITION_KEYWORDS: &[&str] = &[
    "fn",
    "function",
    "def",
    "class",
    "struct",
    "enum",
    "trait",
    "interface",
    "type",
    "const",
    "let",
    "var",
    "val",
    "mod",
    "module",
    "func",
    "impl",
    "macro_rules!",
    "static",
    "record",
    "object",
];

/// Modificadores que pueden preceder a un método definido por nombre
/// (`public async foo(`, `static bar(`).
const MODIFIERS: &[&str] = &[
    "pub",
    "crate",
    "export",
    "default",
    "async",
    "static",
    "public",
    "private",
    "protected",
    "override",
    "readonly",
    "abstract",
    "final",
    "unsafe",
    "extern",
    "inline",
    "virtual",
    "get",
    "set",
];

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Último segmento de un símbolo calificado (`Type::method`, `a.b.c`).
pub fn short_name(symbol: &str) -> &str {
    symbol
        .rsplit([':', '.'])
        .find(|s| !s.is_empty())
        .unwrap_or(symbol)
}

/// Tokens de identificador de una línea, en orden.
fn words(line: &str) -> Vec<&str> {
    line.split(|c: char| !(is_ident_char(c) || c == '!'))
        .filter(|w| !w.is_empty())
        .collect()
}

fn defines_on_line(line: &str, name: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('#') && !trimmed.starts_with("#[") {
        return false;
    }
    // Método de Go con receptor: `func (s *Server) Handle(`.
    if let Some(receiver) = trimmed.strip_prefix("func (") {
        if let Some(close) = receiver.find(')') {
            let rest = receiver[close + 1..].trim_start();
            if rest
                .strip_prefix(name)
                .is_some_and(|after| after.starts_with(['(', '[']))
            {
                return true;
            }
        }
    }
    let tokens = words(trimmed);
    for (index, token) in tokens.iter().enumerate() {
        if *token != name {
            continue;
        }
        // `fn name`, `class Name`, `const name`…
        if index > 0 && DEFINITION_KEYWORDS.contains(&tokens[index - 1]) {
            return true;
        }
        // Método o propiedad al inicio de la línea tras modificadores:
        // `async name(`, `name(args) {`, `name = (`, `name: function`.
        if tokens[..index].iter().all(|t| MODIFIERS.contains(t)) {
            let after = trimmed
                .find(name)
                .map(|at| trimmed[at + name.len()..].trim_start())
                .unwrap_or_default();
            if after.starts_with('(')
                || after.starts_with('=') && !after.starts_with("==")
                || after.starts_with(':')
                || after.starts_with('<')
            {
                return true;
            }
        }
    }
    false
}

/// Línea (1-based) donde `source` define `symbol`, si la hay.
pub fn find_definition(source: &str, symbol: &str) -> Option<u32> {
    let name = short_name(symbol);
    if name.is_empty() || !name.chars().all(is_ident_char) {
        return None;
    }
    source
        .lines()
        .position(|line| line.contains(name) && defines_on_line(line, name))
        .map(|index| index as u32 + 1)
}

/// `true` si el identificador aparece como palabra completa en `line`.
fn mentions(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + name.len()..].chars().next();
        !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char)
    })
}

/// Lecturas del árbol de trabajo cacheadas durante una consulta.
#[derive(Default)]
pub struct WorkingTree {
    root: PathBuf,
    files: HashMap<String, Option<String>>,
}

/// Resultado de contrastar un nodo del índice con el árbol de trabajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCheck {
    Present,
    /// El archivo ya no existe.
    MissingFile,
    /// El archivo existe pero el símbolo no está donde el índice dice.
    Moved,
}

impl WorkingTree {
    pub fn new(root: &Path) -> Self {
        WorkingTree {
            root: root.to_path_buf(),
            files: HashMap::new(),
        }
    }

    pub fn read(&mut self, rel_path: &str) -> Option<&str> {
        let root = &self.root;
        self.files
            .entry(rel_path.to_string())
            .or_insert_with(|| {
                let path = root.join(rel_path);
                path.is_file()
                    .then(|| std::fs::read_to_string(path).ok())
                    .flatten()
            })
            .as_deref()
    }

    pub fn exists(&mut self, rel_path: &str) -> bool {
        self.read(rel_path).is_some() || self.root.join(rel_path).exists()
    }

    pub fn find_definition(&mut self, rel_path: &str, symbol: &str) -> Option<u32> {
        find_definition(self.read(rel_path)?, symbol)
    }

    /// ¿Sigue el nodo donde el índice lo ubica? Tolerante a desplazamientos
    /// pequeños (el índice va unos commits atrás) pero no a un archivo
    /// borrado ni a un símbolo que ya no aparece en el archivo.
    pub fn check_node(&mut self, rel_path: &str, name: &str, start_line: Option<u32>) -> NodeCheck {
        let Some(source) = self.read(rel_path) else {
            return NodeCheck::MissingFile;
        };
        let name = short_name(name);
        let present = match start_line {
            Some(line) if line > 0 => source
                .lines()
                .skip(line.saturating_sub(4) as usize)
                .take(7)
                .any(|text| mentions(text, name)),
            _ => false,
        } || source.lines().any(|text| mentions(text, name));
        if present {
            NodeCheck::Present
        } else {
            NodeCheck::Moved
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_definitions_across_languages() {
        let cases = [
            ("export function splitShipment(id) {}", "splitShipment"),
            ("pub fn compile(input: X) -> Y {", "compile"),
            ("    pub(crate) async fn run() {}", "run"),
            ("def rotate_keys(ids):", "rotate_keys"),
            ("class PaymentService:", "PaymentService"),
            ("  async retryLabel(id: string) {", "retryLabel"),
            ("const quoteRate = (a, b) => a + b;", "quoteRate"),
            ("func (s *Server) Handle(w http.ResponseWriter) {", "Handle"),
            ("impl Budget {", "Budget"),
        ];
        for (line, name) in cases {
            assert_eq!(find_definition(line, name), Some(1), "{line}");
        }
    }

    #[test]
    fn calls_and_comments_are_not_definitions() {
        assert_eq!(
            find_definition("  const x = quoteRate(a, b);", "quoteRate"),
            None
        );
        assert_eq!(find_definition("// fn compile is old", "compile"), None);
        assert_eq!(
            find_definition("let total = compile(input);", "compile"),
            None
        );
        assert_eq!(find_definition("fn compile_all() {}", "compile"), None);
    }

    #[test]
    fn qualified_symbols_use_their_last_segment() {
        let source = "impl Budget {\n    pub fn trim(&mut self) {}\n}\n";
        assert_eq!(find_definition(source, "Budget::trim"), Some(2));
        assert_eq!(find_definition(source, "src.budget.Budget.trim"), Some(2));
    }

    #[test]
    fn node_checks_detect_deleted_files_and_vanished_symbols() {
        let dir = std::env::temp_dir().join(format!("rationale-symbols-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.ts"), "export function kept() {}\n").unwrap();
        let mut tree = WorkingTree::new(&dir);
        assert_eq!(
            tree.check_node("src/a.ts", "kept", Some(1)),
            NodeCheck::Present
        );
        assert_eq!(
            tree.check_node("src/a.ts", "src.a.kept", Some(40)),
            NodeCheck::Present
        );
        assert_eq!(
            tree.check_node("src/a.ts", "gone", Some(1)),
            NodeCheck::Moved
        );
        assert_eq!(
            tree.check_node("src/b.ts", "kept", Some(1)),
            NodeCheck::MissingFile
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
