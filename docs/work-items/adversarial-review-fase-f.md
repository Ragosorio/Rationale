# Revisión adversarial: Fase F (captura, señales, Subject Resolver, `finalize_change`, `rationale review`)

**Rol:** Review Agent independiente (`Proceso §4.4` — "otra sesión" como revisor válido), sin contexto previo de la sesión que implementó Fase F.
**Encargo:** intentar refutar `src/storage.rs`, `src/capture.rs`, `src/signals.rs`, `src/subjects.rs`, `src/pipeline.rs::finalize`, `src/review.rs` y el ciclo completo propuesta→revisión→aprobación, sin autoaprobar nada.
**Esta sesión no aprobó ni rechazó nada** — el veredicto queda para revisión humana, siguiendo el mismo patrón que `docs/work-items/adversarial-review-adr-0001-0002-0006.md` y `docs/work-items/adversarial-review-fase-e5-e6.md`.

Metodología: lectura completa de `src/storage.rs`, `src/capture.rs`, `src/signals.rs`, `src/subjects.rs`, `src/pipeline.rs`, `src/review.rs`, `src/main.rs::cmd_review`, `src/project.rs`; revisión del commit `c9fd5b6` (fix de path traversal aplicado durante el cierre de Fase F); `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test --release` (95 tests); y, sobre todo, ataque empírico contra el **binario real compilado** (`target/release/rationale serve` / `rationale review`) usando un cliente Python que habla el framing `Content-Length` directamente (mismo enfoque que la revisión E5/E6) más invocaciones directas del CLI con `subprocess`, sobre proyectos Git sintéticos desechables en `/tmp/`. Ningún archivo de producción fue modificado; `git status` seguía limpio al terminar.

Commit de partida: `c9fd5b6` (`fix(security): path traversal real vía record_id en finalize_change/review`), el más reciente en `git log` al iniciar esta revisión.

---

## Resumen ejecutivo

| Área | Hallazgo | Severidad |
|---|---|---|
| `subjects::list_subjects` / `storage::list_records` + `pipeline::finalize` | Un solo archivo YAML corrupto en `.rationale/subjects/` o `.rationale/records/` desactiva el Subject Resolver COMPLETO para toda futura propuesta, en silencio, sin ningún diagnóstico | **Alto** |
| Ciclo propuesta→revisión (`review.rs` + `pipeline::finalize`) | TOCTOU real: una propuesta puede perderse en silencio (nunca se mueve a `.rejected/`) si se sobrescribe durante la ventana de revisión humana; una segunda aprobación concurrente sobre la misma propuesta sobrescribe la primera sin detección de colisión | **Alto** |
| `review::describe_effect` (`println!` de contenido controlado por el cliente MCP) | Secuencias de escape ANSI/control en `intent`/`statement`/`risks` de `finalize_change` sobreviven intactas hasta el terminal del humano en `rationale review` | **Alto** |
| `signals::signals_from_paths` (coincidencia por substring) | Falso positivo confirmado: un archivo cosmético sin relación con autorización real (`auth_helper_unrelated.rs`) dispara la señal `Authorization` y genera una propuesta de ruido | **Medio** |
| `signals::determine_level` | Falso negativo confirmado: un bug real de doble cobro en pagos, sin keyword de path ni lenguaje normativo, se clasifica en el nivel más bajo (`Intent`), igual que un cambio trivial | **Medio** |
| `subjects::resolve` — `ALIAS_SIMILARITY_THRESHOLD = 0.85` | Falso positivo confirmado: dos conceptos genuinamente distintos que comparten una plantilla de frase larga alcanzan 0.86 de similitud léxica y BLOQUEAN la propuesta completa | **Medio** |
| `subjects::resolve` — `CANDIDATE_MIN_THRESHOLD = 0.2` | Falso negativo confirmado: el mismo concepto real (no doble cobro) expresado con vocabulario distinto no genera ningún candidato — fragmentación silenciosa de Subjects | **Medio** |
| Path traversal en `record_id` (fix `c9fd5b6`) | Re-verificado independientemente: el fix sostiene; no se encontró bypass ni un lugar equivalente sin cubrir | Sostiene |
| Nunca autoaprueba (`review::approve` es la única función que construye `Approval{status:"approved"}`) | Re-verificado de forma independiente, sin confiar en el hallazgo previo | Sostiene |
| EOF / stdin no interactivo en `rationale review` | Nunca se interpreta como aprobación implícita — cae al camino "Saltado" | Sostiene |
| `statement` corregido vacío en el flujo `'c'` | Rechazado por `storage::validate()` antes de tocar disco; la propuesta permanece pendiente, no se pierde | Sostiene |
| Escritura concurrente cruzando procesos del SO sobre el mismo `record_id` | El rename atómico sostiene incluso con 12 procesos reales concurrentes — nunca corrompe el archivo | Sostiene |
| `cargo fmt` / `cargo clippy -D warnings` / `cargo test --release` | 95/95 tests, cero warnings | Sostiene |

