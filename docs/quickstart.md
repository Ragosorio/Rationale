# Quickstart

Cinco minutos, sin haber visto el proyecto antes.

## Qué hace por ti

Tu memoria de código (Codebase Memory u otra) sabe **dónde** está el código y **cómo** se conecta. Rationale sabe **por qué** existe y **para qué** debe seguir existiendo. Juntos evitan que un agente derribe una valla sin saber qué protegía — la [valla de Chesterton](https://es.wikipedia.org/wiki/Valla_de_Chesterton), literal: no quites algo hasta que sepas por qué está ahí.

Es local-first: sin servidor, sin cuenta, sin API de pago. La memoria de código es opcional — sin ella Rationale sigue funcionando con cobertura degradada, nunca bloquea.

## Instalar

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/download/v0.1.0-alpha.6/rationale-installer.sh | sh
```

Esto coloca el binario en `~/.local/bin` (o `$RATIONALE_INSTALL_DIR`) y, si detecta `codex` en el `PATH`, registra el servidor MCP globalmente. Todavía no toca ningún proyecto — eso ocurre en el siguiente paso.

Después de instalar una versión nueva, las siguientes actualizaciones se hacen
con:

```bash
rationale update
```

Dentro del proyecto que quieres gobernar con Rationale:

```bash
rationale init
```

`init` crea `.rationale/` (el canon del proyecto) **y** detecta qué agente de código estás usando (Claude Code, Codex, Cursor) para avisarle cómo y cuándo llamar a Rationale — sin que tengas que configurar nada a mano. Si prefieres hacerlo tú mismo después, usa `rationale init --skip-agent-config` y corre `rationale install-agent` cuando quieras.

## Qué se instaló y dónde vive cada cosa

| Qué | Dónde | Se versiona en Git |
|---|---|---|
| El binario `rationale` | `~/.local/bin/rationale` | No — es una herramienta |
| **El canon del proyecto** | `<tu-proyecto>/.rationale/` — Subjects, Records, propuestas, aprobaciones | **Sí, y es el punto** — se revisa en PR y se comparte con el equipo |
| Instrucciones para tu agente | `CLAUDE.md` / `AGENTS.md` / `.cursor/rules/rationale.mdc`, en un bloque delimitado | Sí, junto con el resto de las instrucciones de ese agente |
| Registro del servidor MCP | `.mcp.json` / `.cursor/mcp.json` por proyecto, o global vía `codex mcp add` | Depende de tu convención para esos archivos |
| Logs locales | `<proyecto>/.rationale-local/` | No — nunca se envía a ningún servicio |

## El flujo real

Le pides a tu agente algo como:

> "Quiero integrar un action que desasigne vendedores, igual que como se asignan hoy."

Sin que lo menciones, el agente (guiado por las instrucciones que `install-agent` escribió):

1. Usa tu memoria de código para encontrar `assignSeller`, sus llamadas, su ubicación.
2. Llama a `prepare_change(target, intent)` de Rationale — que devuelve restricciones críticas conocidas, conflictos con tu intención declarada, la razón por la que `assignSeller` está hecho así, y riesgos ya documentados.

El agente ahora sabe dónde está, cómo funciona, por qué es así y qué no debe romper — sin que tú lo hayas escrito en el prompt. Si en el camino toca código que se ve innecesariamente raro, puede llamar a `explain_target` antes de "simplificarlo".

## Prompt maestro

Para que el flujo sea repetible entre sesiones, pega el [prompt maestro](prompt-master.md)
o su [versión en español](prompt-master.es.md) al inicio de cada conversación.
La landing cambia entre ambos al cambiar de idioma. `rationale install-agent`
escribe por defecto la versión inglesa en `AGENTS.md`, `CLAUDE.md` o la regla de
Cursor; ambas fuentes viven juntas y se actualizan deliberadamente.

## Sin memoria de código

Rationale sigue funcionando. `rationale health` reporta `"provider_status":"unavailable"` y el contexto que entrega tiene menos cobertura (menos candidatos de vinculación automática), pero nunca bloquea ni falla. Instalar [`codebase-memory-mcp`](research/codebase-memory/) en el `PATH` sube esa cobertura; no es un requisito.

## Verificar que quedó bien

```bash
rationale --version
rationale --help
rationale health
```

Debe imprimir JSON con `project_id`, `git_revision` y `provider_status`. Si algo falla, ver [`docs/runbooks/diagnostics.md`](runbooks/diagnostics.md).

## Quitarlo

```bash
rationale uninstall-agent          # revierte solo lo que install-agent escribió en este proyecto
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/latest/download/rationale-uninstall.sh | sh
```

Ninguno de los dos toca `.rationale/` — es tu canon, y borrarlo es una decisión tuya, no algo que un script haga por ti. Ver [`docs/runbooks/uninstall.md`](runbooks/uninstall.md) para qué es seguro borrar y qué nunca conviene borrar sin pensarlo.

## Siguiente paso

Si vas a construir sobre Rationale (no solo usarlo), sigue con el [índice de documentación](README.md), [CONTRIBUTING.md](../CONTRIBUTING.md) y los tres documentos fundacionales listados en el [`README`](../README.md#documentos-fundacionales-leer-en-este-orden) principal.
