# De alpha a beta — definición y checklist

Este documento existe para que "¿ya es beta?" sea una pregunta verificable,
no una opinión. Cubre solo funcionalidad — no proceso (usuarios externos,
uso sostenido en el tiempo), aunque esos puntos se listan al final porque sin
ellos beta no es defendible.

## Definición

> Rationale entra en beta cuando el flujo completo de preparación, captura,
> revisión y recuperación de decisiones funciona de forma repetible en
> varios repositorios; los bindings básicos son confiables; instalación,
> actualización y desinstalación son idempotentes en cada plataforma que el
> proyecto anuncia como soportada; y no existen fallos conocidos que
> corrompan el canon, inventen autoridad, o aprueben decisiones sin
> intervención humana.

Alpha significa "todavía estamos comprobando que el producto funciona".
Beta significa "ya sabemos que funciona; ahora comprobamos que funciona bien
para más personas, proyectos y entornos". No exige perfección — exige que
los fallos conocidos sean conocidos, controlados, y no destruyan la
confianza del usuario.

## Ya cumplido (verificado, no solo revisado por lectura)

- **Transporte MCP.** `initialize`/`tools/list`/`tools/call` sobre JSON por
  línea, con la sesión persistente sobreviviendo a herramientas
  desconocidas, targets inexistentes, JSON malformado y mensajes anidados
  profundamente (`tests/mcp_server.rs`).
- **Ciclo completo propose → review → approve → recover.** Verificado en
  este repo con el binario de alpha.7 real (no solo tests sintéticos):
  `finalize_change` en una sesión, `rationale review` como subproceso real,
  `prepare_change` en una sesión NUEVA — `governs_target: true`,
  `match_kind: structural`, `linkage: current`, con el proveedor
  estructural real resolviendo el símbolo.
- **Detección de conflictos honesta.** `intent_conflicts` distingue un
  hecho verificable (`governs-target`) de un solapamiento léxico no
  verificado (`lexical-overlap`); `polarity: undetermined` nunca se
  promueve a veredicto. `governance_verdict_required` obliga al agente a
  pronunciarse en vez de proceder en silencio.
- **`prepare_change` activa detección de conflictos por defecto** cuando
  viene `intent`, sin exigir un flag `mode` separado que el prompt maestro
  documentado nunca enseña (bug real corregido esta sesión — antes
  reproducía el síntoma exacto del incidente que motivó el proyecto).
- **Instalación/actualización en Unix.** Instalación limpia, canal
  `preview` sin degradar en silencio a `stable`/`releases/latest`, checksum
  SHA-256 verificado antes de instalar, helper de actualización instalado
  junto al binario.
- **Windows: layout de ZIP corregido y verificado en CI real**
  (`windows-latest` en la matriz, smoke test que expande el ZIP y confirma
  la ruta exacta que `rationale-installer.ps1` espera). Paridad de
  `install-agent --global-only` y limpieza de `rationale-update.ps1` al
  desinstalar.
- **`uninstall-agent` no destructivo.** Extirpa solo lo que Rationale
  escribió — nunca borra un archivo completo solo porque Rationale lo creó,
  si el usuario le agregó contenido después (verificado con `.mcp.json` con
  otro servidor MCP, y `CLAUDE.md` con texto propio bajo el bloque).
- **Canon atómico.** Todas las escrituras a `.rationale/` (Records,
  Subjects, y ahora también `agents.rs`: `CLAUDE.md`/`AGENTS.md`/
  `.mcp.json`/manifest) usan temp-file + `sync_all` + `rename`. Un proceso
  interrumpido nunca deja un archivo truncado.
- **Superficie de panics mínima.** De 216 ocurrencias de
  `unwrap`/`expect`/`panic!` en `src/`, solo una era alcanzable con input de
  usuario real (`extract_block` con marcadores invertidos) — corregida.
- **Camino de migración para canon legado.** `RecordWithoutBindings` (el
  defecto original de Fase 1, ya escrito en cuatro repos) tiene ahora una
  reparación humana explícita vía `doctor --repair`, marcada
  `declared_by: human` — nunca indistinguible de un binding confirmado por
  proveedor.
