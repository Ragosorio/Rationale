---
lang: es
slug: skill
title: El skill rationale
description: Un Agent Skill que le enseña a tu agente cada operación de Rationale — se carga solo cuando una tarea lo necesita, en la misma carpeta para Claude Code y Codex.
section: Empezar
order: 4
---

## Qué agrega

El [prompt maestro](/es/docs/prompt-master) en `CLAUDE.md` y `AGENTS.md` le dice
al agente *que* debe preparar y capturar. El skill `rationale` le dice *cómo*:
qué campos del packet importan, cómo pronunciarse sobre un Record gobernante,
cómo escribir un candidato que el gate de captura conserve y cuándo detenerse y
dejarte a ti una decisión.

Los agentes comparten una ventana de contexto finita, así que el skill está
construido para revelarse de forma progresiva. Hasta que una tarea lo necesita,
el agente solo ve su descripción. Cuando la tarea coincide, lee un `SKILL.md`
corto que lo dirige al único playbook de la operación en curso.

## Operaciones

| Operación | Cuándo la usa el agente |
| --- | --- |
| `preflight` | Antes de cambiar, mover o borrar código no trivial |
| `explain` | Cuando el código parece redundante o raro, o preguntas por qué existe |
| `capture` | Cuando un cambio está hecho y probado |
| `conflicts` | Cuando un candidato choca con una regla fijada |
| `health` | Cuando faltan herramientas o los resultados se ven degradados |
| `adopt` | Al configurar Rationale y sembrar los primeros Records |
| `maintain` | Cuando los bindings quedan obsoletos o `rationale doctor` reporta hallazgos |

Una tarea normal de código es `preflight`, luego el cambio y después `capture`.

## Instalar

| Método | Dónde queda | Disponibilidad |
| --- | --- | --- |
| `npx skills add Ragosorio/Rationale` | Los directorios de skills de los agentes que detecta | Hoy, desde GitHub |
| `rationale install-agent` | `.claude/skills/rationale/` y `.agents/skills/rationale/` | A partir de la release posterior a v1.0.0 |
| Copia manual | Cualquier directorio de skills que lea tu agente | Copia `skills/rationale/` sin `evals/` |

`install-agent` registra un hash por cada archivo que escribe: un archivo que
editas se conserva al reinstalar y al desinstalar, y un directorio de skill que
otra herramienta creó como enlace simbólico se deja intacto. v1.0.0 instala
cinco atajos de Claude Code y un skill `/rationale-protocol`; la próxima
release retira `/rationale-protocol` en favor de `/rationale`.

## Invocarlo

- **Por su cuenta.** Claude Code y Codex pueden cargar el skill cuando una tarea
  coincide con su descripción: cambiar código no trivial en un repositorio con
  `.rationale/`, preguntar por qué existe un código, terminar un cambio o
  diagnosticar Rationale.
- **Claude Code.** `/rationale`, o `/rationale <operación>`, por ejemplo
  `/rationale capture`.
- **Codex.** `$rationale`, o `$rationale explain src/billing/retry.rs::backoff`.

Los atajos `/rationale-preflight`, `/rationale-explain`, `/rationale-capture`,
`/rationale-conflicts` y `/rationale-health` siguen disponibles para ti. Los
agentes no los eligen por su cuenta, así que el skill es la única entrada para
la selección automática.

## Tu idioma

El skill está escrito en inglés y le pide al agente responder en el idioma en
que escribes. Los nombres de herramientas, los ids de Records, valores como
`pinned`, las rutas, los comandos y las afirmaciones citadas de un Record se
mantienen literales en cualquier idioma.

Los Records nuevos siguen el idioma que ya usa tu canon, para que la detección de
duplicados siga funcionando y todo el canon se lea con una sola voz. Un canon
vacío toma tu idioma.

## Revisa antes de escribir

El skill incluye `scripts/check_candidates.py`, un validador que aplica las
reglas del propio gate de captura antes de `finalize_change`: tipos,
durabilidad, umbrales de texto, rationales que repiten el statement, ruido
mecánico, bindings a archivos reales, duplicados probables y `supersedes` que
terminarían en conflicto.

```bash
python3 .claude/skills/rationale/scripts/check_candidates.py - <<'JSON'
[{"kind": "constraint", "statement": "...", "rationale": "...",
  "durability": "durable", "bindings": ["src/billing/retry.rs::backoff"]}]
JSON
```

El código de salida `0` significa que ningún candidato se descartaría, `2` que
al menos uno sí, y `1` que la entrada no se pudo leer. El gate sigue siendo la
autoridad; un test del repositorio falla si el validador se aparta de él.

## Qué contiene

```text
skills/rationale/
├── SKILL.md              router: idioma, operaciones, el ciclo, invariantes, herramientas
├── agents/openai.yaml    metadatos de Codex y la dependencia MCP
├── references/           un playbook por operación, más Records, packet, CLI y anti-patrones
├── scripts/              check_candidates.py
├── assets/               plantillas de reporte y de candidatos
└── evals/                evals de activación y comportamiento para mantenedores (no se instalan)
```

## Personalizarlo

Las ediciones a archivos instalados sobreviven a `install-agent`, pero un
archivo editado deja de recibir actualizaciones hasta que ejecutas
`rationale install-agent --refresh-skills`. Para añadir las convenciones de tu
equipo, prefiere un skill propio que apunte a las operaciones de Rationale en
las que se apoya, así queda independiente del ciclo de releases de Rationale.

La guía completa, incluida la forma de ejecutar las evals, está en el
repositorio en `docs/user-guide/skills.md`.
