//! Framing JSON-RPC 2.0 `Content-Length` — ADR-0007.
//!
//! Verificado empíricamente dos veces contra Codebase Memory real: en el
//! spike de lenguaje (`spikes/language/rust/src/main.rs`, cliente Python de
//! prueba) y en el cliente de producción (`src/providers/codebase_memory.rs`,
//! Fase D). Movido aquí sin cambios de comportamiento para que el servidor
//! MCP de Fase E5 reutilice exactamente el mismo código, no una reescritura.

use serde_json::Value;
use std::io::{Read, Write};

/// Lee un mensaje `Content-Length: N\r\n\r\n<body>` y lo parsea como JSON.
/// Devuelve `None` en EOF o si el mensaje es inválido — nunca bloquea de
/// forma indefinida por sí solo (el caller decide timeouts, si aplica).
pub fn read_message(reader: &mut dyn Read) -> Option<Value> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if reader.read(&mut byte).ok()? == 0 {
            return None;
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8_lossy(&header);
    let length: usize = header_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))?
        .split(':')
        .nth(1)?
        .trim()
        .parse()
        .ok()?;
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// Escribe un mensaje con el mismo framing `Content-Length`.
pub fn write_message(writer: &mut dyn Write, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_string(value).expect("serialize mcp message");
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
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
        write_message(&mut buf, &msg).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let parsed = read_message(&mut cursor).expect("debe parsear el mensaje escrito");
        assert_eq!(parsed, msg);
    }

    #[test]
    fn read_message_returns_none_on_empty_input() {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        assert!(read_message(&mut cursor).is_none());
    }
}
