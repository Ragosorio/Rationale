# Revisión adversarial: ADR-0001, ADR-0002, ADR-0006

**Rol:** Review Agent independiente (`Proceso §4.4` — "otra sesión" como revisor válido).
**Encargo:** intentar refutar los tres ADRs `proposed`, sin autoaprobar. Metodología: lectura completa de ADRs, research notes citadas, código de ambos spikes, historial de git, e implementación real (`src/`).
**Esta sesión no aprobó ni rechazó nada** — el veredicto queda para revisión humana.

---

## Resumen ejecutivo

| ADR | Veredicto | Hallazgo más severo |
|---|---|---|
| ADR-0001 (Rust) | Sostiene con matices | Ninguno bloqueante; la conclusión resiste reponderación agresiva de los pesos (verificado recalculando 5 escenarios distintos — Rust gana en todos, incluso invirtiendo el criterio disputado). Debilidad real: el criterio de mayor peso ("seguridad de memoria y confiabilidad", 20%) se justifica con evidencia de gestión de subprocesos, no de memoria en sentido estricto; y el hallazgo central (footgun de Go) no quedó en un commit auditable, solo en prosa. |
| ADR-0002 (sesión MCP persistente) | Sostiene con matices significativos | **La implementación real de Fase D (`src/main.rs`) no logra la amortización que el ADR reclama** — cada invocación de la CLI (`rationale prepare`) paga el handshake completo de ~6.8s, porque el binario es hoy un proceso de un solo uso, no un daemon. La ventaja medida (15-30ms por llamada) solo se realiza *dentro* de una invocación, nunca *entre* invocaciones. Depende de la pregunta arquitectónica todavía abierta en `Arquitectura §28`: "¿un proceso por sesión o daemon compartido?". |
| ADR-0006 (revisión desde Git) | Sostiene | El más robusto de los tres. Evidencia central doblemente corroborada (`05` y `08` de forma independiente). Matices de implementación: gaps de test en symlinks/submodules/repo sin commits; y el ADR describe "hash de contenido cuando aplique" pero el código real solo calcula un booleano `working_tree_dirty`. |

## Hallazgos completos (por ADR)

### ADR-0001

1. El footgun de Go (5016ms) no existe como commit auditable — solo hay un commit final ya corregido (`6b5d47e`). El "antes" solo se narra en `candidates.md`/`benchmark-results.json`. *(matiz)*
2. `spike-notes.md` admite que el resultado fue "consecuencia del estilo de implementación manual", no una garantía estructural del lenguaje — el ADR generaliza más de lo que su propia evidencia sostiene. *(matiz)*
3. La evidencia citada para "seguridad de memoria y confiabilidad" (20%, el peso más alto) es en realidad sobre ergonomía de gestión de subprocesos (`os/exec` vs manual poll), no sobre memoria en sentido tradicional. *(matiz — rigor metodológico)*
4. Doble conteo: el mismo hecho (Go evita cgo con `modernc.org/sqlite`) se premia en "SQLite y filesystem" y se penaliza en "Interoperabilidad con procesos C" — simétricamente para Rust con `flock` FFI. Los efectos se cancelan aproximadamente, no cambia el resultado. *(cosmético)*
5. **Verificación de sensibilidad de la puntuación (respuesta directa a la pregunta central del encargo):** recalculado bajo 5 escenarios de repeso distintos (incluyendo eliminar por completo el criterio disputado, o invertir su puntuación). Rust gana en todos. *(sostiene — la conclusión es robusta)*
6. "Distribución como binario" (15%) se puntúa solo con tamaño de archivo — nunca se probó firma de código, instaladores, ni empaquetado real, aunque esto ya está reconocido transparentemente en `spike-notes.md`/`compatibility-matrix.md`. *(matiz)*

### ADR-0002

1. **[El más importante del reporte]** `cmd_health`/`cmd_prepare` en `src/main.rs` llaman `CodebaseMemoryClient::spawn()` al inicio de cada función, y el binario retorna al terminar. Cada ejecución de la CLI es un proceso del SO nuevo que paga el handshake completo. Esto es exactamente el perfil de la alternativa "sesión MCP nueva por operación" que el propio ADR-0002 descarta explícitamente. *(bloqueante para la realización actual, matiz para la decisión de transporte en sí)*
2. Sin correlación de `id` de respuesta — el diseño depende de que las llamadas sean estrictamente secuenciales (documentado en un comentario, pero no verificado con un `id` real). Si Rationale necesita concurrencia futura, atribuiría respuestas incorrectamente. *(matiz)*
3. La mitigación de riesgo prometida ("manejo de señales explícito") no está implementada — no hay `ctrlc`/`signal-hook` en el código, solo un `Drop` que no corre ante `SIGINT`/`SIGTERM` no manejado. *(matiz)*
4. Contención de SQLite entre múltiples procesos Rationale concurrentes (dos terminales, dos agentes) no discutida en Risks. *(matiz)*
5. La medición central (15-30ms cálido vs 2.2-6.8s frío, 3 corridas, <5% varianza) es sólida. *(sostiene)*

### ADR-0006

1. Evidencia central doblemente corroborada: `05-revision-and-coverage.md` (detect_changes falla) y `08-workspaces-and-monorepos.md` (capability presente que falla en silencio) son hallazgos independientes que se refuerzan. *(sostiene)*
2. La causa raíz de `detect_changes` sigue siendo "Unknown" en la propia evidencia — el ADR generaliza de una anomalía no aislada causalmente hacia un principio arquitectónico permanente. Razonable como default conservador, pero debería ser más honesto sobre esto. *(matiz)*
3. **Pregunta directa del encargo — ¿hay un caso legítimo de revisión "virtual" que el ADR descarta sin justificación?** No se encontró contraejemplo: `v0.5 §4.19` ya limita el alcance a Git aguas arriba, y el ADR permite la señal del proveedor como metadato diagnóstico, solo no como autoridad. *(sostiene)*
4. Casos borde sin test: symlinks, submodules (`git status --short` puede no recorrerlos según config), repo recién `git init` sin commits todavía. *(matiz — gap de test, no bug confirmado)*
5. `check_consistency` puede etiquetar `WorkingTreeAhead` cuando en realidad hay dos problemas superpuestos (dirty + revisión distinta) — inofensivo funcionalmente, engañoso como diagnóstico. *(cosmético)*
6. El ADR describe "hash de contenido cuando aplique" pero el código real solo calcula un booleano `dirty` — cualquier archivo no confirmado (incluso irrelevante) invalida todos los Records por igual. Consistente con "fallar con humildad", pero el ADR promete más precisión de la que el código tiene. *(matiz)*

## Decisión pendiente

El dueño humano del proyecto decide si estos ADRs pasan a `accepted`, se corrigen primero, o quedan `proposed` con las correcciones aplicadas. El hallazgo de ADR-0002 (#1) se incorpora directamente al propio ADR como reconocimiento explícito, ya que Fase E5 (superficie MCP, siguiente en el plan) es precisamente lo que resuelve ese gap: un servidor MCP es, por construcción, el proceso de larga duración que la CLI de un solo comando no es.
