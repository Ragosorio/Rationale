# Release runbook

Las Releases se construyen directamente desde `main` a partir de un tag
`vMAJOR.MINOR.PATCH`; la versión del binario sale del tag. `docs/RELEASE_VERSION`
es la única fuente de la versión pública en la documentación y
`scripts/check-docs.sh` falla si alguna mención se desvía. Los tags `alpha`,
`beta` y `dogfood.*` son historia de antes de 1.0.

## Antes del tag

- PR o commit directo a `main` con CI verde.
- `cargo fmt --check`.
- `cargo clippy --all-targets -- -D warnings`.
- `cargo test --release`.
- `cargo audit`.
- `npm --prefix ui ci && npm --prefix ui run typecheck && npm --prefix ui test && npm --prefix ui run build`.
- `npm --prefix site ci && npm --prefix site run check`.
- `./scripts/check-docs.sh`.
- security baseline sin P0/P1 abiertos.
- dogfood interno y sus casos registrados.
- matriz de instaladores y smoke de máquina limpia.

## Tag y publicación

Antes de etiquetar, confirmar que el commit que va a recibir el tag es
exactamente el que pasó CI. Un tag apunta a un commit, no a una rama: si el
árbol está sucio o `HEAD` se adelantó a `origin/main`, el artefacto publicado
no correspondería al código verificado.

```bash
git fetch origin
git status --short                 # debe estar vacío
git rev-parse HEAD                 # debe coincidir...
git rev-parse origin/main          # ...con este
```

Solo entonces:

```bash
git tag -a v1.0.0 -m "Rationale 1.0.0"
git push origin v1.0.0
```

`release.yml` marca `--prerelease` únicamente para `-alpha.`, `-rc.` y
`-dogfood.`. Un tag `beta` o final se publica como Release completa y por tanto
puede ser «latest», que es lo que resuelve el canal `stable` de los
instaladores (ADR-0010). Después de publicar, comprobarlo:

```bash
gh api repos/Ragosorio/Rationale/releases/latest --jq .tag_name
```

La workflow [`release.yml`](../../.github/workflows/release.yml) construye el
Control Room (`ui/dist`) y falla si no existe — un binario sin él serviría la
página de fallback de `rationale ui` —, construye los targets, crea archives y
ZIP, calcula SHA-256, publica instaladores y genera attestation. También publica
`rationale-update.sh` y `rationale-update.ps1`, que quedan junto al binario para
que `rationale update` pueda actualizar una instalación existente. Nunca se suben `.rationale-local/`, caches ni secretos.

## Rollback

Si falla un smoke test o aparece un hallazgo de seguridad, no se promueve la
Release. Para un usuario ya instalado, reinstalar una versión anterior con
`RATIONALE_VERSION` devuelve el binario sin tocar `.rationale/`.
