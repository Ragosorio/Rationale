//! Chestie — la mascota de Rationale en la CLI.
//!
//! Nunca aparece en `rationale serve`: ese stdout es protocolo MCP, no
//! decoración, y ni siquiera stderr de ese proceso es lugar para esto —
//! vive solo en los comandos que un humano ejecuta y lee directamente
//! (`init`, `install-agent`, `uninstall-agent`, `prepare`). Cuando
//! aparece junto a un JSON de máquina (`init`, `prepare`), va siempre a
//! stderr; el stdout es y sigue siendo un contrato, no un lienzo.
//!
//! Los cuatro estados corresponden a momentos reales del producto, no a
//! decoración arbitraria: Base (saludo), Searching (`prepare_change`
//! consultando, `install-agent` detectando), Happy (contexto encontrado,
//! nada que objetar), Guarding (una constraint crítica — la valla de
//! Chesterton, literal).

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

/// Chestie señalando una constraint crítica concreta. `message` viene de
/// datos del proyecto (el statement de un Record), no de literales
/// propios — se recorta y se aplana a una línea para que un statement
/// largo, multilínea o con caracteres de control no pueda desalinear ni
/// inyectar nada en la terminal.
pub fn guarding_with_message(message: &str) -> String {
    format!(
        "{}\n  Espera un poco — esa valla está ahí por algo:\n  \"{}\"",
        art(Mood::Guarding),
        single_line(message, 96)
    )
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
        assert!(out.len() < 500 + GUARDING.len() + 100);
    }
}
