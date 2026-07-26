# Revisión adversarial: Fase E5/E6 (Context Compiler, superficie MCP)

**Rol:** Review Agent independiente (`Proceso §4.4` — "otra sesión" como revisor válido), sin contexto previo de la sesión que implementó Fase E.
**Encargo:** intentar refutar el Context Compiler (`src/retrieval.rs`), la superficie MCP (`src/mcp/`, `src/pipeline.rs`, `src/providers/mod.rs`) y la afirmación de amortización real de Fase E5, sin autoaprobar nada.
**Esta sesión no aprobó ni rechazó nada** — el veredicto queda para revisión humana, siguiendo el mismo patrón que `docs/work-items/adversarial-review-adr-0001-0002-0006.md`.

Metodología: lectura completa del código de `src/retrieval.rs`, `src/mcp/server.rs`, `src/mcp/framing.rs`, `src/pipeline.rs`, `src/providers/mod.rs`, `src/providers/codebase_memory.rs`; `cargo test`/`cargo clippy --all-targets -- -D warnings`/`cargo fmt --check`; ataque empírico contra el binario compilado (`target/release/rationale serve`) con un cliente Python que habla el framing `Content-Length` directamente; un proyecto Rationale sintético (`.rationale/records/` con Records de control) para aislar `compile_packet` sin depender del contenido real del repo; un worktree del commit anterior al refactor de pipeline (`f774db7`) para verificar byte-identidad de forma independiente; y medición directa de latencia (CLI vs. servidor MCP vs. `codebase-memory-mcp` en crudo).

Commits revisados: `f774db7` (Context Compiler), `74a16b3` (superficie MCP), `b7d978a` (suite ampliada E6), `a6a78b4` (docs). `git status` estaba limpio al iniciar esta revisión; no se modificó ningún archivo de producción.

---

## Resumen ejecutivo

| Área | Hallazgos | Severidad más alta |
|---|---|---|
| `src/mcp/framing.rs` (framing sin runtime async) | 2 | **Crítico** — abort de proceso y crecimiento de memoria sin cota, ambos triviales de disparar, ninguno cubierto por tests |
| Terminación silenciosa de sesión ante JSON inválido/anidado | 1 | **Alto** — contradice la premisa central de Fase E5 (sesión persistente amortizada) |
| `compile_packet` — budget de tokens (`src/retrieval.rs`) | 1 | Medio |
| `compile_packet` — `additional_history_available` (`src/retrieval.rs`) | 1 | Medio |
| `detect_conflict` (`src/retrieval.rs`) | 1 | Medio |
| `token_estimate` (`src/retrieval.rs`) | 1 | Menor |
| Amortización real medida vs. narrativa de 6.8s | 1 (matiz, no bug) | — |
| `catch_unwind` del servidor MCP | Sostiene | — |
| Byte-identidad del refactor de pipeline | Sostiene | — |
| `cargo test`/`clippy`/`fmt` | Sostiene | — |
| `write_message` con `.expect()` fuera de `catch_unwind` | 1 | Menor/teórico |

**9 hallazgos accionables** (2 críticos, 1 alto, 3 medios, 2 menores, 1 matiz sin severidad de bug) + **4 confirmaciones que sostienen**.

---

## Hallazgos completos

### A. `src/mcp/framing.rs` — el framing no tiene límites (Crítico)

El comentario del módulo dice: "nunca bloquea de forma indefinida por sí solo". Esto es cierto para el *bloqueo*, pero el framing no impone ningún límite de tamaño, ni al header ni al body, y ambos caminos son alcanzables por cualquier cliente (o bug de cliente) antes de que el pipeline o `catch_unwind` entren en juego — el framing corre en el bucle principal de `run()` (`src/mcp/server.rs:34`), fuera de cualquier `catch_unwind`.

**A1 — `Content-Length` extremo aborta el proceso (SIGABRT), no es un panic capturable.**

`src/mcp/framing.rs:36`: `let mut body = vec![0u8; length];` — `length` viene directo del header, sin cota superior. Un valor que excede la memoria disponible dispara `handle_alloc_error`, que en Rust **aborta el proceso** (no es un `panic!` normal, `catch_unwind` no lo captura bajo ninguna circunstancia).

