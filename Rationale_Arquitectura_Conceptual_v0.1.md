# Rationale

## Arquitectura conceptual 0.1

### Contrato técnico previo a implementación

**Versión de arquitectura:** 0.1  
**Fecha de corte:** 2026-07-24  
**Estado:** arquitectura conceptual, deliberadamente no definitiva  
**Documento conceptual obligatorio:** `Rationale_v0.5.md`  
**Documento operativo complementario:** `Rationale_Proceso_Construccion_Agentes_v0.1.md`

---

# 0. Advertencia principal

Este documento no pretende fingir que la arquitectura final ya fue descubierta.

Define:

- Las fronteras que el producto necesita.
- Los componentes conceptuales que deben existir.
- Los contratos que deben verificarse.
- Los experimentos que deben ejecutarse.
- Las decisiones que todavía no pueden tomarse responsablemente.
- El orden correcto para analizar, construir, validar, empaquetar y distribuir Rationale.

La arquitectura final no debe implementarse copiando este documento de forma mecánica.

Antes de fijar:

- Lenguaje.
- Runtime.
- IPC.
- Modelo de concurrencia.
- SDK de MCP.
- Integración con Codebase Memory.
- Formato físico de índices.
- Instaladores.
- Hooks.
- Daemon.
- Distribución.

el equipo y los agentes deben analizar la versión actual de Codebase Memory, construir pequeños prototipos y registrar los resultados.

La regla será:

> La arquitectura conceptual define las responsabilidades.  
> La investigación técnica define su implementación.

Si una observación real de Codebase Memory contradice una suposición de este documento, no debe ocultarse ni forzarse.

Debe:

1. Documentarse.
2. Reproducirse.
3. Medirse.
4. Convertirse en una decisión explícita.
5. Actualizar la arquitectura mediante un ADR.

---

# 1. Relación con el documento conceptual

`Rationale_v0.5.md` continúa siendo la fuente de verdad sobre:

- El problema.
- La definición del producto.
- La Valla de Chesterton.
- La humildad epistemológica.
- Los Subjects.
- Records.
- Bindings.
- Evidence.
- Approvals.
- Assessments.
- Procedencia.
- Autoridad.
- Aplicabilidad.
- Consistencia por revisión.
- Monorepos.
- Context budget.
- Context utility density.
- Baseline mode.
- Intent-aware mode.
- Cold start.
- Captura progresiva.
- Seguridad.
- Métricas.
- Experimento de validación.
- Roadmap del producto.

Este documento no sustituye esa definición.

La arquitectura debe considerarse inválida si implementa un sistema que contradiga el contrato conceptual.

En caso de conflicto, la prioridad será:

```text
1. Evidencia real y reproducible del sistema
2. Rationale_v0.5.md
3. ADRs aprobados
4. Arquitectura conceptual vigente
5. Planes de implementación
6. Código no documentado
```

El código nunca debe convertirse accidentalmente en la única explicación de una decisión arquitectónica.

---

# 2. Objetivo de la arquitectura 0.1

La arquitectura 0.1 debe permitir empezar a construir una primera vertical de Rationale sin comprometer prematuramente la implementación final.

Debe responder:

- Qué componentes necesita el sistema.
- Qué datos son canónicos.
- Qué datos son derivados.
- Cómo se integra con Codebase Memory.
- Cómo se conserva consistencia entre Git, Codebase Memory y Rationale.
- Cómo se recupera contexto con bajo costo.
- Cómo se instrumenta el producto.
- Cómo se prueba en su propio repositorio.
- Cómo se prueba en un monorepo real.
- Cómo se instala inicialmente para desarrollo.
- Qué deberá empaquetarse más adelante.
- Qué decisiones necesitan investigación antes de escribirse en código.

No debe intentar resolver todavía:

- El instalador universal definitivo.
- Una interfaz gráfica final.
- Una landing page.
- Sincronización remota entre organizaciones.
- Un servicio SaaS.
- Un marketplace.
- Una base de datos central.
- Facturación.
- Una extensión completa para cada IDE.
- Compatibilidad perfecta con todos los agentes.
- Un protocolo público estable.
- Linaje conceptual automático perfecto.

---

# 3. Orden obligatorio del proyecto

El orden de trabajo será:

```text
1. Consolidar concepto
2. Analizar Codebase Memory
3. Analizar entorno local
4. Seleccionar lenguaje y toolchain
5. Construir vertical mínima
6. Instrumentar
7. Dogfood en Rationale
8. Ejecutar piloto en monorepo real
9. Corregir arquitectura
10. Completar herramienta
11. Empaquetar macOS, Linux y Windows
12. Diseñar experiencia de instalación
13. Publicar documentación de usuario
14. Crear landing page
```

La landing page no forma parte de la primera construcción.

El empaquetado multiplataforma tampoco debe bloquear la validación del núcleo.

Primero se debe demostrar que la herramienta:

- Recupera contexto correcto.
- Reduce contexto manual.
- No introduce falsedades.
- Respeta revisiones.
- Se integra de forma útil con Codebase Memory.
- Mejora resultados reales de agentes.

---

# 4. Restricciones fundamentales

## 4.1 Local-first

El núcleo debe poder ejecutarse localmente.

No deberá requerir obligatoriamente:

- API propia.
- Servidor remoto.
- Base de datos administrada.
- Servicio de embeddings.
- Cuenta de Rationale.
- Docker.
- Clúster.
- Telemetría central.
- Licencia de pago.

Los agentes externos utilizados para programar pueden tener sus propios costos o suscripciones.

Esos costos no deben transformarse en una dependencia del runtime de Rationale.

## 4.2 Sin LLM embebido obligatorio

Rationale no necesita incorporar un modelo de lenguaje para existir.

El modelo que ya usa el desarrollador será el consumidor del contexto.

El núcleo debe ser capaz de:

- Leer registros.
- Validar esquemas.
- Resolver alcance.
- Evaluar autoridad.
- Consultar proveedores.
- Construir paquetes.
- Aplicar presupuestos.
- Detectar inconsistencias.
- Emitir métricas.

sin realizar una llamada externa a un LLM.

Un modelo local o remoto puede utilizarse de manera opcional para:

- Proponer un resumen.
- Proponer Claims.
- Proponer un Subject.
- Ayudar en arqueología.
- Clasificar candidatos.

Pero sus resultados deben entrar como `inferred` y nunca como autoridad automática.

## 4.3 Offline después de instalar dependencias

El flujo principal debe poder funcionar sin conexión cuando:

- El código está disponible localmente.
- Codebase Memory está instalado.
- Las dependencias necesarias fueron descargadas.
- El agente puede operar sin red o no necesita nuevas llamadas externas.

## 4.4 Modularidad

Cada componente debe tener:

- Responsabilidad clara.
- Contrato explícito.
- Pruebas propias.
- Dependencias dirigidas.
- Capacidad de reemplazo.

La modularidad no significa crear decenas de paquetes prematuramente.

La implementación inicial deberá preferir un monolito modular.

## 4.5 Escalabilidad razonable

La primera meta no es indexar la totalidad del software mundial.

Debe escalar correctamente para:

- Repositorios pequeños.
- Monorepos medianos.
- Proyectos con múltiples paquetes.
- Un proyecto empresarial real.
- Varios agentes locales.
- Miles de registros causales a largo plazo.

La arquitectura debe medir antes de prometer.

## 4.6 Fallar con humildad

Cuando no exista cobertura suficiente, Rationale debe decirlo.

No puede convertir:

```text
No encontré una relación.
```

en:

```text
La relación no existe.
```

No puede convertir:

```text
El índice está atrasado.
```

en:

```text
La decisión sigue vigente.
```

---

# 5. Entorno principal de desarrollo

El entorno principal conocido es:

```text
Equipo: MacBook Air
Chip: Apple M4
Arquitectura: arm64 / Apple Silicon
Memoria: 16 GB RAM
Sistema operativo: macOS
```

La versión exacta de macOS y el almacenamiento disponible deben registrarse al iniciar el repositorio.

El perfil no debe guardar:

- Número de serie.
- Hardware UUID.
- Identificadores privados.
- Rutas personales innecesarias.
- Tokens.
- Credenciales.

## 5.1 Comandos de inventario

```bash
system_profiler SPHardwareDataType
sw_vers
uname -a
uname -m
sysctl -n hw.memsize
sysctl -n hw.ncpu
df -h
git --version
clang --version
xcode-select -p
```

Se deberá crear un script reproducible:

```text
scripts/dev/collect-environment.sh
```

El script producirá:

```text
.rationale-local/environment.json
```

Esta carpeta estará en `.gitignore`.

Una versión anonimizada podrá documentarse en:

```text
docs/environment/reference-development-machine.md
```

## 5.2 Implicaciones para el diseño

En una MacBook Air M4 con 16 GB:

- La arquitectura debe evitar procesos residentes innecesarios.
- No debe duplicar índices completos en memoria.
- Las pruebas de gran escala deben tener límites.
- Los benchmarks deben registrar memoria pico.
- El daemon, si existe, debe ser opcional y austero.
- Las operaciones frecuentes deben usar caché local.
- El sistema debe soportar Apple Silicon desde el inicio.
- El desarrollo inicial puede priorizar macOS arm64.
- La implementación no debe usar APIs exclusivas de macOS en el núcleo.

---

# 6. Codebase Memory como objeto de investigación

Codebase Memory no será tratado únicamente como una dependencia.

Será también un sistema que debe estudiarse.

El repositorio oficial deberá clonarse para comprender:

- Cómo descubre proyectos.
- Cómo identifica workspaces.
- Cómo almacena el grafo.
- Cómo expone MCP.
- Cómo ejecuta CLI.
- Cómo coordina daemon y watchers.
- Cómo representa revisiones.
- Cómo reporta cobertura.
- Cómo resuelve símbolos.
- Cómo calcula impacto.
- Cómo maneja monorepos.
- Cómo instala configuraciones de agentes.
- Cómo empaqueta binarios.
- Cómo protege stdout de MCP.
- Cómo implementa deadlines.
- Cómo falla cuando no tiene información.

## 6.1 Estructura del workspace de investigación

No se debe copiar Codebase Memory dentro del código fuente de Rationale como una dependencia accidental.

Se recomienda:

```text
rationale-lab/
├── rationale/
├── upstream/
│   └── codebase-memory-mcp/
├── pilots/
│   └── work-monorepo/
└── datasets/
    └── historical-cases/
```

`upstream/codebase-memory-mcp/` será:

- Un clon independiente.
- De solo lectura para el trabajo normal.
- Fijado a un commit.
- Actualizable conscientemente.
- No incluido automáticamente en los releases de Rationale.

La revisión analizada se registrará en:

```text
docs/research/codebase-memory/source-lock.yaml
```

Ejemplo:

```yaml
repository: DeusData/codebase-memory-mcp
branch: main
commit: <sha>
analyzed_at: 2026-07-24
binary_version: <detected>
```

## 6.2 Dos formas de usar Codebase Memory durante el desarrollo

### Binario publicado

Se utilizará para:

- Indexar Rationale.
- Indexar el clon de Codebase Memory.
- Obtener una referencia de comportamiento estable.
- Evitar que un build local modificado contamine la observación.
- Usar sus herramientas desde Claude Code, Codex u otros agentes.

### Build desde código fuente

Se utilizará para:

- Entender su arquitectura.
- Ejecutar su suite.
- Reproducir problemas.
- Leer implementaciones.
- Confirmar contratos.
- Probar compatibilidad.
- Investigar rendimiento.
- Comparar CLI contra MCP.

Ambos resultados deben distinguirse.

## 6.3 Bootstrap de investigación

```bash
mkdir -p ../upstream
git clone https://github.com/DeusData/codebase-memory-mcp.git ../upstream/codebase-memory-mcp
cd ../upstream/codebase-memory-mcp
git rev-parse HEAD
scripts/build.sh
make -f Makefile.cbm test
```

Los comandos exactos pueden cambiar.

Los agentes deben leer primero la documentación actual del repositorio.

## 6.4 Codebase Memory se indexará a sí mismo

El análisis inicial debe incluir:

```text
A. Indexar codebase-memory-mcp con el binario oficial.
B. Consultar su arquitectura.
C. Consultar los módulos MCP, store, daemon, watcher, pipeline y CLI.
D. Comparar los resultados estructurales contra el código.
E. Registrar errores, omisiones o cobertura parcial.
```

Esto permitirá observar:

- Qué tan confiable es el proveedor.
- Qué metadatos entrega.
- Qué llamadas son más útiles.
- Qué llamadas son costosas.
- Qué limitaciones existen.
- Qué datos necesita el adaptador.

## 6.5 Rationale también será indexado desde el primer día

Antes de cada cambio no trivial, los agentes deberán poder consultar:

- Arquitectura actual.
- Símbolos.
- Dependencias.
- Impacto.
- Cobertura.
- Cambios no confirmados.