**7 hallazgos accionables** (0 críticos nuevos, 3 altos, 4 medios) + **6 confirmaciones que sostienen** bajo ataque empírico real.

---

## Hallazgos completos

### 1. Un solo archivo corrupto en `.rationale/subjects/` o `.rationale/records/` apaga el Subject Resolver por completo, en silencio (Alto)

`src/subjects.rs:82-90` (`list_subjects`) usa `?` sobre `read_subject(&path)` dentro del `for` que recorre el directorio: si UN SOLO archivo `.yaml` no parsea o le falta `id`/`title`, la función entera devuelve `Err`, descartando también los Subjects que sí se habían leído bien hasta ese punto. Lo mismo aplica a `storage::list_records` para Records.

`src/pipeline.rs:542-543`:
```rust
let existing_subjects = subjects::list_subjects(&subjects_dir).unwrap_or_default();
let existing_records = storage::list_records(&records_dir).unwrap_or_default();
```
Ambas líneas silencian ese `Err` con `.unwrap_or_default()` — sin ningún `diagnostics.push(...)`. Esto contrasta directamente con `pipeline::prepare` (mismo módulo, líneas 87-105), que sí reporta explícitamente cuando falla la lectura de Subjects (`"advertencia: no se pudieron leer Subjects: {e}"`). `finalize` no tiene ese mismo cuidado.

**Evidencia reproducible — caso Subjects.** Proyecto con un Subject casi idéntico en título al propuesto (debería bloquear como `Alias`):
```
=== BEFORE corrupting subjects/ : exact-title duplicate should be detected ===
action: alias blocked_reason: candidato de Subject fuerte sin novelty_reason — ver subject_resolution.candidates

=== AFTER adding one malformed subjects/broken.yaml : same exact-title duplicate ===
action: create blocked_reason: None
candidates: []
proposal_written: True
ALL diagnostics: ['target declarado: .../src/auth/authz.rs',
 'propuesta escrita en .../proposals/constraint.dup-after.yaml (nivel=OperationalKnowledge, subject=authz.new-duplicate-attempt)']
```
El archivo corrupto agregado fue trivial: `.rationale/subjects/broken.yaml` con contenido `"id: \ntitle: \n"`. Ningún diagnóstico menciona que algo falló al leer Subjects.

**Evidencia reproducible — caso Records (binding overlap).** Mismo patrón, pero vía `.rationale/records/`:
```
=== BEFORE corrupting records/: binding-overlap should surface the existing subject as a candidate ===
action: alias candidates: [{'id': 'authz.entity-scoped-staff-access', 'signals': {'binding_overlap': 1.0, ...}}]

=== AFTER adding one malformed records/broken.yaml: same binding-overlap scenario ===
action: create candidates: []
diagnostics: ['target declarado: .../src/auth/authz.rs', 'propuesta escrita en .../proposals/constraint.overlap-after.yaml (nivel=OperationalKnowledge, subject=authz.new-attempt)']
```
El archivo corrupto: `.rationale/records/broken.yaml` con `statement: ""` (un `Record` inválido cualquiera, exactamente el tipo de archivo que ya existe hoy en `.rationale/proposals/.rejected/` de este mismo repositorio tras mis propias pruebas — ver hallazgo 3 — o que podría quedar de una edición manual fallida).

