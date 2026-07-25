# 01 — Build and test (CBM-002, CBM-003)

**Fuente de evidencia:** build real ejecutado sobre el clon en HEAD `97ce23f9` (`~/Desktop/codebase-memory-mcp`), en la máquina de referencia (`docs/environment/reference-development-machine.md`).

## Observed

- `scripts/build.sh` (target estándar, sin `--with-ui`) **compiló exitosamente**: `Built: build/c/codebase-memory-mcp`.
- Tiempo real: **2m48.74s** (`361.32s` user CPU, `19.11s` system, `225%` CPU promedio — usó varios de los 10 núcleos disponibles).
- El build compila ~180 gramáticas de tree-sitter (una por lenguaje soportado) más el runtime, vendored `sqlite3`, `lz4`, `zstd`, `mimalloc`, `yyjson`, y los módulos propios (`foundation`, `store`, `cypher`, `mcp`, `daemon`, `discover`, `pipeline`, `simhash`, `semantic`, `traces`, `watcher`, `git`, `cli`, `ui`).
- **Binario resultante: 296.196.432 bytes (≈296 MB)**, Mach-O 64-bit arm64.
- `./build/c/codebase-memory-mcp --version` reporta **`codebase-memory-mcp dev`** — no un número de versión semántico, a diferencia del binario instalado que reporta `0.8.1`.
- El binario release instalado (`~/.local/bin/codebase-memory-mcp`) pesa 269.322.576 bytes (≈257 MB) — de tamaño comparable al build local, pero no idéntico.
- **`make -f Makefile.cbm test-foundation` falla en el link**, con decenas de símbolos `_suite_*` no resueltos (`_suite_security`, `_suite_semantic`, `_suite_watcher`, `_suite_yaml`, `_suite_zstd`, etc.) y termina con `ld: symbol(s) not found for architecture arm64`. Tiempo hasta el fallo: ~6s.
- **La suite completa (`make -f Makefile.cbm test` / target `test-runner`) no se ejecutó** en esta sesión: el historial de commits reciente del propio proyecto (visible en `git log`) menciona explícitamente sharding de tests, ejecución paralela de suites C, y legs de CI de 3 sistemas operativos (macOS/Linux/Windows) con VMs — señales de que la suite completa es sustancialmente más pesada que `test-foundation` y no acotada al alcance de este spike de investigación.

## Claimed

El `README.md` documenta el build estándar como reproducible con prerequisitos simples (compilador C/C++, zlib, Git) y sin mencionar un tiempo esperado de build ni de test.

## Verified

- El build reproduce exactamente los comandos documentados en `README.md §Build from Source` (`scripts/build.sh`).
- El fallo de `test-foundation` es reproducible (segunda ejecución no intentada, pero el error es determinista de link, no de flake de test).

## Unknown

- Si `test-foundation` es un target mantenido activamente o quedó desincronizado del resto de la suite tras el crecimiento de `tests/` (fuertemente sugerido por el propio historial de commits de "test sharding" y "parallel test process").
- Cuánto tarda realmente la suite completa (`test` / `test-par`) — no se ejecutó por costo de tiempo, dado que el alcance de esta epic es analizar contratos de integración, no certificar la calidad interna de Codebase Memory.
- Por qué el build local reporta versión `dev` en vez de un identificador de commit — si esto es intencional (build no-release) o si el release oficial inyecta la versión vía un flag de build no capturado por `scripts/build.sh` estándar.
- Si `--with-ui` (variante con visualización de grafo) cambia sustancialmente el tamaño o el comportamiento — no probado.

## Risk

**Medio.** El binario de 296 MB es consistente con un producto que embebe soporte para ~180 lenguajes vía tree-sitter — no es evidencia de un problema, pero sí un dato relevante para cualquier comparación futura de tamaño de binario si Rationale decidiera algún día vendorizar gramáticas propias (no está en el plan actual). El fallo de `test-foundation` no bloquea la integración de Rationale (que consume el binario release vía MCP, no compila CBM), pero sí limita la capacidad de este research de certificar independientemente el comportamiento interno vía tests unitarios propios de CBM.

## Decision impact

- Confirma que **la fuente completa es compilable en la máquina de referencia** (`Rationale_Arquitectura_Conceptual_v0.1.md §6.6` fase de bootstrap cumplida).
- El build local reportando `dev` en vez de un SHA o versión reafirma el hallazgo de `00-source-lock.md`: **no depender de que el binario se autoidentifique de forma útil**; el adaptador de Rationale no debe intentar inferir compatibilidad de capacidades a partir de un string de versión no confiable.
- No se recomienda invertir tiempo adicional en arreglar `test-foundation` — no es responsabilidad de Rationale mantener la suite de tests de un proveedor externo; se documenta como limitación conocida y se avanza.

## Reproducir

```bash
cd ~/Desktop/codebase-memory-mcp
git rev-parse HEAD   # confirmar 97ce23f9827177fff3858831156e9795c6832b18
time scripts/build.sh
./build/c/codebase-memory-mcp --version
ls -la build/c/codebase-memory-mcp
make -f Makefile.cbm test-foundation   # reproduce el fallo de link
```