Codebase Memory funcionará como soporte de construcción incluso antes de que Rationale pueda utilizarse a sí mismo.

## 6.6 Hechos observados que deben revalidarse

En la revisión analizada durante la creación de este documento, Codebase Memory declara:

- Implementación principal en C.
- Binario estático para macOS, Linux y Windows.
- Uso local.
- SQLite.
- Tree-sitter.
- MCP y CLI.
- Daemon compartido.
- Watchers.
- Soporte de workspaces y relaciones entre paquetes con límites de cobertura.
- Gestión de ADR.
- Detección de cambios.
- Índice derivado.
- Hooks no bloqueantes.
- Distribución sin runtime obligatorio.

Su build actual muestra módulos separados para:

- Foundation.
- Store.
- Cypher.
- MCP.
- Daemon.
- Discovery.
- Pipeline.
- Semantic.
- Watcher.
- Git.
- CLI.
- UI.
- Tests y reproducciones de bugs.

Estos hechos sirven para preparar preguntas.

No autorizan a copiar su arquitectura sin evaluación.

---

# 7. Protocolo de análisis de Codebase Memory

Antes de congelar la arquitectura de implementación, se deben producir los siguientes documentos.

```text
docs/research/codebase-memory/
├── 00-source-lock.md
├── 01-build-and-test.md
├── 02-module-map.md
├── 03-mcp-contracts.md
├── 04-cli-contracts.md
├── 05-revision-and-coverage.md
├── 06-daemon-and-watcher.md
├── 07-storage-and-cache.md
├── 08-workspaces-and-monorepos.md
├── 09-installation-and-agents.md
├── 10-failure-modes.md
├── 11-performance-observations.md
└── 12-integration-recommendation.md
```

Cada análisis deberá contener:

```text
Observed:
Qué hace realmente.

Claimed:
Qué promete la documentación.

Verified:
Qué fue reproducido.

Unknown:
Qué todavía no está claro.

Risk:
Qué podría afectar a Rationale.

Decision impact:
Qué decisión arquitectónica depende de esto.
```

## 7.1 Preguntas obligatorias

- ¿Cuál es la forma más estable de invocarlo desde otro proceso?
- ¿MCP cliente-a-servidor es mejor que CLI subprocess para la primera vertical?
- ¿Cómo reporta el proyecto indexado?
- ¿Cómo reporta la revisión indexada?
- ¿Cómo distingue working tree de HEAD?
- ¿Cómo reporta cobertura parcial?
- ¿Puede consultarse sin iniciar daemon?
- ¿Qué latencia tiene el CLI frente a una sesión MCP persistente?
- ¿Qué ocurre si dos agentes lo usan?
- ¿Qué ocurre si se actualiza el binario durante una sesión?
- ¿Qué datos pueden considerarse públicos?
- ¿Qué datos pertenecen a internals inestables?
- ¿Qué límites reales tiene en monorepos?
- ¿Qué contratos pueden testearse sin leer su SQLite directamente?
- ¿Qué compatibilidad existe con Windows y Linux?
- ¿Cómo instala hooks e instrucciones?
- ¿Cómo desinstala lo que modifica?
- ¿Qué puede reutilizar Rationale como patrón?
- ¿Qué debe permanecer completamente desacoplado?

## 7.2 Regla de integración

Rationale no deberá:

- Leer directamente tablas internas de Codebase Memory.
- Importar headers internos.
- Asumir rutas privadas.
- Copiar su cache.
- Compartir locks internos.
- Enlazar contra su binario como librería sin un contrato aprobado.
- Depender de nombres de nodos no documentados sin capability negotiation.

La frontera preferida será pública:

```text
MCP
o
CLI estructurado
```

La selección se decidirá mediante un spike.

---

# 8. Selección del lenguaje

El lenguaje del núcleo no está decidido en la arquitectura 0.1.

Esto es intencional.

No debe elegirse únicamente porque:

- Codebase Memory usa C.
- Existe un SDK popular.
- Un agente escribe mejor cierto lenguaje.
- El prototipo se siente rápido.
- Una persona prefiere un lenguaje.

Debe elegirse por evidencia.

## 8.1 Candidatos iniciales

La investigación deberá evaluar al menos:

- Rust.
- Go.
- C.
- TypeScript/Node.js para prototipo o tooling.
- Otra opción únicamente si existe una razón concreta.

Python puede utilizarse para:

- Experimentos.
- Harnesses.
- Análisis de datos.
- Scripts.

No debe convertirse automáticamente en el núcleo distribuido.

## 8.2 Criterios ponderados

```text
20% Seguridad de memoria y confiabilidad
15% Distribución como binario
15% Rendimiento y latencia
10% MCP y JSON-RPC
10% SQLite y filesystem
10% Compatibilidad macOS/Linux/Windows
10% Mantenibilidad con agentes
5%  Tiempo de compilación y desarrollo
5%  Interoperabilidad con procesos C
```

Cada candidato debe probar:

- Servidor MCP mínimo.
- Cliente hacia Codebase Memory o wrapper CLI.
- Lectura y validación de registros.
- SQLite.
- File locking.
- Subprocess.
- Cancelación.
- Deadline.
- Build arm64.
- Cross-compilation o estrategia de CI.
- Binary size.
- Startup time.
- Memoria.
- Test tooling.
- Fuzzing o property tests.
- Packaging.

## 8.3 Entregables

```text
docs/research/language/
├── candidates.md
├── benchmark-results.json
├── compatibility-matrix.md
├── spike-notes.md
└── ADR-0001-core-language.md
```

La decisión debe registrar:

- Evidencia.
- Tradeoffs.
- Alternativas.
- Por qué se descartaron.
- Riesgo de reversión.
- Fecha de revisión.

## 8.4 Arquitectura independiente del lenguaje

Hasta aprobar el ADR:

- Los nombres de módulos serán conceptuales.
- Las interfaces usarán pseudocódigo.
- No se definirán crates, packages o modules definitivos.
- Los scripts no asumirán un package manager.
- El CI tendrá placeholders.
- La estructura evitará acoplar documentación a Rust, Go o C.

---

# 9. Vista general del sistema

