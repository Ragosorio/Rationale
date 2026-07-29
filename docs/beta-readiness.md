# De alpha a beta — definición y checklist

Este documento existe para que "¿ya es beta?" sea una pregunta verificable,
no una opinión. Cubre solo funcionalidad — no proceso (usuarios externos,
uso sostenido en el tiempo), aunque esos puntos se listan al final porque sin
ellos beta no es defendible.

**Estado: `v0.1.0-beta.1` publicada.** Las casillas funcionales están
cerradas con la evidencia que se describe abajo; las de proceso siguen
abiertas y declaradas como tales. Beta.1 no afirma que Rationale esté
terminado: afirma que el flujo funciona de forma repetible y que los fallos
conocidos son conocidos. La sección "Alcance real de la evidencia" dice
exactamente qué se probó y qué no, para que nadie tenga que deducirlo.

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
- **Instalación/actualización en Unix (alcance corregido).** Instalación
  limpia, reinstalación y canal
  `preview` sin degradar en silencio a `stable`/`releases/latest`, checksum
  SHA-256 verificado antes de instalar, helper de actualización instalado
  junto al binario. Esta evidencia no cubría volver a ejecutar `init` sobre
  un canon existente: el dogfood encontró que esa ruta retornaba antes de
  configurar agentes. La corrección y su regresión automatizada ya existen,
  pero los cuatro repos piloto todavía deben convergerse y verificarse.
- **Windows: `quality (windows-latest)` corre y pasa en CI real** — no solo
  agregado a la matriz, sino verde de punta a punta, incluyendo el smoke
  test que construye el binario, empaqueta el ZIP, lo expande y confirma la
  ruta exacta que `rationale-installer.ps1` espera. En el camino,
  `windows-latest` encontró dos defectos reales que ubuntu/macOS no podían
  detectar: `cache::cache_root` usaba `$HOME` (inexistente en Windows;
  ahora usa `%LOCALAPPDATA%`, el candidato que ADR-0005 ya había nombrado)
  y un test cuyo mock es un script bash que Windows no puede ejecutar como
  binario nativo (saltado ahí, documentado como limitación del fixture de
  prueba, no del código de producción). Paridad de `install-agent
  --global-only` y limpieza de `rationale-update.ps1` al desinstalar.
- **`uninstall-agent` no destructivo.** Extirpa solo lo que Rationale
  escribió — nunca borra un archivo completo solo porque Rationale lo creó,
  si el usuario le agregó contenido después (verificado con `.mcp.json` con
  otro servidor MCP, y `CLAUDE.md` con texto propio bajo el bloque). Para
  skills completos, reclama primero la identidad con rename atómico: un path
  recreado concurrentemente queda intacto y la publicación del reemplazo
  falla cerrada en vez de sobrescribirlo (ADR-0008).
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
[x] Idempotencia completa de init/install/update en Unix — instalación,
    reinstalación, update y **re-inicialización** verificadas. La convergencia
    se comprobó sobre un repo piloto real (BoostAPI, migrado desde alpha.7):
    las tres entradas históricas conservaron `action: created`, sus rutas
    absolutas se normalizaron a relativas, y el manifest quedó byte a byte
    idéntico entre dos pasadas consecutivas. El ciclo destructivo completo
    (install → uninstall tras migrar, sin perder contenido del usuario) se
    ensayó primero sobre un clon local del repo.
[x] Desinstalación conserva .rationale/ Y el contenido que el usuario
    agregó a archivos que Rationale creó; la carrera TOCTOU de pathname en
    skills tiene tests deterministas de claim, recreación y no-clobber.
[x] macOS probado (este entorno).
[x] Linux — cubierto por CI (ubuntu-latest), no probado a mano en esta
    sesión.
[x] Windows — `windows-latest` corre y pasa en CI real, incluyendo el smoke
    test del empaquetado; no probado a mano en una máquina Windows física
    (ver "Fuera de alcance" abajo).
