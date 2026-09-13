//! HTTP/1.1 mínimo para `rationale ui`: solo lo que una UI local necesita.
//!
//! Sin dependencias ni runtime async (mismo criterio que ADR-0007 para MCP):
//! una petición por conexión, GET/HEAD, cabecera acotada y validación de
//! `Host` contra DNS rebinding. Todo lo demás se rechaza de forma explícita.

use std::borrow::Cow;
use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

pub const MAX_HEAD_BYTES: usize = 16 * 1024;
pub const IO_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn query_value(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// La conexión se cerró antes de enviar una petición.
    Closed,
    TooLarge,
    Malformed(&'static str),
}

/// Lee la cabecera completa (hasta la línea vacía) sin pasar del techo.
pub fn read_request(stream: &mut impl Read) -> Result<Request, ParseError> {
    let limited = Read::by_ref(stream).take(MAX_HEAD_BYTES as u64 + 1);
    let mut reader = BufReader::new(limited);
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|_| ParseError::Closed)?;
        if read == 0 {
            return Err(if head.is_empty() {
                ParseError::Closed
            } else if head.len() > MAX_HEAD_BYTES {
                ParseError::TooLarge
            } else {
                ParseError::Malformed("cabecera incompleta")
            });
        }
        head.extend_from_slice(&line);
        if head.len() > MAX_HEAD_BYTES {
            return Err(ParseError::TooLarge);
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
    }
    parse_head(&head)
}

pub fn parse_head(head: &[u8]) -> Result<Request, ParseError> {
    let text = std::str::from_utf8(head).map_err(|_| ParseError::Malformed("cabecera no UTF-8"))?;
    let mut lines = text.lines();
    let request_line = lines
        .next()
        .ok_or(ParseError::Malformed("sin línea de petición"))?;
    let mut parts = request_line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(ParseError::Malformed("línea de petición inválida"));
    };
    if !version.starts_with("HTTP/1.") {
        return Err(ParseError::Malformed("versión HTTP no soportada"));
    }
    if !target.starts_with('/') {
        return Err(ParseError::Malformed("el target no es una ruta"));
    }
    let (raw_path, raw_query) = target.split_once('?').unwrap_or((target, ""));
    let path =
        percent_decode(raw_path, false).ok_or(ParseError::Malformed("ruta mal codificada"))?;
    if path
        .split('/')
        .any(|segment| segment == ".." || segment == ".")
        || path.contains('\\')
        || path.contains('\0')
    {
        return Err(ParseError::Malformed("ruta con segmentos no permitidos"));
    }
    let query = raw_query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((percent_decode(key, true)?, percent_decode(value, true)?))
        })
        .collect();
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or(ParseError::Malformed("cabecera sin ':'"))?;
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }
    Ok(Request {
        method: method.to_string(),
        path,
        query,
        headers,
    })
}

fn percent_decode(input: &str, plus_is_space: bool) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = input.get(index + 1..index + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                index += 3;
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Solo nombres de loopback con el puerto real. Una página maliciosa que
/// resuelve su dominio a 127.0.0.1 (DNS rebinding) envía su propio `Host` y
/// se rechaza antes de leer nada.
pub fn host_allowed(host: Option<&str>, port: u16) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.trim().to_ascii_lowercase();
    [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("[::1]:{port}"),
    ]
    .contains(&host)
}

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Cow<'static, [u8]>,
    pub cache_control: &'static str,
    pub extra_headers: Vec<(&'static str, String)>,
}

impl Response {
    pub fn json(value: &serde_json::Value) -> Self {
        Self::json_with_status(200, value)
    }

    pub fn json_with_status(status: u16, value: &serde_json::Value) -> Self {
        Response {
            status,
            content_type: "application/json; charset=utf-8",
            body: Cow::Owned(serde_json::to_vec(value).unwrap_or_default()),
            cache_control: "no-store",
            extra_headers: Vec::new(),
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        Self::json_with_status(status, &serde_json::json!({ "error": message }))
    }

    pub fn html(body: &'static str) -> Self {
        Response {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: Cow::Borrowed(body.as_bytes()),
            cache_control: "no-store",
            extra_headers: Vec::new(),
        }
    }
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        421 => "Misdirected Request",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    }
}

