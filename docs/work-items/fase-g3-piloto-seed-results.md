# G3.1 — sembrado de decisiones históricas en el piloto

Fecha de ejecución: 2026-07-26. Esta evidencia cubre la primera actividad del
piloto comparativo: convertir decisiones ya documentadas en propuestas de
Rationale sin aprobarlas automáticamente.

## Alcance y fuentes

Se trabajó sobre los clones locales autorizados:

| Proyecto | HEAD | Índice Codebase Memory | Propuestas creadas |
|---|---|---:|---:|
| Monorepo | `8588e14329a39e8f206296a94abcc1b840964de9` | 12.555 nodos / 23.910 aristas | 11 |
| BoostAPI | `3102e5d2b9d65861fe9eb5756a5e34d7aeeae96c` | 8.605 nodos / 27.719 aristas | 18 |

Las fuentes fueron documentación contemporánea y un commit verificable, no una
reconstrucción basada únicamente en memoria:

- BoostAPI: decisiones de assignment, workflows, auth/RBAC, sesiones de
  WhatsApp, pagos, interacciones y tres commits incident-driven.
- Monorepo: auditoría BFF/auth, contrato BoostAPI, RBAC, despliegue, sesión,
  plan OAuth mobile, catálogo, ADR de documentación, sanitización y el commit
  `5287080` sobre límites de mensajes.

La cobertura del grafo es útil para ubicar símbolos y fronteras, pero no se
trata como verdad única. La verificación directa de cada fuente documental fue
la base de las afirmaciones; los índices quedaron en estado `ready` y los
providers reportaron `successful/Complete` en ambos repositorios.

## Resultado de validación

| Proyecto | Archivos YAML | Interfaz usada | Resultado |
|---|---:|---|---|
| BoostAPI | 18 | `rationale review --project-root ... </dev/null` | 18 listadas; EOF saltó todas |
| Monorepo | 11 | `rationale review --project-root ... </dev/null` | 11 listadas; EOF saltó todas |

La CLI leyó las 29 propuestas sin errores de YAML ni de deserialización. Cada
una mostró afirmación, razón, Subject, severidad, actor y autoridad declarada.
No se escribió ningún Record aprobado: `approvals` permanece vacío y la
entrada por EOF conserva todas las propuestas pendientes en `proposals/`.

## Estado de los repositorios

La estructura `.rationale/` ya existía vacía en Monorepo; BoostAPI conserva las
18 propuestas recién creadas. Ningún agente instaló bloques en `CLAUDE.md`,
`.mcp.json` u otros archivos de configuración. Ambos clones quedaron dirty
únicamente por artefactos del piloto (`.rationale/` en BoostAPI y
`.rationale-local/` de ejecuciones previas); no se hicieron commits en esos
repositorios.

## Qué demuestra y qué no demuestra

Demuestra que Rationale puede recibir un lote heterogéneo de decisiones reales
de dos repositorios, mantener su evidencia y presentarlas una por pantalla sin
aprobarlas ni ocultar errores.

Todavía no demuestra recall/precisión del contexto ni autoriza captura asistida.
El siguiente paso de G3 es revisión humana de las 29 propuestas (corregir,
rechazar o aprobar explícitamente). Después se ejecutará la matriz read-only de
20–30 targets con ground truth antes de habilitar mutaciones sobre cambios
reales.

## Gate humano pendiente

El actor resuelto por la CLI fue `user:Roo Rolando Osorio <rosorio@roo.com.gt>`
con autoridad declarada `contributor`. Eso identifica quién revisaría, pero no
es una aprobación. El agente no debe convertir estas propuestas en Records sin
la sesión humana explícita de `rationale review`.
