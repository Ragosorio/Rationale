# Rationale

## Capa de contexto causal y confianza para agentes de programación

### Documento fundacional, contrato de producto y plan de desarrollo

**Versión conceptual:** 0.5  
**Estado:** diseño semidefinitivo previo a implementación y validación experimental  
**Principio rector:** compilar el conocimiento confiable del proyecto en el contexto mínimo y suficiente que una tarea necesita; medir si mejora resultados reales; primero demostrar valor, después estabilizar la arquitectura y formalizar el protocolo.

---

# 1. Resumen del proyecto


**Rationale** es una herramienta open source, local y consultable que preserva el contexto causal de decisiones importantes de software y lo entrega a agentes de programación antes de que modifiquen las partes relacionadas de un sistema.

Su propósito no es explicar únicamente qué hace el código ni cómo está conectado.

Su propósito es conservar y recuperar información como:

* Por qué existe una sección del código.
* Qué problema motivó su creación.
* Qué comportamiento se intentaba conseguir.
* Qué decisiones se tomaron.
* Qué restricciones deben mantenerse.
* Qué riesgos ya fueron descubiertos.
* Qué alternativas fueron descartadas.
* Cómo se comprobó que el cambio funcionaba.
* Qué partes del sistema dependen de esa decisión.
* Qué cambios futuros podrían volver obsoleto ese conocimiento.
* Quién expresó o aprobó una afirmación.
* Qué autoridad tenía esa persona para establecerla.
* En qué revisión del repositorio fue evaluada.
* Qué calidad y cobertura tenía la evidencia estructural utilizada.

Rationale se integrará inicialmente con **Codebase Memory MCP**, utilizándolo como motor de inteligencia estructural.

Codebase Memory permite descubrir:

* Funciones.
* Clases.
* Rutas.
* Llamadas.
* Dependencias.
* Flujos.
* Símbolos.
* Cambios estructurales.
* Impacto sobre otras zonas del código.
* Relaciones entre paquetes, servicios y repositorios cuando el proveedor posee cobertura suficiente.

Rationale agregará una capa distinta:

* Intención.
* Causalidad.
* Decisiones normativas.
* Restricciones.
* Riesgos.
* Evidencia.
* Procedencia.
* Autoridad.
* Confianza epistemológica.
* Vigencia.
* Consistencia por revisión.
* Consecuencias.

La relación entre ambos puede resumirse así:

> **Codebase Memory comprende la estructura actual del código.  
> Rationale conserva qué decisiones todavía gobiernan esa estructura, por qué existen y por qué deben considerarse confiables.**

Rationale no pretende recordar absolutamente todo.

Su función es recuperar únicamente el conocimiento que:

1. Continúa siendo confiable.
2. Tiene una procedencia conocida.
3. Fue aprobado por la autoridad adecuada cuando es normativo.
4. Sigue siendo aplicable a la revisión actual.
5. Es relevante para la intención y el alcance del cambio.
6. Está respaldado por evidencia cuya cobertura es conocida.
7. Cabe dentro de un presupuesto razonable de contexto.

Por eso, las frases centrales del proyecto serán:

> **Git remembers what changed. Rationale remembers why it still matters.**

> **Rationale does not remember everything. It remembers what still matters.**

La unidad de valor principal no será una base de datos extensa ni un grafo perfecto.

Será un **preflight de decisiones de software**:

> Antes de modificar código, el agente recibe las restricciones, decisiones y riesgos vigentes que gobiernan esa zona, junto con su procedencia, autoridad, evidencia, revisión e incertidumbre.

Rationale debe entenderse además como un **compilador de contexto del proyecto**. No reemplaza el prompt de la tarea ni entrega toda la memoria disponible. Toma una intención cuando existe, unos targets y un snapshot del repositorio, y compila un paquete pequeño con el conocimiento institucional de mayor utilidad para esa operación.

```text
Memoria compartida del proyecto
        +
Intención y síntomas actuales
        +
Estructura observada
        +
Presupuesto de contexto
        ↓
Context packet específico para la tarea
```

El objetivo no es que el agente reciba más texto. El objetivo es que reciba **más contexto relevante, confiable, accionable y vigente por cada token utilizado**.

## 1.1 Validación inicial del problema

El problema es real: los agentes modernos pueden reconstruir estructura, llamadas y dependencias, pero no pueden conocer con certeza motivos que nunca fueron codificados. El propio ecosistema de Codebase Memory ya muestra interés en ADR, historial por símbolo y drift arquitectónico, lo que valida la necesidad; al mismo tiempo, obliga a que Rationale tenga una frontera clara para no duplicar funciones estructurales o históricas que el proveedor pueda incorporar.

La diferenciación de Rationale no será simplemente “guardar ADR más detallados”.

Será la combinación de:

```text
Decisiones normativas
        +
Procedencia y autoridad
        +
Vigencia por revisión
        +
Anclaje estructural
        +
Comparación contra la intención
        +
Entrega compacta antes del cambio
```

## 1.2 Las preguntas fundamentales

```text
¿Qué?       Git: resultado exacto y líneas modificadas.
¿Dónde?     Codebase Memory: ubicación, símbolos y conexiones.
¿Cuándo?    Git: historia; Rationale: vigencia y revisión evaluada.
¿Quién?     Git identifica autores; Rationale identifica procedencia y autoridad.
¿Cómo?      Codebase Memory explica la mecánica actual; el agente diseña la implementación.
¿Por qué?   Rationale conserva el origen causal y la evidencia.
¿Para qué?  Rationale conserva la intención, restricciones e invariantes que deben sobrevivir.
```

En la analogía de construcción:

* Git conserva los ladrillos colocados y la bitácora de obra.
* Codebase Memory conserva los planos actuales de tuberías, cables y conexiones.
* Rationale conserva el estudio de suelo, las decisiones del ingeniero, las restricciones de seguridad y la revisión en la que esas conclusiones fueron comprobadas.

## 1.3 Panorama de soluciones relacionadas

Rationale no parte de un terreno vacío. Existen varias familias de herramientas cercanas:

| Familia o ejemplo | Qué resuelve bien | Hueco que permanece |
|---|---|---|
| Codebase Memory | Grafo estructural, símbolos, llamadas, rutas, impacto y búsqueda local | No debe asumirse que una relación estructural explica la decisión normativa o su autoridad |
| ADR / MADR | Conservación explícita de decisiones arquitectónicas | Normalmente no mantienen bindings por revisión, aplicabilidad actual ni comparación contra una intención concreta |
| PROJECTMEM y memorias basadas en eventos | Eventos, decisiones y puertas previas a acciones | Debe evaluarse cuánto modelan autoridad, evidencia estructural y consistencia exacta con Git |
| Packmind y sistemas de estándares | Distribución de reglas y estándares de ingeniería hacia agentes | Las reglas pueden no conservar el origen causal, la revisión o el enlace con el comportamiento real |
| Hermes Agent y memorias procedimentales | Aprendizaje de hábitos, habilidades y preferencias de un agente | La memoria general del agente no equivale a gobernanza causal portable del repositorio |

Hermes es probablemente la herramienta recordada como “Hermes o Hércules”. Su enfoque puede inspirar captura de hábitos y aprendizaje procedimental, pero Rationale debe evitar convertirse en memoria personal de un agente.

La frontera competitiva de Rationale será:

```text
No recordar cómo trabaja un agente en general.
No volver a indexar el código.
No limitarse a almacenar ADR.

Sí conservar qué decisión aprobada gobierna un comportamiento,
por qué existe, qué evidencia la respalda y en qué revisión aplica.
```

## 1.4 Hipótesis de producto

Hipótesis del problema:

> En repositorios complejos, una parte significativa del riesgo de los agentes proviene de reconstruir correctamente el cómo, pero desconocer una restricción causal o normativa no visible en el código.

Hipótesis de solución:

> Un preflight compacto, enlazado a la estructura actual y limitado a conocimiento aprobado, reduce regresiones y arqueología repetida sin añadir una carga humana excesiva.

Estas hipótesis deben validarse experimentalmente antes de declarar estable la arquitectura o el protocolo.

## 1.5 Decisiones conceptuales de la versión 0.4

La versión 0.4 incorpora cuatro problemas del desarrollo real, pero corrige varias interpretaciones excesivas del feedback recibido.

### Aceptado: el alcance debe cruzar paquetes y workspaces

Una decisión puede originarse en un paquete y gobernar otros. Una regla de autorización implementada en el backend puede afectar el contrato de una API, los componentes que el dashboard permite mostrar y los tests de integración.

Sin embargo, en un monorepositorio no existe necesariamente “una carpeta `.rationale/` por paquete”. Un monorepo continúa siendo un repositorio Git. El problema real es otro:

> **Rationale necesita identidad de proyecto, scopes jerárquicos, bindings calificados por workspace y recuperación capaz de atravesar paquetes sin contaminar el contexto con reglas irrelevantes.**

La v0.1 debe soportar un repositorio Git con múltiples workspaces o paquetes. La federación completa entre repositorios independientes continuará siendo posterior.

### Aceptado con límites: resolución estricta de Subjects

Antes de crear un Subject nuevo, Rationale debe buscar conceptos existentes mediante IDs, aliases, bindings, scope, FTS y, opcionalmente, similitud semántica local.

Los embeddings pueden producir candidatos. Nunca pueden:

* Fusionar Subjects automáticamente.
* Declarar que dos reglas son equivalentes.
* Convertir similitud textual en identidad conceptual.
* Bloquear la creación de un concepto sin una ruta de revisión.

Cuando exista una coincidencia fuerte y el agente proponga un Subject nuevo, deberá entregar una `novelty_reason` explícita.

### Aceptado con límites: detección pasiva de drift

Rationale debe detectar que Git avanzó aunque nadie haya ejecutado `finalize_change`. La garantía mínima no depende de un daemon:

* Cada consulta compara el `HEAD` actual con la última revisión procesada.
* Si no coinciden, degrada bindings y assessments relacionados.
* Nunca sirve silenciosamente una evaluación antigua como actual.

Hooks de Git o un proceso local pueden acelerar esta detección, pero son optimizaciones opcionales. Los hooks pueden omitirse, no se clonan automáticamente y pueden ser saltados. Por eso no son la frontera de corrección.

### Aceptado con límites: no depender de que el agente recuerde una herramienta

MCP por sí solo no puede interceptar universalmente cada edición ni conocer una intención que el agente nunca expresó. La solución será estratificada:

1. **Baseline target context:** al leer, buscar o editar un target, integraciones compatibles pueden inyectar un paquete mínimo con restricciones críticas vinculadas.
2. **Intent-aware preflight:** cuando existe intención explícita, `prepare_change` compara esa intención contra decisiones activas y entrega contexto más rico.
3. **Post-change audit:** al finalizar o revisar un diff, Rationale detecta posibles violaciones aunque el preflight se haya omitido.
4. **Policy enforcement:** CI solo bloquea reglas críticas, deterministas, aprobadas y evaluadas en una revisión coherente.

La inyección automática mejora cobertura, pero no sustituye el contrato explícito de `prepare_change`.

### Corrección importante: más contexto no siempre significa mejor contexto

Una descripción precisa del bug, su reproducción, síntomas y alcance suele mejorar mucho la solución. Pero añadir texto irrelevante o colocar conocimiento crítico dentro de un contexto enorme puede reducir la capacidad del modelo para utilizarlo.

Rationale adopta este principio:

```text
context_utility_density =
    relevance
  × reliability
  × actionability
  × freshness
  ÷ tokens
```

La meta es maximizar utilidad, no volumen.

### Objetivo realista: continuidad senior, no reemplazo de una persona senior

Rationale puede conservar:

* Decisiones anteriores.
* Invariantes.
* Incidentes y bugs históricos.
* Alternativas descartadas.
* Relaciones estructurales.
* Validaciones conocidas.

Eso permite una continuidad parecida a la memoria técnica de una persona senior. No reemplaza:

* Juicio de producto.
* Priorización empresarial.
* Negociación entre equipos.
* Autoridad humana.
* Conocimiento que nunca fue expresado o evidenciado.

La promesa correcta es **preservar memoria institucional accionable**, no fabricar experiencia humana completa.

## 1.6 Decisiones conceptuales de la versión 0.5

La versión 0.5 no altera la identidad central de Rationale. Convierte varias aspiraciones de la 0.4 en hipótesis medibles y añade dos límites operativos necesarios antes de comenzar a construir.

### Aceptado: la densidad de utilidad debe ser falsificable

`context_utility_density` continuará siendo un principio de diseño, pero no se utilizará como una puntuación autorreferencial con la que Rationale se declare exitoso.

Debe evaluarse sobre el **Context Packet exacto** entregado para una tarea y contrastarse contra:

* Un ground truth preparado para el caso.
* El resultado real de la tarea.
* Los tokens totales hasta completarla correctamente.
* Las restricciones respetadas u omitidas.
* Las falsedades incluidas.
* El contexto manual que todavía debió escribir la persona.

Una puntuación alta del paquete no compensa una solución incorrecta.

### Aceptado con límites: `novelty_reason` debe ser estructurada

Una justificación libre como:

```text
Este Subject es diferente.
```

no demuestra novedad conceptual.

Cuando exista un candidato similar, la propuesta debe contrastar explícitamente:

* El Subject existente.
* La diferencia de comportamiento, scope, ciclo de vida, autoridad o invariante.
* La evidencia que respalda esa diferencia.

La herramienta debe rechazar razones genéricas o circulares. Sin embargo, una similitud alta tampoco convierte al candidato en idéntico ni autoriza una fusión automática.

### Aceptado: baseline necesita una ruta rápida separada

El baseline de alta frecuencia no debe ejecutar el pipeline completo de Rationale.

Debe utilizar una ruta local, precomputada, acotada y no bloqueante para recuperar únicamente restricciones críticas y advertencias de consistencia ya indexadas.

El análisis de intención, arqueología, embeddings, clasificación con LLM y reconstrucción de relaciones pertenece al preflight completo, no a cada lectura o búsqueda.

### Corrección: latencia y densidad son propiedades extremas a extremo

No basta con medir cuántos milisegundos tarda una consulta aislada ni cuántos tokens devuelve `prepare_change`.

Rationale puede ser localmente rápido y, aun así, añadir suficientes llamadas o aclaraciones como para empeorar el flujo completo. También puede entregar pocos tokens, pero omitir la única restricción crítica.

Por eso el piloto medirá simultáneamente:

```text
Calidad del Context Packet
        +
Latencia de recuperación
        +
Tokens y llamadas hasta solución correcta
        +
Regresiones y restricciones respetadas
        +
Contexto manual requerido
```

### Condición para avanzar a implementación estable

La 0.5 se considera semidefinitiva en concepto, no validada en resultados.

Antes de estabilizar la arquitectura, el experimento 0.0 debe producir evidencia de que Rationale mejora al menos una combinación material de:

* Éxito de tarea.
* Recuperación de restricciones críticas.
* Prevención de regresiones históricas.
* Tokens totales.
* Tool calls.
* Tiempo humano de preparación del prompt.
* Tiempo hasta una solución correcta.

Si no lo logra, el producto, el retrieval o incluso la hipótesis central deberán revisarse.

---

# 2. El problema

Cuando una inteligencia artificial trabaja sobre un repositorio, puede reconstruir una gran parte de su funcionamiento actual.

Puede descubrir que:

* Una función llama a otra.
* Una ruta modifica una tabla.
* Un controlador valida un permiso.
* Una clase implementa una interfaz.
* Un cambio afecta múltiples símbolos.
* Un servicio depende de otro.

Sin embargo, leer el código actual no garantiza comprender por qué terminó siendo implementado de esa manera.

Una IA puede encontrar este código:

```ts
if (entityAssignment) {
  return resolveEntityPermissions(entityAssignment);
}

return denyAccess();
```

Puede deducir que el acceso depende de una asignación por entidad.

Pero probablemente no podrá deducir con certeza que:

* Antes, los usuarios con acceso a varias entidades recibían `super_admin` global.
* Esto otorgaba privilegios innecesarios a cuentas de soporte.
* Se decidió que tener acceso a varias entidades no debe implicar administración global.
* Algunos usuarios son excepciones deliberadas porque son dueños del proyecto.
* Un usuario sin asignaciones debe quedarse sin acceso.
* Restaurar el rol global reintroduciría el problema original.
* La migración fue diseñada para ser reversible.
* El cambio debía ser idempotente.
* La decisión surgió después de observar un riesgo real de privilegios excesivos.

Ese conocimiento normalmente queda disperso en:

* Conversaciones con agentes.
* Chats internos.
* Pull requests.
* Issues.
* Reuniones.
* Comentarios temporales.
* Mensajes de commits.
* Incidentes.
* Pruebas.
* La memoria de desarrolladores.

Después de varios meses, el código permanece, pero su contexto causal se pierde.

Esto produce el problema conocido como la **Valla de Chesterton**:

> Antes de eliminar o modificar una estructura, es necesario comprender por qué fue creada.

La IA puede entender la valla.

Lo que no necesariamente entiende es por qué alguien decidió construirla.

---

# 3. El verdadero reto


El reto principal de Rationale no es almacenar texto.

Guardar explicaciones en archivos YAML o en una base de datos es relativamente sencillo.

Los problemas difíciles son:

* Saber si una explicación es verdadera.
* Diferenciar hechos, afirmaciones humanas e inferencias.
* Saber quién tenía autoridad para convertir una afirmación en regla.
* Detectar contradicciones entre humanos, equipos y decisiones posteriores.
* Detectar cuándo una decisión dejó de aplicar sin confundir cambio estructural con cambio conceptual.
* Mantener enlaces aunque el código sea refactorizado.
* Evitar alertas constantes.
* No obligar a los desarrolladores a documentar cada cambio.
* Adoptar la herramienta en proyectos antiguos.
* Evitar llenar al agente con demasiado contexto.
* Decidir qué conocimiento importa para una intención concreta.
* Evitar responder con datos construidos sobre revisiones incompatibles.
* No tratar las ausencias o errores del proveedor estructural como verdad.
* Evitar que registros de texto se conviertan en prompt injection.
* Impedir que secretos o información sensible terminen versionados.
* Garantizar que el agente consulte Rationale cuando realmente importa.
* Probar que la herramienta reduce el costo total de resolver una tarea, no solo el tamaño de una respuesta.
* Resolver scopes e herencia en monorepos sin inyectar políticas de paquetes no relacionados.
* Detectar commits humanos y avances de Git aunque el flujo asistido se haya omitido.
* Evitar Subjects duplicados sin entregar la identidad conceptual a una heurística semántica.
* Compartir la memoria canónica por Git mientras cada máquina mantiene índices y cobertura local distintos.
* Entregar restricciones críticas aunque el agente omita el preflight, sin fingir que puede inferirse siempre la intención.
* Diferenciar contexto suficiente de acumulación indiscriminada de contexto.

