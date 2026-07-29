---
lang: es
slug: versioning
title: Qué se versiona
description: La frontera práctica entre conocimiento canónico del proyecto, caches derivadas y trazas locales.
section: Verificar
order: 7
---

## Versiona esto

Versiona `.rationale/` con el proyecto: Subjects, Records, evidencia,
propuestas hasta su revisión, schemas, configuración e historial de lifecycle.
Esos archivos son la explicación compartible de por qué el código debe
comportarse de cierta manera.

## Mantén esto local

`.rationale-local/` contiene logs y coordinación local. SQLite/FTS es una cache
derivada que se puede reconstruir. El binario instalado y los manifests de
agentes son artefactos del entorno salvo que tu equipo decida versionarlos.

## Verdad de release

El preview público documentado aquí es `v0.1.0-beta.1`. Los cambios del árbol
de trabajo siguen en Unreleased hasta cortar una release y completar checksums,
instaladores, tests y gates de revisión humana.

## Recuperación segura

Borrar una cache derivada no borra autoridad. Borrar un Record canónico puede
eliminar contexto histórico y debe hacerse mediante revisión Git y el lifecycle
documentado, no con un script de limpieza.
