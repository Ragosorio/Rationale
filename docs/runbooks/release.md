# Release runbook

La Release dogfood vigente es `v0.0.0-dogfood.7`; los tags `dogfood.1` a
`dogfood.6` quedan como iteraciones históricas de hardening de CI y no deben
usarse para instalar. La alfa pública prevista es `v0.1.0-alpha.1`.

## Antes del tag

- PR desde `release/fase-g-mvp-local` revisado y CI verde.
- `cargo fmt --check`.
- `cargo clippy --all-targets -- -D warnings`.
- `cargo test --release`.
- `cargo audit`.
- security baseline sin P0/P1 abiertos.
- dogfood interno y sus casos registrados.
- matriz de instaladores y smoke de máquina limpia.

## Tag y publicación

```bash
git tag -a v0.1.0-alpha.1 -m "Rationale alpha 1"
git push origin v0.1.0-alpha.1
```

La workflow [`release.yml`](../../.github/workflows/release.yml) construye los
targets, crea archives y ZIP, calcula SHA-256, publica instaladores y genera
attestation. Nunca se suben `.rationale-local/`, caches ni secretos.

## Rollback

Si falla un smoke test o aparece un hallazgo de seguridad, no se promueve la
Release. Para un usuario ya instalado, reinstalar una versión anterior con
`RATIONALE_VERSION` devuelve el binario sin tocar `.rationale/`.
