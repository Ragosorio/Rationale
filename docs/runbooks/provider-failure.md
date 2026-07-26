# Provider failure

Rationale nunca bloquea si el proveedor estructural (`codebase-memory-mcp`) no está disponible — "fail open" (`Arquitectura §13.5`). Este runbook explica cómo se ve la degradación y cómo diagnosticarla.

## Cómo se ve una falla de proveedor

```bash
rationale health
```

```json
{"provider_status":"unreachable","provider_error":"No such file or directory (os error 2)"}
```

o, si el binario existe pero no responde a tiempo:

```json
{"provider_status":"unavailable","provider_coverage":"unknown"}
```

Ningún comando de Rationale falla por esto — `prepare_change` sigue devolviendo el packet completo (constraints, conflictos, riesgos), solo que `resolved_target` queda `null` y aparece una advertencia en `warnings` (`"no se pudo iniciar Codebase Memory: ..."`).

## Diagnóstico

1. **¿Está el binario en el PATH?**

   ```bash
   which codebase-memory-mcp
   ```

2. **¿Responde directamente?** (mismo framing que usa Rationale — ver `docs/research/codebase-memory/11-performance-observations.md` para el script de referencia)

   ```bash
   codebase-memory-mcp --version
   ```

3. **¿El servidor MCP de Rationale usa la sesión persistente correcta?** Si `rationale serve` lleva mucho tiempo corriendo, la sesión al proveedor se estableció una sola vez al arrancar (ADR-0002/ADR-0007) — un problema de proveedor que apareció DESPUÉS de arrancar el servidor no se resuelve solo; hay que reiniciar `rationale serve`.

## Qué nunca hace Rationale ante esta falla

- Nunca inventa un `resolved_target`.
- Nunca trata "el proveedor no respondió" como "el símbolo no existe" (`Rationale_v0.5.md §19.2` — ausencia de evidencia no es evidencia de ausencia).
- Nunca deja de responder — un timeout de proveedor mata el proceso hijo y reporta `Unavailable` dentro de segundos (`providers::codebase_memory::tests::provider_timeout_reports_unavailable_and_kills_process` lo verifica).
