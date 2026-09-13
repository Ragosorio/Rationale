---
lang: es
slug: versioning
title: Qué se versiona
description: El conocimiento canónico en Git, las trazas locales en tu máquina y cómo funcionan las releases y los canales desde 1.0.
section: Verificar
order: 9
---

## Commitea esto

`.rationale/` viaja con el proyecto: `config.yaml` (incluida la autoridad
declarada), `records/`, `subjects/`, `schemas/`, `migrations/` y el archivo de
propuestas migradas. Estos archivos son la explicación compartible de por qué
el código se comporta como lo hace, y se revisan en el mismo pull request que
el código.

Las instrucciones de los agentes (`CLAUDE.md`, `AGENTS.md`, `.cursor/rules/`) y
las skills de Claude Code también son archivos normales del proyecto. Nunca
contienen una ruta personal.

## Mantén esto local

| Ruta | Contenido |
| --- | --- |
| `.rationale-local/activity/` | Un archivo NDJSON por sesión de agente. |
| `.rationale-local/operations/` | Snapshots de contexto por operación. |
| `.rationale-local/conflicts/` | Conflictos pendientes con Records fijados. |
| `.rationale-local/installed-agent-files.json` | Lo que escribió `install-agent`, para revertirlo con exactitud. |
| `~/.cache/rationale/projects/` | Cache SQLite de búsqueda, derivada. |

Rationale añade `.rationale-local/` a `.git/info/exclude` antes de su primera
escritura, así nada de eso aparece en `git status`.

## Releases y canales

Las releases se etiquetan `vMAJOR.MINOR.PATCH` y la versión del binario sale de
ese tag; las pre-releases añaden un sufijo como `-rc.1`. Cada release publica
archivos con checksum para cinco targets, los instaladores y attestations de
procedencia del build.

| Canal | Resuelve a |
| --- | --- |
| `stable` (por defecto) | La última release completa. |
| `preview` | La release más nueva, incluidas las pre-releases (`-rc`, `-alpha`). |

Elige con `RATIONALE_CHANNEL`, tanto al instalar como al ejecutar
`rationale update` (que también usa `stable` por defecto). Actualizar o
desinstalar nunca toca `.rationale/`.

## Recuperación segura

Borrar la cache, los snapshots o la actividad no pierde ninguna decisión.
Borrar un Record canónico elimina historia y pertenece a un commit revisado — o
mejor, revócalo o reemplázalo con `rationale review-record` para que el
lifecycle conserve el motivo.