Existe además una limitación inevitable:

```text
El porqué verdadero no siempre puede deducirse del código.
Los humanos no quieren documentar cada cambio.
Las inferencias automáticas pueden ser falsas.
```

Rationale no puede eliminar esta contradicción. Debe diseñarse alrededor de ella.

La respuesta correcta es aceptar una **cobertura parcial e intencional**:

* Capturar hacia adelante.
* Priorizar áreas de alto riesgo.
* Permitir motivo desconocido.
* Solicitar confirmación solo para deltas normativos importantes.
* Recuperar historia antigua únicamente bajo demanda.
* Nunca presentar cobertura parcial como conocimiento completo.

Por lo tanto, Rationale no debe definirse únicamente como una memoria del porqué.

Debe definirse como:

> **Una capa local de contexto causal, procedencia, autoridad y control de vigencia que conecta decisiones, restricciones, riesgos y evidencia con los comportamientos reales de un sistema, y recupera únicamente aquello que sigue siendo relevante y confiable para un cambio específico.**

El objetivo no es construir una memoria perfecta.

El objetivo es impedir que una decisión importante sea destruida porque el agente solo pudo observar su implementación actual.

---

# 4. Principios fundamentales


## 4.1 No tener una explicación es mejor que conservar una explicación falsa

Una inferencia generada por una IA nunca debe convertirse silenciosamente en un hecho.

Si un agente observa un límite de 50 elementos y deduce que existe por rendimiento, pero el verdadero motivo es una restricción comercial de una API externa, guardar la explicación incorrecta sería más peligroso que admitir que el motivo es desconocido.

Rationale debe poder responder:

```text
Motivo confirmado: desconocido.

Hipótesis:
El límite podría estar relacionado con rendimiento.

Confianza: baja.
No confirmado por una persona ni por evidencia directa.
```

Nunca debe responder:

```text
El límite existe por rendimiento.
```

si eso no fue demostrado.

---

## 4.2 Un cambio estructural no implica un cambio conceptual

Una función puede:

* Cambiar de nombre.
* Moverse de archivo.
* Dividirse.
* Ser extraída a una clase.
* Convertirse en un servicio.
* Reescribirse en otro lenguaje.

Y aun así continuar implementando exactamente la misma decisión.

Rationale debe diferenciar:

```text
El código cambió.
```

de:

```text
La decisión dejó de ser aplicable.
```

No debe marcar una decisión como obsoleta únicamente porque cambió el fingerprint de una función.

---

## 4.3 El conocimiento pertenece primero a un concepto, no a un archivo

La identidad principal de una decisión no debe ser:

```yaml
path: src/auth/authorization.ts
symbol: resolveEntityRole
```

La identidad principal debe ser el comportamiento que representa:

```yaml
subject:
  type: system-behavior
  id: authorization.entity-scoped-staff-access
```

Los archivos, funciones, tablas y rutas serán anclas de la implementación actual.

Esto permite que la decisión sobreviva a refactors y migraciones arquitectónicas.

Sin embargo, un ID conceptual tampoco es mágicamente estable. Rationale debe soportar operaciones explícitas de identidad:

```text
alias
rename
merge
split
scope-narrowed
scope-expanded
supersede
```

La herramienta puede proponer linaje, pero no debe prometer reconstrucción conceptual automática perfecta.

---

## 4.4 La captura debe tener poca fricción

Los desarrolladores no deben llenar formularios gigantes ni aprobar documentos completos después de cada cambio.

Rationale debe obtener automáticamente los datos verificables:

* Archivos modificados.
* Símbolos modificados.
* Dependencias añadidas.
* Dependencias eliminadas.
* Pruebas ejecutadas.
* Commits relacionados.
* Rutas afectadas.
* Cambios de esquema.

La participación humana debe limitarse a confirmar las afirmaciones que una herramienta no puede conocer por sí sola:

* Por qué se hizo el cambio.
* Qué decisión se tomó.
* Qué alternativa fue descartada.
* Qué comportamiento nunca debe romperse.
* Qué riesgo no es visible directamente en el código.
* Quién tiene autoridad para aprobar la regla.

Una pregunta útil:

```text
Detecté una posible restricción normativa:
“El acceso multi-entidad no debe implicar administración global”.

¿Debe conservarse como regla aprobada del sistema?
```

Una pregunta inútil:

```text
¿Deseas documentar este cambio?
```

Siempre debe ser válido responder:

```text
Motivo desconocido.
No crear una restricción.
```

---

## 4.5 El sistema debe adoptarse progresivamente

Rationale no necesita conocer la historia completa de un repositorio para ser útil.

Al instalarlo en un proyecto antiguo debe aceptar:

```text
La razón de gran parte del código todavía es desconocida.

A partir de hoy, los cambios importantes comenzarán a conservar su contexto.

El conocimiento histórico será recuperado únicamente cuando sea necesario.
```

La herramienta debe empezar a generar valor desde el primer cambio nuevo.

El objetivo no es cobertura del 100%.

El objetivo es cobertura suficiente en las áreas donde perder contexto tiene consecuencias graves.

---

## 4.6 La recuperación debe respetar un presupuesto de contexto

Un agente no debe recibir quince registros históricos completos antes de modificar una función central.

Rationale debe construir respuestas priorizadas y limitadas por tokens.

Primero debe entregar:

1. Restricciones críticas aprobadas.
2. Conflictos con la intención actual.
3. Decisión vigente.
4. Riesgos directamente relevantes.
5. Calidad y revisión de la evidencia.
6. Conexiones estructurales principales.

El resto del historial debe mantenerse disponible mediante consultas adicionales.

---

## 4.7 Procedencia y autoridad son dimensiones diferentes

Saber que una afirmación fue expresada por una persona no indica que esa persona tuviera autoridad para convertirla en política.

Rationale debe separar:

```text
Procedencia: quién o qué produjo la afirmación.
Autoridad: qué capacidad tenía para aprobarla dentro de ese dominio.
```

Un desarrollador puede describir correctamente lo que cree que ocurre y aun así no ser el responsable de producto, seguridad o arquitectura que puede establecer la regla.

Una restricción crítica no debe considerarse aprobada únicamente porque tenga `human-confirmed`.

Debe poseer una política de aprobación explícita.

---

## 4.8 Toda respuesta debe pertenecer a una revisión coherente

Rationale nunca debe construir una respuesta aparentemente actual usando:

```text
Git HEAD: revisión C
Índice de Codebase Memory: revisión B
Evaluación de Rationale: revisión A
```

Cada paquete debe declarar:

* Revisión de Git.
* Revisión o generación del proveedor estructural.
* Revisión de los registros.
* Revisión de la última evaluación de aplicabilidad.
* Estado de consistencia.

Si no existe coherencia, la herramienta debe degradar o rechazar la respuesta en lugar de servir contexto plausible pero incorrecto.

---

## 4.9 La evidencia estructural es falible

Codebase Memory es un proveedor valioso, no un oráculo.

Una relación ausente puede significar:

* Que no existe.
* Que el índice está atrasado.
* Que el lenguaje o framework no fue resuelto.
* Que el código usa reflexión o configuración dinámica.
* Que una carpeta fue ignorada.
* Que hubo un error del proveedor.

Rationale debe diferenciar:

```text
No se encontró una relación.
```

de:

```text
Se comprobó que la relación no existe.
```

La segunda afirmación requerirá evidencia mucho más fuerte.

---

## 4.10 El texto almacenado es dato, no instrucción

Los registros, evidencias, ADR, issues y conversaciones pueden contener contenido malicioso o accidentalmente instructivo.

Rationale debe tratarlos como datos no ejecutables.

Nunca debe permitir que un registro modifique:

* Las instrucciones del sistema.
* Los permisos del agente.
* Las políticas de seguridad.
* El alcance de herramientas externas.
* La obligación de proteger secretos.

Las restricciones críticas deben preferir una representación declarativa estructurada y usar el texto libre como explicación, no como programa.

---

## 4.11 Lo sensible debe minimizarse y clasificarse

El porqué puede incluir vulnerabilidades, nombres de clientes, contratos, incidentes, costos o información privada.

Rationale debe aplicar:

* Minimización.
* Referencias externas cuando sea posible.
* Clasificación de sensibilidad.
* Controles de visibilidad.
* Redacción de secretos.
* Políticas de exportación.

No debe copiar conversaciones completas dentro del repositorio por comodidad.

---

## 4.12 La conversación es la interfaz principal, no el único mecanismo

El agente puede olvidar llamar una herramienta MCP.

Por eso Rationale debe ofrecer capas progresivas:

```text
Informativa: consulta voluntaria del agente.
Asistida: instrucciones del cliente para ejecutar preflight.
Revisión: análisis del diff al finalizar.
Política: CI solo para reglas críticas deterministas y aprobadas.
```

La experiencia humana puede vivir principalmente en el chat del IDE, pero los casos críticos necesitan un mecanismo verificable fuera de la buena voluntad del modelo.

---

## 4.13 El ahorro debe medirse de extremo a extremo

Rationale consume llamadas y tokens propios.

No debe prometer que toda consulta será más barata.

El ahorro real aparece cuando evita:

* Leer decenas de archivos.
* Repetir arqueología histórica.
* Proponer una arquitectura incompatible.
* Reintroducir un incidente.
* Ejecutar varios intentos fallidos.

La métrica correcta es el costo total hasta resolver la tarea, no únicamente el tamaño del paquete de contexto.

## 4.14 Rationale compila contexto; no descarga memoria

El prompt del usuario seguirá describiendo la tarea actual:

* Qué se quiere lograr.
* Qué bug se observa.
* Cómo se reproduce.
* Qué comportamiento se esperaba.
* Qué restricciones temporales existen.

Rationale aporta aquello que el usuario no debería tener que repetir:

* Decisiones históricas.
* Reglas de negocio.
* Riesgos conocidos.
* Incidentes anteriores.
* Contratos entre paquetes.
* Validaciones que deben repetirse.

La combinación forma el contexto efectivo. Rationale no convierte un prompt ambiguo en una especificación perfecta.

## 4.15 La identidad del proyecto no es idéntica a la carpeta ni al repositorio

Rationale utilizará una identidad lógica de proyecto. En la primera implementación normalmente corresponderá a un repositorio Git, pero el modelo distinguirá:

```text
Project
└── Repository
    └── Workspace / Package / Service
        └── Component / Domain
            └── Target
```

Esto permite que un Subject de proyecto gobierne varios paquetes, mientras un registro local permanece limitado a un workspace o componente.

## 4.16 La frescura se comprueba activamente en cada frontera de lectura

Un daemon puede fallar y un hook puede no ejecutarse. Por eso, antes de devolver conocimiento como actual, Rationale debe comparar la revisión observada con la revisión evaluada.

La detección pasiva es una mejora de latencia. La comprobación en consulta es la garantía de corrección.

## 4.17 La identidad conceptual se resuelve de forma determinista antes de usar semántica

La creación de Subjects no será una escritura libre del agente. Pasará por un resolvedor que consulta:

1. ID exacto.
2. Alias exactos y normalizados.
3. Bindings compartidos.
4. Parent domain y scope.
5. Coincidencia textual FTS.
6. Similitud semántica opcional.

La similitud solo amplía candidatos. La decisión final queda explícita y auditable.

## 4.18 El preflight posee dos modos

```text
Baseline mode:
Targets conocidos, intención ausente o incompleta.
Entrega restricciones críticas y riesgos directamente vinculados.

Intent-aware mode:
Targets + intención + síntomas o reproducción.
Compara el cambio propuesto contra decisiones y antecedentes.
```

Esto reduce el punto único de fallo del agente olvidadizo sin afirmar que Rationale conoce una intención inexistente.

## 4.19 La memoria compartida y el estado local son capas diferentes

Rationale distinguirá:

```text
Shared canonical layer
- Subjects
- Records
- Binding declarations
- Approvals
- Supersession events
- Portable evidence references

Local derived layer
- Binding resolutions
- Provider coverage
- Assessments recalculables
- FTS / embeddings / caches
- Working-tree overlays

Ephemeral session layer
- Hipótesis
- Descubrimientos temporales
- Intención actual
- Tool traces
```

Un equipo comparte el conocimiento canónico mediante Git. Cada computadora reconstruye la representación estructural y declara su propia cobertura.

---

# 5. Definición exacta del producto


Rationale es una herramienta local, estructurada y consultable que registra el razonamiento consolidado de cambios importantes de software y lo convierte en un **preflight de decisiones** antes de cambios futuros.

Su unidad principal continúa siendo un **Rationale Record**, pero el sistema no debe mezclar en un único objeto mutable todo lo que ocurrió y todo lo que hoy se cree sobre ello.

La v1 diferenciará seis entidades principales:

## 5.1 `Subject`

Representa el comportamiento o dominio conceptual gobernado.

```yaml
id: authorization.entity-scoped-staff-access
type: system-behavior
title: Entity-scoped staff authorization
scope: project
aliases:
  - auth.staff-per-entity
applies_to:
  - workspace:apps/api
  - workspace:apps/dashboard
```

## 5.2 `Record`

Representa una decisión, restricción, riesgo o conocimiento operativo consolidado.

```yaml
id: constraint.no-global-admin-for-staff
kind: constraint
statement: Staff users must not receive global super_admin.
rationale: Multi-entity access must not imply global administration.
severity: critical
scope:
  subjects:
    - authorization.entity-scoped-staff-access
```

Un registro puede contener:

* Problema.
* Intención.
* Decisión.
* Restricción.
* Riesgo.
* Alternativas.
* Non-goals.
* Consecuencias.
* Validaciones.
* Condiciones de revisión.

En la v1, estos elementos serán campos estructurados dentro del registro. Solo se convertirán en nodos independientes cuando exista una consulta real que justifique esa complejidad.

## 5.3 `Binding`

Relaciona el concepto con una implementación esperada o conocida. Debe separar la declaración portable de su resolución local.

Declaración canónica compartida:

```yaml
id: binding.authorization.resolve-entity-role
subject_id: authorization.entity-scoped-staff-access
provider: codebase-memory
structural_id: function:typescript:auth.resolveEntityRole
path_hint: apps/api/src/auth/authorization.ts
scope: package:npm:@boost/api
```

Resolución local derivada:

```yaml
binding_id: binding.authorization.resolve-entity-role
provider_version: 0.9.x
resolved_revision: def456
provider_generation: 184
coverage: complete
status: current
```

La declaración permite compartir el ancla conceptual. La resolución expresa lo que una computadora y una versión concreta del proveedor pudieron comprobar.

## 5.4 `Evidence`

Describe evidencia verificable o referenciada.

```yaml
type: migration
revision: 91ac21f
path: database/migrations/remove_staff_super_admin.sql
content_hash: sha256:...
visibility: repository
```

## 5.5 `Approval`

Describe quién aprobó una afirmación normativa y con qué autoridad.

```yaml
actor: user:security-owner
authority: domain-owner
domain: authorization
approved_at: 2026-07-23
policy: codeowners
```

## 5.6 `Assessment`

Describe una evaluación mutable sobre la relación entre un registro y el sistema actual.

```yaml
record_id: constraint.no-global-admin-for-staff
applicability: active
linkage: current
assessed_revision: def456
provider_generation: 184
assessment_reason: implementation-still-enforces-entity-assignments
```

Esta separación es fundamental:

```text
Record = lo que el proyecto decidió.
Assessment = lo que Rationale puede afirmar hoy sobre su vigencia y enlace.
```

La unidad principal no es:

* Un archivo.
* Un commit.
* Una conversación.
* Un resumen generado por IA.
* Un documento libre.
* Un embedding.
* Una salida de Codebase Memory tomada como verdad absoluta.

El sistema debe responder:

> ¿Qué aprendió el proyecto durante este cambio, quién tenía autoridad para aprobarlo, qué evidencia lo respalda y qué parte de ese aprendizaje continúa gobernando la revisión actual?

## 5.7 `ProjectScope` y referencias calificadas

`ProjectScope` será un value object, no una séptima entidad persistida independiente. Define dónde gobierna un Subject o Record.

Jerarquía inicial:

```text
project
workspace:<path-or-name>
package:<manager>:<name>
service:<id>
domain:<id>
target:<provider>:<structural-id>
```

Ejemplo:

```yaml
project_id: boost
repository_id: git:sha256:...

scope:
  kind: project

applies_to:
  - package:npm:@boost/api
  - package:npm:@boost/dashboard

excludes:
  - package:npm:@boost/legacy-admin
```

Reglas conceptuales:

* Los registros de proyecto pueden heredarse hacia scopes hijos.
* La herencia nunca implica inclusión automática en el paquete de contexto; todavía debe existir relevancia estructural o conflicto con la intención.
* Un registro local no se eleva a proyecto por similitud textual.
* Un binding siempre incluye proyecto, repositorio, workspace o package cuando estén disponibles.
* Las dependencias entre paquetes pueden propagar relevancia, pero deben conservar la ruta que explica por qué el registro fue incluido.
* La federación entre varios repositorios utilizará referencias calificadas en el futuro, sin cambiar la identidad de los Subjects actuales.

## 5.8 Contrato de producto de la v1

Antes del cambio, Rationale debe:

1. Recibir targets, revisión y presupuesto; además intención, síntomas o reproducción cuando existan.
2. Verificar coherencia entre Git, proveedor estructural y evaluaciones.
3. Resolver los sujetos afectados.
4. Recuperar decisiones y restricciones aprobadas.
5. Comparar la intención contra ellas.
6. Entregar un paquete compacto con incertidumbre explícita.

Después del cambio, Rationale debe:

1. Recibir el diff y las validaciones.
2. Guardar hechos mecánicos.
3. Proponer únicamente nuevos deltas normativos.
4. Solicitar confirmaciones concretas.
5. Actualizar bindings y assessments.
6. Conservar supersesiones y linaje sin reescribir la historia.
7. Detectar si Git avanzó fuera del flujo y degradar los assessments afectados.
8. Resolver Subjects existentes antes de permitir crear uno nuevo.
9. Compartir declaraciones canónicas mediante Git sin compartir obligatoriamente índices locales.

---

# 6. Qué preguntas debe responder

## 6.1 Sobre código existente

* ¿Por qué existe esta función?
* ¿Qué problema originó este comportamiento?
* ¿Esta condición es deliberada?
* ¿Qué decisión gobierna este módulo?
* ¿Qué restricciones debo preservar?
* ¿Qué riesgo ya fue descubierto?
* ¿Qué pruebas protegen este comportamiento?
* ¿Qué partes del sistema dependen conceptualmente de esto?
* ¿Qué podría romper aunque el código compile?
* ¿Qué información todavía es desconocida?

