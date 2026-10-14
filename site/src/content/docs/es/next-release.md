---
lang: es
slug: next-release
title: En la próxima versión
description: Qué cambia la versión posterior a v1.1.0 en el contexto que reciben los agentes, los comandos que mantienen el canon verdadero, y la evidencia.
section: Verificar
order: 12
---

Estos cambios están en `main` y llegan en **la versión posterior a v1.1.0**.
Con v1.1.0, que es lo que el instalador entrega hoy, el ciclo funciona como
describe el resto de esta documentación; nada de esto hace falta para usarlo.

Una prerelease, **v1.2.0-rc.1**, trae estos cambios para quien los quiera ya;
el canal estable sigue en v1.1.0:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Ragosorio/Rationale/releases/download/v1.2.0-rc.1/rationale-installer.sh | RATIONALE_VERSION=v1.2.0-rc.1 sh
```

Actualízala con `RATIONALE_CHANNEL=preview rationale update`; un
`rationale update` sin más vuelve a v1.1.0.

## El contexto que recibe un agente

- **Solo las reglas vigentes son autoridad.** Los Records reemplazados,
  revocados o retirados aparecen en `history` con su estado y su reemplazo, y
  `explain_target(target, include_history=true)` devuelve su texto. Un agente
  nunca recibe una decisión reemplazada junto a la que la reemplazó.
- **El alcance es el que pediste.** Una consulta por símbolo devuelve lo atado
  a ese símbolo más las reglas de todo el archivo; una por archivo, todo lo atado
  en él; una por carpeta, los Records atados dentro, por prioridad. Un símbolo
  nuevo en tu rama queda atado a sí mismo, no a todo su archivo, aunque el índice
  estructural todavía no lo conozca.
- **Cada respuesta dice qué es.** `retrieval.status` es `found`, `none_found`,
  `index_incomplete` o `target_missing`; cada Record dice `why` está ahí. La
  estructura que el árbol de trabajo ya no tiene se descarta como índice
  atrasado.
- **Los invariantes cruzan módulos por llamadas reales.** Una restricción
  crítica, alta o fijada del código que llama tu target llega como restricción
  heredada, con la llamada que la trajo.
- **El presupuesto cubre la respuesta completa.** Lo que no cupo queda en
  `omitted`, con la llamada que lo devuelve. Las reglas que gobiernan nunca se
  recortan.

## Mantener el canon verdadero

```bash
rationale operations                                  # cambios preparados que nadie cerró
rationale doctor --update-baseline && rationale doctor --check   # CI que falla solo ante problemas nuevos
rationale retire src/legacy --reason "Replaced by src/notifications"   # plan; --apply escribe, --undo revierte
rationale rule add constraint "Amounts are integer cents." --why "…" --bind src/billing --pin
```

Los agentes cierran las operaciones con `finalize_change`, o con
`abandon=true` cuando el cambio no ocurrió. En Claude Code, `install-agent`
añade hooks que recuperan el contexto al empezar una sesión, recuerdan antes de
editar un archivo gobernado y piden una vez cerrar una operación que la sesión
dejó abierta; nunca bloquean una edición.

## Evidencia

Rationale ahora incluye una evaluación (`eval/` en el repositorio) que corre
cualquier binario sobre un proyecto inventado que reproduce fallos reportados.
Sobre él, v1.1.0 pasó 1 de 15 escenarios de recuperación y esta versión pasa
los 15; los Records reemplazados o revocados servidos como vigentes bajaron de
7 a 0, y los de otro símbolo de 3 a 0. Son resultados **sintéticos**.

En el propio repositorio de Rationale, con el proveedor estructural real, la
mediana de la respuesta estimada bajó de 3.068 a 2.370 tokens y los targets por
encima del presupuesto por defecto de 10 de 12 a 2 (ambos desbordes declarados
de reglas que gobiernan), sin regresión de latencia en una comparación
intercalada. El registro completo, con lo que no se verificó, está en
`docs/work-items/context-integrity-1.2.md` del repositorio.
