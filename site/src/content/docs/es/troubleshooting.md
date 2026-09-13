---
lang: es
slug: troubleshooting
title: Solución de problemas
description: Diagnostica cobertura del proveedor, registro MCP, memoria ausente, conflictos y el Control Room sin adivinar.
section: Verificar
order: 11
---

## `health` dice que el proveedor no está disponible

Comprueba que `codebase-memory-mcp` esté instalado y en el `PATH`. Rationale
sigue funcionando, pero los bindings de símbolo y la vecindad estructural
pierden cobertura. El packet lo dice en `snapshot` y `warnings`; trata
`unavailable` como desconocido, nunca como completo.

Si Codebase Memory perdió el proyecto que tenía indexado, Rationale detecta el
mapeo obsoleto, lo olvida y lo vuelve a resolver mediante las herramientas
públicas del proveedor en la siguiente llamada.

## El agente no ve las herramientas de Rationale

Ejecuta `rationale install-agent` de nuevo y reinicia el cliente. El registro
es por usuario y usa la ruta absoluta del binario instalado, así las
aplicaciones gráficas funcionan sin el `PATH` de tu shell. Si moviste el
binario, el siguiente `install-agent` migra el registro a la nueva ruta. En
Claude Code, `/rationale-health` combina la herramienta MCP `health` con
`rationale doctor` para mostrar qué funciona y qué está degradado.

## El agente no usa el skill

Comprueba que exista `rationale/SKILL.md` en `.claude/skills/` (Claude Code) o en
`.agents/skills/` (Codex) y reinicia el agente para que vuelva a leer los
skills. Invócalo por nombre para confirmar que carga: `/rationale` en Claude
Code, `$rationale` en Codex. La selección automática es probabilística; el
protocolo en `CLAUDE.md` o `AGENTS.md` sigue aplicando cuando el skill no se
elige. Si `install-agent` informa que el directorio del skill es un enlace
simbólico, lo gestiona otra herramienta y Rationale no lo toca.

## El agente responde en otro idioma

El protocolo y el skill le piden al agente responder en el idioma en que
escribes. Escribe tu petición en el idioma en que quieres la respuesta. Los
identificadores, los comandos y las afirmaciones citadas del canon se mantienen
tal cual a propósito.

## `serve` parece mudo

Es lo esperado al lanzarlo a mano: espera JSON-RPC por stdin y mantiene stdout
limpio. Envía un mensaje JSON por línea.

## Se descartó un candidato

Lee el motivo en la lista `discarded` de `finalize_change`. Los más comunes:

- `transient` o `durability_not_declared` — describía este cambio, no algo que
  sigue siendo cierto;
- `rationale_restates_statement` — el rationale debe dar la causa;
- `no_meaningful_binding` — átalo a un archivo o símbolo que exista;
- `duplicate` — ese conocimiento ya existe bajo el id indicado.

## Un Record no aparece en `prepare_change`

Revisa sus bindings con `rationale doctor --check`: un binding a una ruta que
ya no existe deja el Record obsoleto para ese target, y un Record sin bindings
no puede gobernar nada. Los Records revocados y reemplazados son historia, no
gobierno.

## `finalize_change` devolvió un conflicto

Un candidato intentó reemplazar un Record fijado, así que no se escribió.
Ejecuta `rationale conflicts` para ver las dos afirmaciones y después
`rationale resolve <conflict-id> keep-pinned|adopt-new`. Solo un actor
declarado puede adoptar la afirmación nueva.

## El agente quiere simplificar código raro

Pídele que llame primero a `explain_target`. Una rama extraña puede ser una
valla de Chesterton cuyo motivo está guardado en un Record.

## El Control Room está vacío

Muestra las operaciones y la actividad de este proyecto. Si
`RATIONALE_ACTIVITY=off` estaba activo mientras el agente trabajaba, no hay
nada que mostrar. Si no, ejecuta un `prepare_change` y el grafo aparece en
menos de un segundo.

## `rationale ui` sirve una página de instrucciones

Ese binario se compiló sin la interfaz embebida (un build desde el código sin
`ui/dist`). Los binarios de release siempre la incluyen. Desde el código:
`npm --prefix ui ci && npm --prefix ui run build`, y vuelve a compilar.

## Siguen pendientes propuestas anteriores a 1.0

Ejecuta `rationale migrate --dry-run` y después `rationale migrate`. Las
propuestas válidas se vuelven Records con procedencia `migrated`; las ruidosas
se archivan con su motivo en `.rationale/archive/proposals/`.