```text
┌──────────────────────────────────────────────────────┐
│ Coding Agent                                         │
│ Claude Code / Codex / otro cliente MCP               │
└───────────────────────────┬──────────────────────────┘
                            │
                            │ MCP / CLI / hook surface
                            ▼
┌──────────────────────────────────────────────────────┐
│ Rationale Application Boundary                       │
│                                                      │
│ - MCP server                                         │
│ - CLI                                                │
│ - configuration                                      │
│ - health                                             │
│ - output formatting                                  │
└───────────────────────────┬──────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────┐
│ Context Coordination                                 │
│                                                      │
│ - revision coordinator                               │
│ - workspace/scope resolver                           │
│ - context compiler                                   │
│ - trust/authority/policy evaluator                   │
│ - subject resolver                                   │
│ - binding resolver                                   │
│ - capture/finalize lifecycle                         │
└───────────────┬───────────────────┬──────────────────┘
                │                   │
                ▼                   ▼
┌──────────────────────────┐  ┌────────────────────────┐
│ Structural Provider      │  │ Rationale Data         │
│ Adapter                   │  │                        │
│                          │  │ Canonical Git records  │
│ Codebase Memory          │  │ Local derived index    │
│ Future providers         │  │ Session state          │
└───────────────┬──────────┘  └────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────┐
│ Git + filesystem + tests + external evidence refs    │
└──────────────────────────────────────────────────────┘
```

---

# 10. Capas de datos

## 10.1 Capa canónica compartida

Ubicación:

```text
<project-root>/.rationale/
```

Contendrá únicamente datos portables y revisables.

```text
.rationale/
├── config.yaml
├── subjects/
├── records/
├── bindings/
├── approvals/
├── schemas/
└── migrations/
```

Características:

- Versionada en Git.
- Revisable en PR.
- Legible sin la herramienta.
- Sin cache.
- Sin embeddings obligatorios.
- Sin paths absolutos de una máquina.
- Sin tokens.
- Sin datos secretos innecesarios.
- Con schema version.

## 10.2 Capa derivada local

Ubicación conceptual:

```text
<user-cache>/rationale/projects/<project-id>/
```

En macOS deberá respetarse una ruta apropiada.

La decisión exacta se realizará durante implementación.

Podría mapearse a:

```text
~/Library/Caches/Rationale/
```

o a un root configurable compatible con XDG.

Contendrá:

- SQLite.
- Resoluciones de bindings.
- Índices FTS.
- Scores.
- Cobertura.
- Provider capabilities.
- Revisión indexada.
- Cache de paquetes.
- Métricas locales.
- Estado de hooks.
- Locks.
- Logs privados.

Será:

- Regenerable.
- No versionada.
- Específica de máquina.
- Borrable.
- Migrable.
- Con permisos restrictivos.

## 10.3 Capa efímera

Contendrá:

- Intent actual.
- Targets.
- Hipótesis.
- Señales.
- Context packet.
- Tool call state.
- Drafts de Records.
- Resultado provisional de evaluación.

Debe tener:

- TTL.
- Identificador de sesión.
- Revisión base.
- Limpieza segura.
- No promoción automática a conocimiento aprobado.

---

# 11. Módulos conceptuales

## 11.1 Application Boundary

Responsabilidades:

- Exponer MCP.
- Exponer CLI.
- Validar argumentos.
- Negociar versión.
- Formatear respuestas.
- Aplicar deadlines externos.
- Mantener stdout de MCP limpio.
- Enviar logs a stderr o archivo.
- Traducir errores internos a estados explícitos.

No contiene lógica de dominio.

## 11.2 Configuration

Responsabilidades:

- Encontrar project root.
- Leer `.rationale/config.yaml`.
- Resolver cache root.
- Aplicar límites.
- Leer modo de operación.
- Activar proveedores.
- Configurar privacidad.
- Configurar políticas de bloqueo.
- Configurar hooks opcionales.

Debe soportar:

```text
project config
user config
environment overrides
safe defaults
```

La precedencia debe documentarse.

## 11.3 Project and Workspace Discovery

Responsabilidades:

- Detectar raíz Git.
- Detectar monorepo.
- Identificar packages.
- Resolver ProjectScope.
- Normalizar paths.
- Distinguir workspace root de package.
- Mapear target a scopes.
- Evitar escapar del root.

Debe comprender que:

```text
repository != package
package != domain
domain != subject
```

## 11.4 Revision Coordinator

Responsabilidades:

- Leer Git HEAD.
- Identificar working tree.
- Obtener revisión del proveedor.
- Obtener revisión de assessments.
- Calcular consistency status.
- Impedir respuestas que aparenten exactitud cuando las revisiones no coinciden.
- Activar fast path o revalidación.

Entrada conceptual:

```json
{
  "git_head": "...",
  "working_tree_hash": "...",
  "provider_revision": "...",
  "rationale_assessment_revision": "..."
}
```

Salida:

```text
exact
working-tree-ahead
provider-behind
rationale-behind
mixed
unknown
```

## 11.5 Structural Provider Adapter

Responsabilidades:

- Capability negotiation.
- Resolver targets.
- Obtener relaciones.
- Obtener impacto.
- Obtener cambios.
- Obtener coverage.
- Obtener provider revision.
- Producir evidencia estructural.
- Aplicar timeout.
- Normalizar errores.

Contrato conceptual:

```text
capabilities()
health()
indexed_revision()
resolve_target()
relationships()
impact()
changed_targets()
coverage()
architecture()
```

Cada respuesta debe incluir:

```text
provider
provider_version
capability
revision
coverage
status
data
warnings
latency
```

## 11.6 Canonical Store

Responsabilidades:

- Leer registros.
- Validar schemas.
- Escribir cambios atómicos.
- Mantener schema versions.
- Proteger IDs.
- Mantener supersession.
- Resolver approvals.
- No mezclar cache.

## 11.7 Derived Index

Responsabilidades:

- Indexar Records.
- FTS.
- Aliases.
- Scope paths.
- Binding resolutions.
- Candidate retrieval.
- Deduplicación.
- Cache de assessments.
- Invalidación por revisión.

La base derivada nunca puede ser la única copia de una decisión.

## 11.8 Subject Resolver

Orden inicial:

```text
1. Exact ID
2. Alias
3. Explicit binding
4. Scope compatibility
5. FTS
6. Optional semantic candidates
```

No puede:

- Crear un Subject automáticamente por similitud.
- Fusionar Subjects automáticamente.
- Tratar un embedding como identidad.

Cuando proponga uno nuevo y haya candidatos similares, deberá requerir `novelty_reason`.

## 11.9 Binding Resolver

Responsabilidades:

- Leer declaraciones portables.
- Resolverlas contra proveedor actual.
- Registrar revisión.
- Registrar coverage.
- Detectar stale/unresolved.
- Mantener historial derivado.
- Resolver package/workspace scope.
- No editar la declaración canónica de forma silenciosa.