**Por qué es alto, no medio:** `Rationale_Arquitectura_Conceptual_v0.1.md §27` prohíbe explícitamente "Ocultar cobertura parcial". Este es exactamente ese caso: el Subject Resolver completo (Fase F4, la pieza central de esta fase junto con `finalize_change`) queda ciego a TODO el canon existente de Subjects y Records — no solo al archivo corrupto — sin que `FinalizeOutcome` lleve ninguna señal de ello. No requiere un atacante: un humano editando un Subject a mano con un YAML mal formado, o una escritura futura interrumpida sobre Subjects, produce el mismo efecto de forma completamente accidental y silenciosa. El resultado observable es indistinguible de "no existen Subjects/Records similares" cuando la realidad es "no pude leerlos".

**Corrección sugerida (no aplicada):** que `list_subjects`/`list_records` acumulen errores por archivo en vez de abortar al primero (ya existe precedente: `review::list_pending` omite silenciosamente entradas que no parsean, pero al menos no descarta las demás); y que `pipeline::finalize` reporte en `diagnostics` cualquier error de lectura de Subjects/Records, igual que ya hace `pipeline::prepare`.

---

### 2. TOCTOU real en el ciclo propuesta→revisión: pérdida silenciosa de propuestas y de aprobaciones (Alto)

`review::list_pending` (`src/review.rs:40-65`) lee **una vez** todo `.rationale/proposals/` al inicio de `cmd_review` (`src/main.rs:204`) y mantiene cada `Record` propuesto **en memoria** mientras espera input humano (que puede tardar minutos). `review::approve` (`src/review.rs:111-148`) nunca vuelve a leer el archivo de disco antes de promoverlo: escribe directamente el `Record` que tiene en memoria a `records/<id>.yaml` y luego borra `proposals/<id>.yaml` — sin comprobar que ese archivo siga conteniendo lo mismo que se le mostró al humano, ni que siga existiendo siquiera.

**2a. Una propuesta nueva sobre el mismo `record_id`, escrita mientras la primera espera revisión, se pierde sin dejar rastro.**

Secuencia reproducida contra el binario real:
1. `finalize_change` escribe `proposals/constraint.race-test.yaml` con `statement: "FIRST VERSION..."`.
2. Se arranca `rationale review` real; su `list_pending()` ya cargó la propuesta "FIRST VERSION" en memoria y está bloqueado esperando la respuesta del humano (confirmado leyendo su stdout hasta el prompt).
3. **Mientras tanto**, una segunda llamada a `finalize_change` con el MISMO `record_id="constraint.race-test"` pero `statement: "SECOND VERSION..."` sobrescribe `proposals/constraint.race-test.yaml` en disco (escritura atómica exitosa, `proposal_written: true`).
4. Se confirma "approve" al proceso de revisión, que promueve lo que tenía en memoria.

```
=== proposals/constraint.race-test.yaml on disk RIGHT BEFORE approving ===
statement: 'SECOND VERSION: staff must never receive global super_admin (overwritten during review window).'
...

=== rest of review stdout ===
Aprobado -> /tmp/rationale-atk-race/.rationale/records/constraint.race-test.yaml

records/ contents: ['constraint.race-test.yaml']
proposals/ contents: []
=== promoted record statement line ===
statement: 'FIRST VERSION: staff must never receive global super_admin.'
```
`records/` termina con "FIRST VERSION" (coherente con lo que el humano vio y aprobó — no hay engaño sobre lo que aprobó), pero `proposals/` queda **vacío**: la propuesta "SECOND VERSION" — que llegó a existir en disco, con `proposal_written: true` — desaparece por completo. No se mueve a `.rejected/` (que `review::reject` sí usa explícitamente "nunca se borra en silencio"), no queda en ningún log, no hay ningún diagnóstico. Es indistinguible de que esa propuesta nunca hubiera existido.

**2b. Dos revisores humanos concurrentes sobre la misma propuesta: el segundo `approve` sobrescribe al primero sin detectar la colisión.**

