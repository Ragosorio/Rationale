//! Codecs de transporte JSON-RPC.
//!
//! Codebase Memory conserva el framing histórico `Content-Length`. El
//! servidor de Rationale usa el transporte stdio MCP estándar, con un objeto
//! JSON por línea. Son fronteras distintas y no deben compartir el codec.
//!
//! Límites explícitos (E7 — revisión adversarial de Fase E,
//! `docs/work-items/adversarial-review-fase-e5-e6.md`, hallazgos A1/A2/B):
//! sin cota, un `Content-Length` astronómico dispara `handle_alloc_error`
//! (SIGABRT, no capturable por `catch_unwind`), un header sin terminador
//! crece sin límite, y un body que no parsea como JSON (malformado, o por
//! encima del límite de recursión de `serde_json`) era indistinguible de
//! EOF — terminando la sesión completa en silencio. `read_message` ahora
//! distingue explícitamente EOF real de un mensaje inválido, y rechaza
//! tamaños fuera de cualquier packet real observado sin abortar el proceso.

use serde_json::Value;
use std::io::{Read, Write};

/// Generoso frente a cualquier header MCP real observado (unas pocas
/// decenas de bytes) — protege contra un cliente que nunca completa
/// `\r\n\r\n` (hallazgo A2).
const MAX_HEADER_BYTES: usize = 8 * 1024;

/// Muy por encima del mayor `token_estimate` observado en este repo (unos
/// cientos) — protege contra un `Content-Length` que fuerza una asignación
/// que aborta el proceso (hallazgo A1).
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Resultado de intentar leer un mensaje. Distingue EOF real (el cliente
/// cerró la conexión — fin normal de sesión) de un mensaje inválido (el
/// cliente envió algo que no se pudo interpretar) — el caller decide qué
/// hacer con cada caso; el servidor MCP (`src/mcp/server.rs`) responde con
/// un error JSON-RPC ante `Invalid` en vez de terminar la sesión.
#[derive(Debug)]
pub enum Frame {
    Message(Value),
    Eof,
    Invalid(String),
}

/// Lee un mensaje `Content-Length: N\r\n\r\n<body>`. Nunca bloquea de forma
/// indefinida por sí solo (el caller decide timeouts, si aplica) y nunca
/// asigna memoria sin cota ni dispara un abort del proceso.
pub fn read_content_length(reader: &mut dyn Read) -> Frame {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => return Frame::Eof,
            Ok(_) => {}
            Err(_) => return Frame::Eof,
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
        if header.len() > MAX_HEADER_BYTES {
            return Frame::Invalid(format!(
                "header sin terminador \\r\\n\\r\\n tras {MAX_HEADER_BYTES} bytes"
            ));
        }
    }

    let header_str = String::from_utf8_lossy(&header);
    let length: usize = match header_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
    {
        Some(n) => n,
        None => return Frame::Invalid("falta Content-Length o no es un entero válido".to_string()),
    };

    if length > MAX_BODY_BYTES {
        return Frame::Invalid(format!(
            "Content-Length {length} excede el máximo permitido de {MAX_BODY_BYTES} bytes"
        ));
    }

    let mut body = vec![0u8; length];
    if reader.read_exact(&mut body).is_err() {
        return Frame::Invalid(
            "el body no pudo leerse completo (conexión cerrada a mitad)".to_string(),
        );
    }

    match serde_json::from_slice(&body) {
        Ok(v) => Frame::Message(v),
        Err(e) => Frame::Invalid(format!("body no es JSON válido: {e}")),
    }
}

/// Escribe un mensaje con el framing `Content-Length` de Codebase Memory.
pub fn write_content_length(writer: &mut dyn Write, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_string(value).expect("serialize mcp message");
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    writer.flush()
}

const MAX_STDIO_LINE_BYTES: usize = 16 * 1024 * 1024;

