//! Chestie — la mascota de Rationale en la CLI.
//!
//! Nunca aparece en `rationale serve`: ese stdout es protocolo MCP, no
//! decoración. Vive solo en los comandos que un humano ejecuta y lee
//! directamente. Cuando aparece junto a un JSON de máquina (`init`,
//! `health`, `prepare`), va siempre a stderr; el stdout es y sigue siendo un
//! contrato, no un lienzo.
//!
//! Los cuatro estados corresponden a momentos reales del producto, no a
//! decoración arbitraria: Base (saludo), Searching (`prepare_change`
//! consultando, `install-agent` detectando), Happy (contexto encontrado,
//! nada que objetar), Guarding (una constraint crítica o un proveedor
//! preocupado — la valla de Chesterton, literal).

use std::io::IsTerminal;

pub enum Mood {
    Base,
    Searching,
    Happy,
    Guarding,
}

const BASE: &str = "      (\\__/)
     ( ˶•ᴗ•˶)
    ╭/  R  \\╮
  ━━┿━━━━━━━┿━━";

const SEARCHING: &str = "      (\\__/)   🔎
     ( ˶•o•˶) /
    ╭/  R  \\╮
  ━━┿━━━━━━━┿━━";

const HAPPY: &str = "      (\\__/)   ✨
     ( ˶>ᴗ<˶) /
    ╭/  R  \\╮
  ━━┿━━━━━━━┿━━";

const GUARDING: &str = "      (\\__/)
     ( ˶•~•˶)
    ╭/  R  \\╮
  ━━┿━━━━━━━┿━━";

pub fn art(mood: Mood) -> &'static str {
    match mood {
        Mood::Base => BASE,
        Mood::Searching => SEARCHING,
        Mood::Happy => HAPPY,
        Mood::Guarding => GUARDING,
    }
}

/// Decide si una decoración para humanos es segura en este proceso.
///
/// `NO_COLOR`, `CI`, una salida redirigida y `--no-mascot` son señales
/// explícitas de que el consumidor quiere una salida sobria y composable.
/// La función recibe la terminal como argumento para que stdout y stderr
/// puedan aplicar la misma política sin duplicarla.
pub fn should_show(args: &[String], output_is_terminal: bool) -> bool {
    !args.iter().any(|arg| arg == "--no-mascot")
        && output_is_terminal
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("CI").is_none()
        && std::env::var_os("RATIONALE_NO_MASCOT").as_deref() != Some(std::ffi::OsStr::new("1"))
}

pub fn print_stdout(args: &[String], mood: Mood) {
    if stdout_enabled(args) {
        println!("{}", art(mood));
    }
}

pub fn print_stderr(args: &[String], mood: Mood) {
    if stderr_enabled(args) {
        eprintln!("{}", art(mood));
    }
}

pub fn print_guarding_stdout(args: &[String], message: &str) {
    if stdout_enabled(args) {
        println!("{}", guarding_with_message(message));
    }
}

pub fn print_guarding_stderr(args: &[String], message: &str) {
    if stderr_enabled(args) {
        eprintln!("{}", guarding_with_message(message));
    }
}

pub fn stdout_enabled(args: &[String]) -> bool {
    should_show(args, std::io::stdout().is_terminal())
}

pub fn stderr_enabled(args: &[String]) -> bool {
    should_show(args, std::io::stderr().is_terminal())
}

/// Chestie señalando una constraint crítica concreta. `message` viene de
/// datos del proyecto (el statement de un Record), no de literales
/// propios — se recorta y se aplana a una línea para que un statement
/// largo, multilínea o con caracteres de control no pueda desalinear ni
/// inyectar nada en la terminal.
pub fn guarding_with_message(message: &str) -> String {
    let statement = single_line(message, 96);
    let bubble_lines = [
        "¡Espera un poquito! Esa valla está ahí por algo…".to_string(),
        format!("\"{statement}\""),
    ];
    let width = bubble_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    let top = format!("  ╭{}╮", "━".repeat(width));
    let bottom = format!("  ╰{}╯", "━".repeat(width));
    let mut lines = art(Mood::Guarding)
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let bubble = [
        top,
        format!("  │ {:width$} │", bubble_lines[0], width = width - 2),
        format!("  │ {:width$} │", bubble_lines[1], width = width - 2),
        bottom,
    ];
    let line_count = lines.len().max(bubble.len());
    lines.resize(line_count, String::new());
    lines
        .into_iter()
        .enumerate()
        .map(|(index, left)| {
            if index < bubble.len() {
                format!("{left}{}", bubble[index])
            } else {
                left
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn single_line(text: &str, max_chars: usize) -> String {
    let collapsed: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let collapsed = collapsed.trim();
    if collapsed.chars().count() <= max_chars {
        return collapsed.to_string();
    }
    let truncated: String = collapsed
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect();
    format!("{}…", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_collapses_newlines_and_strips_ansi_escapes() {
        let malicious = "primera línea\n\x1b[31msegunda\x1b[0m\tlínea";
        let flattened = single_line(malicious, 96);
        assert!(!flattened.contains('\n'));
        assert!(!flattened.contains('\x1b'));
    }

    #[test]
    fn guarding_message_embeds_the_flattened_statement() {
        let out = guarding_with_message("una razón concreta");
        assert!(out.contains("una razón concreta"));
    }

    #[test]
    fn guarding_message_truncates_long_statements() {
        let long = "x".repeat(500);
        let out = guarding_with_message(&long);
        assert!(out.contains('…'));
        assert!(out.len() < 1_000);
    }

    #[test]
    fn guarding_message_grows_with_the_statement() {
        let short = guarding_with_message("corto");
        let long = guarding_with_message("una razón bastante más larga que el texto corto");
        assert!(
            long.lines().next().unwrap().chars().count()
                > short.lines().next().unwrap().chars().count()
        );
    }

    #[test]
    fn no_mascot_and_non_terminal_output_are_silent() {
        let no_mascot = vec!["--no-mascot".to_string()];
        assert!(!should_show(&no_mascot, true));
        assert!(!should_show(&[], false));
    }
}
