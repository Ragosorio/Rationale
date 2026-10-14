//! Estimación de tokens sin tokenizer — declarada como cota superior.
//!
//! Rationale no tiene el tokenizer de Claude offline. Hasta v1.1 usaba
//! `bytes / 4` sobre el packet solo; el tokenizer actual (Claude 4.7 en
//! adelante) produce ~30% más tokens que los anteriores, y una medición de
//! terceros con `count_tokens` (playcode.io, julio 2026) dio ~3.0 caracteres
//! por token para JSON, ~3.3 para inglés y 2.2–2.45 para código: `bytes / 4`
//! subestimaba entre 1.3× y 1.8×.
//!
//! Esta estimación cuenta los caracteres ASCII a 2.7 por token, recuenta el
//! código fuente a 2.0 y cada carácter no ASCII como un token. Con esas
//! mediciones sobreestima JSON y prosa ~10–25% y código ~10–20%: está pensada
//! para que la respuesta real quede por debajo del presupuesto declarado. No
//! se validó contra `count_tokens` en este repositorio (requiere red y una
//! clave). `eval/mcp_client.py` implementa la misma fórmula.

pub const PROSE_CHARS_PER_TOKEN: f64 = 2.7;
pub const CODE_CHARS_PER_TOKEN: f64 = 2.0;
/// Identificador de la fórmula, servido junto al número para que nadie lo
/// confunda con una medición.
pub const ESTIMATOR: &str = "upper-bound";

fn split(text: &str) -> (usize, usize) {
    let ascii = text.chars().filter(char::is_ascii).count();
    (ascii, text.chars().count() - ascii)
}

/// Tokens estimados de un texto que el agente recibe tal cual. `code` es la
/// parte de ese texto que es código fuente (se recuenta a 2.0).
pub fn estimate(text: &str, code: &str) -> usize {
    let (ascii, other) = split(text);
    let (code_ascii, _) = split(code);
    let base = (ascii as f64 / PROSE_CHARS_PER_TOKEN).ceil() as usize + other;
    let code_extra = (code_ascii as f64 / CODE_CHARS_PER_TOKEN).ceil() as usize
        - (code_ascii as f64 / PROSE_CHARS_PER_TOKEN).floor() as usize;
    base + code_extra
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_estimate_is_above_the_legacy_bytes_over_four_proxy() {
        let text = r#"{"id":"decision.rate-cache-ttl-60s","statement":"Carrier rate quotes are cached for 60 seconds."}"#;
        let legacy = text.len().div_ceil(4);
        assert!(estimate(text, "") > legacy);
    }

    #[test]
    fn code_counts_more_than_prose_and_non_ascii_counts_one_each() {
        let code = "fn compile(input: X) -> Y { input.run() }";
        assert!(estimate(code, code) > estimate(code, ""));
        assert_eq!(estimate("ñññ", ""), 3);
        assert_eq!(estimate("", ""), 0);
    }

    /// Mismo número que la fórmula de `eval/mcp_client.py`.
    #[test]
    fn matches_the_evaluation_harness() {
        assert_eq!(estimate("abcdefghij", ""), 4);
        assert_eq!(estimate("abcdefghij", "abcdefghij"), 4 + 5 - 3);
    }
}
