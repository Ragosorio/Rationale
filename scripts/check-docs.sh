#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

required_files=(
  README.md
  docs/README.md
  docs/quickstart.md
  CONTRIBUTING.md
  SECURITY.md
  SUPPORT.md
  CODE_OF_CONDUCT.md
  CHANGELOG.md
)

for file in "${required_files[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "missing required documentation: $file" >&2
    exit 1
  fi
done

for script in scripts/*.sh; do
  bash -n "$script"
done

# This command used to appear in installation guidance but only works before
# packaging. Keep the public instructions on the installed CLI path.
if rg -n 'target/release/rationale health' README.md docs; then
  echo "stale source-tree health command found in public documentation" >&2
  exit 1
fi

if rg -n 'dogfood\.7|releases/latest/download/rationale-installer\.sh' site/src; then
  echo "stale or floating Rationale installer reference found in the preview site" >&2
  exit 1
fi

# --- Una sola fuente para la versión pública -------------------------------
#
# La versión del binario viene del tag (`RATIONALE_VERSION` en build.rs), así
# que `Cargo.toml` no puede ser la fuente para la documentación. Antes cada
# mención era una copia a mano: la alpha.7 quedó escrita en once sitios y nada
# avisaba cuando envejecían. `docs/RELEASE_VERSION` es ahora la única fuente y
# esto falla si alguna mención se desvía.
release_version="$(tr -d '[:space:]' < docs/RELEASE_VERSION)"
if [[ -z "$release_version" ]]; then
  echo "docs/RELEASE_VERSION está vacío" >&2
  exit 1
fi

# `docs/adr/**`, `docs/work-items/**` y el CHANGELOG son historia: citan la
# versión vigente cuando se escribieron y reescribirlos falsificaría el
# registro. La guarda cubre lo que un usuario lee para instalar HOY.
#
# `release/vX.Y.Z-...` se excluye porque es el nombre de una rama Git —un hecho
# histórico— no una versión que alguien deba instalar.
stale_versions="$(
  rg -n --no-heading -o '(release/)?v0\.[0-9]+\.[0-9]+-[a-z]+\.[0-9]+' \
    README.md docs site/src \
    -g '!docs/adr/**' \
    -g '!docs/work-items/**' \
    -g '!docs/RELEASE_VERSION' \
    | grep -v ':release/' \
    | grep -v ":${release_version}\$" || true
)"
if [[ -n "$stale_versions" ]]; then
  echo "referencia de versión desactualizada — se esperaba ${release_version}:" >&2
  echo "$stale_versions" >&2
  echo "actualiza esas líneas o corrige docs/RELEASE_VERSION" >&2
  exit 1
fi

# --- Las dos versiones del prompt maestro no pueden divergir ---------------
#
# `docs/prompt-master.md` (inglés) se compila DENTRO del binario con
# `include_str!`: es el protocolo que `install-agent` escribe de verdad. Las
# páginas del sitio no lo copian, lo inyectan con `?raw` en build time, así que
# ahí no puede haber drift.
#
# El que sí puede divergir es `docs/prompt-master.es.md`: existe solo para el
# sitio, nadie lo compila, y ya se quedó un paso atrás una vez. Un usuario que
# lea el protocolo en español recibiría instrucciones distintas de las que su
# propio agente tiene instaladas.
en_steps="$(rg -c '^[0-9]+\. ' docs/prompt-master.md || echo 0)"
es_steps="$(rg -c '^[0-9]+\. ' docs/prompt-master.es.md || echo 0)"
if [[ "$en_steps" != "$es_steps" ]]; then
  echo "las dos versiones del prompt maestro divergieron:" >&2
  echo "  docs/prompt-master.md (compilado al binario): ${en_steps} pasos" >&2
  echo "  docs/prompt-master.es.md (solo sitio):        ${es_steps} pasos" >&2
  exit 1
fi

git diff --check
echo "documentation checks passed"
