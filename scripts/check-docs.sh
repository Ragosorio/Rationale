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

if rg -n 'dogfood\.7' site/src; then
  echo "stale dogfood release reference found in the preview site" >&2
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

# --- Un solo texto del prompt maestro ---------------------------------------
#
# `docs/prompt-master.md` se compila DENTRO del binario con `include_str!`: es el
# protocolo que `install-agent` escribe de verdad, en inglés, y le pide al agente
# responder en el idioma de la persona. Las páginas del sitio lo inyectan con
# `?raw`, también la española. Hubo una traducción solo para el sitio y ya se
# quedó atrás una vez: quien la leía recibía instrucciones distintas de las que
# su agente tenía instaladas. Esto impide que vuelva una copia traducida.
if rg -n 'prompt-master\.[a-z]{2}\.md' site/src scripts src; then
  echo "el sitio no debe usar una traducción del prompt maestro: inyecta docs/prompt-master.md" >&2
  exit 1
fi

# --- El canal `preview` no puede seleccionar por el flag prerelease ---------
#
# `preview` significa "la Release más reciente", no "la más reciente marcada
# prerelease". Seleccionar por el flag funcionó mientras TODAS las versiones
# eran prerelease y se rompió en el momento exacto en que dejó de ser cierto:
# `v0.1.0-beta.1` es una Release completa, así que el canal la saltaba y
# `rationale update` devolvía `alpha.7` — una versión ANTERIOR. Se observó en la
# prueba de actualización real, no en ningún test.
# Se busca el patrón de SELECCIÓN, no la palabra: los comentarios que explican
# por qué no se selecciona así son justamente lo que se quiere conservar.
if rg -n '"prerelease".*true|Where-Object \{ \$_\.prerelease \}' \
     scripts/rationale-installer.sh scripts/rationale-update.sh \
     scripts/rationale-installer.ps1 scripts/rationale-update.ps1; then
  echo "el canal preview no debe seleccionar por el flag prerelease: la Release más \
reciente puede ser una Release completa" >&2
  exit 1
fi

git diff --check
echo "documentation checks passed"
