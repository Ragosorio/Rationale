---
lang: es
slug: limits
title: Límites conocidos
description: Lo que Rationale deliberadamente todavía no afirma resolver.
section: Verificar
order: 9
---

## La contradicción semántica sigue siendo juicio del agente

Rationale puede probar que un Record aprobado gobierna el target y reportar
solapamiento léxico y pistas conservadoras de polaridad. No usa un LLM remoto
para decidir significado. El agente debe pronunciarse sobre un conflicto
aparente; `undetermined` nunca se convierte en veredicto bloqueante.

## La cobertura del proveedor puede ser parcial

Codebase Memory es opcional y falible. Un proveedor atrasado o no disponible
puede dejar bindings estructurales sin resolver. El packet expone cobertura y
warnings para que una persona calibre la confianza.

## El proyecto está en pre-1.0

La evidencia pública actual es alfa. El empaquetado multiplataforma, gates del
piloto y comportamiento en repositorios inusuales todavía necesitan más
dogfood. La documentación describe comportamiento probado, no promete que
cada lenguaje o workspace sea indexado igual.

## No es un sistema de aprobación alojado

Rationale es local-first. No requiere cuenta, canon remoto, SaaS ni servicio de
aprobación automática. La autoridad humana permanece en el lifecycle versionado
del proyecto.
