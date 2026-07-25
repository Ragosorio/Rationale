# Rust — style guide

Aplica a todo código Rust del repositorio (`spikes/language/rust/` hoy; el núcleo de Fase D en adelante). Complementa, no repite, `AGENTS.md`.

## Herramientas obligatorias

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # necesario en shells no interactivos, ver docs/environment/
cargo fmt --check                       # formato (rustfmt.toml en la raíz del repo)
cargo clippy --all-targets -- -D warnings   # lint, cero warnings tolerados
```

Ningún commit debe introducir código que falle `cargo fmt --check` o que produzca un warning nuevo de clippy. Ambos deben correr antes de cerrar cualquier work item (`AGENTS.md §Quality gates`).

## Convenciones

- **Edition 2021**, toolchain `stable` (no nightly) — fijado por ADR-0001; no usar features nightly-only.
- **`panic = "abort"` en release** (ya configurado en `Cargo.toml` del spike): evita el overhead de unwind en el binario distribuido; el desarrollo/test sigue usando unwind por defecto.
- Nombrar funciones que implementan un paso de un contrato externo con el mismo vocabulario del contrato (ej. `op1_read_record` en el spike, mapeado 1:1 a `spike-protocol.md`) — facilita la trazabilidad para revisión cruzada.
- Preferir `Result<T, E>` explícito sobre `.unwrap()`/`.expect()` en código de producción (Fase D en adelante). El spike usa `.expect()` liberalmente porque es código de investigación de corta vida, no producción — esto **no** es el estándar para Fase D.
- Comentarios solo cuando expliquen un porqué no obvio (invariante, workaround, decisión de diseño) — igual que la política general del proyecto. No repetir en comentarios lo que el nombre de la función ya dice.

## Dependencias

Antes de añadir una dependencia nueva, seguir `docs/dependencies/inventory.yaml` y `Rationale_Proceso_Construccion_Agentes_v0.1.md §19`: ¿es necesaria?, ¿tiene alternativa en std?, ¿licencia compatible?, ¿mantenida?, ¿compila en los tres SO objetivo?

Dependencias ya validadas en el spike (`docs/research/language/candidates.md`), disponibles como punto de partida para Fase D, no como decisión final:

| Crate | Propósito | Nota |
|---|---|---|
| `rusqlite` (feature `bundled`) | SQLite embebido | Vendoriza su propio SQLite en C — sin dependencia del sistema |
| `serde` + `serde_json` | Serialización JSON | Estándar de facto del ecosistema |
| `serde_yaml` | Parseo de Records YAML | Marcado `deprecated` upstream (mantenimiento reducido) — evaluar alternativa (`serde_yml`, `yaml-rust2`) antes de Fase D si esto se confirma como riesgo real |

## Referencias

- [The Rust Book](https://doc.rust-lang.org/book/) (oficial)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- `rustfmt.toml` en la raíz del repo — configuración de formato vigente