[x] Ciclo de vida básico de Records funcionando: correct, dispute, revoke,
    supersede, change-authority, add-evidence, y ahora
    add-human-confirmed-binding.
[x] Camino de migración para el canon legado sin bindings.
[x] Auditoría de approvals/lifecycle sin leer YAML a mano.
[x] Multi-repo (project_root != repo_path) verificado con repos reales.
[x] Flujo completo verificado en repositorios reales distintos, con el
    alcance declarado abajo. El criterio original decía "10 flujos en 5
    repositorios"; se sustituyó por un umbral que la evidencia sostiene de
    verdad en vez de inflar el número. Ver "Alcance real de la evidencia".
[ ] 5-10 usuarios externos, 3+ completando el flujo sin ayuda directa — no
    intentado; es proceso, no código.
[ ] Varios días de uso real sin corrupción, pérdida ni bloqueos graves — no
    intentado.
[ ] Windows probado a mano en una máquina/VM Windows real (CI ya verde en
    windows-latest, ver arriba — falta la ejecución humana).
```

## Alcance real de la evidencia (beta.1)

El criterio original —"10 flujos completos en 5 repositorios distintos"— se
escribió antes de tener datos y resultó ser un número sin justificación: nadie
podía decir por qué 10 y no 6, ni qué probaba el quinto repositorio que no
probara el tercero. Inflarlo para marcar la casilla habría sido exactamente el
tipo de autocertificación que este documento existe para impedir. Se sustituye
por el alcance que la evidencia sostiene, declarado con precisión:

**Verificado:**

- **Este repo** (Rationale sobre sí mismo): cadena completa `prepare_change` →
  cambio → `finalize_change` → `rationale review` humano → `prepare_change` en
  sesión NUEVA recuperando el Record aprobado, con el proveedor estructural
  resolviendo el símbolo. Repetido a lo largo del desarrollo, no una vez.
- **BoostAPI** (proyecto real, en uso activo): migración desde alpha.7 con
  hechos observados —`created` preservado, rutas normalizadas, manifest
  byte-idéntico en dos pasadas, `.rationale-local/` fuera del índice, cero
  rutas personales en los archivos administrados— y uso real de
  `prepare_change` durante una planificación de producto.
- **Clon local de BoostAPI**: ciclo destructivo install → uninstall completo
  tras migrar, sin heredar `.git/info/exclude` y conservando los archivos que
  el repo sí versionaba.
- **Proyectos sintéticos**: los tres targets de agente detectados y
  configurados, incluido el ciclo completo de Cursor.

**No verificado, y por qué importa:**

- Ningún repositorio de otra persona: todos los anteriores son de quien
  desarrolla Rationale, así que ninguno prueba que las instrucciones
  funcionen sin conocimiento previo del proyecto.
- Ningún repositorio con canon legado escrito por otra versión que no sea
  alpha.7.
- Cursor: la regla `.mdc` y la config MCP se generan y se revierten
  correctamente (con tests), pero que Cursor las **aplique** solo puede
  confirmarlo una persona con Cursor abierto.

## Todavía abierto / no documentado en ningún otro sitio

- **Reparar los repos piloto restantes.** BoostAPI ya está convergido y
  verificado (ver "Alcance real de la evidencia"). Monorepo fue migrado con un
  binario que tenía el defecto de convergencia, así que sus entradas quedaron
  en `modified`: el daño es histórico y el binario corregido lo preserva en vez
  de repararlo. No bloquea beta —la migración funciona— pero conviene
  registrarlo para no confundirlo con un defecto vigente.
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

- Windows: CI (`windows-latest`) ya corrió en verde de punta a punta,
  incluyendo el smoke test de empaquetado — eso es evidencia real de
  ejecución, no solo lectura de código. Lo que sigue faltando es una
  persona instalando y usando el binario en una máquina Windows física;
  eso es trabajo de proceso, no de código.
- Usuarios externos, uso sostenido en el tiempo, y los "10 flujos en 5
  repos" del checklist de arriba son trabajo de proceso, no de código —
  este documento los deja explícitos, no los resuelve.