Se lanzaron dos procesos `rationale review` reales casi simultáneamente contra el mismo proyecto con UNA propuesta pendiente (ambos ejecutan `list_pending()` antes de que ninguno apruebe). El primero aprueba y promueve con éxito. El segundo, que sigue con la MISMA copia en memoria (de antes de que el primero promoviera), también aprueba:
```
=== Reviewer A result ===
Aprobado -> .../records/constraint.double-reviewer-test.yaml

=== Reviewer B result (same proposal, approved after A already promoted it) ===
Aprobado -> .../records/constraint.double-reviewer-test.yaml
```
Ambos reportan éxito (`Aprobado -> ...`, sin ningún error), y el reviewer B nunca comprueba que `proposals/constraint.double-reviewer-test.yaml` ya no existía cuando intentó promoverlo (`std::fs::remove_file` en `approve()` usa `let _ = ...`, ignorando el fallo). En este experimento ambos procesos comparten la misma identidad Git local (`user.name`/`user.email`), así que el `approvals` final solo muestra una entrada — pero la segunda escritura de `write_record` **reemplaza el archivo completo**, no fusiona `approvals`: si el segundo revisor tuviera una identidad Git distinta (dos personas reales, o dos agentes con configuraciones distintas — el escenario natural de Fase G, dogfooding con más de un colaborador), la segunda escritura habría descartado la `Approval` real que el primer revisor ya había persistido, sin ningún aviso a ninguno de los dos.

**Por qué es alto:** el mecanismo entero de Fase F6 existe para que "nunca se autoaprueba, siempre con efecto visible y deliberado" (`review.rs:1-21`). Ambas variantes de esta race violan esa garantía en su forma más silenciosa: no es que algo se apruebe sin querer (el humano sí aprobó conscientemente lo que vio), es que el **resultado persistido en disco no corresponde a la única fuente de verdad esperada** — una propuesta real desaparece sin dejar evidencia (2a), o una aprobación real ya persistida puede ser pisada por otra sin detección de colisión (2b). Ninguno de los dos casos está cubierto por los tests existentes de `review.rs` o `tests/mcp_server.rs`, que solo prueban invocaciones secuenciales sin solape.

**Corrección sugerida:** antes de escribir en `approve()`, releer `proposal.path` y comparar contra la copia en memoria (o simplemente comparar mtime/hash); si difiere o el archivo ya no existe, abortar con un error explícito en vez de proceder silenciosamente. Considerar además un lock de archivo (`flock`) sobre `proposals/<id>.yaml` durante la ventana de revisión.

---

### 3. Inyección de secuencias de control/ANSI en el terminal del revisor humano vía `intent`/`statement`/`risks` (Alto)

`finalize_change` acepta `intent`, `statement` y `risks` como texto libre proveniente del cliente MCP (un agente, potencialmente comprometido o simplemente con un bug) — nunca se sanea. Estos campos se persisten tal cual en el `Record` propuesto y, más tarde, `review::describe_effect` (`src/review.rs:70-92`) los imprime con `println!` directamente en el terminal del humano durante `rationale review` — exactamente el momento que Fase F6 diseñó para que el humano vea "el efecto práctico" antes de aprobar (v0.5 §15.5).

**Vector descartado primero (filenames vía Git):** se probó si un nombre de archivo real con secuencias de escape ANSI (legal en POSIX, solo prohíbe `/` y NUL) sobrevivía a través de `capture::diff_since` → `path_hint`. Confirmado que **no**: Git cita (quotea) automáticamente cualquier byte de control en un path con `--name-status`, incluso con `core.quotepath=false` (verificado con `xxd` sobre el diff crudo — los bytes de escape aparecen como el texto literal `\033`, nunca como el byte `0x1b` real). Este vector concreto no aplica — Git ya lo neutraliza antes de que Rationale lo vea.

**Vector confirmado (texto libre del propio agente):** `intent`/`statement` no pasan por ningún filtro de Git. Se llamó a `finalize_change` con:
```python
malicious_statement = ("Staff must never receive global super_admin.\x1b[2K\r"
                        "\x1b[32mAUTO-APPROVED BY SECURITY TEAM - no review needed\x1b[0m")
malicious_intent = "Normal intent text \x1b[8mhidden-instruction-here\x1b[28m end"
```
`yaml_serde` sí escapa correctamente estos bytes al persistir el YAML (`\e[2K` como escape de cadena, forma válida de YAML 1.1) — la persistencia en disco es segura. Pero al releer y mostrar con `rationale review`:
```
=== raw stdout bytes from `rationale review` ===
b"...Afirmaci\xc3\xb3n propuesta: Staff must never receive global super_admin.\x1b[2K\r\x1b[32mAUTO-APPROVED BY SECURITY TEAM - no review needed\x1b[0m\nRaz\xc3\xb3n: Normal intent text \x1b[8mhidden-instruction-here\x1b[28m end\n..."
Contains raw ESC byte (0x1b) in review's stdout: True
```
Los bytes de escape reales (`0x1b`) llegan intactos al stdout que el humano lee — `\x1b[2K\r` borra la línea actual y mueve el cursor, `\x1b[32m...\x1b[0m` puede pintar un falso mensaje "AUTO-APPROVED BY SECURITY TEAM" en verde, y `\x1b[8m...\x1b[28m` (SGR "conceal") puede ocultar texto en terminales que lo soportan.