/// Lee un mensaje MCP stdio: un objeto JSON UTF-8 terminado por `\n`.
pub fn read_stdio_message(reader: &mut dyn Read) -> Frame {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match reader.read(&mut byte) {
            Ok(0) if line.is_empty() => return Frame::Eof,
            Ok(0) => return Frame::Invalid("mensaje stdio terminado antes de newline".to_string()),
            Ok(_) if byte[0] == b'\n' => break,
            Ok(_) => {
                line.push(byte[0]);
                if line.len() > MAX_STDIO_LINE_BYTES {
                    while let Ok(1) = reader.read(&mut byte) {
                        if byte[0] == b'\n' {
                            break;
                        }
                    }
                    return Frame::Invalid(format!(
                        "mensaje stdio excede el máximo permitido de {MAX_STDIO_LINE_BYTES} bytes"
                    ));
                }
            }
            Err(e) => return Frame::Invalid(format!("no se pudo leer stdin: {e}")),
        }
    }

    if line.last() == Some(&b'\r') {
        line.pop();
    }
    if line.is_empty() {
        return Frame::Invalid("mensaje stdio vacío".to_string());
    }
    match serde_json::from_slice(&line) {
        Ok(v) => Frame::Message(v),
        Err(e) => Frame::Invalid(format!("mensaje stdio no es JSON válido: {e}")),
    }
}

/// Escribe un mensaje MCP stdio sin contaminar stdout con texto auxiliar.
pub fn write_stdio_message(writer: &mut dyn Write, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_string(value).expect("serialize mcp message");
    writeln!(writer, "{body}")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_then_read_roundtrips() {
        let mut buf: Vec<u8> = Vec::new();
        let msg = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        write_content_length(&mut buf, &msg).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        match read_content_length(&mut cursor) {
            Frame::Message(parsed) => assert_eq!(parsed, msg),
            other => panic!("esperaba Frame::Message, obtuve {other:?}"),
        }
    }

    #[test]
    fn read_message_returns_eof_on_empty_input() {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        assert!(matches!(read_content_length(&mut cursor), Frame::Eof));
    }

    /// E7 hallazgo A1 — un `Content-Length` astronómico nunca debe intentar
    /// asignar esa memoria; debe rechazarse como `Invalid`, no abortar.
    #[test]
    fn content_length_above_max_is_rejected_without_allocating() {
        let mut cursor =
            std::io::Cursor::new(b"Content-Length: 999999999999999999\r\n\r\n".to_vec());
        assert!(matches!(
            read_content_length(&mut cursor),
            Frame::Invalid(_)
        ));
    }

    /// E7 hallazgo A2 — un header que nunca completa `\r\n\r\n` debe
    /// rechazarse tras `MAX_HEADER_BYTES`, no crecer sin límite.
    #[test]
    fn header_without_terminator_is_rejected_after_max_size() {
        let junk = vec![b'X'; MAX_HEADER_BYTES + 1024];
        let mut cursor = std::io::Cursor::new(junk);
        assert!(matches!(
            read_content_length(&mut cursor),
            Frame::Invalid(_)
        ));
    }

    /// E7 hallazgo B — un body que no parsea como JSON debe distinguirse de
    /// EOF; antes ambos colapsaban a `None`.
    #[test]
    fn malformed_json_body_is_invalid_not_eof() {
        let body = b"{not valid json!!!";
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut bytes = header.into_bytes();
        bytes.extend_from_slice(body);
        let mut cursor = std::io::Cursor::new(bytes);
        assert!(matches!(
            read_content_length(&mut cursor),
            Frame::Invalid(_)
        ));
    }

    /// E7 hallazgo B — JSON sintácticamente válido pero por encima del
    /// límite de recursión de `serde_json` también debe ser `Invalid`, no
    /// tumbar la sesión silenciosamente como EOF.
    #[test]
    fn deeply_nested_json_body_is_invalid_not_eof() {
        let depth = 200;
        let body = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut bytes = header.into_bytes();
        bytes.extend_from_slice(body.as_bytes());
        let mut cursor = std::io::Cursor::new(bytes);
        assert!(matches!(
            read_content_length(&mut cursor),
            Frame::Invalid(_)
        ));
    }

    #[test]
    fn stdio_newline_roundtrips() {
        let msg = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let mut buf = Vec::new();
        write_stdio_message(&mut buf, &msg).unwrap();
        assert!(buf.ends_with(b"\n"));
        let mut cursor = std::io::Cursor::new(buf);
        match read_stdio_message(&mut cursor) {
            Frame::Message(parsed) => assert_eq!(parsed, msg),
            other => panic!("esperaba Frame::Message, obtuve {other:?}"),
        }
    }

    #[test]
    fn stdio_malformed_message_is_invalid() {
        let mut cursor = std::io::Cursor::new(b"{not json}\n".to_vec());
        assert!(matches!(read_stdio_message(&mut cursor), Frame::Invalid(_)));
    }
}
