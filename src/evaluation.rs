//! Tiempo — timestamps RFC3339 para todo lo que Rationale escribe.
//!
//! Este módulo contenía también `RunLog`, la telemetría local de Fase D
//! (`.rationale-local/runs/vertical-slice.ndjson`: latencia, revisión,
//! consistencia, proveedor y bytes). vNext la reemplaza por la actividad por
//! sesión (`src/activity.rs`, ADR-0017); aquí queda la base temporal.

use std::time::Duration;

fn unix_now() -> Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
}

/// Timestamp UTC RFC3339 con precisión de segundos — el formato de todo
/// timestamp canónico nuevo (`approved_at`, eventos de lifecycle, Subjects).
///
/// Antes devolvía `epoch:<segundos>`: el comentario lo justificaba como
/// "suficiente para logs locales", pero el mismo valor terminaba dentro del
/// canon versionado (`approved_at: epoch:1785512885`), donde ni el schema
/// (`format: date-time`) ni un lector humano podían interpretarlo. Los
/// valores legados siguen siendo legibles vía `parse_timestamp_millis`.
pub fn now_iso8601() -> String {
    format_rfc3339(unix_now().as_millis() as u64, false)
}

/// Variante con milisegundos para eventos de actividad: varios eventos de
/// una misma operación caen en el mismo segundo y la UI necesita ordenarlos
/// sin depender solo de `seq`, que es por sesión.
pub fn now_rfc3339_millis() -> String {
    format_rfc3339(unix_now().as_millis() as u64, true)
}