**Alcance real, sin exagerar:** esto NO permite saltarse la palabra de confirmación en sí — el humano todavía debe teclear `approve`/`approve-critical` literalmente, y ese chequeo (`src/main.rs:242`) compara el string tecleado, no afectado por lo que se muestra. El riesgo real es **engaño visual**: el humano puede aprobar creyendo que lee algo distinto de lo que el sistema realmente va a persistir (statement real oculto o sustituido visualmente por un banner falso), lo cual ataca directamente la única salvaguarda que Fase F6 ofrece ("una afirmación por pantalla... nunca el YAML completo" — pero la "pantalla" misma puede falsificarse).

**Por qué es alto:** el punto de confianza humano es el único control real contra autoaprobación en todo el sistema (`review.rs:1-21`); un vector que permite manipular exactamente lo que ese humano ve, sin que nada en el pipeline lo detecte o lo sanee, ataca ese control en su raíz.

**Corrección sugerida:** despojar (strip) caracteres de control (`\x00-\x1f` salvo `\n`/`\t` si se desea preservar formato multilínea intencional) de `statement`/`rationale`/`risks` antes de escribirlos en `describe_effect`, o al menos antes de persistirlos — la opción más simple es sanear en la frontera de entrada (`pipeline::finalize`), donde ya se sanea `record_id`.

---

### 4. `signals::signals_from_paths` — falso positivo real por coincidencia de substring (Medio)

`PATH_KEYWORDS` (`src/signals.rs:37-56`) usa `path_lower.contains(kw)` — substring puro, no palabra completa. Un archivo puramente cosmético cuyo nombre simplemente contiene la subcadena `"auth"` dispara la señal `Authorization`, sin relación alguna con lógica de autorización real.

Evidencia reproducible contra el binario real:
```
=== FP test: auth_helper_unrelated.rs (decorative banner, no real auth logic) ===
signals: ['authorization']
level: decision
proposal_written: True
```
El archivo real usado: `src/auth_helper_unrelated.rs` con contenido `// renders a decorative header banner for the CLI splash screen` — cero relación con autorización. El commit real generó una propuesta de Nivel `Decision` completa.

**Por qué importa (medio, no alto):** el propio módulo se declara honesto ("deliberadamente corta y ampliable... coincidencia por substring... barata"), y el mecanismo es aditivo (nunca bloquea, solo genera ruido) — pero el ruido erosiona exactamente la promesa central de Fase F ("Rationale no pregunta por todo cambio; activa captura asistida solo cuando detecta señales concretas", `signals.rs:4-10`). Palabras como `"client"` (bajo `ExternalIntegration`) son aún más amplias — coincidirían con casi cualquier archivo llamado `*_client.rs`, incluyendo los propios `src/providers/*.rs` de este repositorio.

**Corrección sugerida:** usar coincidencia por palabra completa sobre segmentos de path (dividir por `/`, `_`, `-`, `.` y comparar tokens exactos) en vez de substring crudo — mismo patrón que `contains_word` ya usa para `NORMATIVE_WORDS` en el propio archivo (línea 68-75), que si evita el caso análogo (`avoid` dentro de `avoidance-list`, cubierto por el test `does_not_false_positive_on_substring_of_normative_word`). La inconsistencia entre ambas funciones del mismo módulo (una hace matching por palabra, la otra por substring crudo) no está justificada en los comentarios.

---

### 5. `signals::determine_level` — falso negativo real: un cambio críticamente peligroso se clasifica en el nivel más bajo (Medio)

Un cambio real en lógica de liquidación de pagos que dobla el monto cobrado en ciertas condiciones, sin ningún keyword de `PATH_KEYWORDS` en el path y sin ninguna `NORMATIVE_WORDS` en `intent`/`statement`, se clasifica como `Intent` — el mismo nivel que un refactor trivial sin ninguna señal.

