# 09 — Installation and agents

**Fuente de evidencia:** `install.sh` (leído, no re-ejecutado — ya estaba instalado en esta máquina), `--help` del binario compilado, y verificación de que el registro en `~/.claude.json` existe (sin volcar su contenido — es configuración personal del usuario, no evidencia que deba citarse textualmente).

## Observed

- `install.sh` es un script wrapper: valida que la URL de descarga sea HTTPS (o loopback explícito para smoke tests locales, con redirects deshabilitados), descarga el binario desde `https://github.com/DeusData/codebase-memory-mcp/releases/latest/download`, lo coloca en `$HOME/.local/bin` (configurable con `--dir`), y luego **delega la configuración de agentes al propio binario** invocándolo con `install -y --force --dir=<path>`.
- El binario expone `install [-y|-n] [--force] [--dry-run] [--dir=<path>] [--skip-config]` como subcomando propio — la lógica de detección/registro de clientes vive en el binario compilado, no en el script bash.
- **Confirmado en esta máquina:** `~/.claude.json` contiene una entrada de registro para `codebase-memory-mcp` (verificado por coincidencia de patrón, sin volcar el archivo completo — es configuración personal, potencialmente con otras entradas no relacionadas con esta investigación).
- `--help` declara soporte "automático/condicional" para **43 superficies de cliente** nombradas explícitamente (Claude Code, Codex CLI, Gemini CLI, Cursor, Windsurf, VS Code, Zed, Aider, etc.) más un segundo grupo de "manual/UI MCP boundaries" (Qodo, Warp, JetBrains AI/ACP, Replit, GitHub cloud agents, Jules, CodeRabbit) donde la integración requiere pasos manuales.
- El propio `--help` aclara: *"Conditional/explicit targets are changed only when their documented platform, marker, or explicit existing config path is present"* — es decir, no escribe configuración de un cliente que no detecta como presente en la máquina.
- El subcomando `uninstall [-y|-n] [--dry-run]` existe como contraparte simétrica de `install`.
- `--skip-config` en `install` permite instalar el binario sin tocar configuración de ningún agente — separación explícita entre "instalar el binario" e "instalar la integración".

## Claimed

El uso de `--dry-run` en ambos `install` y `uninstall` sugiere una promesa implícita de auditabilidad (poder ver qué cambiaría antes de aplicarlo) — no se ejecutó `--dry-run` en esta sesión para no alterar una instalación ya funcional usada como evidencia en el resto de la epic.

## Verified

- La cadena real de instalación (`install.sh` → descarga → binario `install`) es consistente con lo que efectivamente resultó en un registro funcional en `~/.claude.json` y un binario operativo en `~/.local/bin/codebase-memory-mcp` en esta misma máquina.

## Unknown

- Contenido exacto de lo que `install` escribe en cada uno de los 43 clientes soportados — no se auditó cliente por cliente, solo se confirmó el caso de Claude Code (el cliente activo de esta sesión).
- Comportamiento exacto de `uninstall`: si revierte limpiamente solo lo que `install` agregó, o si puede remover configuración preexistente no relacionada — no probado (ejecutar `uninstall` destruiría la instalación funcional usada como evidencia en toda la epic).
- Si `--dry-run` realmente enumera cada archivo que tocaría, con el mismo nivel de detalle que exige `Rationale_Arquitectura_Conceptual_v0.1.md §24` ("El instalador debe registrar exactamente: binario, config, hooks, agent entries, skills, cache, PATH changes").

## Risk

**Bajo para CBM, informativo para Rationale.** No se detectó comportamiento inseguro (descarga HTTPS forzada, opción de skip-config, dry-run disponible, uninstall simétrico). El riesgo relevante es de diseño futuro: replicar esta superficie de 43+ clientes es un esfuerzo de ingeniería no trivial que Rationale **no debe intentar igualar en la v1** (`Rationale_Arquitectura_Conceptual_v0.1.md §2`: "Compatibilidad perfecta con todos los agentes" está explícitamente fuera de alcance de la arquitectura 0.1).

## Decision impact

- Confirma que el patrón correcto de instalador para Rationale (Fase K, muy posterior) es: **binario primero, configuración de agente como paso separado y auditable (`--dry-run`), con `uninstall` simétrico** — igual que CBM. Este patrón es una referencia de diseño válida a futuro, no una prioridad actual.
- El principio *"changed only when their documented platform, marker, or explicit existing config path is present"* es exactamente el tipo de detección conservadora que evita romper configuración de un cliente no instalado — aplicable al futuro `rationale install-agent` (`Rationale_Arquitectura_Conceptual_v0.1.md §24`).
- No genera ningún cambio de decisión inmediata para Fase A/B de Rationale — este research queda registrado para cuando la Fase K (packaging/distribución) sea relevante, mucho más adelante en el roadmap.

## Reproducir

```bash
cat ~/Desktop/codebase-memory-mcp/install.sh | head -30
./build/c/codebase-memory-mcp install --help 2>&1 || ./build/c/codebase-memory-mcp --help
grep -c "codebase-memory-mcp" ~/.claude.json   # confirma registro sin volcar contenido
```