Evidencia reproducible:
```
$ python3 - <<'EOF'
# (script completo en el reporte; envía tras 'initialize':)
# Content-Length: 999999999999999999\r\n\r\n{}
EOF
huge_content_length: proceso murió con code=-6; stderr=memory allocation of 999999999999999999 bytes failed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
Código de salida -6 = `SIGABRT`. Un solo mensaje malformado de ~40 bytes tumba el proceso del servidor MCP por completo — exactamente lo que Fase E5 dice evitar ("un target inválido... nunca tumba la sesión completa"), pero para un vector distinto al que el test cubre.

**A2 — sin terminador de header, el buffer crece sin cota (memoria sin límite).**

`src/mcp/framing.rs:16-26`: el bucle que busca `\r\n\r\n` empuja bytes a un `Vec<u8>` indefinidamente si el terminador nunca llega. No hay límite de tamaño de header en el código de producción (el test de integración sí impone uno — `assert!(header.len() < 4096, ...)` en `tests/mcp_server.rs:66` — pero esa aserción vive solo en el *test*, no en `framing.rs`).

Evidencia reproducible: enviar 150MB de bytes `X` sin nunca completar `\r\n\r\n`:
```
RSS del proceso tras enviar 150MB sin terminador de header: 154.0 MB (pid=64175, vivo=True)
```
El proceso queda vivo, consumiendo memoria proporcional a lo que el cliente decida enviar — sin cota, sin timeout, sin desconexión. Un cliente que se cuelga a mitad de un mensaje (conexión lenta, bug, o adversario) puede crecer la memoria del servidor indefinidamente.

**Por qué esto es crítico y no un matiz:** ambos vectores están completamente fuera de `catch_unwind` (viven en `framing::read_message`, llamado antes de cualquier despacho a herramienta), ninguno está cubierto por `tests/mcp_server.rs` ni por ningún otro test del repo, y ambos se disparan con payloads triviales (40 bytes y 150MB respectivamente — ninguno requiere sofisticación). El comentario en `server.rs:10-13` promete que "stdout es EXCLUSIVAMENTE del protocolo" y que los panics de herramienta se capturan — pero un proceso abortado (A1) no deja ni eso: no hay stdout que corromper porque no hay proceso.

**Corrección sugerida (no aplicada — decisión del dueño humano):** imponer un límite superior explícito a `Content-Length` (p. ej. unos pocos MB, muy por encima de cualquier packet real observado — el mayor `token_estimate` medido en este reporte fue de unos cientos) y un límite de tamaño de header antes de intentar `vec![0u8; length]`; devolver `None` (o un error JSON-RPC) en vez de abortar.

---

### B. Un solo mensaje JSON inválido termina la sesión persistente completa, en silencio (Alto)

Fase E5 existe explícitamente para amortizar el costo de una sesión de larga duración (`src/mcp/server.rs:1-8`, `src/providers/mod.rs:62-69`). Si un mensaje entrante no parsea como JSON — ya sea por estar mal formado, truncado, o por exceder el **límite de recursión por defecto de `serde_json` (128 niveles, `de.rs:63`)** — `read_message` devuelve `None` (línea 38: `serde_json::from_slice(&body).ok()?`), indistinguible de EOF. El bucle principal `while let Some(msg) = framing::read_message(...)` (`server.rs:34`) termina, y `run()` retorna: el proceso sale limpiamente (`exit code 0`), sin escribir ningún mensaje de error al cliente y sin ningún diagnóstico en stderr.

Evidencia reproducible — un payload de **260 bytes**, JSON sintácticamente válido (130 niveles de arrays anidados, muy por debajo de cualquier "ataque" imaginable — podría ocurrir con una estructura de datos real anidada, como un AST o config profundamente anidada):
```python
depth = 130
body = ("[" * depth + "]" * depth).encode()  # 260 bytes
# enviado tras initialize + notifications/initialized
```
Resultado:
```
returncode: 0
stdout leftover: b''
stderr: ''
```
Confirmado también con JSON directamente malformado (`{not valid json!!!`) — mismo resultado: `returncode: 0`, sin stderr.

**Por qué es alto, no solo un matiz:** esto no es un panic — `catch_unwind` nunca tiene oportunidad de actuar, porque el problema ocurre antes de que el mensaje llegue a `handle_tools_call`. La sesión completa (el activo que Fase E5 existe para amortizar) muere ante el primer mensaje que no logre parsear, sin ningún aviso — un cliente MCP real con un bug menor en su serialización (o que envíe una estructura de datos legítimamente anidada por encima de 128 niveles) pierde toda la sesión sin saber por qué, y debe reconectar pagando de nuevo el costo de arranque que se buscaba evitar. **Ningún test del repo (`tests/mcp_server.rs`) cubre este camino** — la suite de E6 solo prueba herramienta desconocida, target inexistente y `project_root` sin `.rationale/`, los tres casos que sí pasan por `catch_unwind` con éxito.

**Corrección sugerida:** distinguir explícitamente EOF real (cierre del stdin del cliente) de "no se pudo parsear un mensaje" en `read_message` — devolver una variante de error en vez de `None` para el segundo caso, y responder con un error JSON-RPC (`-32700 Parse error`) sin terminar la sesión.

---

### C. `compile_packet` puede exceder `max_tokens` en silencio (Medio)

El bucle de recorte (`src/retrieval.rs:218-230`) solo puede reducir `affected_targets` y `known_risks` — nunca `protected_tokens` (niveles 0-3: constraints críticas, conflictos de intención, razón principal), por diseño explícito y correcto (v0.5 §30.1.7: nunca omitir una constraint crítica). Pero si `protected_tokens` por sí solo ya excede `budget.max_tokens`, el bucle vacía `affected_targets` y `known_risks` por completo y luego hace `break` — el packet final se sirve con `token_estimate > max_tokens`, y **no se añade ninguna advertencia a `warnings`** indicando que el budget no se respetó.

Evidencia reproducible (proyecto de control con un Record cuyo `statement`+`rationale` sencillos suman más que el budget):
```
=== Caso D: max_tokens=1 ===
critical_constraints count: 1
known_risks: []
affected_targets: []
token_estimate: 39  (excede max_tokens=1: True)
warnings: ['no se encontró el símbolo dentro de la cobertura disponible; no implica que no exista']
```
El único warning presente es uno no relacionado (símbolo no encontrado); nada en `warnings` menciona que `token_estimate` (39) excede `max_tokens` (1). El test existente `tiny_budget_never_drops_critical_constraints` (`retrieval.rs:425`) verifica que las constraints críticas no se recortan — correcto — pero no verifica la ausencia de aviso de sobre-presupuesto, así que este comportamiento pasó sin detectarse.

**Por qué importa:** un caller (agente o herramienta downstream) que confía en `token_estimate` para decidir si el packet cabe en su ventana de contexto no tiene forma de saber, solo mirando el packet, que el budget solicitado no se cumplió — tendría que comparar `token_estimate` contra el `max_tokens` que él mismo pidió, un chequeo que el propio protocolo debería hacerle innecesario.

**Corrección sugerida:** si `token_estimate_total > budget.max_tokens` al final de `compile_packet`, añadir una entrada a `warnings` (p. ej. `"budget de tokens excedido: N > max_tokens M — el contenido protegido (niveles 0-3) no se recorta nunca"`).

---

### D. `additional_history_available` subestima sistemáticamente lo que se recortó (Medio)

El contador (`src/retrieval.rs:198-239`) se compone de dos partes: (1) constraints críticas no incluidas por `max_critical_constraints` — esta parte es correcta y está bien testeada (`budget_caps_critical_constraints`); y (2) un **flag fijo de `+1`** si `known_risks.len()` terminó por debajo del mínimo entre el total de risks y `max_risks` — sin importar cuántos risks se recortaron realmente, y **sin considerar en absoluto cuántos `affected_targets` se recortaron por presupuesto**, porque el bucle de recorte (línea 218-230) pop-ea primero de `affected_targets` y solo después de `known_risks`.

Evidencia reproducible (proyecto de control, un Record con 6 `binding_declarations` distintos y 5 `risks`, variando `max_tokens`):
```
max_tokens=104: known_risks=5 affected_targets=5 additional_history_available=0
max_tokens=100: known_risks=5 affected_targets=4 additional_history_available=0
max_tokens=95:  known_risks=5 affected_targets=2 additional_history_available=0
max_tokens=90:  known_risks=5 affected_targets=0 additional_history_available=0   <-- 6 targets eliminados, contador en 0
max_tokens=85:  known_risks=4 affected_targets=0 additional_history_available=1   <-- 1 risk eliminado -> "+1"
max_tokens=40:  known_risks=0 affected_targets=0 additional_history_available=1   <-- 5 risks + 6 targets eliminados -> sigue en "+1"
```
En `max_tokens=90`, los 6 `affected_targets` (bindings estructurales reales hacia `src/one.rs`...`src/six.rs`) desaparecen del packet por completo, y el campo diseñado exactamente para señalar "hay más disponible, expande si lo necesitas" (v0.5 §18.2, progressive disclosure) reporta **0** — el caller no tiene ninguna señal de que algo se omitió. En `max_tokens=40`, se eliminaron 11 elementos en total (5 risks + 6 targets) y el contador solo llega a "1".

**Por qué importa:** esto rompe la garantía de "progressive disclosure" que el campo dice implementar — un agente que confía en `additional_history_available == 0` para decidir que vio todo el contexto relevante estaría equivocado en el caso más común de recorte por presupuesto (afectando `affected_targets`, que es precisamente donde vive la estructura de código impactada).

**Corrección sugerida:** llevar dos contadores separados de "elementos recortados por presupuesto" (uno para `affected_targets`, otro para `known_risks`), sumando cuántos se eliminaron realmente en el bucle de recorte, no un flag booleano.

---

### E. `detect_conflict` produce falsos positivos y falsos negativos reales (Medio)

El código ya es honesto en el comentario ("deliberadamente crudo... nunca pretende comprensión semántica"), pero el packet no propaga ninguna calificación de confianza al string que sí suena definitivo: `"La intención puede entrar en conflicto con '{id}': {statement}"`.

**Falso positivo confirmado** — dos temas sin relación real, solapamiento de vocabulario de dominio genérico ("checkout", "page"):
```
intent: "Update the login button color and add a loading spinner for the checkout page"
constraint: "The checkout page must load a fraud-detection script before allowing payment."
-> intent_conflicts: ["La intención puede entrar en conflicto con 'constraint.conflict-test': ..."]
```
Cambiar el color de un botón de login no tiene relación real con un constraint de fraude en checkout; el solapamiento es accidental ("checkout" + "page", ambos de dominio, no de conflicto semántico).

**Falso negativo confirmado** — el mismo concepto peligroso (fuga de secretos vía logs), vocabulario distinto:
```
intent: "I'm going to dump auth secrets into the debug console output for troubleshooting"
constraint: "Passwords must never be written to the application log files for any reason."
-> intent_conflicts: []   (vacío — ningún conflicto detectado)
```
"Auth secrets"/"debug console output" no comparte ninguna palabra de más de 3 letras con "Passwords"/"log files" — el umbral de `overlap >= 2` (línea 125) nunca se activa pese a ser, en cualquier lectura razonable, exactamente el escenario que el constraint intenta prevenir.

**Por qué importa (medio, no crítico):** el mecanismo de nivel 2 es aditivo — nunca bloquea nada por sí mismo (correcto, v0.5 §19.1: recuperación determinista, sin heurísticas semánticas que decidan). El riesgo real es de **falsa confianza en ambas direcciones**: un agente podría descartar una advertencia de conflicto genuinamente irrelevante como "ruido" (entrenándose a ignorarlas), y en el caso simétrico, un intento real de violar el constraint pasaría sin ninguna señal. Ninguno de los dos casos está cubierto por los tests existentes (`intent_conflict_detected_by_word_overlap` solo prueba un solapamiento directo de vocabulario compartido, no adversarial).

**Corrección sugerida:** ninguna aquí es trivial sin introducir heurísticas semánticas (explícitamente fuera de alcance, §28.3) — la opción más barata es matizar el string servido (p. ej. `"posible solapamiento léxico, no verificado semánticamente"`) para que el consumidor sepa que es una señal de recall barato, no un veredicto.

---

### F. `token_estimate` (chars/4) no es un proxy estable — la dirección del error cambia según el contenido (Menor)

Medido contra `tiktoken` (`cl100k_base`, el mismo vocabulario de referencia usado ampliamente para modelos de esta familia) sobre muestras representativas del propio repo:

| Muestra | chars | tokens reales | estimado (chars/4) | error |
|---|---:|---:|---:|---:|
| Prosa en inglés (statement real del repo) | 289 | 52 | 72 | **+38.5%** (sobreestima) |
| Prosa en inglés corta | 82 | 12 | 20 | **+66.7%** (sobreestima) |
| Equivalente en español | 90 | 25 | 22 | **-12.0%** (subestima) |
| ID técnico (`constraint.no-provider-internal-access`) | 38 | 6 | 9 | **+50.0%** (sobreestima) |
| Path + símbolo (`src/providers/....rs::Cliente::método`) | 70 | 14 | 17 | +21.4% (sobreestima) |

El comentario del código (`retrieval.rs:69-72`) ya es honesto — es un "proxy", no una medición exacta — pero la dirección del error **no es consistente**: para prosa en inglés e IDs técnicos con puntuación, chars/4 sobreestima considerablemente (hasta +66%); para texto en español, subestima (-12%). Esto contradice la hipótesis inicial de este encargo (que el español subestimaría *más* que el inglés por acentos) — el resultado real es más sutil: el inglés se sobreestima fuerte, el español se subestima moderadamente, ninguno es "cercano".

**Por qué importa (menor por sí solo, pero compone con el hallazgo C):** dado que el hallazgo C ya muestra que exceder el budget no genera ninguna advertencia, un error de estimación del ±12-66% amplía el rango de posibles sobrepasos silenciosos de presupuesto real de tokens frente al que el packet reporta. Los Records actuales del propio repo están en inglés, así que el caso "español subestima" es hoy teórico para este proyecto en particular — pero el propio repo y su documentación están en español, así que no es descartable que Records futuros lo estén.

---

### G. Amortización real: el "6.8s" documentado no se reproduce en este entorno (matiz, no bug de código)

Medí directamente en este entorno (no solo leí el código):

| Escenario | Tiempo medido |
|---|---:|
| `rationale prepare` (CLI, proveedor real spawneado cada vez) | ~130–190ms (5 corridas) |
| `rationale prepare` (CLI, proveedor forzado a `Unavailable` vía `PATH` sin `codebase-memory-mcp`) | ~30–40ms |
| `rationale serve`, llamada `prepare_change` en caliente (sesión ya inicializada) | ~32–37ms (10 corridas) |
| `initialize` directo y **fresco** contra `codebase-memory-mcp` 0.8.1 (proceso nuevo cada vez, sin pasar por Rationale) | **~15–20ms** (3 corridas independientes) |

El último dato contradice directamente `docs/research/codebase-memory/11-performance-observations.md`, que documenta `initialize` en **6.79–6.86s** contra el mismo binario (versión no confirmada como distinta). En este entorno, el handshake `initialize` puro — el costo que Fase E5 dice amortizar — ya no cuesta 6.8s: cuesta ~15-20ms, sea cual sea la sesión.

Esto **no invalida el mecanismo** de Fase E5 (la sesión persistente sigue siendo correcta y sí reduce el costo por-llamada de ~150ms a ~33ms, un factor ~4-5x real y medido) — pero la magnitud dramática (200x, "6.8s -> 33ms") que motiva el commit de Fase E5 y el hallazgo #1 de la revisión adversarial de ADR-0002 **no se reprodujo en esta sesión**. Verifiqué explícitamente que no es una ilusión de fallback silencioso: `provider_status` fue `"successful"` y `provider_coverage` `"complete"`/`"unknown"` en todas las llamadas — el proveedor real se está invocando y respondiendo, no cayendo a `Unavailable` sin que se note.

**Causa de la discrepancia: `Unknown`.** Candidatos no descartados: (a) cachés en disco ya calientes en `~/.cache/codebase-memory-mcp/*.db` (confirmé que existen, varias decenas de MB, acumuladas de sesiones previas contra este y otros repos) que evitarían el costo de indexación que el research original pudo estar midiendo bajo el nombre de "`initialize`"; (b) una versión o entorno de medición distinto al de `11-performance-observations.md`; (c) un cambio real de comportamiento en `codebase-memory-mcp` 0.8.1 entre la fecha de esa investigación y hoy. **Riesgo:** si la cifra de 6.8s era específica de un entorno con caché fría que ya no se reproduce en desarrollo, el beneficio dramático de Fase E5 podría estar sobrestimado en la documentación actual — sin que esto sea un defecto del código de Fase E5 en sí. **Próximo experimento sugerido:** medir `initialize` contra `codebase-memory-mcp` en un entorno con `~/.cache/codebase-memory-mcp/` vacío (contenedor limpio) para aislar la variable de caché.

---

## Lo que sostiene bajo ataque

1. **`catch_unwind` en `src/mcp/server.rs:142-163` captura correctamente los panics de herramienta que sí llegan a él** (`.expect("cargar configuración")`, `.expect("leer records")`, `.expect("no hay Records...")` en `src/pipeline.rs`). Verificado empíricamente con un `project_root` sin `.rationale/`: la llamada devuelve `isError: true` con el mensaje genérico esperado, y la sesión responde correctamente a una llamada `health` inmediatamente después. Esto sostiene exactamente como lo describe el commit `74a16b3` — **para los panics que ocurren dentro de la ejecución de la herramienta**. Los hallazgos A y B de este reporte muestran que existen panics/aborts/terminaciones que ocurren *antes* de esa frontera (en `framing::read_message`), fuera del alcance de esta garantía — el commit no lo declara falso, pero tampoco delimita el alcance real de la protección.

2. **Byte-identidad del refactor de pipeline, verificada de forma independiente.** Compilé un worktree del commit inmediatamente anterior (`f774db7`, antes de `feat(mcp)`) y comparé `rationale health`, `rationale prepare src/main.rs --project-root . --repo-path .`, y la misma llamada con `--intent "test intent phrase"`, contra el binario actual (`a6a78b4`). `diff` vacío en stdout y stderr en los tres casos. La afirmación del commit `74a16b3` ("Verificado byte-idéntico contra el binario pre-refactor en CLI") se reproduce de forma independiente.

3. **`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, y `cargo test` (50 tests: 40 unitarios + 2 de integración MCP + 8 de validación de schema) pasan limpio**, sin advertencias ni fallos, en este entorno.

4. **La regla "stdout es exclusivamente del protocolo" se sostiene en el sentido estricto que el test verifica**: en ninguno de los casos que sí llegan a producir una respuesta (herramienta desconocida, target inválido, panic capturado) se corrompió el framing — cada mensaje que el proceso efectivamente emitió fue un `Content-Length` válido. Los hallazgos A/B no contradicen esto: en A1 el proceso aborta sin emitir nada más; en A2/B el proceso permanece silencioso o termina limpiamente, pero no emite bytes corruptos.

---

## Resumen de severidades

| Severidad | Cantidad | Hallazgos |
|---|---:|---|
| Crítico | 2 | A1 (Content-Length astronómico → SIGABRT), A2 (header sin terminador → memoria sin cota) |
| Alto | 1 | B (JSON inválido/anidado mata la sesión persistente en silencio) |
| Medio | 3 | C (budget excedido sin warning), D (`additional_history_available` subestima), E (`detect_conflict` falsos positivos/negativos) |
| Menor | 2 | F (`token_estimate` no estable), `write_message` con `.expect()` fuera de `catch_unwind` (teórico, no se encontró forma práctica de disparar con los tipos actuales) |
| Matiz (no bug) | 1 | G (narrativa de amortización de 6.8s no reproducida en este entorno; causa `Unknown`) |

**Recomendación sobre bloqueo:** a criterio de esta revisión, **`src/mcp/framing.rs` (hallazgos A1, A2) y el manejo de mensajes no parseables en `src/mcp/server.rs`/`src/mcp/framing.rs` (hallazgo B) deberían tratarse como bloqueantes antes de considerar cerrada Fase E**, porque contradicen directamente la garantía de robustez que la propia suite de tests de Fase E6 (`tests/mcp_server.rs`) afirma cubrir ("si un solo byte de stdout dejara de ser un mensaje Content-Length bien formado... esa es la aserción real") sin en realidad ejercitar los caminos donde el proceso completo muere o crece sin cota. Los hallazgos C, D y E son reales y deberían corregirse, pero no bloquean por sí solos: los niveles 0-3 del packet (la garantía más importante, v0.5 §30.1.7) nunca se vieron comprometidos en ningún experimento. El hallazgo G no es un defecto de código — es una discrepancia de evidencia entre el research histórico y el entorno actual que amerita una nota en `docs/research/codebase-memory/11-performance-observations.md` o un nuevo research item, no una corrección de código.

La decisión sobre qué corregir, y si algún ADR o pieza de código pasa a `accepted`, queda enteramente para el dueño humano del proyecto (`evaluation.no-self-certification`).