- **Auditoría de lifecycle.** `review-record` imprime `approvals[]`
  (incluyendo `approved_at`, que antes no se escribía) y
  `lifecycle.events[]` completos — auditar "quién aprobó y cuándo" ya no
  exige leer el YAML a mano.
- **Multi-repo.** `project_root` (canon) y `repo_path` (código) verificados
  como independientes con dos repos Git reales, no solo cableado sin probar.

## Checklist de salida de alpha

```text
[x] Cadena de gobernanza completa verificada con el binario real (no solo
    tests sintéticos), incluyendo una sesión MCP nueva recuperando un
    Record aprobado en una sesión anterior.
[x] intent activa detección de conflictos sin flags adicionales no
    documentados.
[x] Bindings exactos creados y recuperados correctamente (archivo + símbolo
    + propagación archivo→símbolo).
[x] explain_target devuelve los mismos Records gobernantes que
    prepare_change para el mismo target.
[x] Conflictos diferenciados entre lexicales y "gobierna el target".
[x] Cero panics conocidos en entradas normales.
[x] Cero pérdida o corrupción del canon (escrituras atómicas, incluyendo
    agents.rs).
[x] Instalación y actualización idempotentes en Unix.
[x] Desinstalación conserva .rationale/ Y el contenido que el usuario
    agregó a archivos que Rationale creó.
[x] macOS probado (este entorno).
[x] Linux — cubierto por CI (ubuntu-latest), no probado a mano en esta
    sesión.
[x] Windows — layout de ZIP corregido y verificado en CI real; no probado
    a mano en una máquina Windows real (ver "Fuera de alcance" abajo).
[x] Ciclo de vida básico de Records funcionando: correct, dispute, revoke,
    supersede, change-authority, add-evidence, y ahora
    add-human-confirmed-binding.
[x] Camino de migración para el canon legado sin bindings.
[x] Auditoría de approvals/lifecycle sin leer YAML a mano.
[x] Multi-repo (project_root != repo_path) verificado con repos reales.
[ ] 10 flujos completos exitosos en 5 repositorios distintos — solo se
    verificó en este repo y en un proyecto sintético de prueba esta sesión.
[ ] 5-10 usuarios externos, 3+ completando el flujo sin ayuda directa — no
    intentado; es proceso, no código.
[ ] Varios días de uso real sin corrupción, pérdida ni bloqueos graves — no
    intentado.
[ ] Windows probado a mano en una máquina/VM Windows real.
```

## Todavía abierto / no documentado en ningún otro sitio

- **`.rationale/migrations/` es una afordancia vacía.** `rationale doctor`
  ya detecta `schema_version` desconocido como puerta visible, pero no hay
  lógica de migración real. No hace falta para beta (solo una versión de
  schema existe hoy), pero el día que exista una segunda, esto es lo
  primero que hay que construir.
- **Ruido de binding en `finalize_change`.** Se excluyeron los archivos que
  `install-agent`/`init` administran, pero `finalize_change` sigue atando
  binding a TODO archivo sin commitear del repo, no solo a los relacionados
  con el target. Un cambio real junto a archivos de scratch/no
  relacionados sin commitear seguirá produciendo bindings de más. No
  bloquea beta (el binding del target real siempre está presente), pero
  vale la pena acotarlo.
- **CI no valida Linux/macOS a mano**, solo vía GitHub Actions. Suficiente
  para beta, pero "probado" en este documento significa "CI verde", no
  "una persona lo instaló en su propia máquina limpia" excepto en macOS
  (este entorno).

## Fuera de alcance de este documento

Validado por revisión de código y tests automatizados, NO por ejecución
real en el hardware correspondiente:

- Windows: el fix del ZIP se verificó leyendo el código y simulando la
  extracción; la validación real requiere `windows-latest` en CI (ya
  agregado) ejecutándose al menos una vez, y idealmente una persona
  instalando en una máquina Windows real.
- Usuarios externos, uso sostenido en el tiempo, y los "10 flujos en 5
  repos" del checklist de arriba son trabajo de proceso, no de código —
  este documento los deja explícitos, no los resuelve.