## 11.10 Trust, Authority and Policy Evaluator

Responsabilidades:

- Diferenciar observado, declarado e inferido.
- Resolver Approval.
- Verificar autoridad por dominio.
- Calcular aplicabilidad.
- Identificar contradicciones.
- Decidir atención.
- Aplicar reglas de bloqueo.

Solo puede bloquear cuando:

```text
critical
AND approved-or-policy
AND active
AND exact-current-linkage
AND deterministic-contradiction
```

## 11.11 Context Compiler

Responsabilidades:

- Interpretar target.
- Interpretar intención si existe.
- Recuperar candidatos.
- Filtrar por alcance.
- Filtrar por aplicabilidad.
- Priorizar restricciones.
- Deduplicar.
- Respetar token budget.
- Emitir incertidumbre.
- Registrar por qué incluyó cada elemento.

No debe generar un ensayo.

Debe generar un paquete operativo.

## 11.12 Capture and Finalization

Responsabilidades:

- Capturar hechos mecánicos.
- Recibir señales.
- Comparar diff.
- Proponer Records.
- Proponer Subjects.
- Solicitar confirmación mínima.
- Guardar evidence.
- Actualizar bindings.
- No aprobar sus propias inferencias.

## 11.13 Lifecycle Accelerators

Incluye:

- Git hooks.
- File watcher.
- Daemon.
- Agent hooks.
- Post-commit detection.

Son opcionales.

La corrección principal debe depender del revision gate, no de que un hook siempre funcione.

## 11.14 Evaluation and Telemetry

Responsabilidades:

- Registrar latencia.
- Registrar tokens cuando estén disponibles.
- Registrar tamaño del packet.
- Registrar elementos.
- Registrar tool calls.
- Registrar resultado de tests.
- Registrar condición experimental.
- Exportar datos para análisis.
- No enviar datos automáticamente.

---

# 12. Interfaces públicas iniciales

La superficie pública inicial debe ser pequeña.

## `prepare_change`

Entrada:

- Targets.
- Intent opcional.
- Symptoms opcionales.
- Base revision.
- Budget.
- Scope hints.

Salida:

- Critical constraints.
- Conflicts.
- Decisions.
- Risks.
- Required validations.
- Affected areas.
- Coverage.
- Revision consistency.
- Trust.
- Additional history count.

## `explain_target`

Explica:

- Subject.
- Decisiones activas.
- Motivo.
- Evidencia.
- Restricciones.
- Incertidumbre.
- Revisión.

## `finalize_change`

Entrada:

- Base.
- Head o working tree.
- Tests.
- Signals.
- Optional human confirmations.

Salida:

- Mechanical evidence.
- Proposed records.
- Proposed binding updates.
- Required reviews.
- Stored canonical changes.

## `review_record`

Permite:

- Aprobar.
- Disputar.
- Corregir.
- Supersede.
- Cambiar authority.
- Añadir evidence.

## `trace_rationale`

Recorre:

```text
target
→ binding
→ subject
→ record
→ decision
→ approval
→ evidence
→ validation
```

## `health`

Informa:

- Proyecto.
- Git revision.
- Provider.
- Provider revision.
- Cache.
- Coverage.
- Schema.
- Pending migrations.
- Stale assessments.
- Hook status.
- Latency summary.

Las operaciones administrativas adicionales pueden existir en CLI sin inflar MCP.

---

# 13. Flujos principales

## 13.1 Baseline fast path

```text
Agent reads/searches target
        ↓
Client hook or explicit baseline request
        ↓
Resolve project and target cheaply
        ↓
Check revision fingerprint
        ↓
Read precomputed critical bindings
        ↓
Return compact context or no-op
```

Restricciones:

- Sin embeddings.
- Sin arqueología.
- Sin LLM.
- Sin reindex completo.
- Sin llamadas largas.
- Fail open.
- No bloquear lectura.

Objetivos iniciales:

```text
P50 warm ≤ 50 ms
P95 warm ≤ 150 ms
Hard deadline ≤ 250 ms
```

Son objetivos del piloto, no garantías públicas.

## 13.2 Intent-aware preflight

```text
Agent expresses task
        ↓
prepare_change
        ↓
Revision coordination
        ↓
Structural provider query
        ↓
Subject + scope resolution
        ↓
Policy evaluation
        ↓
Ranking and budget
        ↓
Context packet
```

Puede ejecutar análisis más costoso.

Debe seguir siendo acotado.

Objetivo provisional:

```text
P95 warm, excluding indexing ≤ 2 s
```

Debe medirse antes de prometer.

## 13.3 Finalize

```text
Implementation complete
        ↓
Collect diff and tests
        ↓
Compare against preflight
        ↓
Extract mechanical evidence
        ↓
Generate proposals
        ↓
Resolve duplicate subjects
        ↓
Ask minimal confirmation
        ↓
Write canonical files atomically
        ↓
Reindex derived state
```

## 13.4 Commit fuera del flujo

```text
Human commits without finalize
        ↓
Git revision advances
        ↓
Optional hook marks dirty
        ↓
Next query runs revision gate
        ↓
Assessments become behind
        ↓
Rationale degrades confidence
        ↓
Selective revalidation
```

No debe fingir que está actualizado.

## 13.5 Provider unavailable

```text
Codebase Memory unavailable
        ↓
Use exact canonical bindings only
        ↓
Report provider unavailable
        ↓
Do not claim structural completeness
        ↓
Do not issue new deterministic block based on absent structure
```

---

# 14. Monorepos

La raíz canónica inicial será una sola:

```text
<monorepo-root>/.rationale/
```

Los records no se duplicarán por package.

Cada Subject y Binding puede declarar:

```yaml
scope:
  root: .
  includes:
    - apps/dashboard/**
    - services/api/**
    - packages/auth/**
  excludes:
    - examples/**
```

La recuperación cruzada requiere un camino de relevancia.

Ejemplo:

```text
backend authorization decision
→ API contract
→ frontend permission rendering
```

No basta con estar en el mismo repositorio.

El contexto compiler debe aplicar:

- Package overlap.
- Domain relationship.
- Explicit Subject dependency.
- Structural provider relationship.
- Intent.
- Severity.
- Budget.

## 14.1 Limitación del proveedor

La arquitectura no asumirá que Codebase Memory resuelve perfectamente cada workspace.

Toda relación entre paquetes deberá incluir coverage y provider revision.

## 14.2 Piloto real