## 6.2 Antes de realizar un cambio

* ¿La intención contradice una decisión vigente?
* ¿Estoy intentando restaurar un comportamiento eliminado?
* ¿Qué invariantes debo preservar?
* ¿Qué contratos pueden cambiar?
* ¿Qué riesgos están relacionados con esta intención?
* ¿Qué validaciones deberían repetirse?
* ¿Qué otros componentes están implicados?
* ¿Qué conocimiento es confiable y qué parte es inferida?

## 6.3 Sobre la vigencia del conocimiento

* ¿Esta decisión continúa activa?
* ¿La implementación se movió?
* ¿El enlace con el código se degradó?
* ¿Cambió el comportamiento o solamente la estructura?
* ¿Qué evidencia sigue siendo válida?
* ¿Existe una decisión posterior que la reemplaza?
* ¿Hay registros contradictorios?
* ¿Qué necesita revisión humana?

## 6.4 Sobre el historial

* ¿Qué cambio introdujo esta restricción?
* ¿Qué incidente motivó la decisión?
* ¿Qué alternativas se descartaron?
* ¿Qué intentos anteriores fallaron?
* ¿Qué consecuencia inesperada se descubrió?
* ¿Cuándo se validó por última vez?

---

# 7. Lo que Rationale no es


## 7.1 No es otro indexador de código

Rationale no debe volver a implementar todo lo que Codebase Memory ya resuelve.

No necesita reconstruir por sí solo:

* AST completos.
* Call graphs.
* Resolución de símbolos.
* Búsqueda estructural.
* Impacto de cambios.
* Relaciones entre servicios.
* Búsqueda semántica general de código.
* Historial estructural que el proveedor ya exponga con garantías suficientes.

Estas capacidades se consumirán mediante un adaptador versionado.

---

## 7.2 No es un reemplazo de Git

Git continuará siendo la fuente de verdad sobre:

* Qué cambió.
* Quién lo cambió.
* Cuándo ocurrió.
* Qué revisión contiene el cambio.
* Qué líneas fueron modificadas.

Rationale añadirá significado causal y evaluación de vigencia.

> Git remembers what changed.  
> Rationale remembers why it still matters.

---

## 7.3 No es un almacén de conversaciones

Rationale no debe indexar y recuperar conversaciones completas.

Las conversaciones contienen:

* Ideas temporales.
* Hipótesis incorrectas.
* Repeticiones.
* Confusión.
* Caminos descartados.
* Decisiones que luego cambiaron.
* Datos privados.
* Posibles instrucciones maliciosas.

El sistema debe extraer el aprendizaje consolidado, no preservar todo el ruido.

---

## 7.4 No es solamente un sistema de ADR

Los Architecture Decision Records suelen representar decisiones grandes:

* Elección de base de datos.
* Arquitectura distribuida.
* Estrategia de autenticación.
* Cambio de framework.

Rationale también debe representar decisiones más locales:

* Por qué una función valida una asignación.
* Por qué un límite específico existe.
* Por qué una migración desactiva en lugar de borrar.
* Por qué un usuario es una excepción.
* Por qué un proceso debe ser idempotente.
* Por qué un endpoint rechaza determinado estado.

Un ADR puede ser evidencia o una fuente importada de un Rationale Record, pero no sustituye:

* Procedencia.
* Autoridad.
* Binding por revisión.
* Assessment de aplicabilidad.
* Comparación contra la intención.

---

## 7.5 No es una memoria completa de la organización

La primera versión no intentará modelar:

* Personas como red social completa.
* Reuniones completas.
* Cultura empresarial.
* Clientes.
* Contratos.
* Estrategia.
* Toda la historia de producto.

El dominio inicial será:

> Contexto causal necesario para modificar software de forma segura.

---

## 7.6 No es un sistema autónomo que inventa la historia

Rationale puede:

* Recuperar evidencia.
* Proponer hipótesis.
* Detectar señales.
* Corroborar fuentes.
* Solicitar confirmación.

No puede garantizar que reconstruirá un motivo que nunca fue documentado.

---

## 7.7 No es una capa que siempre reduce tokens

En tareas pequeñas puede agregar costo.

Debe activarse con presupuestos y políticas, y demostrar valor en tareas donde evita lectura extensa, arqueología repetida o errores costosos.

---

## 7.8 No es un protocolo abierto desde el primer commit

La visión final puede convertirse en protocolo, pero primero debe existir:

* Una implementación útil.
* Casos reales.
* Más de un consumidor o proveedor.
* Versionado probado.
* Conformance tests.

La primera entrega será un producto y un modelo de contexto versionado.

El protocolo abierto será una consecuencia del uso, no una declaración inicial.

---

## 7.9 No es un fork interno de Codebase Memory

Rationale no debe:

* Leer tablas internas no públicas de Codebase Memory.
* Importar headers privados.
* Compilarse dentro de su repositorio.
* Depender de IDs sin contrato de estabilidad.
* Asumir que una herramienta o schema interno nunca cambiará.

La integración correcta será mediante una interfaz pública, adaptador y negociación de capacidades.

---

# 8. Modelo conceptual


El modelo conceptual completo continúa siendo útil para comprender el dominio:

```text
Problema
   ↓
Intención
   ↓
Cambio
   ↓
Decisiones
   ↓
Restricciones
   ↓
Comportamientos del sistema
   ↓
Anclas de implementación
   ↓
Validaciones
   ↓
Consecuencias
   ↓
Vigencia, autoridad y confianza
```

Los conceptos de dominio serán:

* `Problem`
* `Intent`
* `Change`
* `Decision`
* `Constraint`
* `Risk`
* `Evidence`
* `Validation`
* `Subject`
* `Binding`
* `Claim`
* `Consequence`
* `Approval`
* `Assessment`

Relaciones conceptuales:

```text
Change       --SOLVES----------> Problem
Change       --HAS_INTENT------> Intent
Change       --MADE_DECISION---> Decision
Decision     --GOVERNS---------> Subject
Constraint   --PROTECTS--------> Subject
Risk         --THREATENS-------> Subject
Subject      --IMPLEMENTED_BY--> Binding
Claim        --SUPPORTED_BY----> Evidence
Claim        --APPROVED_BY-----> Approval
Assessment   --EVALUATES-------> Record
Validation   --VALIDATES-------> Change
Change       --SUPERSEDES------> Change
Decision     --SUPERSEDES------> Decision
Binding      --RELOCATED_TO----> Binding
Subject      --DEPENDS_ON------> Subject
Subject      --ALIASES---------> Subject
Subject      --SPLIT_INTO------> Subject
```

## 8.1 Modelo conceptual completo vs. modelo persistido de la v1

El sistema no necesita materializar todos estos conceptos como nodos desde el inicio.

La persistencia de la v1 utilizará:

```text
Subject
Record
Binding
Evidence
Approval
Assessment
```

`Problem`, `Intent`, `Risk`, `Validation`, `Consequence` y `Change` permanecerán como estructuras internas del `Record` hasta que casos reales demuestren que requieren identidad independiente.

Esto conserva la visión completa sin pagar prematuramente el costo de un grafo de dominio excesivo.

---

# 9. Concept-first, code-anchored


Cada registro debe tener un sujeto conceptual estable.

Ejemplo:

```yaml
subject:
  id: authorization.entity-scoped-staff-access
  type: system-behavior
  title: Entity-scoped access for staff users
  aliases:
    - auth.staff-per-entity
```

Después tendrá múltiples bindings o anclas:

```yaml
bindings:
  - type: symbol
    provider: codebase-memory
    provider_version: 0.9.x
    structural_id: function:typescript:auth.resolveEntityRole
    path_hint: src/auth/authorization.ts
    bound_revision: def456
    provider_generation: 184
    coverage: complete

  - type: database-table
    id: entity_user_roles
    bound_revision: def456

  - type: route
    id: GET /entities/:id
    bound_revision: def456

  - type: migration
    path: database/migrations/remove_staff_super_admin.sql
    revision: 91ac21f

  - type: test
    id: staff_cannot_access_unassigned_entity
    bound_revision: def456

  - type: commit
    revision: 91ac21f
```

Las anclas pueden cambiar.

El concepto debe mantenerse cuando la decisión siga representando el mismo comportamiento.

Si una implementación es dividida:

```yaml
concept_lineage:
  - type: split
    from:
      - authorization.staff-access
    into:
      - authorization.staff-global-role
      - authorization.staff-entity-assignment
```

Si una función se divide sin que el concepto cambie:

```yaml
binding_lineage:
  - type: implementation-split
    from:
      - function:typescript:auth.resolveEntityRole
    to:
      - service:identity.resolveGlobalRole
      - service:entity.resolveAssignment
      - gateway:authorizeEntityRequest
```

Si no puede reconstruirse automáticamente:

```yaml
assessment:
  applicability: active
  linkage: unresolved
  assessed_revision: def456
  requires_review: true
```

La decisión no se pierde solamente porque su implementación se movió.

## 9.1 Identidad conceptual y deduplicación

Dos agentes pueden crear IDs diferentes para el mismo concepto:

```text
authorization.entity-scoped-staff-access
auth.staff-per-entity-permissions
```

Por eso un agente no escribirá directamente un Subject nuevo. `finalize_change` enviará una propuesta al **Subject Resolver**.

Orden obligatorio de resolución:

```text
1. ID o alias exacto.
2. Nombre normalizado dentro del mismo proyecto y domain.
3. Overlap de bindings estructurales.
4. Relación parent/child y compatibilidad de scope.
5. Búsqueda FTS por título, descripción e invariantes.
6. Similitud semántica local, si está habilitada.
7. Revisión de candidatos y decisión explícita.
```

Resultado posible:

```yaml
resolution:
  action: reuse | create | alias | merge_candidate | split_candidate
  selected_subject: authorization.entity-scoped-staff-access
  candidates:
    - id: auth.staff-per-entity-permissions
      signals:
        binding_overlap: 0.92
        lexical_similarity: 0.81
        semantic_similarity: 0.88
        scope_compatible: true
  novelty_reason: null
```

Si el agente decide `create` frente a un candidato de alta similitud, debe explicar una `novelty_reason` concreta:

```text
Este Subject representa el contrato visual del dashboard, no la política de asignación del backend; ambos se relacionan, pero poseen ciclos de vida y autoridades diferentes.
```

Operaciones permitidas:

```text
alias
merge
split
rename
scope-narrowed
scope-expanded
supersede
```

Reglas:

* La similitud no fusiona automáticamente.
* Un overlap de bindings no demuestra identidad conceptual.
* Un merge o split requiere evento auditable.
* Los aliases deben ser únicos dentro del proyecto.
* La base local puede mantener una cola de colisiones conceptuales sin bloquear el trabajo cotidiano.
* Cada operación conserva linaje, actor, evidencia, scope y revisión.

## 9.2 `novelty_reason` estructurada y resistente al bypass

La `novelty_reason` no debe depender solamente de un prompt bien redactado. Debe tener una forma auditable que obligue a comparar el concepto propuesto contra candidatos concretos.

Ejemplo:

```yaml
novelty_reason:
  compared_against:
    - authorization.entity-scoped-staff-access

  difference_types:
    - different_behavior
    - different_scope

  contrast:
    existing: >
      Governs how staff permissions are assigned inside an entity.

    proposed: >
      Governs which owner-level capabilities can cross every entity.

  evidence:
    - type: symbol
      id: auth.resolveOwnerRole

    - type: test
      id: owner_has_global_access

  produced_by:
    type: agent
    name: codex

  review_state: unreviewed
```

Tipos iniciales de diferencia:

```text
different_behavior
different_scope
different_lifecycle
different_authority_domain
different_invariant
existing_subject_is_superseded
existing_subject_is_too_broad
existing_subject_is_too_narrow
```

Una razón se considera insuficiente cuando:

* No identifica qué candidatos fueron comparados.
* Repite que el concepto es nuevo sin expresar una diferencia observable.
* Utiliza frases genéricas como `is different`, `separate concern` o `new functionality` sin contraste.
* Confunde archivos diferentes con conceptos diferentes.
* Confunde wording diferente con identidad diferente.
* No aporta evidencia cuando existe similitud alta de bindings, invariantes o scope.
* Declara una autoridad distinta sin indicar el dominio responsable.

El Subject Resolver debe devolver un resultado explícito:

```yaml
novelty_validation:
  status: accepted | insufficient | requires_review
  candidate_threshold_triggered: true
  missing_fields:
    - contrast.existing
  generic_reason_detected: false
  decision_source: deterministic_rules
```

Reglas de producto:

* Un LLM puede redactar la propuesta, pero no autoaprobar su suficiencia mediante otra explicación libre.
* La validación básica debe ser determinista y basada en schema, candidatos mencionados, contraste y evidencia.
* Una razón insuficiente devuelve los candidatos y solicita corrección; no inventa el Subject por defecto.
* En cambios no críticos puede permitirse continuar con una colisión pendiente para no bloquear el trabajo.
* En dominios críticos, la creación puede requerir revisión humana antes de adquirir autoridad normativa.
* La similitud semántica nunca es prueba de identidad ni motivo suficiente para bloquear permanentemente una creación.

Durante el piloto se medirán:

* Subjects nuevos propuestos.
* Candidatos similares presentados.
* Porcentaje de `novelty_reason` rechazadas por ser genéricas.
* Subjects duplicados detectados posteriormente.
* Reutilizaciones correctas.
* Fusiones o splits que tuvieron que revertirse.
* Tiempo humano añadido por la resolución.

## 9.3 Promesa realista

La v1 promete:

> Mantener bindings explícitos, detectar candidatos de relocalización y conservar linaje aprobado.

La v1 no promete:

> Reconstruir automáticamente identidad conceptual perfecta después de cualquier migración entre repositorios, lenguajes o arquitecturas.

---

# 10. Modelo de confianza, procedencia y autoridad


Rationale debe separar tres preguntas distintas:

```text
Epistemología: ¿cómo se obtuvo esta afirmación?
Procedencia: ¿quién o qué la produjo?
Autoridad: ¿quién podía aprobarla como norma?
```

## 10.1 Hechos mecánicos

Datos verificables automáticamente:

* Este commit modificó el archivo.
* Esta función llama otra función según el proveedor.
* Esta prueba fue ejecutada.
* Esta dependencia fue eliminada.
* Esta ruta cambió.
* Esta tabla fue modificada.

Clasificación:

```yaml
epistemic_status: observed
```

La observación debe registrar proveedor, versión, revisión y cobertura.

## 10.2 Afirmaciones humanas

Información expresada explícitamente por una persona:

```text
El límite de 50 existe porque la API cobra más a partir de 51.
```

Clasificación:

```yaml
epistemic_status: stated
provenance:
  type: human
  actor: user:rolando
```

Esto no significa automáticamente que sea una política aprobada.

## 10.3 Afirmaciones corroboradas

Información respaldada por varias fuentes independientes.

Ejemplo:

* Un issue menciona aislamiento entre entidades.
* Se añade una prueba de acceso cruzado.
* La migración elimina permisos globales.

Clasificación:

```yaml
epistemic_status: corroborated
confidence: 0.88
```

## 10.4 Inferencias

Conclusiones producidas por un agente:

```yaml
epistemic_status: inferred
confidence: 0.54
requires_confirmation: true
```

Una inferencia jamás debe transformarse automáticamente en una decisión aprobada.

## 10.5 Afirmaciones disputadas y desconocidas

```yaml
epistemic_status: disputed
```

```yaml
epistemic_status: unknown
```

El estado `unknown` es una respuesta válida y preferible a una explicación inventada.

## 10.6 Autoridad

Estados mínimos:

```text
unreviewed
approved
policy
revoked
```

Roles de autoridad posibles:

```text
contributor
domain-maintainer
domain-owner
security-owner
product-owner
architecture-owner
repository-policy
```

Ejemplo:

```yaml
authority:
  status: approved
  role: security-owner
  domain: authorization
  approval_policy: codeowners
```

## 10.7 Regla para conocimiento crítico

Una restricción crítica solo puede bloquear si cumple simultáneamente:

```text
kind = constraint
severity = critical
authority ∈ {approved, policy}
applicability = active
linkage = current
revision_consistency = exact
conflict = concrete
```

Nunca se bloqueará por:

* Inferencia.
* Similitud vectorial.
* Binding stale.
* Cobertura estructural desconocida.
* Resumen generado por LLM.
* Ausencia de una relación en el proveedor.

---

# 11. Estructura de una afirmación


```yaml
claim:
  id: claim.staff-global-access-was-unsafe

  statement: >
    Staff users received excessive global privileges.

  epistemic_status: corroborated
  confidence: 0.88

  provenance:
    produced_by:
      type: agent
      name: codex
      version: unknown

    created_at: 2026-07-23T18:20:00Z

  authority:
    status: approved
    role: security-owner
    domain: authorization

    approved_by:
      - actor: user:security-owner
        approved_at: 2026-07-23T19:00:00Z

  evidence:
    - type: source-code
      revision: 91ac21f^
      path: src/auth/authorization.ts
      verified: true
      provider: git

    - type: database-migration
      revision: 91ac21f
      path: database/migrations/remove_staff_super_admin.sql
      verified: true

    - type: human-statement
      content_hash: sha256:...
      verified: true
      visibility: restricted

  sensitivity:
    classification: internal

  scope:
    subjects:
      - authorization.entity-scoped-staff-access
```

Estados epistemológicos posibles:

```text
observed
stated
corroborated
inferred
hypothetical
disputed
unknown
```

Estados de autoridad:

```text
unreviewed
approved
policy
revoked
```

## 11.1 El texto libre nunca es ejecutable

Todo campo textual debe considerarse contenido no confiable.

El paquete final debe envolverlo como dato y nunca permitir que instrucciones contenidas en evidencia o explicaciones alteren el comportamiento del agente.

## 11.2 Representación declarativa de restricciones

Cuando sea posible, una restricción tendrá una forma estructurada:

```yaml
constraint_expression:
  subject: authorization.staff
  predicate: must_not_have
  object: role.global_super_admin
  conditions:
    - actor_type: staff
```

Y una explicación humana separada:

```yaml
rationale: >
  Multi-entity access must not imply global administration.
```

La representación declarativa facilita validación y reduce ambigüedad; la explicación conserva el motivo.

---

# 12. Estado multidimensional


Rationale no debe utilizar un único estado general como `possibly-stale`.

Tampoco debe multiplicar estados hasta que cada combinación sea imposible de entender.

La v1 utilizará cuatro dimensiones mínimas.

## 12.1 Estado epistemológico