/// Fecha civil gregoriana a partir de días Unix (algoritmo `civil_from_days`
/// de Howard Hinnant): eras de 400 años para resolver siglos bisiestos sin
/// una dependencia de fechas.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_epoch + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Inversa exacta de `civil_from_days`.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let month = i64::from(month);
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn format_rfc3339(unix_millis: u64, with_millis: bool) -> String {
    let seconds = unix_millis / 1000;
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let time = seconds % 86_400;
    let (hour, minute, second) = (time / 3600, time / 60 % 60, time % 60);
    if with_millis {
        format!(
            "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{:03}Z",
            unix_millis % 1000
        )
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn digits(text: &str, range: std::ops::Range<usize>) -> Option<u32> {
    let slice = text.get(range)?;
    if slice.bytes().all(|b| b.is_ascii_digit()) {
        slice.parse().ok()
    } else {
        None
    }
}

/// Lectura tolerante de timestamps: `epoch:<segundos>` (el formato que
/// escribieron las betas) o RFC3339 (`Z` o desplazamiento `±HH:MM`,
/// fracción de segundo opcional). Devuelve milisegundos Unix, o `None` si el
/// valor no es ninguno de los dos — nunca adivina un formato distinto.
pub fn parse_timestamp_millis(value: &str) -> Option<i64> {
    let value = value.trim();
    if let Some(seconds) = value.strip_prefix("epoch:") {
        return seconds.parse::<i64>().ok().map(|s| s * 1000);
    }

    let bytes = value.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    if !matches!(bytes[10], b'T' | b't' | b' ') || bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let year = i64::from(digits(value, 0..4)?);
    let month = digits(value, 5..7)?;
    let day = digits(value, 8..10)?;
    let hour = digits(value, 11..13)?;
    let minute = digits(value, 14..16)?;
    let second = digits(value, 17..19)?;
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let mut rest = &value[19..];
    let mut millis = 0i64;
    if let Some(fraction) = rest.strip_prefix('.') {
        let length = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if length == 0 {
            return None;
        }
        let padded = format!("{:0<3}", &fraction[..length.min(3)]);
        millis = padded.parse().ok()?;
        rest = &fraction[length..];
    }

    let offset_minutes = match rest {
        "Z" | "z" => 0,
        offset if offset.len() == 6 && offset.as_bytes()[3] == b':' => {
            let sign = match offset.as_bytes()[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let hours = i64::from(digits(offset, 1..3)?);
            let minutes = i64::from(digits(offset, 4..6)?);
            if hours > 23 || minutes > 59 {
                return None;
            }
            sign * (hours * 60 + minutes)
        }
        _ => return None,
    };

    // Un segundo intercalar (`:60`) se normaliza al último segundo del
    // minuto: el orden relativo se conserva y nadie recibe una fecha inválida.
    let second = i64::from(second.min(59));
    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + i64::from(hour) * 3600 + i64::from(minute) * 60 + second;
    Some((seconds - offset_minutes * 60) * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Referencias calculadas de forma independiente (Python
    /// `datetime.fromtimestamp(s, timezone.utc)`), no con este mismo código.
    const REFERENCES: &[(u64, &str)] = &[
        (0, "1970-01-01T00:00:00Z"),
        (68_169_599, "1972-02-28T23:59:59Z"),
        (951_827_696, "2000-02-29T12:34:56Z"),
        (1_709_210_096, "2024-02-29T12:34:56Z"),
        (1_785_512_885, "2026-07-31T15:48:05Z"),
        (4_107_542_400, "2100-03-01T00:00:00Z"),
        (253_402_300_799, "9999-12-31T23:59:59Z"),
    ];

    #[test]
    fn formats_utc_rfc3339_across_leap_days_and_centuries() {
        for (seconds, expected) in REFERENCES {
            assert_eq!(format_rfc3339(seconds * 1000, false), *expected);
        }
    }

    #[test]
    fn now_is_rfc3339_not_the_legacy_epoch_format() {
        let now = now_iso8601();
        assert!(!now.starts_with("epoch:"), "{now}");
        assert!(now.ends_with('Z'));
        assert!(parse_timestamp_millis(&now).is_some());

        let precise = now_rfc3339_millis();
        assert_eq!(precise.len(), "2026-09-12T00:00:00.000Z".len(), "{precise}");
        assert!(parse_timestamp_millis(&precise).is_some());
    }

    #[test]
    fn parse_roundtrips_every_reference() {
        for (seconds, text) in REFERENCES {
            assert_eq!(parse_timestamp_millis(text), Some(*seconds as i64 * 1000));
        }
        assert_eq!(
            parse_timestamp_millis(&format_rfc3339(1_785_512_885_123, true)),
            Some(1_785_512_885_123)
        );
    }

    /// El canon real conserva `approved_at: epoch:1785512885` escrito por
    /// las betas: debe seguir siendo legible y ordenable.
    #[test]
    fn legacy_epoch_values_remain_readable() {
        assert_eq!(
            parse_timestamp_millis("epoch:1785512885"),
            Some(1_785_512_885_000)
        );
        assert_eq!(
            parse_timestamp_millis("epoch:1785512885"),
            parse_timestamp_millis("2026-07-31T15:48:05Z")
        );
    }

    #[test]
    fn parse_honors_offsets_and_fractions() {
        assert_eq!(
            parse_timestamp_millis("2026-07-31T09:48:05-06:00"),
            parse_timestamp_millis("2026-07-31T15:48:05Z")
        );
        assert_eq!(
            parse_timestamp_millis("2026-07-31T15:48:05.5Z"),
            Some(1_785_512_885_500)
        );
        assert_eq!(
            parse_timestamp_millis("2026-07-31 15:48:05Z"),
            parse_timestamp_millis("2026-07-31T15:48:05Z")
        );
    }

    #[test]
    fn parse_rejects_values_it_cannot_interpret_instead_of_guessing() {
        for invalid in [
            "",
            "epoch:",
            "epoch:abc",
            "2026-02-30T00:00:00Z",
            "2025-02-29T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-07-31T24:00:00Z",
            "2026-07-31T15:48:05",
            "2026-07-31T15:48:05+0600",
            "31/07/2026",
            "2026-07-31T15:48:05.Z",
        ] {
            assert_eq!(parse_timestamp_millis(invalid), None, "{invalid}");
        }
    }
}
