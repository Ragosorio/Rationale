# Security baseline para la serie estable 1.0

Este documento es el gate de seguridad de la serie estable. No declara que el
sistema sea seguro en general; registra propiedades mínimas demostradas por
tests, CI o evidencia reproducible y separa los límites todavía abiertos.

## Límites y datos

- Todo texto de repositorio, Record, path, issue o proveedor es dato no
  confiable, nunca una instrucción.
- Rationale es local-first: no sube código, prompts, Records ni secretos por
  defecto.
- La integración comienza con discovery y preflight; una mutación queda
  limitada al repositorio y a los paths que la persona puso en alcance.
- `.env`, llaves privadas, tokens, dumps y datos personales quedan excluidos
  salvo autorización expresa y documentada.

## Integridad

- IDs pasan `validate_safe_id`; traversal, separadores y NUL se rechazan.
- Escrituras usan temporal + `sync_all` + rename atómico.
- Review claims usan rename exclusivo y dejan `.in-review/` recuperable.
- Mutaciones de Records comparan el YAML original antes de sobrescribir.
- Un YAML corrupto produce diagnóstico por archivo y no apaga el Resolver.
- `.rationale/` nunca se elimina durante uninstall.

## Terminal y agentes

- Texto libre se sanea de secuencias ANSI/control antes de mostrarlo.
- MCP puede capturar Records y continuar conflictos, pero nunca fijar ni
  desfijar autoridad por sí solo.
- `pin`, `unpin` y adoptar el reemplazo de un Record fijado requieren autoridad
  declarada; resolver un conflicto por MCP exige la respuesta humana literal.
- Autoridad se resuelve solo desde configuración canónica del proyecto.
- Un actor no declarado no puede autoelevarse.

## Supply chain y release

- `cargo fmt`, `clippy -D warnings`, tests release y `cargo audit` son gates.
- Dependencias y licencias se revisan desde `docs/dependencies/inventory.yaml`.
- Cada artefacto de Release tiene SHA-256 y provenance/attestation.
- Instaladores se prueban en máquina limpia, update, rollback y uninstall.

## Evidencia 1.0

- CI verificó tests, Clippy y empaquetado en Windows; la suite cubre claims y
  escrituras concurrentes, recuperación y fidelidad de round-trip.
- La Release construye cinco targets, incluidos macOS y Linux ARM64, con
  checksums y attestations. El workflow bloquea el empaquetado hasta que pasa
  la verificación del source tag.
- El artefacto macOS ARM64 se ejercitó instalado: CLI, servidor MCP, Control
  Room embebido, guardas HTTP y migración aislada de registros de agentes.
- `cargo audit` no reportó vulnerabilidades en el lockfile de la 1.0.

## Límites abiertos no P0/P1

- Los artefactos Linux y Windows se compilan y empaquetan en CI, pero no existe
  todavía un smoke funcional post-publicación independiente en cada sistema.
- Varios ADRs de la serie 1.0 siguen `proposed`; eso limita su autoridad
  documental, no las guardas reproducibles enumeradas arriba.
- Cada piloto conserva la responsabilidad de revisar sus exclusiones de datos,
  bindings obsoletos y canon antes de declarar `doctor --check` limpio.