Indica qué sabemos sobre la afirmación:

```text
observed
stated
corroborated
inferred
disputed
unknown
```

## 12.2 Autoridad

Indica si la afirmación normativa fue aprobada:

```text
unreviewed
approved
policy
revoked
```

## 12.3 Aplicabilidad

Indica si la decisión continúa gobernando el sistema:

```text
active
superseded
unknown
```

`Suspected drift` no será una aplicabilidad permanente. Será una señal o evaluación pendiente.

## 12.4 Estado del enlace

Indica la calidad de la conexión con la implementación actual:

```text
current
stale
unresolved
```

Los detalles `relocated`, `partially-linked`, `degraded` y `orphaned` pueden mantenerse como razones internas, pero no necesitan convertirse en estados públicos separados en la v1.

## 12.5 Consistencia por revisión

```text
exact
working-tree-overlay
structural-index-behind
assessment-behind
unresolved
```

## 12.6 Atención calculada

La atención no será un estado persistente.

Se calculará:

```text
attention =
    severity
  × authority
  × applicability
  × target_overlap
  × intent_conflict
  × linkage_quality
  × revision_consistency
```

Ejemplo:

```yaml
state:
  epistemic: stated
  authority: approved
  applicability: active
  linkage: current
  revision_consistency: exact
```

Una función pudo haberse movido y el binding haberse reparado. Si la decisión continúa activa, la herramienta no debe generar una alerta solo por el movimiento.

---

# 13. Clasificación de cambios

Antes de invalidar o alertar, Rationale debe clasificar el cambio.

## 13.1 Cambio cosmético

Ejemplos:

* Formato.
* Comentarios.
* Renombre local de variables.
* Reordenamiento.
* Cambio de estilo.

Acción:

```text
No alertar.
No modificar aplicabilidad.
```

## 13.2 Refactor estructural

Ejemplos:

* Extract Method.
* Movimiento de función.
* División de clase.
* Cambio de archivo.
* Conversión de función a servicio.

Acción:

```text
Intentar reconectar las anclas.
Actualizar linkage.
No asumir cambio conceptual.
```

## 13.3 Cambio local de comportamiento

Ejemplos:

* Modificación de una condición.
* Nuevo caso de error.
* Ajuste en validación.
* Cambio de fallback.

Acción:

```text
Revisar restricciones relacionadas.
Mostrar aviso solo si existe riesgo suficiente.
```

## 13.4 Cambio de contrato

Ejemplos:

* Entradas o salidas.
* Respuestas HTTP.
* Persistencia.
* Eventos.
* Autorización.
* Integraciones externas.

Acción:

```text
Marcar decisiones relacionadas para análisis.
Recomendar validaciones.
```

## 13.5 Cambio conceptual

Ejemplos:

```text
Antes: acceso explícito por entidad.
Ahora: acceso global heredado.
```

Acción:

```text
Comparar contra decisiones activas.
Advertir conflicto.
Bloquear únicamente si existe una restricción crítica confirmada.
```

---

# 14. Prevención de fatiga de alertas


Rationale debe evitar que todos los cambios produzcan advertencias.

Principios:

* Los refactors deben procesarse silenciosamente cuando los bindings puedan repararse.
* Los enlaces stale no siempre deben mostrarse.
* Las advertencias repetidas deben agruparse.
* Las alertas deben tener prioridad.
* Solo las restricciones críticas aprobadas pueden producir bloqueos.
* Una inferencia nunca debe bloquear un cambio.
* Un cambio estructural no debe presentarse como violación conceptual.
* Las alertas deben estar relacionadas con la intención o el diff actual.
* La inconsistencia de revisión debe mostrarse antes de cualquier conclusión.
* Una cobertura incompleta no debe convertirse en certeza negativa.

Una advertencia buena sería:

```text
La intención propuesta puede restaurar acceso global para usuarios de soporte.

Esto contradice una restricción crítica aprobada:
El acceso multi-entidad debe resolverse mediante asignaciones por entidad.

Autoridad: security-owner.
Aplicabilidad: activa.
Revisión estructural: exacta.
Evidencia: migración, pruebas y decisión aprobada.
```

Una advertencia mala sería:

```text
El archivo authorization.ts cambió.
Quince memorias pueden estar desactualizadas.
```

## 14.1 Condición única de bloqueo

Un cambio solo puede bloquearse cuando:

1. Existe una restricción estructurada.
2. Su severidad es crítica.
3. Fue aprobada por la autoridad adecuada o declarada por política del repositorio.
4. Continúa activa.
5. El binding corresponde a la revisión actual.
6. La contradicción es concreta y suficientemente determinista.
7. La salida explica cómo resolver o revisar el conflicto.

Si cualquiera de estas condiciones falla, la respuesta será informativa o advisory, nunca blocking.

## 14.2 Presupuesto de interrupción

Además del presupuesto de tokens, el repositorio puede definir un presupuesto de interrupción:

```yaml
alert_policy:
  max_advisories_per_change: 3
  group_repeated_records: true
  suppress_structural_refactors: true
  blocking_requires_exact_revision: true
```

---

# 15. Captura con baja fricción


## 15.1 Lo que Rationale captura automáticamente

* Diff.
* Revisión base y revisión final.
* Estado del working tree.
* Commits.
* Archivos.
* Símbolos.
* Relaciones reportadas por el proveedor.
* Cobertura y versión del proveedor.
* Tests ejecutados.
* Resultados.
* Cambios de esquema.
* Dependencias.
* Rutas.
* Issues y PR vinculados.
* Identidad del agente.
* Momento de creación.

## 15.2 Lo que puede proponer el agente

* Problema aparente.
* Intención.
* Decisiones.
* Riesgos.
* Alternativas.
* Consecuencias.
* Posibles supersesiones.
* Posibles bindings relocalizados.

Estas propuestas deben comenzar como inferencias o candidatos.

## 15.3 Lo que debe confirmar una persona

Solamente las afirmaciones normativas más importantes:

* Motivo principal.
* Decisión.
* Restricción crítica.
* Excepción deliberada.
* Riesgo no visible en el código.
* Autoridad o política de aprobación.
* Supersesión conceptual.

Ejemplo de confirmación:

```text
Detecté estas dos nuevas afirmaciones normativas:

1. El acceso multi-entidad no debe implicar administración global.
2. Los usuarios propietarios son excepciones deliberadas.

¿Las apruebas para el dominio authorization?
```

No se debe pedir al desarrollador que revise todo el YAML.

## 15.4 Señales de captura de alto valor

Rationale no preguntará por todos los cambios.

Activará captura asistida cuando detecte señales como:

* Autorización.
* Pagos.
* Facturación.
* Seguridad.
* Migraciones destructivas.
* Cambios de esquema.
* Excepciones deliberadas.
* Procesos irreversibles.
* Integraciones externas.
* Corrección de incidentes.
* Lenguaje normativo en PR o conversación: `must`, `never`, `because`, `avoid`, `do not`.
* Alternativas descartadas con consecuencias relevantes.

## 15.5 Prevención de confirmación automática

Para reducir el síndrome de “aceptar todo”:

* Una confirmación debe mostrar una sola afirmación por decisión importante.
* Debe incluir el efecto práctico de aprobarla.
* Debe permitir corregir el texto antes de aprobar.
* No debe preseleccionar aprobación para restricciones críticas.
* Debe registrar cuánto tiempo pasó entre propuesta y confirmación como señal de calidad, sin asumir mala fe.
* Puede requerir segunda aprobación en dominios de seguridad o dinero.

## 15.6 Captura desde documentos fundacionales

Rationale puede importar ADR, Markdown o documentación existente.

La importación produce:

```text
Afirmaciones stated o inferred.
Autoridad unreviewed.
Bindings candidatos.
```

Nunca convierte automáticamente un documento completo en restricciones críticas aprobadas.

## 15.7 Detección de cambios fuera del flujo

La primera garantía será barata y síncrona:

```text
current_git_revision != last_processed_revision
        ↓
identify changed paths and targets
        ↓
mark related assessments stale or unknown
        ↓
serve qualified context or request revalidation
```

Opcionalmente, una integración puede ejecutar este paso desde:

* `post-commit`.
* Lifecycle hooks del agente.
* File watcher o daemon local.
* Inicio de sesión del IDE.
* Revisión de PR o CI.

Estas integraciones nunca deben bloquear silenciosamente el commit ni asumir que lograron revalidar el significado. Su tarea inicial es **detectar avance y degradar confianza**, no inventar una nueva evaluación.

## 15.8 Captura compartida sin adopción total del equipo

No todos los desarrolladores necesitan ejecutar Rationale para que el repositorio conserve Records aprobados.

Un colaborador sin la herramienta puede modificar código normalmente. Cuando otra máquina con Rationale consulte el proyecto:

1. Detectará que Git avanzó.
2. Comparará el diff desde la última revisión evaluada.
3. Marcará bindings relacionados como stale.
4. Evitará presentar assessments anteriores como actuales.
5. Podrá proponer revalidación o un nuevo delta normativo.

La utilidad aumenta cuando más miembros capturan decisiones, pero la corrección no puede asumir adopción universal.

---

# 16. Niveles de captura


## Nivel 0 — Git only

Para:

* Formato.
* Renombres.
* Dependencias menores.
* Cambios mecánicos.

No se crea registro.

## Nivel 1 — Intent

Guarda:

* Objetivo.
* Sectores modificados.
* Validación.
* Revisión base/final.

## Nivel 2 — Decision

Agrega:

* Decisión.
* Alternativas.
* Motivo.
* Non-goals.
* Procedencia.

## Nivel 3 — Operational knowledge

Agrega:

* Riesgos.
* Restricciones.
* Rollback.
* Consecuencias.
* Incidentes.
* Sensibilidad.

## Nivel 4 — Critical invariant

Conocimiento que ningún agente debe ignorar:

```text
Un pago no puede procesarse dos veces.
El personal no puede recibir super_admin global.
Una entidad no puede ver datos de otra.
Una migración no puede borrar auditoría.
```

Requisitos adicionales:

* Autoridad aprobada o policy.
* Scope explícito.
* Evidencia.
* Binding actual.
* Regla declarativa cuando sea posible.
* Revisión evaluada.
* Política de supersesión.

El sistema puede recomendar un nivel según:

* Autorización.
* Pagos.
* Seguridad.
* Migraciones.
* Infraestructura.
* Cambios de esquema.
* Número de archivos.
* Reversibilidad.
* Incidentes.
* Impacto estructural.
* Sensibilidad de datos.

## Nivel 5 — Repository policy

Reservado para reglas deterministas aprobadas como política del repositorio.

Ejemplos:

* Ninguna migración puede borrar la tabla de auditoría.
* Los endpoints públicos deben exigir rate limiting.
* Los cambios en pagos requieren dos aprobaciones.

Este nivel puede integrarse con CI, pero no debe contener reglas ambiguas dependientes de interpretación libre del LLM.

---

# 17. Cold start y proyectos legacy


Rationale no debe intentar reconstruir automáticamente toda la historia de un monolito antiguo.

Debe utilizar una estrategia progresiva.

## 17.1 Captura hacia adelante

Desde la instalación:

* Los nuevos cambios importantes se registran.
* Las nuevas decisiones se enlazan.
* Las nuevas restricciones quedan disponibles.
* Las revisiones y cobertura se registran desde el inicio.

## 17.2 Arqueología bajo demanda

Cuando se modifica una zona antigua sin contexto:

```bash
rationale investigate src/auth/authorization.ts
```

El sistema puede analizar:

* Git blame.
* Commits.
* Pull requests.
* Issues.
* ADR.
* Tests.
* Migraciones.
* Comentarios.
* Versiones anteriores.
* Documentación.
* Historial por símbolo si el proveedor lo expone.

Resultado:

```text
Motivo confirmado: desconocido.

Evidencia encontrada:
- La condición apareció en el commit 83af12.
- El commit referencia AUTH-184.
- El issue menciona accesos cruzados entre entidades.
- Se añadió una prueba de aislamiento en el mismo cambio.

Hipótesis corroborada:
Esta condición probablemente protege el aislamiento de entidades.

Confianza: media-alta.
Autoridad: no revisada.
Requiere confirmación humana: sí.
```

## 17.3 Límites de la arqueología

La herramienta debe declarar:

* Si el clon es shallow.
* Hasta qué revisión existe historia.
* Qué fuentes no estaban disponibles.
* Si PR o issues no pudieron consultarse.
* Si el símbolo cambió de rango.
* Si la evidencia es temporalmente incompleta.

La ausencia de evidencia no es evidencia de ausencia.

## 17.4 Priorización

No todo el repositorio necesita contexto causal.

Prioridad inicial:

* Autorización.
* Pagos.
* Facturación.
* Seguridad.
* Migraciones.
* Datos.
* Integraciones externas.
* Sincronización.
* Procesos irreversibles.

## 17.5 Cobertura

Rationale puede mostrar:

```text
Authorization      High coverage
Payments           Partial coverage
Billing            Partial coverage
Notifications      No coverage
UI components      Coverage unnecessary
```

Debe distinguir entre:

```text
No coverage
Unknown coverage
Provider gap
Coverage unnecessary
```

El objetivo no es alcanzar 100%.

El objetivo es cubrir las áreas donde perder contexto produce consecuencias graves.

---

# 18. Recuperación con presupuesto


Cada consulta tendrá un presupuesto explícito.

```json
{
  "mode": "intent-aware",
  "max_tokens": 900,
  "max_critical_constraints": 5,
  "max_decisions": 3,
  "max_risks": 3,
  "include_history": false,
  "require_exact_revision": true,
  "scope": "auto"
}
```

## 18.0 Entradas del compilador de contexto

La calidad del paquete depende de combinar tres fuentes diferentes:

```yaml
task_context:
  intent: Fix duplicate invoice generation
  symptoms:
    - Two invoices appear after retrying a timed-out request
  reproduction:
    - Trigger checkout
    - Interrupt response after payment succeeds
    - Retry checkout
  expected_behavior: A payment creates at most one invoice

target_context:
  paths:
    - apps/api/src/billing/checkout.ts
  symbols:
    - billing.finalizeCheckout

context_budget:
  max_tokens: 900
```

El usuario o agente sigue proporcionando la realidad inmediata de la tarea. Rationale recupera la realidad durable del proyecto.

## 18.1 Orden de prioridad

### Nivel 0 — Salud y consistencia

```text
Git revision: def456
Structural revision: def456
Assessment revision: def456
Consistency: exact
Coverage: complete for requested targets
```

Si este nivel falla, la salida debe declararlo antes de cualquier conclusión.

### Nivel 1 — Restricciones críticas aprobadas

```text
CRITICAL

- Staff users must never receive global super_admin.
- Cross-entity access requires an explicit assignment.
```

### Nivel 2 — Conflictos con la intención

```text
Your proposed change may recreate a previously removed authorization path.
```

### Nivel 3 — Razón principal

```text
This behavior was introduced after staff accounts received excessive global privileges.
```

### Nivel 4 — Riesgos relevantes

```text
Users without valid entity assignments may lose access.
```

### Nivel 5 — Estructura

```text
Affected:
- resolveEntityRole
- authorizeRequest
- entity_user_roles
- 3 tests
```

### Nivel 6 — Historia expandible

```text
4 additional historical records available.
Use trace_rationale for details.
```

## 18.2 Progressive disclosure

La respuesta inicial debe ser suficiente para actuar con seguridad, no para contar toda la historia.

El agente puede expandir:

* Evidencia.
* Alternativas descartadas.
* Historial de supersesiones.
* PR e issues.
* Bindings secundarios.

## 18.3 Activación adaptativa

Para cambios triviales, Rationale puede devolver un paquete mínimo:

```text
No approved constraints found for this target.
Structural revision is current.
No additional context required.
```

El costo del preflight debe adaptarse al riesgo.

---

# 19. Selección de relevancia


No basta con encontrar registros enlazados al mismo símbolo.

Rationale debe evaluar:

```text
relevance =
    exact_binding_overlap
  + conceptual_scope_overlap
  + intent_conflict
  + constraint_severity
  + authority_strength
  + evidence_quality
  + behavioral_impact
  + current_applicability
  + revision_consistency
  - redundancy
  - historical_distance
  - provider_uncertainty
```

La intención cambia la respuesta.

## Renombre

```text
Intent: Rename resolveEntityRole.
```

Respuesta:

```text
The symbol is connected to an active authorization decision.
The proposed rename does not appear to change its behavior.
Binding repair can be performed silently after the change.
```

## Cambio conceptual

```text
Intent: Allow support users to access all entities automatically.
```

Respuesta:

```text
Warning: this intent conflicts with a critical approved constraint.

Multi-entity access must not imply global administration.
```

## 19.1 Recuperación determinista antes de semántica

Orden recomendado:

1. Binding exacto.
2. Vecindad estructural.
3. Scope conceptual.
4. Restricciones críticas.
5. Aplicabilidad y autoridad.
6. Búsqueda textual FTS.
7. Embeddings como fallback futuro.

Las restricciones críticas no deben recuperarse únicamente porque su texto se parece semánticamente a la consulta.

## 19.2 Deducción negativa prohibida

La respuesta no debe inferir:

```text
No existe una decisión.
```

solo porque no encontró un binding.

Debe responder:

```text
No se encontró una decisión vinculada dentro de la cobertura disponible.
```

## 19.3 Recuperación consciente de workspaces

En un monorepo, el ranking debe considerar:

```text
workspace_overlap
package_dependency_path
contract_relationship
subject_scope_inheritance
explicit_applies_to
explicit_excludes
```

Ejemplo:

```text
Task target: apps/dashboard/src/users/RoleBadge.tsx

Included backend record:
constraint.no-global-admin-for-staff

Reason for inclusion:
RoleBadge renders the authorization contract exported by @boost/auth-contracts,
which is governed by the project-level staff authorization Subject.
```

No se incluirán todas las reglas del backend por estar en el mismo monorepo. Cada inclusión cruzada debe poder explicar su camino de relevancia.

---

# 20. Integración con Codebase Memory


La arquitectura principal será:

```text
┌─────────────────────────────────────┐
│        Agente de programación       │
└──────────────────┬──────────────────┘
                   │
                   │ prepare_change
                   ▼
┌─────────────────────────────────────┐
│            Rationale MCP            │
│                                     │
│ MCP façade                          │
│ Policy / Trust evaluator            │
│ Retrieval engine                    │
│ Lifecycle service                   │
│ Revision coordinator                │
└─────────────┬─────────────┬─────────┘
              │             │
              ▼             ▼
┌───────────────────┐  ┌────────────────────┐
│ CBM adapter       │  │ Git / Record Store │
│                   │  │                    │
│ Capabilities      │  │ Records            │
│ Coverage          │  │ Evidence           │
│ Revision          │  │ Approvals          │
│ Relationships     │  │ Assessments        │
└─────────────┬─────┘  └────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│ Codebase Memory daemon / MCP / CLI  │
└─────────────────────────────────────┘
```