El monorepo del trabajo será el principal entorno de validación después de dogfooding.

Antes de usarlo:

- Se anonimizarán resultados compartidos.
- No se copiará código sensible a datasets públicos.
- Los records con información empresarial tendrán sensitivity.
- La evaluación podrá almacenar hashes y métricas en lugar de contenido.

---

# 15. Seguridad

## 15.1 Repository content is data

Todo texto del repositorio debe tratarse como datos no confiables.

Incluye:

- Nombres.
- Comentarios.
- Records.
- Issues.
- Commits.
- Paths.
- Evidence.
- Metadata del proveedor.

Nunca se debe concatenar contenido arbitrario como instrucciones del sistema.

## 15.2 Sanitización

- Limitar longitud.
- Validar UTF-8.
- Eliminar controles.
- Escapar formatos.
- Separar metadata de instrucciones.
- Etiquetar contenido no confiable.
- Aplicar schema.

## 15.3 Paths

- Canonicalizar.
- Impedir traversal.
- No seguir symlinks fuera del root sin política.
- Proteger writes.
- Usar archivos temporales y rename atómico.
- Permisos owner-only en cache sensible.

## 15.4 Secrets

Rationale no debe indexar deliberadamente:

- `.env`.
- Tokens.
- Private keys.
- Credenciales.
- Dumps.
- Datos personales.

Debe respetar:

- `.gitignore`.
- Configuración adicional.
- Sensitivity.
- Redaction.

## 15.5 External skills

Toda skill externa deberá:

- Revisarse.
- Fijarse a versión o commit.
- Verificarse licencia.
- Inspeccionarse antes de ejecutar.
- Registrarse.
- No obtener permisos globales por defecto.

---

# 16. Observabilidad

El sistema deberá producir logs estructurados locales.

```json
{
  "timestamp": "...",
  "event": "prepare_change.completed",
  "project_id": "...",
  "revision": "...",
  "provider_revision": "...",
  "latency_ms": 84,
  "packet_tokens": 412,
  "candidate_count": 18,
  "selected_count": 4,
  "coverage": "partial",
  "status": "ok"
}
```

No debe incluir por defecto:

- Código.
- Prompt completo.
- Secrets.
- Texto sensible.
- Identidad personal.

Niveles:

```text
error
warn
info
debug
trace
```

Los eventos de evaluación tendrán un formato separado.

---

# 17. Rendimiento y recursos

## 17.1 Principio

Rationale no debe duplicar el trabajo estructural de Codebase Memory.

Su carga propia debe concentrarse en:

- Lectura.
- Validación.
- FTS.
- Joins pequeños.
- Ranking.
- Policy evaluation.
- Serialización.

## 17.2 Presupuestos provisionales

En la MacBook Air M4 de 16 GB:

```text
Baseline warm P95: ≤ 150 ms
Baseline hard deadline: ≤ 250 ms
Intent-aware warm P95: ≤ 2 s, sin index
Steady resident memory target: ≤ 300 MB
Canonical store: proporcional a Records, normalmente pequeño
Derived cache: configurable y regenerable
```

Estos valores son hipótesis.

Se revisarán después del piloto.

## 17.3 Backpressure

- Límites de concurrencia.
- Cancelación.
- Timeouts.
- Query budgets.
- Queue bounds.
- Cache cap.
- No spawn infinito de subprocesses.
- Lock por proyecto para writes.
- Reads concurrentes cuando sea seguro.

---

# 18. Costos

## 18.1 Costos obligatorios del núcleo

Objetivo:

```text
Costo obligatorio de infraestructura: $0
```

El núcleo deberá usar:

- Máquina local.
- Git.
- Filesystem.
- SQLite u otra dependencia embebida.
- Codebase Memory local.
- Dependencias open source compatibles.

## 18.2 Costos externos no controlados

Pueden existir:

- Suscripción de Claude Code.
- Uso de Codex u OpenAI.
- API tokens.
- CI fuera del free tier.
- Almacenamiento de artifacts.
- Firma de código.
- Notarización.
- Certificado de Windows.
- Dominio.
- Hosting de la landing page.

No serán dependencia del MVP local.

## 18.3 Inventario provisional de dependencias

Mientras el lenguaje no esté seleccionado, las dependencias se dividirán por función.

### Dependencias obligatorias para investigación y desarrollo inicial

| Dependencia | Propósito | Runtime de Rationale | Costo obligatorio |
|---|---|---:|---:|
| Git | Revisión, historial y colaboración | Sí | $0 |
| Codebase Memory | Proveedor estructural inicial | Sí para la integración inicial | $0 |
| Xcode Command Line Tools en macOS | Compiladores y herramientas base | No necesariamente | $0 |
| C compiler y C++ compiler | Construir y estudiar Codebase Memory | No necesariamente | $0 |
| zlib | Build actual de Codebase Memory | No necesariamente | $0 |
| Shell y herramientas POSIX | Scripts de bootstrap | Desarrollo | $0 |
| Agente MCP compatible | Consumir Rationale durante desarrollo | Externo | Variable/existente |

### Dependencias probables del núcleo, todavía no seleccionadas

| Capacidad | Tipo de dependencia esperada | Restricción |
|---|---|---|
| MCP / JSON-RPC | SDK o implementación pequeña | Debe ser mantenible y local |
| Persistencia derivada | SQLite embebido | Sin servidor |
| Serialización | YAML, JSON o ambos | Schema versioned |
| Schema validation | Librería local | Errores deterministas |
| Hashing | Implementación estándar | Sin servicio remoto |
| File locking | API portable | macOS, Linux y Windows |
| Logging | Estructurado y local | Sin telemetry obligatoria |
| CLI | Librería o estándar | Instalación simple |
| Testing | Framework del lenguaje | Unit, contract, property y fuzz |
| Compression | Opcional | Solo si la evidencia lo justifica |

### Dependencias de evaluación

Podrán usarse únicamente como tooling:

- Python para análisis estadístico.
- Scripts para bootstrap confidence intervals.
- Parsers NDJSON.
- Herramientas de gráficos.
- Fixtures y datasets locales.

No serán necesariamente parte del binario distribuido.

### Dependencias opcionales

- Ollama u otro modelo local.
- GitHub CLI.
- UI local.
- Integraciones específicas por IDE.
- Firma y notarización.
- CI remoto.

Ninguna opción podrá convertirse accidentalmente en requisito del núcleo.

## 18.4 Política de dependencias

Cada dependencia deberá registrar:

