# Release runbook

La Release pública vigente es `v0.1.0-alpha.4`, construida desde la rama
`release/v0.1.0-alpha.1`. Los tags `dogfood.*` quedan como iteraciones
históricas de hardening y no deben usarse para validar el flujo actual. Los
cambios posteriores de gobernanza y landing permanecen en `Unreleased` hasta
que pasen los gates de esta guía.

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
git tag -a v0.1.0-alpha.4 -m "Rationale alpha 4"
git push origin v0.1.0-alpha.4
```

La workflow [`release.yml`](../../.github/workflows/release.yml) construye los
targets, crea archives y ZIP, calcula SHA-256, publica instaladores y genera
attestation. También publica `rationale-update.sh` y `rationale-update.ps1`,
que quedan junto al binario para que `rationale update` pueda actualizar una
instalación existente. Nunca se suben `.rationale-local/`, caches ni secretos.

## Rollback

Si falla un smoke test o aparece un hallazgo de seguridad, no se promueve la
Release. Para un usuario ya instalado, reinstalar una versión anterior con
`RATIONALE_VERSION` devuelve el binario sin tocar `.rationale/`.