Rationale debe consultar internamente Codebase Memory.

El agente no debería necesitar coordinar manualmente ambas herramientas.

Flujo:

```text
Agente → Rationale → Codebase Memory
```

## 20.1 Frontera de responsabilidad

### Codebase Memory

* Estructura actual.
* Símbolos.
* Llamadas.
* Dependencias.
* Rutas.
* Impacto estructural.
* Búsqueda y relaciones que su cobertura permita.
* Historial estructural cuando exista una API pública adecuada.

### Rationale

* Decisiones normativas.
* Restricciones.
* Intención.
* Procedencia.
* Autoridad.
* Evidencia causal.
* Assessment de aplicabilidad.
* Comparación contra la intención.
* Presupuesto de contexto.
* Política de bloqueo.

## 20.2 Codebase Memory ya se mueve hacia ADR e historia

La integración debe asumir que Codebase Memory puede incorporar nuevas capacidades como ADR, historial por símbolo o drift.

Rationale no debe competir duplicando esas superficies.

Debe consumirlas como evidencia y conservar su diferencial:

> Autoridad, aplicabilidad, consistencia por revisión y preflight de intención.

## 20.3 Proveedor falible

Cada consulta al proveedor debe devolver o registrar:

```yaml
provider:
  name: codebase-memory
  version: 0.9.x
  indexed_revision: def456
  generation: 184
  coverage: complete
  status: successful
  capabilities:
    - resolve_target
    - relationships
    - impact
```

Si la consulta falla o su cobertura es incompleta, Rationale debe degradar su conclusión.

## 20.4 No acoplamiento interno

Rationale no leerá directamente la base SQLite interna de Codebase Memory ni dependerá de detalles privados.

Usará:

* MCP público.
* CLI documentada cuando sea necesario.
* Adaptador versionado.
* Negotiación de capacidades.
* Pruebas contractuales.

## 20.5 Latencia

La latencia no debe resolverse con arquitectura distribuida prematura.

Medidas:

* Procesos locales.
* Caché por revisión.
* Consulta única coordinada desde Rationale.
* Resultados pequeños.
* Timeouts.
* Evitar múltiples tool calls del agente.
* Invalidación por generación del proveedor.

El agente debe hacer una llamada principal; Rationale coordina las dependencias internamente.

### 20.5.1 Dos rutas de ejecución

Rationale debe separar dos perfiles de latencia.

#### Fast path baseline

Se utiliza en superficies de alta frecuencia como lectura, búsqueda, inicio de edición o navegación.

Debe:

* Leer únicamente almacenamiento local.
* Utilizar bindings y scopes ya resueltos.
* Consultar un índice por revisión o generación.
* Entregar solo constraints críticas y warnings de consistencia.
* Evitar embeddings.
* Evitar llamadas a un LLM.
* Evitar arqueología e historial profundo.
* Evitar reconstruir el grafo.
* Evitar múltiples llamadas MCP encadenadas cuando el cache sea válido.
* Terminar sin salida cuando no exista contexto de alta prioridad.

Clave conceptual de cache:

```text
project_id
+ git_revision_or_worktree_generation
+ target_identity
+ provider_generation
+ policy_generation
+ baseline_budget
```

La clave concreta pertenece al documento de arquitectura, pero la consistencia por revisión no es opcional.

#### Full path intent-aware

Se utiliza cuando existe una intención, síntomas, reproducción, resultado esperado o un cambio que necesita análisis profundo.

Puede:

* Consultar impacto estructural.
* Resolver relaciones cross-workspace.
* Comparar intención contra decisiones.
* Evaluar conflictos y aplicabilidad.
* Buscar mediante FTS.
* Utilizar embeddings locales como fallback de candidatos.
* Consultar historial o evidencia adicional.
* Construir un paquete más rico dentro del presupuesto solicitado.

Esta ruta ocurre en fronteras de cambio, no antes de cada operación de lectura.

### 20.5.2 Presupuestos experimentales de latencia

Los siguientes valores son objetivos iniciales para el piloto, no garantías públicas definitivas:

```text
Warm baseline:
P50 <= 50 ms
P95 <= 150 ms
hard deadline <= 250 ms

Cold baseline:
medir por separado; nunca ocultarlo dentro de la distribución warm

Intent-aware preflight:
medir por complejidad, cache state y proveedor
```

Si el baseline excede su deadline:

```text
fail open
no bloquear la operación del agente
eventualmente devolver ningún contexto
registrar telemetría local de timeout
no presentar resultados parciales como completos
```

`fail open` no significa declarar que no existen restricciones. Significa que la operación original continúa y la falta de contexto queda registrada como una degradación observable.

### 20.5.3 Qué debe medirse

Por cada ejecución:

* Latencia total.
* Tiempo de apertura del índice.
* Tiempo de lookup.
* Cache hit o miss.
* Cold o warm start.
* Número de llamadas al proveedor.
* Timeout.
* Tamaño del paquete.
* Revisión y generación utilizadas.
* Contexto entregado, omitido o degradado.

La latencia deberá analizarse junto al costo extremo a extremo. Ahorrar 100 milisegundos en el preflight no justifica aumentar varios minutos la resolución de la tarea, y añadir 300 milisegundos puede ser aceptable si evita una regresión crítica; la política exacta dependerá de la superficie y severidad.

## 20.6 Hallazgos concretos del repositorio de Codebase Memory

La revisión previa del repositorio confirmó varios puntos relevantes para el diseño:

* El núcleo está implementado principalmente como un binario local en C.
* Expone MCP, CLI y un daemon local.
* Utiliza almacenamiento local y capacidades estructurales amplias.
* Ya incluye una superficie relacionada con ADR.
* Existen propuestas para múltiples ADR, historial por símbolo y drift arquitectónico.
* Existen casos reportados de relaciones falsas, trazas vacías, gaps entre paquetes, resultados silenciosamente vacíos y problemas de recursos en determinados repositorios o lenguajes.

Esto no invalida Codebase Memory. Confirma dos decisiones:

1. Rationale debe apoyarse en él en lugar de duplicarlo.
2. Rationale debe registrar cobertura, versión, revisión y warnings en lugar de convertir cualquier salida estructural en verdad mecánica absoluta.

También significa que el adaptador debe diseñarse después de revisar las APIs públicas y capacidades reales de la versión objetivo, no únicamente a partir de una interfaz imaginada.

## 20.7 Implicaciones de workspaces y hooks observadas en Codebase Memory

Codebase Memory ya reconoce estructura de paquetes mediante manifests y mantiene propuestas específicas para identidad de workspaces. Su historial reciente también muestra que la resolución entre paquetes puede variar por plataforma, layout, límites internos o versión del proveedor.

Consecuencia para Rationale:

* Debe consumir workspace y package identity cuando el proveedor la ofrezca.
* Debe declarar `provider_gap` cuando la relación cruzada no pueda comprobarse.
* No debe interpretar cero edges como inexistencia de relación.
* Debe aceptar bindings manuales o contractuales como fallback en el piloto.

Codebase Memory también implementa un patrón de hook de augmentación no bloqueante, con deadlines, sanitización y salida silenciosa ante errores. Ese patrón valida que puede existir una capa automática de contexto, pero también demuestra que una integración de hooks debe ser best-effort y observable.

Rationale tomará estos principios, no una dependencia interna sobre esa implementación:

```text
non-blocking by default
bounded latency
untrusted metadata as data
observable timeout/no-op
query-time correctness check
```

---

# 21. Interfaz de proveedor estructural


```rust
#[async_trait]
pub trait CodeIntelligenceProvider {
    async fn capabilities(
        &self,
    ) -> Result<ProviderCapabilities>;

    async fn health(
        &self,
        repository: &RepositoryRef,
    ) -> Result<ProviderHealth>;

    async fn resolve_target(
        &self,
        target: &UnresolvedTarget,
        revision: &Revision,
    ) -> Result<ResolvedTarget>;

    async fn get_relationships(
        &self,
        target: &ResolvedTarget,
        budget: &RelationshipBudget,
    ) -> Result<ProviderResult<Vec<CodeRelationship>>>;

    async fn get_impact(
        &self,
        target: &ResolvedTarget,
        revision: &Revision,
    ) -> Result<ProviderResult<ImpactReport>>;

    async fn changed_targets(
        &self,
        base: &Revision,
        head: &Revision,
    ) -> Result<ProviderResult<Vec<TargetChange>>>;

    async fn classify_change(
        &self,
        change: &TargetChange,
    ) -> Result<ProviderResult<ChangeClassification>>;

    async fn find_lineage_candidates(
        &self,
        target: &ResolvedTarget,
        base: &Revision,
        head: &Revision,
    ) -> Result<ProviderResult<Vec<TargetLineageCandidate>>>;
}
```

Primera implementación:

```rust
pub struct CodebaseMemoryProvider {
    client: McpClient,
}
```

## 21.1 Resultado con calidad explícita

```rust
pub struct ProviderResult<T> {
    pub data: T,
    pub provider_version: String,
    pub indexed_revision: Revision,
    pub generation: String,
    pub coverage: Coverage,
    pub warnings: Vec<ProviderWarning>,
}
```

## 21.2 Negociación de capacidades

Rationale no asumirá que todas las versiones ofrecen:

* Historial por símbolo.
* Cross-repo.
* Drift.
* Linaje.
* Framework resolution.

El adaptador debe poder responder:

```text
supported
unsupported
degraded
unknown
```

## 21.3 Adaptadores futuros

* Tree-sitter.
* LSP.
* SCIP.
* LSIF.
* Sourcegraph.
* GitHub.
* Motores propios.

Estos adaptadores justifican eventualmente un protocolo abierto, pero no son requisito del MVP.

---

# 22. Módulos principales


La arquitectura conceptual conserva las responsabilidades originales, pero la implementación inicial no debe convertir cada “engine” en un servicio o crate independiente.

## 22.1 Capture module

Responsable de recopilar:

* Diff.
* Commits.
* Símbolos.
* Tests.
* Issues.
* PR.
* Señales.
* Afirmaciones.
* Cambios de esquema.
* Revisiones y cobertura.

No decide qué es verdad.

## 22.2 Trust and policy module

Responsable de:

* Clasificar afirmaciones.
* Verificar procedencia.
* Evaluar autoridad.
* Verificar evidencia.
* Detectar contradicciones.
* Impedir que inferencias se conviertan en hechos.
* Controlar qué puede bloquear cambios.
* Aplicar reglas de sensibilidad.

## 22.3 Concept and linkage module

Responsable de:

* Crear sujetos conceptuales.
* Resolver bindings.
* Mantener linaje.
* Proponer reconexiones.
* Asociar pruebas, tablas, rutas y servicios.
* Separar identidad conceptual de implementación.
* Gestionar alias, merge y split.

## 22.4 Drift and lifecycle module

Responsable de distinguir:

* Cambio cosmético.
* Refactor.
* Movimiento.
* Cambio de contrato.
* Cambio conceptual.
* Posible violación.
* Posible supersesión.

No puede invalidar automáticamente una decisión normativa solo por una heurística de IA.

## 22.5 Retrieval module

Responsable de:

* Interpretar la intención.
* Seleccionar registros relevantes.
* Ordenarlos.
* Eliminar redundancia.
* Aplicar presupuesto.
* Construir el paquete de contexto.
* Permitir expansión progresiva.

## 22.6 Revision coordinator

Responsable de:

* Comparar Git HEAD, working tree, índice estructural y assessments.
* Crear snapshots coherentes.
* Rechazar respuestas inconsistentes.
* Invalidar cachés por revisión o generación.
* Diferenciar `exact`, `overlay`, `behind` y `unresolved`.

## 22.7 Implementación inicial

Estos módulos vivirán dentro de un monolito modular.

No serán microservicios.

No habrá un proceso independiente por engine.

El objetivo será reducir complejidad operacional y validar el producto antes de separar componentes.

---

# 23. Ciclo completo


## 23.1 Antes del cambio

Entrada:

```json
{
  "targets": [
    {
      "path": "src/auth/authorization.ts",
      "symbol": "resolveEntityRole"
    }
  ],
  "intent": "Allow support users to access multiple entities",
  "git_revision": "def456",
  "context_budget": {
    "max_tokens": 900,
    "require_exact_revision": true
  }
}
```

Proceso:

1. Resolver repositorio y revisión.
2. Comprobar el estado del working tree.
3. Consultar salud, versión, revisión y cobertura de Codebase Memory.
4. Resolver los símbolos.
5. Obtener relaciones dentro del presupuesto.
6. Resolver sujetos relacionados.
7. Recuperar decisiones y restricciones.
8. Evaluar estado epistemológico, autoridad, aplicabilidad y linkage.
9. Comparar intención.
10. Clasificar riesgos.
11. Aplicar presupuesto.
12. Construir respuesta con snapshot de consistencia.

Salida:

```json
{
  "snapshot": {
    "repository_id": "boost",
    "git_revision": "def456",
    "structural_provider": "codebase-memory",
    "structural_revision": "def456",
    "structural_generation": "184",
    "rationale_revision": "a82cf1",
    "assessment_revision": "def456",
    "consistency": "exact"
  },
  "critical_constraints": [
    {
      "statement": "Multi-entity access must not imply global administration.",
      "authority": "approved",
      "applicability": "active"
    }
  ],
  "decision_conflicts": [
    "The proposed intent may recreate a previously removed privilege path."
  ],
  "why": "Staff previously received excessive global privileges.",
  "known_risks": [
    "Users without entity assignments may lose access."
  ],
  "affected_targets": [
    "auth.resolveEntityRole",
    "auth.authorizeRequest",
    "table:entity_user_roles"
  ],
  "coverage": {
    "status": "complete",
    "warnings": []
  },
  "additional_history_available": 3
}
```

## 23.2 Durante el cambio

Señales temporales:

```json
{
  "type": "hypothesis",
  "statement": "Multi-entity users may require a global role",
  "status": "unverified"
}
```

```json
{
  "type": "discovery",
  "statement": "Multiple entity assignments are already supported",
  "evidence": {
    "symbol": "auth.resolveEntityRole",
    "revision": "def456"
  }
}
```

```json
{
  "type": "decision_candidate",
  "statement": "Keep authorization entity-scoped",
  "confirmation": "pending"
}
```

Las señales no se convierten automáticamente en registros permanentes.

## 23.3 Al finalizar

Rationale obtiene:

* Objetivo.
* Revisión base y final.
* Diff.
* Commits.
* Símbolos.
* Pruebas.
* Resultados.
* Señales.
* Errores.
* Decisiones candidatas.

Después:

1. Verifica que el diff corresponde al preflight o declara la divergencia.
2. Guarda hechos mecánicos.
3. Genera propuestas de delta normativo.
4. Clasifica inferencias.
5. Solicita confirmación mínima.
6. Crea o actualiza registros.
7. Actualiza bindings.
8. Crea assessments para la revisión final.
9. Registra supersesiones explícitas.
10. Redacta o restringe evidencia sensible.

## 23.4 En cambios futuros

Cuando otro agente modifica el sector:

1. Resuelve la implementación actual.
2. Comprueba revisión y cobertura.
3. Encuentra el sujeto.
4. Recupera decisiones activas aprobadas.
5. Evalúa linkage y aplicabilidad.
6. Selecciona conocimiento relevante.
7. Construye un paquete compacto.

## 23.5 Cambios fuera del flujo

Si el código cambia sin `prepare_change` o `finalize_change`, Rationale lo detectará como máximo en la siguiente frontera de consulta, aunque ningún hook haya funcionado.

```text
La implementación vinculada cambió después de la última evaluación.

Decision state: active, not revalidated.
Linkage: stale.
Last processed revision: abc123.
Current revision: def456.
Detection source: query-time revision gate.
```

Proceso:

1. Comparar `HEAD`, working tree y última revisión procesada.
2. Obtener targets modificados mediante Git y el proveedor cuando esté disponible.
3. Marcar únicamente assessments relacionados como `stale` o `unknown`.
4. Mantener el Record histórico intacto.
5. Servir baseline constraints con advertencia cuando sea seguro.
6. Ejecutar revalidación advisory o solicitar revisión cuando el cambio pueda ser conceptual.

Un hook o daemon puede ejecutar los primeros tres pasos antes, pero no es requerido para detectar la inconsistencia.

Rationale no declarará automáticamente que la decisión dejó de aplicar.

---

# 24. Herramientas MCP


La superficie pública de la v1 debe ser pequeña para reducir errores de selección y llamadas innecesarias.

## `prepare_change`

Herramienta principal.

Entrada:

* Objetivo o targets.
* Intención opcional.
* Síntomas, reproducción y resultado esperado opcionales.
* Alcance o workspace.
* Revisión.
* Presupuesto.
* Modo: `baseline` o `intent-aware`.

Salida:

* Snapshot de consistencia.
* Restricciones.
* Conflictos.
* Decisiones.
* Riesgos.
* Relaciones.
* Autoridad.
* Confianza.
* Vigencia.
* Cobertura.

---

## `explain_target`

Responde:

* Por qué existe un objetivo.
* Qué decisiones lo gobiernan.
* Qué parte es conocida, inferida o desconocida.
* Qué evidencia y autoridad existen.

---

## `finalize_change`

Consolida el trabajo.

* Registra hechos mecánicos.
* Propone decisiones nuevas.
* Actualiza bindings y assessments.
* Solicita confirmaciones mínimas.

---

## `review_record`

Permite:

* Aprobar.
* Corregir.
* Disputar.
* Revocar.
* Superseder.
* Asignar autoridad y scope.

---

## `trace_rationale`

Recorre:

```text
Código
→ sujeto
→ registro
→ problema
→ decisión
→ evidencia
→ aprobación
→ assessment
→ historial
```

---

## `health`

Comprueba:

* Git revision.
* Working tree.
* Proveedor estructural.
* Índice y generación.
* Cobertura.
* Bindings stale.
* Assessments atrasados.
* Errores de schema.

---

## 24.1 Superficies automáticas de contexto

La herramienta pública continúa siendo pequeña, pero clientes compatibles pueden invocar internamente un preflight baseline en eventos como:

* Búsqueda de símbolos.
* Lectura de archivos.
* Inicio de edición.
* Generación de diff.
* Finalización de tarea.

El paquete baseline debe ser extremadamente pequeño:

```text
Target is governed by 1 critical approved constraint.
Staff users must never receive global super_admin.
Assessment is stale since revision abc123; verify before behavioral changes.
```