- Licencia.
- Versión.
- Tamaño.
- Riesgo.
- Motivo.
- Alternativas.
- CVEs conocidas.
- Si se puede vendorizar.
- Si requiere runtime.
- Si agrega llamadas externas.

Se mantendrá un inventario legible por máquina:

```text
docs/dependencies/inventory.yaml
```

Cada actualización de dependencia deberá:

1. Pasar tests.
2. Registrar cambio.
3. Revisar licencia.
4. Revisar advisories.
5. Medir impacto cuando afecte hot paths.
6. Poder revertirse.

---

# 19. Testing

## 19.1 Pirámide

```text
Unit
Contract
Integration
Property
Fuzz
Security
Performance
End-to-end
Evaluation
Cross-platform
```

## 19.2 Tests obligatorios

- Schema validation.
- Atomic writes.
- Migrations.
- Subject resolution.
- novelty_reason.
- Scope filtering.
- Revision consistency.
- Provider timeout.
- Provider unavailable.
- Partial coverage.
- Token budget.
- Deduplication.
- Critical blocking predicate.
- Prompt injection sanitization.
- Path traversal.
- Concurrent reads.
- Write locks.
- Cache rebuild.
- Monorepo cross-package relevance.
- Baseline deadline.
- Context packet determinism.

## 19.3 Contract tests con Codebase Memory

Se crearán fixtures propios.

No dependerán solamente del repositorio upstream.

```text
tests/fixtures/codebase-memory/
├── simple-repo/
├── monorepo/
├── moved-symbol/
├── partial-coverage/
├── stale-index/
└── provider-unavailable/
```

## 19.4 Golden packets

Para inputs fijos:

- El packet debe ser estable.
- El orden debe ser determinista.
- El budget debe respetarse.
- La incertidumbre debe preservarse.

---

# 20. Evaluación del producto

La arquitectura incluirá instrumentación desde la primera vertical.

No se agregará al final.

## 20.1 Unidad

La unidad es:

```text
task execution
```

Incluye:

- Modelo/agente.
- Condición.
- Prompt inicial.
- Context packet.
- Tool calls.
- Revisión.
- Resultado.
- Tests.
- Tokens.
- Latencia.
- Intervención humana.

## 20.2 Condiciones

```text
A. Código + Git
B. Documentación tradicional
C. Codebase Memory
D. Codebase Memory + Rationale
E. Prompt de experto
```

## 20.3 Autoinstrumentación

Los agentes que construyan el proyecto podrán registrar:

- Herramientas invocadas.
- Files read.
- Context recibido.
- Errores.
- Intentos.
- Tests.
- Duración.
- Tokens si el cliente los expone.

Si los tokens no están disponibles, se registrarán proxies:

- Caracteres.
- Palabras.
- Bytes.
- Tool result size.
- Número de mensajes.

## 20.4 Límite epistemológico

El mismo agente que implementó una función no puede ser la única entidad que puntúe su calidad.

Se requiere al menos una combinación de:

- Tests deterministas.
- Evaluador separado.
- Otra ejecución.
- Otro modelo.
- Revisión humana.
- Rubrica ciega.
- Ground truth predefinido.

## 20.5 Éxito

La arquitectura funciona si permite medir:

- Critical constraint recall.
- Context precision.
- Harmful context rate.
- Context utility density.
- Total tokens to successful completion.
- Manual context reduction.
- Bug reintroduction.
- Baseline latency.
- False blocks.
- Coverage and revision consistency.

---

# 21. Estructura propuesta del repositorio

Hasta decidir lenguaje:

```text
rationale/
├── README.md
├── LICENSE
├── AGENTS.md
├── Rationale_v0.5.md
├── .rationale/
├── docs/
│   ├── architecture/
│   ├── adr/
│   ├── research/
│   │   ├── codebase-memory/
│   │   └── language/
│   ├── experiments/
│   ├── environment/
│   ├── runbooks/
│   ├── security/
│   └── product/
├── schemas/
├── src/
│   ├── application/
│   ├── configuration/
│   ├── project/
│   ├── revision/
│   ├── providers/
│   ├── storage/
│   ├── subjects/
│   ├── bindings/
│   ├── policy/
│   ├── retrieval/
│   ├── capture/
│   └── evaluation/
├── tests/
│   ├── unit/
│   ├── contract/
│   ├── integration/
│   ├── security/
│   ├── performance/
│   ├── end-to-end/
│   └── evaluation/
├── fixtures/
├── scripts/
│   ├── dev/
│   ├── ci/
│   ├── research/
│   └── release/
├── tools/
└── .rationale-local/      # ignored
```

La estructura concreta se adaptará al lenguaje aprobado.

---

# 22. ADRs iniciales obligatorios

```text
ADR-0001 Core language and toolchain
ADR-0002 Codebase Memory transport
ADR-0003 Canonical serialization
ADR-0004 Derived database
ADR-0005 Cache root and project identity
ADR-0006 Revision fingerprint
ADR-0007 MCP SDK and protocol version
ADR-0008 Concurrency and locking
ADR-0009 Baseline integration surfaces
ADR-0010 Packaging strategy
ADR-0011 Licensing and dependency policy
ADR-0012 Telemetry and privacy
```

Ningún ADR puede decir solamente:

```text
Elegimos X porque es rápido.
```

Debe contener evidencia.

---

# 23. Fases de implementación

## Fase A — Repository bootstrap

- Crear repo.
- Copiar documentos.
- Crear AGENTS.md.
- Crear estructura docs.
- Crear templates.
- Configurar Git.
- Configurar Codebase Memory.
- Capturar entorno.
- No elegir lenguaje aún.

## Fase B — Upstream analysis

- Clonar Codebase Memory.
- Build.
- Tests.
- Index self.
- Index Rationale.
- Documentar contratos.
- Medir CLI vs MCP.
- Analizar monorepos.
- Producir recomendación.

## Fase C — Language spike

- Implementar prototipos.
- Medir.
- ADR-0001.
- Crear toolchain.

## Fase D — Vertical slice

Debe hacer:

```text
init
read canonical record
resolve exact target
query Codebase Memory
check revisions
return one compact constraint
```

No debe incluir toda la visión.

## Fase E — Local store and compiler

- Subjects.
- Records.
- Bindings.
- Approvals.
- Assessments.
- FTS.
- Budget.
- Packets.

## Fase F — Capture

- Signals.
- Diff.
- Tests.
- finalize_change.
- Confirmation.

## Fase G — Dogfood

Rationale se instalará en Rationale.