/// Política de contenido de la UI: solo recursos propios; `data:` para
/// texturas generadas y estilos en línea de React.
const CONTENT_SECURITY_POLICY: &str =
    "default-src 'self'; connect-src 'self'; img-src 'self' data:; \
     style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self' data:; \
     frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

pub fn write_response(
    stream: &mut impl Write,
    response: &Response,
    head_only: bool,
) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: {}\r\n\
         X-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n\
         Cross-Origin-Resource-Policy: same-origin\r\nConnection: close\r\n",
        response.status,
        reason_phrase(response.status),
        response.content_type,
        response.body.len(),
        response.cache_control,
    );
    if response.content_type.starts_with("text/html") {
        head.push_str(&format!(
            "Content-Security-Policy: {CONTENT_SECURITY_POLICY}\r\n"
        ));
    }
    for (name, value) in &response.extra_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if !head_only {
        stream.write_all(&response.body)?;
    }
    stream.flush()
}

pub fn write_sse_head(stream: &mut impl Write) -> std::io::Result<()> {
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\n\
          Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
          Connection: keep-alive\r\n\r\nretry: 2000\n\n",
    )?;
    stream.flush()
}

/// Un evento SSE. `data` debe ser una sola línea (JSON serializado).
pub fn sse_event(id: &str, event: &str, data: &str) -> String {
    format!("id: {id}\nevent: {event}\ndata: {data}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_request_line_path_query_and_headers() {
        let request = parse_head(
            b"GET /api/graph?operation=op_1&recent=3&q=a+b%21 HTTP/1.1\r\nHost: 127.0.0.1:9748\r\nAccept: */*\r\n\r\n",
        )
        .unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/graph");
        assert_eq!(request.query_value("operation"), Some("op_1"));
        assert_eq!(request.query_value("q"), Some("a b!"));
        assert_eq!(request.header("host"), Some("127.0.0.1:9748"));
    }

    #[test]
    fn rejects_relative_segments_encoded_or_not() {
        for target in [
            "/../secret",
            "/api/operations/%2E%2E/x",
            "/a/./b",
            "/a%5Cb",
            "/a%00",
        ] {
            let head = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:1\r\n\r\n");
            assert!(
                matches!(parse_head(head.as_bytes()), Err(ParseError::Malformed(_))),
                "{target}"
            );
        }
        assert!(parse_head(b"GET http://evil/ HTTP/1.1\r\n\r\n").is_err());
        assert!(parse_head(b"GET / HTTP/2\r\n\r\n").is_err());
    }

    #[test]
    fn an_oversized_head_is_rejected_without_reading_forever() {
        let mut huge = b"GET / HTTP/1.1\r\nX: ".to_vec();
        huge.extend(std::iter::repeat_n(b'a', MAX_HEAD_BYTES * 2));
        huge.extend_from_slice(b"\r\n\r\n");
        assert_eq!(
            read_request(&mut huge.as_slice()),
            Err(ParseError::TooLarge)
        );
        assert_eq!(read_request(&mut &b""[..]), Err(ParseError::Closed));
    }

    #[test]
    fn only_loopback_hosts_with_the_real_port_are_allowed() {
        assert!(host_allowed(Some("127.0.0.1:9748"), 9748));
        assert!(host_allowed(Some("LOCALHOST:9748"), 9748));
        assert!(host_allowed(Some("[::1]:9748"), 9748));
        assert!(!host_allowed(Some("127.0.0.1:9749"), 9748));
        assert!(!host_allowed(Some("evil.example:9748"), 9748));
        assert!(!host_allowed(Some("localhost"), 9748));
        assert!(!host_allowed(None, 9748));
    }

    #[test]
    fn html_responses_carry_a_restrictive_content_security_policy() {
        let mut out = Vec::new();
        write_response(&mut out, &Response::html("<p>x</p>"), false).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Content-Security-Policy: default-src 'self'"));
        assert!(text.contains("X-Content-Type-Options: nosniff"));
        assert!(text.ends_with("<p>x</p>"));
        let mut head_only = Vec::new();
        write_response(&mut head_only, &Response::error(404, "no"), true).unwrap();
        assert!(String::from_utf8(head_only).unwrap().ends_with("\r\n\r\n"));
    }
}