Cuando exista una intención explícita, el cliente debe preferir `prepare_change` en modo intent-aware.

La superficie automática debe aplicar estas reglas:

* No ejecutar el Subject Resolver completo.
* No crear Records ni Subjects.
* No modificar autoridad o aplicabilidad.
* No llamar un LLM.
* No realizar embeddings.
* No bloquear por timeout o falta de cache.
* No repetir la misma constraint en cada lectura de una sesión.
* Deduplicar por target, constraint, revisión y ventana de sesión.
* Permitir expansión explícita cuando el agente necesite evidencia o historia.

La capacidad exacta de interceptar eventos depende de cada IDE o agente y no forma parte de la garantía universal del MCP.

## 24.2 Operaciones internas o administrativas

Las siguientes capacidades continúan existiendo, pero no necesitan exponerse como herramientas públicas independientes al agente:

```text
get_constraints
capture_signal
investigate_history
check_drift
repair_links
revalidate_record
supersede_record
find_conflicts
```

Se implementarán como:

* Operaciones internas de `prepare_change` o `finalize_change`.
* Subcomandos CLI administrativos.
* Funciones de biblioteca.

Esto conserva todas las capacidades originales sin obligar al modelo a coordinar doce herramientas distintas.

---

# 25. CLI inicial


```bash
rationale init

rationale health

rationale add \
  --subject authorization.entity-scoped-staff-access \
  --target src/auth/authorization.ts::resolveEntityRole

rationale why \
  src/auth/authorization.ts::resolveEntityRole

rationale prepare \
  src/auth/authorization.ts::resolveEntityRole \
  --intent "Allow support users to access every entity" \
  --revision HEAD

rationale finalize \
  --base abc123 \
  --head def456

rationale review \
  constraint.no-global-admin-for-staff

rationale trace \
  constraint.no-global-admin-for-staff

rationale investigate \
  src/auth/authorization.ts

rationale drift

rationale repair-links

rationale revalidate \
  constraint.no-global-admin-for-staff

rationale supersede \
  decision.old-auth-model \
  --by decision.new-auth-model
```

La CLI no será la interfaz cotidiana principal.

Servirá para:

* Bootstrap.
* Diagnóstico.
* CI.
* Administración.
* Automatización.
* Recuperación cuando el cliente MCP no esté disponible.

La interacción humana diaria debe poder realizarse desde el chat del IDE.

---

# 26. Almacenamiento


Fuente portable para un repositorio, incluido un monorepo:

```text
.rationale/
├── project.yaml
├── records/
├── subjects/
├── approvals/
├── evidence/
├── scopes/
├── schemas/
└── config.yaml
```

No se requiere una carpeta completa por package. Los scopes y `applies_to` expresan alcance dentro del repositorio. Un package puede contener archivos auxiliares o referencias locales solo cuando exista una razón concreta, pero la fuente canónica permanece en la raíz del proyecto.

Índice local:

```text
~/.cache/rationale/
└── projects/
    └── <project-id>/
        ├── repositories.db
        ├── scopes.db
        ├── index.db
        ├── bindings.db
        ├── assessments.db
        ├── retrieval.db
        └── state.json
```

Dos computadoras pueden tener la misma memoria canónica y distintos bindings o coverage reports debido a versiones, plataforma, working tree o estado del índice estructural. Toda respuesta debe declarar esa realidad local.

## 26.1 Archivos versionados

* YAML o JSON.
* Revisables en PR.
* Compartibles.
* Portables.
* Independientes del índice.
* Un archivo por registro para reducir conflictos.
* IDs estables.
* Sin embeddings ni cachés.

## 26.2 Índice derivado

* SQLite.
* Regenerable.
* Optimizado.
* No necesariamente versionado.
* Invalidado por revisión, schema o generación del proveedor.

## 26.3 Capas de persistencia y colaboración

| Capa | Contenido | Compartida | Regenerable |
|---|---|---:|---:|
| Canónica | Subjects, Records, declaraciones de Binding, Approvals, supersesiones | Sí, mediante Git | No |
| Derivada | Resoluciones de Binding, assessments, FTS, caches | No por defecto | Sí |
| Efímera | Intención, hipótesis, señales de sesión | No | Sí / descartable |

Las revisiones de Records y Approvals pueden realizarse mediante PR. La ausencia de Rationale en una computadora no impide modificar el repositorio, pero sí reduce la captura asistida disponible en esa sesión.

## 26.4 Declaraciones estables y assessments mutables

Declaración estable:

```yaml
record:
  id: constraint.no-global-admin-for-staff
  statement: Staff users must not receive global super_admin.
  created_at: 2026-07-23
  provenance: ...
  approvals: ...
```

Assessment derivado:

```yaml
assessment:
  record_id: constraint.no-global-admin-for-staff
  applicability: active
  linkage: current
  assessed_revision: def456
  provider_generation: 184
```

La primera representa la decisión histórica.

La segunda puede recalcularse sin reescribir el significado original.

## 26.5 Visibilidad y sensibilidad

```yaml
visibility: repository | local | restricted
sensitivity: public | internal | confidential | security
```

Políticas:

* Secret scanning antes de versionar.
* Patches sensibles no se almacenan por defecto.
* Conversaciones completas no se copian.
* Referencias externas se prefieren a duplicación.
* Exportación y MCP respetan visibilidad.

## 26.6 Referencias externas

```yaml
evidence:
  type: external-reference
  source: jira
  id: AUTH-184
  content_hash: sha256:...
  visibility: restricted
```

Esto permite conservar procedencia sin publicar el contenido completo.

---

# 27. Ejemplo actualizado de registro


```yaml
schema_version: rationale/0.4

id: constraint.no-global-admin-for-staff
project_id: boost
kind: constraint
title: Staff accounts must not receive global super-admin
severity: critical
scope: project
applies_to:
  - package:npm:@boost/api
  - package:npm:@boost/dashboard

subject:
  id: authorization.entity-scoped-staff-access
  type: system-behavior
  title: Entity-scoped staff authorization
  aliases:
    - auth.staff-per-entity

statement: >
  Staff users must never receive global super_admin.

constraint_expression:
  subject: authorization.staff
  predicate: must_not_have
  object: role.global_super_admin

rationale: >
  Access to multiple entities must not imply global administration.

problem:
  statement: >
    Staff users who required access to multiple entities were assigned
    the global super_admin role.

  symptoms:
    - Staff could access unrelated entities.
    - Multi-entity access implied system-wide privileges.
    - Authorization intent was not represented by assignments.

intent:
  primary: >
    Move staff authorization to explicit entity-scoped assignments.

  non_goals:
    - Replace the entire authorization architecture.
    - Remove global access from project owners.
    - Delete historical authorization records.

decisions:
  - id: decision.staff-access-is-entity-scoped
    statement: >
      Staff access must be assigned independently for each entity.

exceptions:
  - id: exception.project-owner-global-access
    statement: >
      Users 1, 2 and 3 retain global super_admin.

risks:
  - id: risk.staff-without-assignment
    statement: >
      Staff without entity assignments may lose access.
    epistemic_status: corroborated

provenance:
  created_by:
    type: human
    actor: user:rolando
  created_at: 2026-07-23T18:20:00Z

approvals:
  - actor: user:security-owner
    authority: security-owner
    domain: authorization
    status: approved
    approved_at: 2026-07-23T19:00:00Z

binding_declarations:
  - id: binding.authorization.resolve-entity-role
    type: symbol
    provider: codebase-memory
    structural_id: function:typescript:auth.resolveEntityRole
    path_hint: apps/api/src/auth/authorization.ts
    scope: package:npm:@boost/api

  - id: binding.authorization.entity-user-roles
    type: database-table
    target_id: entity_user_roles
    scope: package:npm:@boost/api

  - id: binding.authorization.remove-staff-super-admin
    type: migration
    path_hint: apps/api/database/migrations/remove_staff_super_admin.sql
    introduced_revision: 91ac21f

  - id: binding.authorization.staff-access-test
    type: test
    target_id: staff_cannot_access_unassigned_entity
    scope: package:npm:@boost/api

validation:
  - type: integration-test
    statement: Owners retain global access.
    result: passed
    revision: def456

  - type: migration-reexecution
    statement: The migration is idempotent.
    result: passed
    revision: def456

claims:
  - id: claim.staff-previously-received-global-access
    statement: >
      Staff previously received global access.
    epistemic_status: observed
    evidence:
      - type: source-code
        revision: 91ac21f^
      - type: migration
        path: database/migrations/remove_staff_super_admin.sql
        revision: 91ac21f

applicability_policy:
  superseded_by: null
  review_conditions:
    - Staff access becomes globally inherited.
    - Entity assignments stop governing authorization.
    - A new authorization model is explicitly approved.

binding_policy:
  structural_refactors: repair-silently
  behavioral_changes: revalidate
  conceptual_conflicts: advisory-unless-exact

context_policy:
  priority: critical
  always_include:
    - constraint.no-global-admin-for-staff
  max_default_tokens: 250

sensitivity:
  classification: internal
  visibility: repository
```

Assessment derivado para la revisión actual:

```yaml
schema_version: rationale-assessment/0.4

record_id: constraint.no-global-admin-for-staff
repository_id: boost

snapshot:
  git_revision: def456
  structural_provider: codebase-memory
  structural_revision: def456
  structural_generation: 184
  rationale_revision: a82cf1
  consistency: exact

binding_resolutions:
  - binding_id: binding.authorization.resolve-entity-role
    resolved_target: function:typescript:auth.resolveEntityRole
    resolved_revision: def456
    provider_version: 0.9.x
    provider_generation: 184
    coverage: complete
    status: current

state:
  epistemic: stated
  authority: approved
  applicability: active
  linkage: current

assessment_reason: >
  Current bindings and tests still implement entity-scoped authorization.

assessed_at: 2026-07-24T10:00:00Z
```

---

# 28. Arquitectura del repositorio


La implementación inicial debe ser un monolito modular en Rust.

```text
rationale/
├── crates/
│   ├── rationale-core/
│   │   ├── records/
│   │   ├── subjects/
│   │   ├── trust/
│   │   ├── policy/
│   │   └── lifecycle/
│   │
│   ├── rationale-storage/
│   │   ├── portable/
│   │   ├── sqlite/
│   │   └── schemas/
│   │
│   ├── rationale-providers/
│   │   ├── codebase_memory/
│   │   └── git/
│   │
│   └── rationale-app/
│       ├── mcp/
│       ├── cli/
│       ├── retrieval/
│       ├── capture/
│       └── revision/
│
├── schemas/
│   ├── record.schema.json
│   ├── subject.schema.json
│   ├── approval.schema.json
│   ├── evidence.schema.json
│   ├── assessment.schema.json
│   └── context-packet.schema.json
│
├── specification/
│   ├── context-model.md
│   ├── trust-and-authority.md
│   ├── revision-consistency.md
│   ├── provider-contract.md
│   ├── security-model.md
│   ├── retrieval-budget.md
│   └── lifecycle.md
│
└── examples/
    ├── authorization/
    ├── payments/
    ├── migrations/
    └── legacy-investigation/
```

## 28.0 Límite de este documento

Esta sección expresa restricciones conceptuales, no cierra todavía decisiones de implementación como:

* Runtime exacto del daemon opcional.
* Formato de IPC.
* Estrategia de file watching.
* Integración concreta con cada IDE.
* Algoritmo de propagación entre workspaces.
* Esquema físico final de SQLite.

Esas decisiones pertenecerán al documento específico de arquitectura. La v0.4 únicamente exige que la arquitectura futura respete scopes, revisión coherente, capas compartidas/locales y mecanismos que no dependan de hooks para ser correctos.

## 28.1 Lenguaje

Se recomienda Rust porque ofrece:

* Binario local distribuible.
* Tipado fuerte para estados y schemas.
* Seguridad de memoria.
* Buen soporte para SQLite, Git y MCP.
* Concurrencia controlada.
* Compatibilidad multiplataforma.

No es necesario utilizar C aunque Codebase Memory esté escrito principalmente en C.

La frontera correcta es el protocolo y el adaptador, no compartir lenguaje o proceso.

## 28.2 Por qué no once crates

La arquitectura original separaba cada engine en un crate.

Eso puede ser útil más adelante, pero agrega:

* APIs internas prematuras.
* Tiempo de compilación.
* Complejidad de dependencias.
* Fragmentación del dominio.
* Dificultad para cambiar el modelo durante el MVP.

La v1 conservará separación lógica sin separación física excesiva.

## 28.3 Embeddings

La v1 no dependerá de embeddings propios.

Primero utilizará:

1. Bindings exactos.
2. Grafo estructural del proveedor.
3. Scope conceptual.
4. FTS local.

Los embeddings se evaluarán después como fallback para recuperación y como señal de candidatos de identidad. Nunca serán la única base para recuperar políticas críticas, fusionar Subjects o decidir scope.

---

# 29. Plan de versiones y MVP

La versión conceptual de este documento y las versiones de implementación describen cosas distintas. `Documento 0.5` significa que el contrato conceptual fue revisado cinco veces; `producto 0.1`, `0.2` o `0.5` representan hitos futuros del software.


## Versión 0.0 — Experimento de validación

Objetivo:

> Demostrar que Rationale supera claramente a un ADR tradicional y a Codebase Memory sin Rationale en tareas reales.

Debe incluir:

* 20 a 30 cambios históricos.
* Dos o tres repositorios, incluyendo al menos un monorepo real con varios paquetes.
* El monorepo laboral seleccionado como piloto controlado, utilizando únicamente información autorizada para la prueba.
* Casos de autorización, migraciones, pagos, contratos entre paquetes, refactors y regresiones.
* Registros manuales controlados.
* Evaluación con y sin preflight.
* Ground truth por caso preparado antes de evaluar los paquetes.
* Comparación pareada entre código/Git, documentación tradicional, Codebase Memory y Codebase Memory + Rationale.
* Condición opcional de prompt escrito por una persona con experiencia del dominio.
* Varias ejecuciones por condición cuando el costo lo permita.
* Evaluación ciega de los Context Packets y resultados.
* Registro de tokens, tool calls, archivos abiertos, latencia, intentos, tests y resultado final.
* Medición explícita de contexto manual escrito por la persona.
* Instrumentación local que no envíe código, prompts ni información laboral sin autorización.
* Análisis de fallos y casos donde Rationale empeoró el resultado.

No requiere todavía producto distribuible.

El objetivo de 0.0 no es optimizar una implementación final. Es responder:

```text
¿El contexto causal estructurado cambia materialmente la calidad,
el costo o la seguridad de tareas reales frente a alternativas más simples?
```

Si la respuesta es negativa o marginal, no debe maquillarse mediante la métrica de densidad. Se deberá revisar el retrieval, la captura, el modelo de producto o la necesidad de la herramienta.

---

## Versión 0.1 — Trusted context preflight

Objetivo:

> Impedir o advertir un cambio peligroso entregando una decisión relevante, aprobada, consistente y compacta.

Debe incluir:

* Integración con Codebase Memory.
* Health y revisión exacta.
* Subjects.
* Records manuales.
* Bindings estructurales.
* Evidence.
* Approvals.
* Assessments.
* `prepare_change`.
* `explain_target`.
* Presupuesto de contexto.
* Sin alertas automáticas agresivas.
* Soporte conceptual y de retrieval para workspaces/packages dentro de un único repositorio Git.
* Revision gate en cada consulta para detectar commits fuera del flujo.
* Modo baseline cuando no exista intención explícita.
* Memoria canónica compartida y cache local regenerable.
* Sin embeddings propios.
* Sin bloqueo de CI.

No debe incluir todavía:

* Importación completa de proyectos legacy.
* Reconstrucción perfecta de linaje.
* Integraciones con Slack.
* Inferencia automática de motivos.
* Sincronización distribuida.
* Protocolo abierto estable.

---

## Versión 0.2 — Assisted capture

Agregar:

* Captura desde Git.
* Símbolos modificados.
* Pruebas ejecutadas.
* Propuestas de decisiones.
* Confirmación humana selectiva.
* `finalize_change`.
* Evidencia mecánica.
* Subject Resolver obligatorio antes de crear conceptos.
* Detección de duplicados y `novelty_reason`.
* Autoridad por dominio.
* Sensibilidad y redacción.
* Captura de cambios cruzados entre packages del monorepo.

---

## Versión 0.3 — Drift and legacy

Agregar:

* Clasificación de cambios.
* Posible drift conceptual.
* Arqueología bajo demanda.
* Git blame.
* Recuperación desde PR e issues.
* Reparación avanzada de bindings.
* Linaje por división y movimiento.
* Declaración de shallow history y gaps.
* Hooks o daemon opcionales para adelantar la detección de drift.
* Post-change audit cuando se omitió el preflight.

---

## Versión 0.4 — Team workflows

Agregar:

* Integración con PR.
* Revisión de registros.
* Reglas por repositorio.
* CODEOWNERS o políticas de autoridad.
* Cobertura por sector.
* Reportes de conflictos.
* Validaciones sugeridas.
* Hooks de agentes e IDE cuando existan.
* Revisión final de diff.
* Revisión colaborativa de Subjects y colisiones conceptuales.
* Importación o referencias entre repositorios de forma experimental.

---

## Versión 0.5 — Critical policies

Agregar de forma opt-in:

* Reglas deterministas.
* CI para constraints críticas aprobadas.
* Dos aprobaciones en dominios configurados.
* Auditoría de cambios de política.
* Excepciones temporales.

---

## Versión 1.0 — Stable context model

Agregar:

* Esquema estable.
* SDK.
* Conformance tests.
* Paquetes de contexto portables.
* Multi-repositorio.
* Adaptadores externos comprobados.
* Modelo formal de procedencia y autoridad.
* Política de compatibilidad.

El nombre “Open protocol” solo debe utilizarse cuando exista interoperabilidad real con más de una implementación o consumidor.

---

# 30. Métricas de éxito


## Utilidad

* Cuántas veces se recuperó una restricción relevante.
* Cuántos cambios peligrosos fueron advertidos.
* Cuántas consultas evitaron lectura manual extensa.
* Cuántas regresiones históricas se evitaron.

## Precisión

* Porcentaje de alertas consideradas útiles.
* Porcentaje de inferencias confirmadas.
* Número de falsos bloqueos.
* Número de registros disputados.
* Porcentaje de conflictos realmente normativos.

## Ruido

* Alertas por cambio.
* Alertas ignoradas.
* Refactors procesados silenciosamente.
* Registros redundantes eliminados del paquete.
* Confirmaciones solicitadas por cambio.

## Contexto

* Tokens promedio por `prepare_change`.
* Percentil 95 de tokens.
* Número de registros candidatos.
* Número de registros entregados.
* Reducción frente a recuperar todo el historial.

