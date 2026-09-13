---
lang: es
slug: limits
title: Límites conocidos
description: Lo que Rationale 1.0 deliberadamente no afirma — dicho con claridad para que puedas calibrar tu confianza.
section: Verificar
order: 11
---

## El significado sigue siendo juicio del agente

Rationale prueba qué Records gobiernan un target e informa conflictos léxicos
con la intención, con una pista de polaridad conservadora. No llama a un modelo
remoto para decidir el significado. La pista es ruidosa en ambas direcciones —
puede marcar como `opposed` una regla ajena y pasar por alto una contradicción
directa — por eso cada conflicto se marca como solapamiento no verificado, y
`undetermined` nunca se convierte en un veredicto que bloquea. Los Records
gobernantes sí son fiables; léelos a ellos.

## La estructura es tan buena como el proveedor

Codebase Memory es opcional y falible. Algunos lenguajes resuelven llamadas por
nombre, así que una arista `observed` significa «presente en el índice», no una
garantía semántica. Los estados de las relaciones se derivan en cada llamada y
nunca borran una explicación: `orphaned` y `unknown` piden una revisión, no una
limpieza.

## La captura es autónoma

Los agentes escriben Records sin cola de aprobación. El gate elimina ruido y
duplicados, y la procedencia siempre dice que lo afirmó un agente, pero la
calidad de la memoria sigue dependiendo de que el agente siga el protocolo.
Fija las reglas que no deben moverse y revisa `.rationale/records/` en los pull
requests como si fuera código.

## Un repositorio a la vez

Un proyecto es un repositorio Git. No hay federación entre repositorios, ni
canon alojado, ni cuenta.

## Documentos de gobierno en curso

Algunas decisiones de arquitectura detrás de 1.0 (lifecycle de Records,
exclusión de datos locales, registro por usuario, el stream de actividad) están
registradas como ADRs cuyo estado sigue siendo `proposed` a la espera de una
revisión independiente. El comportamiento está implementado y probado; la
aceptación formal sigue abierta.