Se usarán sus propios Records para construirlo.

La herramienta no podrá aprobar automáticamente sus decisiones fundacionales.

## Fase H — Monorepo pilot

- Instalar en proyecto real.
- Seleccionar 20–30 cambios históricos.
- Ejecutar condiciones.
- Medir.
- Corregir.

## Fase I — Architecture 0.2

Después de evidencia:

- Actualizar módulos.
- Cerrar decisiones.
- Eliminar componentes innecesarios.
- Estabilizar APIs internas.

## Fase J — Packaging

Solo después de validación:

- macOS arm64.
- macOS amd64 si se mantiene.
- Linux amd64.
- Linux arm64.
- Windows amd64.
- Checksums.
- Install.
- Update.
- Uninstall.
- Rollback.

## Fase K — Distribution experience

- Documentación de usuario.
- Quick start.
- Troubleshooting.
- Security.
- Landing page.
- Releases.

---

# 24. Instalación conceptual

## Desarrollo

```text
clone rationale
install toolchain
install Codebase Memory
index project
run health
run tests
start agent
```

## Proyecto consumidor

La experiencia final deseada:

```text
install rationale
cd project
rationale init
rationale provider add codebase-memory
rationale install-agent
rationale health
```

Los comandos son conceptuales.

No se implementarán hasta definir CLI.

## Archivos modificados

El instalador debe registrar exactamente:

- Binario.
- Config.
- Hooks.
- Agent entries.
- Skills.
- Cache.
- PATH changes.

Uninstall debe poder revertir lo que instaló.

---

# 25. Trazabilidad con Rationale 0.5

| Requisito conceptual | Componente |
|---|---|
| Contexto causal | Canonical Store + Context Compiler |
| Humildad epistemológica | Trust Evaluator |
| Autoridad | Approval + Policy Evaluator |
| Revisión | Revision Coordinator |
| Concept-first | Subject Resolver |
| Code-anchored | Binding Resolver |
| Monorepo | Workspace Discovery + Scope |
| Context budget | Context Compiler |
| Baseline | Fast path |
| Intent-aware | prepare_change |
| Cold start | Capture + partial coverage |
| Legacy archaeology | Future provider/evidence workflow |
| No false block | Blocking predicate |
| Prompt injection | Security boundary |
| Shared project memory | `.rationale/` |
| Local machine state | Derived Index |
| Agent forgetfulness | Hooks + baseline + explicit preflight |
| Human commits | Revision gate |
| Deduplication | Subject Resolver + novelty_reason |
| Token savings | Evaluation instrumentation |
| Senior continuity | Records + evaluation |
| Progressive adoption | Optional levels and forward capture |
| Provider fallibility | Coverage + capability negotiation |
| No mandatory cloud | Local-first |
| Cross-platform | Packaging phase |
| Team collaboration | Git canonical layer |
| Metrics | Evaluation module |

---

# 26. Criterios de salida de arquitectura 0.1

La arquitectura 0.1 podrá considerarse lista para implementar cuando:

- El documento conceptual 0.5 está versionado.
- El repositorio de Codebase Memory fue clonado y fijado.
- El build upstream funciona en la MacBook Air M4.
- Se ejecutaron sus tests relevantes.
- Codebase Memory indexó su propio repo.
- Codebase Memory indexó Rationale.
- Se documentó MCP vs CLI.
- Se documentó revision and coverage.
- Se documentaron límites de monorepo.
- Se completaron spikes de lenguaje.
- ADR-0001 fue aprobado.
- Existe una vertical slice planificada.
- Existe instrumentation schema.
- Existe security baseline.
- Existe proceso de agentes.
- No hay dependencia pagada obligatoria.

---

# 27. Lo que ningún agente debe hacer

- Empezar la landing page.
- Elegir lenguaje sin ADR.
- Copiar internals de Codebase Memory.
- Leer su SQLite privado como contrato.
- Introducir un SaaS.
- Agregar embeddings remotos obligatorios.
- Crear veinte servicios.
- Crear un daemon antes de medir necesidad.
- Bloquear cambios con inferencias.
- Aprobar automáticamente Records.
- Ocultar cobertura parcial.
- Declarar éxito usando únicamente opinión del mismo agente.
- Saltarse documentación.
- Cambiar arquitectura sin ADR.
- Empaquetar antes de validar el núcleo.
- Optimizar antes de instrumentar.

---

# 28. Preguntas abiertas

- ¿MCP client interno o CLI subprocess?
- ¿Un proceso por sesión o daemon compartido?
- ¿Qué revisión exacta ofrece Codebase Memory?
- ¿Cómo representa working tree?
- ¿Qué capability falta?
- ¿Rust, Go, C u otra?
- ¿YAML, JSON o combinación?
- ¿SQLite puro o abstracción?
- ¿Cómo se calcula project ID?
- ¿Cómo se coordinan varios agentes?
- ¿Qué hooks soporta cada cliente?
- ¿Qué métricas de tokens son accesibles?
- ¿Cómo se firman releases?
- ¿Qué parte del cache puede compartirse?
- ¿Cómo se prueba Windows antes de empaquetar?
- ¿Qué Records son sensibles?
- ¿Qué paquete mínimo convence en el piloto?

Estas preguntas son parte de la arquitectura.

No son una señal de que falte trabajo.

Son la lista de trabajo que evita fingir certeza.

---

# 29. Definición de la arquitectura 0.1

> Rationale será inicialmente un sistema local, modular y auditable que expone MCP y CLI, mantiene una memoria canónica versionada en Git, deriva un índice local regenerable, coordina revisiones entre Git y proveedores estructurales, y compila decisiones, restricciones y riesgos en paquetes pequeños de contexto. Codebase Memory será su primer proveedor estructural, integrado únicamente mediante contratos públicos verificados. El lenguaje, el transporte interno, el daemon y el empaquetado serán decididos después de investigación reproducible. La herramienta se instrumentará desde su primera vertical y se validará primero sobre sí misma y después sobre un monorepo real antes de distribuirse para macOS, Linux y Windows.

---

# 30. Conclusión

La arquitectura 0.1 no intenta impresionar con complejidad.

Intenta proteger el proyecto de decisiones tempranas mal fundamentadas.

La primera responsabilidad técnica es comprender el sistema del que Rationale dependerá.

La segunda es construir el camino mínimo entre:

```text
target
→ estructura
→ decisión
→ restricción
→ contexto útil
```

La tercera es medir si ese camino realmente ayuda.

Solo después corresponde convertirlo en un producto distribuible.
