# Build and test

Requiere el toolchain de Rust (`rustc`/`cargo`). En shells no interactivos, asegúrate de que esté en el `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Verificación completa (lo que corre antes de cada commit)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Los tres deben pasar limpio. `cargo test` corre tres binarios: los tests unitarios (`src/`), `tests/mcp_server.rs` (integración contra el binario real compilado, incluyendo los ataques de la revisión adversarial de Fase E) y `tests/schema_validation.rs`.

## Build de desarrollo vs release

```bash
cargo build              # debug, target/debug/rationale
cargo build --release    # optimizado, target/release/rationale — el que usa .mcp.json
```

## Auditoría de dependencias

```bash
cargo audit
```

Requiere `cargo-audit` instalado (`cargo install cargo-audit --locked`). Corrido por última vez el 2026-07-25 sobre 38 dependencias: 0 vulnerabilidades, 0 advertencias — ver `docs/dependencies/inventory.yaml`. Vuelve a correrlo cada vez que cambien las dependencias del `Cargo.toml` raíz.

## Regenerar el fixture de la vertical slice

```bash
bash fixtures/vertical-slice/setup.sh
```

Genera un repo Git determinista en `fixtures/vertical-slice/repo/` (ignorado por Git, ver `.gitignore`) usado por varios tests (`storage::tests::reads_fixture_record_with_approval_and_binding`, etc.).

## Estabilidad bajo carga

La suite es determinista incluso corriendo muchas veces seguidas — no debería fallar de forma intermitente. Si un test falla solo bajo `cargo test` repetido pero nunca en aislamiento, es una señal real de contención (ver el historial de Fase F5: dos bugs de flakiness reales — un `SQLITE_BUSY` por falta de `busy_timeout`/DDL redundante en `cache.rs`, y colisión de nombres de directorio temporal por resolución de reloj insuficiente — encontrados exactamente así). Reproducir con:

```bash
for i in $(seq 1 20); do cargo test 2>&1 | grep -E "test result|FAILED"; done
```