Evidencia reproducible:
```
=== FN test: real critical payment-doubling bug, no keyword path, no normative language ===
signals: []
level: intent
proposal_written: True
```
Path usado deliberadamente sin ningún keyword: `src/core/ledger_math.rs`. `intent`: *"Updated the settlement calculation used when closing out international customer orders."* `statement`: *"International order settlement now doubles the charged amount when currency mismatch is detected."* `risks`: *"Customers could be charged twice the correct amount for international orders."* — ninguno de estos textos contiene `must`/`never`/`because`/`avoid`/`do not`. El campo `severity: "critical"` que el caller sí pasó **no influye en absoluto** en `determine_level` — es un campo completamente separado que solo afecta la palabra de confirmación en `rationale review`, no el nivel de captura.

**Por qué importa (medio, no alto):** el mecanismo sigue escribiendo una propuesta (`proposal_written: true`, nunca se pierde el evento), así que no hay pérdida de datos — pero el propósito explícito de los niveles (v0.5 §16) es priorizar dónde debe mirar primero un humano con tiempo limitado, y este es precisamente el caso — un bug financiero real y grave — donde priorizar mal tiene el costo más alto.

**Corrección sugerida:** ninguna trivial sin ampliar la taxonomía de keywords (que el propio módulo ya admite como incompleta a propósito) — la opción más barata es que `determine_level` considere el `severity` declarado por el caller como una señal adicional (no autoritativa, pero sí visible) cuando no hay match de dominio ni lenguaje normativo, en vez de ignorarlo del todo.

---

### 6. `subjects::resolve` — el umbral `ALIAS_SIMILARITY_THRESHOLD = 0.85` bloquea dos conceptos genuinamente distintos por compartir una plantilla de frase (Medio)

Jaccard sobre tokens normalizados (`lexical_similarity`, `src/subjects.rs:169-182`) no distingue "la misma frase con una palabra de dominio distinta" de "la misma idea". Dos títulos de gobernanza con una plantilla larga compartida, que describen constraints reales y distintos (gobernanza de migraciones de esquema vs. gobernanza de logging de auditoría), alcanzan 0.86 — por encima del umbral de `Alias` (0.85) — y la propuesta completa se BLOQUEA (no solo se marca como candidato).

Evidencia reproducible contra el binario real:
```
=== Jaccard FP test: distinct concept (migration governance vs audit-logging governance) ===
subject_resolution action: alias
candidates: [{"id": "db.migration-governance", "signals": {"binding_overlap": 0.0, "lexical_similarity": 0.8636363636363636, "scope_compatible": true}}]
blocked_reason: candidato de Subject fuerte sin novelty_reason — ver subject_resolution.candidates
proposal_written: False
```
Título existente: *"Ensure that the system never allows a background job to write directly to the production database without going through the approved **migration** pipeline"*. Título propuesto: la misma frase, sustituyendo solo *"migration"* por *"audit logging"*. `binding_overlap: 0.0` (ningún archivo en común) — la única señal que dispara el bloqueo es puramente léxica.

**Por qué importa (medio, no alto):** a diferencia de `retrieval::detect_conflict` (que solo añade una advertencia, nunca bloquea — v0.5 §19.1), aquí `finalize_change` sí bloquea la escritura de la propuesta por completo (`proposal_written: false`) a menos que el caller provea `novelty_reason` explícito. Esto significa que cualquier organización que use plantillas de redacción consistentes para sus constraints (razonable, incluso recomendable) generará falsos bloqueos recurrentes, entrenando a los agentes a rellenar `novelty_reason` casi por reflejo — exactamente el "aceptar todo" que v0.5 §294 quiere evitar, solo que en la dirección opuesta (aceptar la anulación del bloqueo, no la aprobación).

**Corrección sugerida:** ponderar `lexical_similarity` con algo más que Jaccard de tokens crudos — por ejemplo, excluir stopwords estructurales del cómputo (`ensure`, `that`, `the`, `system`, `never`, `allows`, `to`, `without`, `going`, `through`, `approved`, `pipeline` son puro andamiaje sintáctico, no señal de concepto) antes de aplicar el umbral. No es un cambio trivial sin evidencia adicional sobre qué tan comunes son las plantillas repetidas en Records reales — pero el umbral actual (0.85, sin justificación documentada más allá del número) no resiste este contraejemplo.

---

