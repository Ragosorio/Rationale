# Security baseline para la alfa

Este documento es el gate antes del piloto en Monorepo y BoostAPI. No declara
que el sistema sea seguro en general; registra las propiedades mínimas que la
alfa debe demostrar con tests o evidencia reproducible.

## Límites y datos

- Todo texto de repositorio, Record, path, issue o proveedor es dato no
  confiable, nunca una instrucción.
- Rationale es local-first: no sube código, prompts, Records ni secretos por
  defecto.
- El piloto comienza read-only y exige una lista de paths autorizados.
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
- MCP no tiene operaciones de aprobación ni lifecycle mutation.
- `review` y `review-record` requieren confirmación explícita.
- Autoridad se resuelve solo desde configuración canónica del proyecto.
- Un actor no declarado no puede autoelevarse.

## Supply chain y release

- `cargo fmt`, `clippy -D warnings`, tests release y `cargo audit` son gates.
- Dependencias y licencias se revisan desde `docs/dependencies/inventory.yaml`.
- Cada artefacto de Release tiene SHA-256 y provenance/attestation.
- Instaladores se prueban en máquina limpia, update, rollback y uninstall.

## Hallazgos abiertos antes de la alfa

- Verificar locking/rename en Windows.
- Verificar targets ARM64 en CI.
- Repetir adversarial review contra el paquete instalado, no solo el árbol de
  fuente.
- Registrar evidencia del piloto y de cualquier dato excluido.