## Costo extremo a extremo

* Tokens totales hasta resolver la tarea.
* Número de archivos abiertos.
* Número de tool calls.
* Tiempo hasta solución.
* Intentos fallidos.
* Correcciones posteriores.

## Fricción

* Tiempo humano requerido por registro.
* Número promedio de confirmaciones.
* Porcentaje de cambios sin interacción humana.
* Porcentaje de registros abandonados.
* Tiempo entre propuesta y aprobación.

## Vigencia

* Bindings reparados automáticamente.
* Registros sin binding.
* Decisiones superseded correctamente.
* Tiempo entre cambio conceptual y revisión.
* Assessments atrasados.

## Autoridad

* Restricciones críticas sin aprobación.
* Aprobaciones por dominio.
* Conflictos entre autoridades.
* Políticas revocadas correctamente.

## Consistencia

* Respuestas servidas con revisión exacta.
* Consultas degradadas por índice atrasado.
* Casos donde se evitó una respuesta stale.
* Errores por incompatibilidad de proveedor.

## Seguridad

* Registros rechazados por schema.
* Intentos de prompt injection neutralizados.
* Secretos detectados antes de versionar.
* Evidencia restringida excluida del paquete.

## Monorepo y continuidad

* Restricciones cruzadas recuperadas correctamente entre packages.
* Porcentaje de inclusiones cross-workspace con camino de relevancia explicable.
* Contexto irrelevante introducido por herencia de scope.
* Commits fuera del flujo detectados en la siguiente consulta.
* Subjects duplicados prevenidos o enviados a revisión.
* Tiempo para reconstruir el índice local desde la capa canónica compartida.

## Calidad del contexto

* Densidad de contexto útil por token.
* Información crítica omitida.
* Registros incluidos sin cambiar la decisión del agente.
* Comparación entre baseline e intent-aware.
* Reducción de líneas de prompt manual repetitivo.

## Objetivos iniciales del piloto

* Al menos 90% de restricciones críticas recuperadas.
* Cero bloqueos falsos durante el piloto.
* Más de 80% de advertencias valoradas como útiles.
* Menos de 90 segundos humanos para aprobar un registro importante.
* Una o dos confirmaciones como máximo por cambio.
* Menos de 600 tokens de mediana.
* Menos de 1,000 tokens en percentil 95.
* Nunca servir un resultado como actual cuando el índice esté atrasado.

## 30.1 Protocolo de evaluación empírica

La evaluación de Rationale debe ser reproducible, auditable y capaz de refutar la hipótesis del producto.

La unidad principal de evaluación será el **Context Packet exacto entregado a un agente para una tarea concreta**, no el número total de Records ni la calidad percibida del documento completo.

### 30.1.1 Unidad experimental

Cada caso debe fijar:

```yaml
case:
  id: auth-remove-global-admin
  repository_id: pilot-monorepo
  base_revision: abc123
  allowed_sources:
    - source-code
    - git-history
    - approved-rationale-records

  task:
    statement: Allow support users to access several entities.
    symptoms: null
    expected_result: null

  target_scope:
    - apps/api
    - packages/authorization

  evaluation_policy:
    context_budget_tokens: 900
    max_tool_calls: null
    timeout_seconds: null
```

La tarea debe comenzar desde la misma revisión y condiciones comparables para todas las variantes.

### 30.1.2 Ground truth del caso

Antes de evaluar los paquetes, se preparará una ficha de referencia:

```yaml
ground_truth:
  must_know:
    - id: gt.no-global-admin-for-staff
      statement: Multi-entity access must not imply global administration.
      importance: critical

    - id: gt.owner-exceptions
      statement: Project owners are deliberate exceptions.
      importance: critical

    - id: gt.entity-assignments
      statement: Access must be represented through entity assignments.
      importance: high

  useful:
    - id: gt.idempotent-migration
      statement: The migration was designed to be idempotent.
      importance: medium

  irrelevant:
    - statement: The authorization service was renamed months earlier.

  dangerous_falsehoods:
    - statement: Support users require a global role for multi-entity access.
```

El ground truth puede construirse mediante:

* Diff de la solución histórica.
* Issue o requerimiento original.
* Pull request y comentarios.
* Commits.
* Tests y migraciones.
* Incidentes.
* Documentación disponible en ese momento.
* Revisión de una persona con conocimiento del dominio.

Debe registrar incertidumbre y desacuerdo. Si dos expertos no coinciden, el caso no se fuerza artificialmente a una única verdad; se marca como disputado o se excluye de métricas que requieren certeza.

El ground truth no debe filtrarse al agente fuera de la condición experimental correspondiente.

### 30.1.3 Fórmula operacional de Context Utility Density

Para un paquete `P` con elementos `i`:

```text
utility(i) =
    relevance(i)
  × reliability(i)
  × actionability(i)
  × applicability(i)
  × importance(i)
  × uniqueness(i)
```

```text
context_utility_density(P) =
    1000 × sum(utility(i)) / max(tokens(P), 1)
```

El resultado se interpreta como **utilidad ponderada por cada mil tokens**.

La multiplicación es deliberadamente estricta: un elemento muy relevante pero falso, obsoleto o no accionable no debe conservar una puntuación alta. Durante el piloto también se guardarán los componentes por separado para comprobar que la fórmula no oculta el motivo de un resultado.

Esta fórmula no es una ley universal. Sus pesos y escalas son una hipótesis inicial que deberá someterse a análisis de sensibilidad.

### 30.1.4 Escalas de puntuación

#### Relevancia

Pregunta:

> ¿Este elemento era necesario o directamente útil para resolver la tarea?

```text
1.00  necesario para una solución segura o correcta
0.75  muy útil y reduce materialmente la búsqueda
0.50  útil, pero no esencial
0.25  relación débil o contextual
0.00  irrelevante
```

La relevancia se evalúa contra el caso y su ground truth, no únicamente mediante similitud semántica.

#### Confiabilidad

Pregunta:

> ¿El contenido es correcto y está respaldado para este caso?

Valores iniciales orientativos:

```text
1.00  observado mecánicamente y verificado para la revisión
1.00  política aprobada y confirmada como correcta
0.90  afirmación humana aprobada y respaldada
0.75  corroborada por fuentes independientes
0.40  inferencia razonable
0.15  hipótesis
0.00  falsa, contradicha o fabricada
```

La procedencia no garantiza corrección. Una afirmación humana aprobada puede recibir una puntuación inferior si el caso demuestra que estaba equivocada o dejó de aplicar.

#### Accionabilidad

Pregunta:

> ¿Este elemento cambia o mejora una acción concreta del agente?

```text
1.00  determina una restricción, solución o validación necesaria
0.75  reduce considerablemente el espacio de soluciones
0.50  orienta una investigación útil
0.25  aporta comprensión general sin cambiar la acción
0.00  no afecta ninguna decisión
```

#### Aplicabilidad o frescura

No significa antigüedad cronológica.

Pregunta:

> ¿Continúa gobernando la revisión evaluada?

```text
1.00  confirmada para la revisión actual
0.75  probablemente activa con linkage parcialmente degradado
0.50  aplicabilidad desconocida
0.25  señales de supersesión o drift
0.00  superseded, inválida o fuera de scope
```

#### Importancia

```text
1.00  constraint crítica o riesgo severo
0.80  decisión normativa importante
0.60  riesgo operacional
0.40  razón histórica útil
0.20  contexto auxiliar
```

La importancia debe provenir del ground truth o una rúbrica preparada antes de observar qué condición produjo el paquete.

#### Unicidad

Penaliza repetición y parafraseo redundante:

```text
1.00  aporta información nueva
0.50  solapa parcialmente con otro elemento
0.10  casi totalmente redundante
0.00  duplicado exacto
```

### 30.1.5 Segmentación del Context Packet

Para puntuar de forma consistente, el paquete se descompone en unidades semánticas mínimas:

* Una constraint.
* Una decisión.
* Un riesgo.
* Una advertencia de consistencia.
* Una afirmación causal.
* Una validación sugerida.
* Una relación estructural accionable.

No se debe dividir una misma afirmación en muchas frases para inflar la suma de utilidad.

La instrumentación conservará:

```yaml
context_item_evaluation:
  packet_id: packet-123
  item_id: constraint.no-global-admin-for-staff
  token_count: 17
  relevance: 1.0
  reliability: 0.9
  actionability: 1.0
  applicability: 1.0
  importance: 1.0
  uniqueness: 1.0
  evaluator_notes: Prevents the historical regression directly.
```

### 30.1.6 Condiciones de comparación

Como mínimo, el mismo caso se probará en:

#### Condición A — Código y Git

Acceso normal al repositorio y sus herramientas básicas.

#### Condición B — Documentación tradicional

Código, Git, `AGENTS.md`, ADR y documentación disponible.

#### Condición C — Codebase Memory

Código, Git y contexto estructural recuperado mediante Codebase Memory.

#### Condición D — Codebase Memory + Rationale

La experiencia completa propuesta.

#### Condición E — Prompt humano experto, opcional

Una persona con conocimiento del dominio entrega manualmente el contexto que normalmente explicaría al agente.

Esta condición no es un competidor trivial. Sirve como aproximación a cuánto conocimiento institucional valioso logra conservar Rationale y cuánto todavía depende de una persona concreta.

Las condiciones deben usar:

* El mismo modelo y versión.
* La misma revisión inicial.
* La misma tarea.
* Configuraciones equivalentes.
* Presupuestos comparables.
* Reinicio o aislamiento de memoria entre ejecuciones.

Cuando no sea posible igualar exactamente una variable, debe registrarse como limitación.

### 30.1.7 Capa 1: calidad del contexto

#### Critical Constraint Recall

```text
critical_constraint_recall =
    critical constraints delivered
    / critical constraints required
```

Esta métrica tiene prioridad sobre la densidad promedio. Omitir una única regla crítica puede invalidar un paquete aparentemente eficiente.

#### Context Precision

```text
context_precision =
    useful delivered items
    / total delivered items
```

#### Harmful Context Rate

```text
harmful_context_rate =
    false, contradictory or inapplicable items
    / total delivered items
```

#### Weighted Context Recall

```text
weighted_context_recall =
    sum(importance of correctly delivered ground-truth items)
    / sum(importance of all required ground-truth items)
```

#### Redundancy Rate

```text
redundancy_rate =
    redundant context tokens
    / total context tokens
```

También se registrarán:

* Información crítica omitida.
* Contexto verdadero pero inútil.
* Evidencia no disponible.
* Items cuyo assessment estaba stale.
* Tokens consumidos por metadata de confianza y autoridad.

### 30.1.8 Capa 2: resultado de la tarea

La calidad del paquete no es suficiente. Cada ejecución debe evaluarse por:

* Solución correcta o incorrecta.
* Restricciones respetadas.
* Bug histórico reintroducido.
* Tests existentes aprobados.
* Nuevas pruebas adecuadas.
* Intentos necesarios.
* Archivos abiertos.
* Tool calls.
* Tokens de entrada y salida totales.
* Tiempo hasta una solución aceptable.
* Correcciones posteriores.
* Intervenciones humanas.

Métrica central de costo:

```text
total_tokens_to_successful_completion
```

No solamente:

```text
tokens_returned_by_prepare_change
```

Cuando una ejecución no logra una solución correcta dentro del límite establecido, debe registrarse como censurada o fallida; no se puede comparar su bajo consumo de tokens como una victoria.

### 30.1.9 Reducción de contexto manual

La promesa del producto incluye reducir cuánto conocimiento institucional debe repetir la persona.

```text
manual_context_reduction_tokens =
    manual context tokens without Rationale
    - manual context tokens with Rationale
```

```text
manual_fact_reduction =
    project facts manually supplied without Rationale
    - project facts manually supplied with Rationale
```

También se medirán:

* Tiempo preparando el prompt.
* Número de aclaraciones.
* Cantidad de veces que la persona tuvo que señalar un archivo o módulo.
* Cantidad de restricciones que tuvo que repetir.
* Contexto manual que Rationale suministró correctamente.
* Contexto que Rationale no podía conocer y debió seguir aportando la persona.

La meta no es eliminar el prompt. La persona continúa expresando qué quiere conseguir, síntomas nuevos, prioridades y restricciones no registradas.

### 30.1.10 Continuidad respecto al conocimiento senior

Para casos adecuados, una persona experimentada del dominio preparará una lista independiente de:

* Precauciones.
* Restricciones.
* Componentes que investigaría.
* Pruebas que exigiría.
* Errores históricos que evitaría.

Después se medirá:

```text
senior_context_recall =
    supported senior-context items retrieved by Rationale
    / supported senior-context items identified by the reviewer
```

```text
unsupported_advice_rate =
    Rationale recommendations without supporting evidence
    / total Rationale recommendations
```

Esto no afirma que la lista capture toda la mente de una persona senior. Evalúa cuánto conocimiento verificable y transferible logra preservar el proyecto.

### 30.1.11 Instrumentación mínima

Cada ejecución debe producir un registro local estructurado:

```yaml
experiment_run:
  run_id: run-001
  case_id: auth-remove-global-admin
  condition: rationale
  model:
    provider: configured-provider
    model_id: exact-version
    temperature: 0

  snapshot:
    base_revision: abc123
    provider_version: x.y.z
    provider_generation: 441
    rationale_schema: rationale/0.5

  context:
    packet_id: packet-123
    mode: intent-aware
    tokens: 542
    latency_ms: 184
    cache_state: warm
    degraded: false

  execution:
    started_at: ...
    completed_at: ...
    input_tokens_total: 10340
    output_tokens_total: 2240
    tool_calls: 12
    files_read: 7
    attempts: 1

  outcome:
    task_success: true
    constraints_respected: true
    historical_regression: false
    tests_passed: true
    evaluator_score: null
```

La implementación exacta del harness pertenece al documento del experimento, pero el contrato conceptual exige capturar estos datos.

### 30.1.12 Privacidad del piloto

Para un repositorio laboral:

* Solo se usarán datos y revisiones autorizadas.
* La telemetría será local por defecto.
* No se enviará código, prompts, diffs, Records ni resultados a servicios adicionales sin autorización.
* Los reportes podrán utilizar IDs y métricas agregadas.
* Los casos sensibles podrán conservar únicamente hashes, categorías y resultados.
* El dataset público futuro deberá construirse con repositorios abiertos o casos sintéticos equivalentes.

### 30.1.13 Evaluación ciega y reducción de sesgo

Cuando sea posible:

* Los paquetes se presentarán sin indicar qué condición los produjo.
* Los evaluadores usarán una rúbrica común.
* Dos evaluadores puntuarán una muestra.
* Se medirán desacuerdos y acuerdo interevaluador.
* El ground truth se cerrará antes de observar resultados agregados.
* Los casos no se seleccionarán únicamente porque favorecen a Rationale.
* Se conservarán también los resultados negativos.

La persona que construyó un Rationale Record no debería ser la única evaluadora de su utilidad.

### 30.1.14 Análisis estadístico

Con 20 a 30 casos, el piloto no probará universalmente que Rationale funciona para toda clase de repositorio.

Debe utilizar:

* Comparaciones pareadas sobre las mismas tareas.
* Medianas y percentiles, no solo promedios.
* Intervalos de confianza mediante bootstrap cuando sea apropiado.
* Distribuciones por tipo de cambio.
* Resultados separados para monorepo y repositorios simples.
* Resultados separados para baseline e intent-aware.
* Análisis de sensibilidad de pesos de CUD.
* Reporte de tamaños de efecto prácticos.

No debe utilizar una diferencia estadísticamente ruidosa como afirmación comercial definitiva.

### 30.1.15 Criterios iniciales de éxito

Objetivos del piloto:

```text
critical_constraint_recall >= 90%
context_precision >= 80%
harmful_context_rate < 2%
ideal harmful_context_rate = 0%
false_blocking_rate = 0%
median_context_packet <= 600 tokens
p95_context_packet <= 1000 tokens
manual_prompt_context_reduction >= 50%
```

Además:

* `total_tokens_to_successful_completion` debe mejorar materialmente frente a una o más condiciones de control, o justificar cualquier aumento mediante una mejora clara de seguridad o éxito.
* La tasa de éxito de tarea debe superar de forma práctica a alternativas simples en los casos donde existe contexto causal relevante.
* La latencia baseline debe respetar el presupuesto experimental definido.
* No debe servirse un assessment stale como exacto.
* Los beneficios deben persistir en el monorepo piloto y no únicamente en casos artificiales pequeños.

Estos umbrales pueden ajustarse antes de comenzar el piloto, pero no después de observar resultados solo para declarar victoria.

### 30.1.16 Reglas de falsificación

Rationale no supera el piloto cuando ocurre alguna de estas condiciones:

* Recupera mucho contexto correcto, pero no mejora decisiones ni resultados.
* Reduce tokens del paquete, pero aumenta tokens totales o tool calls sin beneficio material.
* Omite constraints críticas con frecuencia.
* Introduce falsedades o reglas superseded.
* Requiere tanto trabajo humano de mantenimiento como el contexto que pretende ahorrar.
* Funciona únicamente cuando su creador prepara manualmente cada caso perfecto.
* La documentación tradicional ofrece resultados equivalentes con mucha menos complejidad.
* El baseline añade una latencia perceptible sin aportar contexto utilizado.

Un resultado negativo no invalida necesariamente el problema. Puede indicar que el modelo de captura, retrieval, integración o producto debe simplificarse.

### 30.1.17 Interpretación final

Rationale solo podrá declararse superior a ADR, `AGENTS.md` o Codebase Memory aislado si sus paquetes:

1. Recuperan más conocimiento crítico correcto.
2. Entregan menos ruido y menos falsedades.
3. Producen soluciones mejores o más seguras.
4. Reducen el costo total o el contexto manual requerido.
5. Mantienen una latencia aceptable para la superficie donde se utilizan.

La densidad de utilidad es una explicación de **por qué** un paquete fue eficiente. El resultado de la tarea determina **si realmente lo fue**.

---

# 31. Riesgos del proyecto


## Riesgo: explicaciones falsas

Mitigación:

* Inferencias claramente marcadas.
* Evidencia obligatoria.
* Confirmación selectiva.
* Nunca bloquear con conocimiento inferido.
* Permitir estado `unknown`.

## Riesgo: autoridad incorrecta

Mitigación:

* Separar procedencia de autoridad.
* Approval policy por dominio.
* CODEOWNERS o configuración explícita.
* Revocación y auditoría.
* Doble aprobación en dominios críticos.

## Riesgo: fatiga de alertas

Mitigación:

* Estado multidimensional reducido.
* Clasificación de cambios.
* Reparación silenciosa.
* Alertas basadas en intención.
* Bloqueos únicamente críticos.
* Presupuesto de interrupción.

## Riesgo: fatiga de confirmación

Mitigación:

* Confirmar solo deltas normativos.
* Una afirmación concreta por interacción.
* No preaprobar restricciones críticas.
* Permitir edición y rechazo.
* No crear registro si el motivo es desconocido.

## Riesgo: demasiada fricción

Mitigación:

* Captura automática.
* Confirmaciones mínimas.
* Permitir motivo desconocido.
* Niveles de captura.
* No documentar cambios triviales.

## Riesgo: repositorios legacy

Mitigación:

* Captura hacia adelante.
* Arqueología bajo demanda.
* Priorización.
* Cobertura parcial aceptada.
* Declaración de shallow history y gaps.

## Riesgo: fragilidad de símbolos

Mitigación:

* Identidad conceptual.
* Múltiples bindings.
* Linaje.
* Git.
* Tests.
* Tablas.
* Rutas.
* Linkage separado de aplicabilidad.

## Riesgo: duplicación conceptual

Mitigación:

* Alias.
* Candidatos de merge.
* Split explícito.
* No fusionar automáticamente.
* Historial de identidad.

## Riesgo: saturación de contexto

Mitigación:

* Context budget.
* Ranking.
* Deduplicación.
* Progressive disclosure.
* Resúmenes por prioridad.

## Riesgo: inconsistencia por revisión

Mitigación:

* Revision coordinator.
* Snapshot obligatorio.
* Rechazar o degradar resultados.
* Caché por revisión y generación.
* Health tool.

## Riesgo: errores o gaps de Codebase Memory

Mitigación:

* Registrar versión, cobertura y warnings.
* No asumir que ausencia significa inexistencia.
* Fallback a Git o source cuando corresponda.
* Pruebas contractuales del adaptador.
* Capabilities negotiation.

## Riesgo: prompt injection en registros

Mitigación:

* Tratar contenido como dato no ejecutable.
* Schema estricto.
* Campos declarativos.
* Sanitización del paquete.
* Separación de evidencia e instrucciones.
* Límites de texto libre.

## Riesgo: secretos e información sensible

Mitigación:

* Clasificación de sensibilidad.
* Visibilidad.
* Secret scanning.
* Referencias externas.
* Redacción.
* No cachear patches sensibles por defecto.

## Riesgo: el agente no llama Rationale

Mitigación:

* Baseline target context en integraciones compatibles.
* `prepare_change` intent-aware.
* Instrucciones del cliente.
* Hooks opcionales y no bloqueantes.
* Revision gate independiente del hook.
* Revisión del diff al finalizar.
* CI solo para políticas críticas deterministas.
* Health check antes de operar en dominios configurados.

## Riesgo: no ahorrar tokens

Mitigación:

* Activación adaptativa.
* Paquetes pequeños.
* Medición extremo a extremo.
* Comparación contra ADR y lectura directa.
* No usar Rationale para cambios triviales.

## Riesgo: Codebase Memory absorbe la función

Mitigación:

* Diferenciarse en autoridad, procedencia, aplicabilidad y preflight.
* Consumir ADR e historia del proveedor como evidencia.
* Mantener modelo portable.
* Soportar proveedores futuros.

## Riesgo: fuga de scope en monorepos

Mitigación:

* Scopes jerárquicos explícitos.
* `applies_to` y `excludes`.
* Camino de relevancia en cada inclusión cruzada.
* Métricas de contexto irrelevante.
* Tests con frontend, backend y packages compartidos.

## Riesgo: depender de hooks para la corrección

Mitigación:

* Revision gate en cada consulta.
* Hooks y daemon únicamente como aceleradores.
* Estado observable cuando una integración no se ejecutó.
* CI como verificación posterior, no sustituto de la frescura local.

## Riesgo: deduplicación semántica incorrecta

Mitigación:

* Resolución determinista primero.
* Embeddings solo como candidatos.
* `novelty_reason`.
* Merge y split auditables.
* Nunca bloquear por similitud aislada.

## Riesgo: assessments diferentes entre computadoras

Mitigación:

* Capa canónica separada de la derivada.
* Snapshot con versión y cobertura local.
* Assessments regenerables.
* No versionar conclusiones dependientes de un índice incompleto como hechos globales.

## Riesgo: contexto abundante pero poco útil

Mitigación:

* Context utility density.
* Baseline pequeño.
* Intent-aware retrieval.
* Progressive disclosure.
* Evals de omisión y ruido, no solo conteo de tokens.

## Riesgo: prometer reemplazar experiencia senior

Mitigación:

* Definir el producto como continuidad institucional.
* Mantener autoridad humana.
* Mostrar desconocidos.
* No inferir prioridades empresariales.
* Evaluar reducción de arqueología y regresiones, no “nivel senior” abstracto.

## Riesgo: optimizar para la métrica

Un sistema podría aumentar artificialmente `context_utility_density` entregando paquetes mínimos que omiten información difícil, segmentando items para inflar la suma o calibrando pesos después de observar resultados.

Mitigación:

* Ground truth previo.
* Métricas de recall y harmful context separadas.
* Segmentación semántica definida.
* Análisis de sensibilidad.
* Congelar umbrales antes del piloto.
* Dar prioridad al resultado real de la tarea.

## Riesgo: sesgo del ground truth

La persona que conoce la solución histórica puede incluir solamente el conocimiento que Rationale ya modela o confundir la solución final con la única solución válida.

Mitigación:

* Múltiples fuentes.
* Revisión independiente.
* Estados disputados.
* Evaluación ciega.
* Conservar soluciones alternativas válidas.
* Excluir casos donde no pueda establecerse una referencia razonable.

## Riesgo: `novelty_reason` genérica

Un agente puede aprender a superar el control escribiendo justificaciones vacías o fabricando diferencias.

Mitigación:

* Schema estructurado.
* Candidatos comparados obligatorios.
* Contraste explícito.
* Evidencia cuando exista similitud alta.
* Validación determinista básica.
* Revisión humana en dominios críticos.
* Medir duplicados descubiertos posteriormente.

## Riesgo: latencia del baseline

Una inyección útil pero lenta puede interrumpir búsquedas y lecturas frecuentes hasta que los desarrolladores desactiven la herramienta.

Mitigación:

* Fast path local separado.
* Cache por revisión y generación.
* Sin LLM ni embeddings.
* Deadline estricto.
* Fail open.
* Deduplicación por sesión.
* Telemetría local de P50, P95, cold start y timeouts.

## Riesgo: contaminación entre condiciones experimentales

El agente o evaluador puede recordar información de una ejecución anterior y favorecer condiciones posteriores.

Mitigación:

* Sesiones aisladas.
* Orden aleatorio o contrabalanceado.
* Reinicio de memoria.
* Evaluación ciega.
* Identificar explícitamente cualquier contaminación inevitable.

## Riesgo: sobrearquitectura

Mitigación:

* Experimento 0.0 antes de producto completo.
* Seis entidades persistidas.
* Seis herramientas públicas.
* Cuatro crates.
* Monolito modular.
* Sin embeddings propios inicialmente.

---

# 32. Criterio definitivo de éxito


Rationale será exitoso si ocurre lo siguiente:

1. Un agente nuevo abre un repositorio sin conocer conversaciones anteriores.
2. Intenta modificar un sistema de autorización.
3. Rationale comprueba que Git, Codebase Memory y assessments corresponden a una revisión coherente.
4. Identifica el comportamiento conceptual relacionado.
5. Recupera una decisión aprobada por la autoridad adecuada.
6. Le muestra una restricción crítica.
7. Explica por qué existe y qué evidencia la respalda.
8. Detecta que la intención propuesta puede reintroducir un problema anterior.
9. Entrega el contexto en menos de aproximadamente mil tokens.
10. No le muestra quince registros históricos irrelevantes.
11. No genera una advertencia por un simple renombre.
12. No bloquea por una inferencia o por un índice incompleto.
13. El agente puede continuar con una solución mejor informada.
14. El costo total de resolver la tarea mejora frente a no usar Rationale.
15. Una restricción de backend relevante puede recuperarse al modificar un package frontend conectado, mostrando el camino de relevancia.
16. Un commit humano directo vuelve stale el assessment correspondiente en la siguiente consulta, aunque no exista hook.
17. Un agente no puede crear silenciosamente un Subject casi duplicado sin reutilizarlo o justificar su novedad.
18. Una máquina nueva puede reconstruir el contexto local desde los archivos canónicos versionados.
19. Si el agente no declara intención, recibe un baseline pequeño sin que Rationale invente el objetivo.
20. La herramienta reduce contexto manual repetitivo sin reemplazar los síntomas y requisitos específicos de la tarea.

La experiencia ideal sería:

```text
You are modifying entity authorization.

Snapshot:
Git, structural index and rationale assessment match revision def456.

Critical approved constraint:
Staff users must never receive global super_admin.

Why:
This rule was introduced after multi-entity staff accounts received
system-wide privileges.

Authority:
Approved by the authorization security owner.

Your intent may conflict with this decision.

Known safe direction:
Grant explicit assignments for each entity.

Evidence:
Migration, integration tests and approved decision record.

Additional history:
2 records available.
```

## 32.0 Caso de éxito cross-workspace

```text
Task:
Update the dashboard role badge for support users.

Target:
apps/dashboard/src/users/RoleBadge.tsx

Cross-workspace context:
This UI consumes @boost/auth-contracts, which is governed by the project-level
entity-scoped staff authorization decision.

Critical constraint:
Do not represent support users as global administrators.

Why:
Multi-entity staff previously received system-wide privileges.

Relevant backend targets:
apps/api/src/auth/resolveEntityRole.ts
packages/auth-contracts/src/roles.ts

Context path:
RoleBadge → @boost/auth-contracts → authorization subject → approved constraint
```

El paquete no incluye toda la memoria del backend. Incluye la única decisión cruzada que cambia cómo debe resolverse la tarea.

## 32.1 Comparación mínima para justificar el producto

Rationale debe superar de forma clara a:

1. Agente sin memoria.
2. `AGENTS.md` o ADR tradicionales.
3. Codebase Memory con ADR o historia, sin Rationale.
4. Lectura manual de commits y PR.

Si no ofrece una mejora clara en seguridad, precisión, fricción o costo total, la arquitectura completa no se justifica.

---

# 33. Definición pública


## Descripción corta

**Rationale is an open-source project-context compiler and provenance layer for AI coding agents. It turns shared decisions, constraints, risks, and structural bindings into the smallest reliable context packet needed for a specific code change.**

## Descripción en español

**Rationale es un compilador open source de contexto del proyecto y una capa de procedencia para agentes de programación. Convierte decisiones, restricciones, riesgos y bindings estructurales compartidos en el paquete de contexto confiable más pequeño que necesita un cambio específico.**

## Relación con Codebase Memory

**Codebase Memory understands what is connected. Rationale tells the agent which decisions still govern those connections—and why they are trusted.**

## Relación con Git

**Git records what changed. Rationale preserves why the change still matters and whether the decision remains applicable.**

## Frases centrales

> **Git remembers what changed. Rationale remembers why it still matters.**

> **Rationale does not remember everything. It remembers what still matters.**

> **Rationale compiles what the project learned into what the agent needs now.**

> **No explanation is better than a false explanation.**

## Definición técnica

> **Rationale es un compilador de contexto y preflight de decisiones de software con scopes, procedencia, autoridad, bindings estructurales y consistencia por revisión.**

---

# 34. Visión de largo plazo


Rationale será el primer producto concreto dentro de una visión más amplia:

```text
Project Cognition Protocol
└── Context Provenance Model
    └── Rationale Context Model
        └── Rationale CLI + MCP
```

## Rationale

Producto inicial enfocado en cambios de software.

## Rationale Context Model

Modelo portable para:

* Subjects.
* Records.
* Bindings.
* Evidence.
* Approvals.
* Assessments.
* Snapshots de revisión.

## Context Provenance Model

Modelo más general que define:

* Origen.
* Transformaciones.
* Autoridad.
* Confianza.
* Vigencia.
* Evidencia.
* Invalidación.
* Sensibilidad.

## Project Cognition Protocol

Visión futura para preservar conocimiento operativo más amplio:

* Decisiones.
* Incidentes.
* Experimentos.
* Migraciones.
* Suposiciones.
* Consecuencias.
* Linaje.

## Condición para llamarlo protocolo

No se declarará estable hasta contar con:

* Al menos dos consumidores o proveedores independientes.
* Conformance tests.
* Versionado y migraciones.
* Casos multi-repositorio reales.
* Política de compatibilidad.

La implementación debe comenzar con el problema específico.

No con la ambición de modelar todo.

## 34.1 Registro de decisiones de la versión 0.4

| Propuesta evaluada | Decisión | Razón |
|---|---|---|
| Subjects globales y bindings locales para monorepos | Adoptada con scopes jerárquicos | El problema es alcance e herencia, no crear una base separada por package |
| Embeddings obligatorios antes de crear Subject | Adoptada parcialmente | Se usarán como señal de candidato; la resolución determinista tiene prioridad |
| Justificación al no reutilizar concepto similar | Adoptada | `novelty_reason` vuelve auditable la creación |
| Daemon o `post-commit` obligatorio | Rechazada como garantía | Puede omitirse; query-time revision gate es la frontera correcta |
| Hook o daemon opcional para detectar antes | Adoptada | Reduce la ventana stale sin bloquear el flujo |
| IDE intercepta siempre la intención | Rechazada como promesa universal | Las capacidades varían y la intención puede no estar expresada |
| Baseline automático por target | Adoptada | Protege constraints críticas aun sin intención completa |
| Preflight explícito intent-aware | Conservado | Es necesario para comparar el cambio propuesto contra decisiones |
| “Mientras más contexto, mejor” | Reformulado | Más contexto enfocado ayuda; ruido y posición pueden perjudicar |
| Visión similar a una persona senior | Reformulada | Continuidad institucional sí; reemplazo del juicio senior no |
| Memoria compartida aunque CBM sea local | Adoptada | Records canónicos viven en Git; índices y assessments se reconstruyen localmente |

Esta adjudicación debe preservarse para evitar que futuras iteraciones reintroduzcan supuestos ya descartados.

## 34.2 Registro de decisiones de la versión 0.5

| Propuesta evaluada | Decisión | Razón |
|---|---|---|
| Medir CUD únicamente con percepción subjetiva | Rechazada | La utilidad debe compararse contra ground truth y resultados reales |
| Utilidad por cada mil tokens | Adoptada como métrica diagnóstica | Hace comparable la eficiencia de paquetes de tamaños distintos |
| CUD como única métrica de éxito | Rechazada | Puede ocultar omisiones críticas, falsedades o tareas fallidas |
| Multiplicar relevancia, confiabilidad, accionabilidad, aplicabilidad, importancia y unicidad | Adoptada como hipótesis inicial | Penaliza estrictamente items débiles, pero requiere sensibilidad y validación |
| Ground truth por caso | Adoptado | Permite evaluar recall, precisión y falsedades de forma reproducible |
| Comparar solo contra ejecución sin memoria | Rechazada | Debe incluir documentación tradicional, Codebase Memory y, cuando sea posible, contexto experto |
| Medir solo tokens de `prepare_change` | Rechazada | El costo válido es extremo a extremo hasta una solución correcta |
| Medir reducción del prompt humano | Adoptada | Representa una promesa central del producto |
| Declarar “visión senior” por similitud de estilo | Rechazada | Se medirá únicamente recall de conocimiento verificable identificado por una persona experimentada |
| `novelty_reason` en texto libre | Rechazada como única defensa | Se requiere contraste estructurado, candidatos y evidencia |
| Baseline ejecutando el pipeline completo | Rechazado | Las superficies de alta frecuencia necesitan un fast path precomputado |
| Baseline bloqueante | Rechazado | Debe tener deadline, fail open y degradación observable |
| Publicar umbrales después de ver resultados | Rechazado | Los criterios deben congelarse antes del piloto para evitar metric gaming |
| Ignorar resultados donde Rationale empeora | Rechazado | Los fallos son necesarios para validar o corregir la hipótesis |

Esta versión se considera semidefinitiva para comenzar el experimento, no evidencia de que el producto ya funciona.

---

# 35. Conclusión


Rationale no debe ser una base de datos llena de explicaciones históricas.

Tampoco debe convertirse en una herramienta que interrumpe a los desarrolladores cada vez que una función cambia.

Su valor estará en encontrar el equilibrio entre:

* Memoria y olvido.
* Automatización y confirmación.
* Estructura y concepto.
* Historia y relevancia.
* Advertencia y ruido.
* Confianza y humildad.
* Procedencia y autoridad.
* Declaraciones históricas y assessments actuales.
* Utilidad y costo operacional.
* Memoria compartida y estado local.
* Contexto suficiente y ruido.
* Alcance global y scopes de paquetes.
* Automatización best-effort y garantías verificables.
* Densidad del paquete y resultado real de la tarea.
* Ahorro de contexto y costo extremo a extremo.
* Hipótesis atractivas y evidencia capaz de refutarlas.

La definición definitiva del proyecto es:

> **Rationale es un compilador local de contexto causal y una capa de procedencia, autoridad y vigencia para agentes de programación. Conserva como memoria canónica compartida por qué se realizaron cambios importantes, qué decisiones y restricciones gobiernan los comportamientos del sistema, quién podía aprobarlas y qué evidencia las respalda. En cada computadora utiliza motores estructurales como Codebase Memory para resolver scopes y las resoluciones locales de bindings en la revisión actual, y compila únicamente el contexto confiable, relevante y accionable que una tarea necesita.**

La mejor versión del producto no es la que recuerda más. Tampoco es la que obtiene la puntuación interna más alta. Es la que demuestra que su contexto ayuda a completar tareas reales con mayor seguridad, menos repetición humana y un costo total razonable.


Es la que sabe:

* Qué no sabe.
* Qué fue inferido.
* Qué fue aprobado.
* Qué evidencia puede estar incompleta.
* Qué revisión está observando.
* Cuándo debe guardar silencio.
* Cuándo una decisión es suficientemente crítica para detener un cambio.

Rationale tampoco busca reemplazar el prompt específico de la tarea ni convertir al agente en una persona senior completa. Busca conservar la continuidad técnica que normalmente desaparece cuando una conversación termina o una persona abandona el proyecto.

Rationale no busca que una inteligencia artificial recuerde todo lo ocurrido en un proyecto.

Busca que nunca destruya una decisión importante únicamente porque nadie logró explicarle por qué existía.

Y, al mismo tiempo, busca que nunca preserve una explicación falsa únicamente porque sonaba convincente.

---