### 7. `subjects::resolve` — el mismo concepto real, vocabulario distinto, nunca surge como candidato (Medio)

Contraparte exacta del hallazgo 6: un Subject ya existente (`payments.no-double-charge`, título *"Payments must never be processed twice for the same order"*) y una propuesta nueva que describe el MISMO concepto (evitar doble cobro, esta vez en reintentos de checkout) con vocabulario completamente distinto (*"Idempotent settlement retries must not re-bill the customer's card on transient network failures"*) no comparten suficientes tokens ni para superar `CANDIDATE_MIN_THRESHOLD` (0.2).

Evidencia reproducible:
```
=== Jaccard FN test: same real concept (no double billing), different vocabulary ===
subject_resolution action: create
candidates: []
proposal_written: True
```
`candidates: []` — ni siquiera aparece como candidato débil para que un humano lo revise en `rationale review`; el Subject nuevo se crea sin ninguna señal de que ya existe un Subject gobernando la misma preocupación real.

**Por qué importa (medio, no alto):** esto es fragmentación silenciosa del canon — exactamente lo que el Subject Resolver (Fase F4) existe para prevenir (v0.5 §9.1, pasos 2-5). No bloquea nada ni corrompe datos, pero erosiona la garantía central de la fase con el paso del tiempo: cada concepto real terminaría con N Subjects distintos según qué agente lo redactó primero, sin que nadie lo note hasta una auditoría manual.

**Corrección sugerida:** el propio módulo ya reconoce esto como límite conocido y diferido (`resolve()` doc: "5. Similitud semántica local. -> diferido, §28.3 (embeddings)") — es coherente con la decisión arquitectónica de v0.5 de no usar embeddings todavía. No es un bug de implementación tanto como una limitación estructural ya documentada; se incluye aquí porque el encargo pidió construir el contraejemplo explícito, y queda confirmado con datos reales, no solo teóricos.

---

## Lo que sostiene bajo ataque

1. **El fix de path traversal (`c9fd5b6`) sostiene, y no se encontró un lugar equivalente sin cubrir.** Se re-verificó `storage::validate_safe_id` de forma independiente (no solo lectura de código): `../../../../etc/pwned`, `..`, `.`, `sub/dir`, `back\slash`, `nul\0byte` — todos rechazados por los tests existentes, reconfirmado con `cargo test`. Se buscó explícitamente, vía `grep`, si `subject_id`/`subject_title` alguna vez se usan para construir un path — confirmado que NO (`src/pipeline.rs:86,376,540` solo hacen `config.rationale_dir.join("subjects")`, un literal fijo; Fase F no escribe Subjects nuevos todavía, consistente con `docs/architecture/code-map.md`). Se probó además si un nombre de archivo Git real con secuencias de control podía llegar sin escapar hasta `path_hint` (ver hallazgo 3) — Git lo neutraliza antes de que Rationale lo vea, incluso con `core.quotepath=false` (verificado con `xxd`).

2. **La garantía "nunca autoaprueba" se re-verificó de forma independiente, sin confiar en el hallazgo previo de esta misma revisión.** `grep -rn "status: \"approved\"" src/` encuentra 4 sitios; los 3 que no son `review.rs:130` están dentro de `#[cfg(test)]` (fixtures de `retrieval.rs` y `assessment.rs` para probar que los Records ya-aprobados se muestran correctamente — nunca código de producción). `grep` sobre `src/mcp/server.rs` confirma que el módulo `review` nunca se referencia desde la superficie MCP (ni `finalize_change` ni ninguna otra tool) — la única forma de producir una `Approval` real sigue siendo `rationale review`, un proceso CLI interactivo separado.

3. **EOF / stdin no interactivo nunca se interpreta como aprobación implícita.** Se corrió `rationale review --project-root <dir>` con `stdin=/dev/null` contra una propuesta real de severidad `critical`:
   ```
   returncode: 0
   ...
   Escribe 'approve-critical' para aprobar tal cual...
   Saltado — la propuesta sigue pendiente.
   records/ contents: []
   proposals/ contents: ['constraint.eof-test.yaml']
   ```
   `stdin.read_line` devuelve `Ok(0)` en EOF (no es un error), la cadena vacía resultante no coincide con ninguna palabra de confirmación válida, y cae al camino `else` ("Saltado"). La propuesta crítica permanece intacta y pendiente — el diseño sostiene contra este vector concreto.

