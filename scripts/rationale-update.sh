#!/usr/bin/env bash
set -euo pipefail

REPOSITORY="${RATIONALE_REPOSITORY:-Ragosorio/Rationale}"
DEFAULT_CHANNEL="preview"
CHANNEL="${RATIONALE_CHANNEL:-$DEFAULT_CHANNEL}"
VERSION="${RATIONALE_VERSION:-}"
tmp="$(mktemp "${TMPDIR:-/tmp}/rationale-update.XXXXXX")"
trap 'rm -f "$tmp"' EXIT

if [[ -z "$VERSION" || "$VERSION" == "latest" ]]; then
  if [[ "$CHANNEL" == "preview" ]]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/$REPOSITORY/releases?per_page=100" | awk '
      # `preview` significa "la Release más reciente, sea prerelease o no".
      # La API las devuelve de más nueva a más vieja, así que basta el primer
      # tag_name. Antes se buscaba el primer prerelease, y eso rompió en
      # cuanto una Release completa pasó a ser la más nueva: `beta.1` no es
      # prerelease, así que el canal saltaba a `alpha.7` y "actualizar"
      # devolvía una versión anterior.
      /"tag_name"[[:space:]]*:/ && !found {
        value = $0
        sub(/^.*"tag_name"[[:space:]]*:[[:space:]]*"/, "", value)
        sub(/".*$/, "", value)
        print value
        found = 1
      }
    ')"
    [[ -n "$VERSION" ]] || { echo "no se encontró una Release preview para $REPOSITORY" >&2; exit 1; }
    installer_url="https://github.com/$REPOSITORY/releases/download/v${VERSION#v}/rationale-installer.sh"
  elif [[ "$CHANNEL" == "stable" ]]; then
    installer_url="https://github.com/$REPOSITORY/releases/latest/download/rationale-installer.sh"
    VERSION="latest"
  else
    echo "RATIONALE_CHANNEL debe ser stable o preview" >&2
    exit 1
  fi
else
  tag="${VERSION#v}"
  installer_url="https://github.com/$REPOSITORY/releases/download/v$tag/rationale-installer.sh"
fi

curl --proto '=https' --tlsv1.2 -fsSL \
  "$installer_url" \
  -o "$tmp"

export RATIONALE_REPOSITORY="$REPOSITORY"
export RATIONALE_VERSION="$VERSION"
export RATIONALE_CHANNEL="$CHANNEL"
sh "$tmp"
