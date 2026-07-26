//! Superficie MCP de Rationale — Fase E5.
//!
//! `framing` es el transporte compartido (ya usado como cliente desde Fase
//! D contra Codebase Memory, ADR-0007); `server` lo usa del lado servidor
//! para que un agente pueda consumir `prepare_change`/`explain_target`/
//! `health`.

pub mod framing;
pub mod server;