4. **`statement` corregido vacío en el flujo `'c'` se rechaza antes de tocar disco, sin perder la propuesta.** Se envió una línea vacía como nuevo statement seguida de la palabra de confirmación real:
   ```
   error aprobando: Record inválido: falta campo obligatorio 'statement'
   records/: []
   proposals/: ['constraint.empty-correction-test.yaml']
   ```
   `storage::validate()` (compartida entre lectura y escritura) rechaza el `Record` antes de que `write_record` toque disco; `approve()` propaga el error vía `?`, así que `std::fs::remove_file(&proposal.path)` nunca se ejecuta — la propuesta original permanece intacta en `proposals/`, no se pierde.

5. **Escritura concurrente CRUZANDO PROCESOS reales del SO sobre el mismo `record_id` nunca corrompe el archivo.** A diferencia del test unitario existente (`concurrent_writes_to_same_record_never_corrupt_the_file`, que usa hilos dentro de un mismo proceso), se lanzaron **12 procesos `rationale serve` reales y separados**, cada uno llamando `finalize_change` con el mismo `record_id` simultáneamente:
   ```
   12/12 finalize_change calls reported proposal_written=True
   === final proposal file content ===
   statement: statement-from-writer-11
   ... (YAML completo, bien formado, un único candidato limpio)
   leftover tmp files: []
   ```
   El resultado final es exactamente uno de los 12 candidatos, completo y válido — nunca una mezcla ni un archivo a medio escribir, y sin temporales huérfanos. El patrón de escritura atómica (archivo temporal + `rename` en el mismo directorio) sostiene también entre procesos del SO, no solo entre hilos.

6. **`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test --release` pasan limpio**: 79 tests unitarios + 8 de integración MCP + 8 de validación de schema = 95/95, cero advertencias, en este entorno.

---

## Resumen de severidades

| Severidad | Cantidad | Hallazgos |
|---|---:|---|
| Crítico | 0 (nuevo) | El único crítico de esta fase (path traversal vía `record_id`) ya fue encontrado y corregido antes de esta revisión (`c9fd5b6`); re-verificado independientemente, sostiene. |
| Alto | 3 | 1 (lectura de Subjects/Records corrupta apaga el Resolver en silencio), 2 (TOCTOU propuesta↔revisión: pérdida de propuestas y de aprobaciones), 3 (inyección de control/ANSI en el terminal del revisor) |
| Medio | 4 | 4 (`signals_from_paths` falso positivo por substring), 5 (`determine_level` falso negativo en cambio crítico sin keyword/lenguaje normativo), 6 (Jaccard `Alias` falso positivo bloquea propuesta legítima), 7 (Jaccard falso negativo permite fragmentación silenciosa de Subjects) |
| Menor | 0 | — |
| Sostiene | 6 | fix de path traversal re-verificado; garantía de no-autoaprobación re-verificada independientemente; EOF/stdin no interactivo nunca aprueba; corrección vacía rechazada sin pérdida; concurrencia cruzando procesos nunca corrompe; suite completa limpia |

**Recomendación sobre bloqueo (a criterio de esta revisión, la decisión final es del dueño humano):** los hallazgos 1, 2 y 3 (los tres "Alto") comparten una característica que los hace más urgentes que los "Medio": los tres son **silenciosos** — ninguno produce un error visible, un panic capturado, o siquiera una entrada en `diagnostics`; en los tres casos el sistema reporta éxito (`proposal_written: true` o `Aprobado -> ...`) mientras hace algo distinto de lo que su propia documentación promete (cobertura completa, ninguna pérdida silenciosa, "una afirmación por pantalla" fiel a lo que se persiste). Los hallazgos 4-7 son reales y merecen corregirse, pero son ruido o gaps de precisión conocidos y ya parcialmente reconocidos en los comentarios del propio código (`signals.rs`/`subjects.rs` se declaran "deliberadamente crudos"), no violaciones silenciosas de una garantía ya prometida como cumplida.

La decisión sobre qué corregir, y si Fase F se considera cerrada tal cual o requiere una iteración de seguridad adicional (al estilo del propio `c9fd5b6`, que corrigió un hallazgo de esta misma naturaleza durante el cierre de esta fase), queda enteramente para el dueño humano del proyecto (`evaluation.no-self-certification`).
