#!/usr/bin/env bash
set -euo pipefail

REPOSITORY="${RATIONALE_REPOSITORY:-Ragosorio/Rationale}"
DEFAULT_CHANNEL="stable"
CHANNEL="${RATIONALE_CHANNEL:-$DEFAULT_CHANNEL}"
VERSION="${RATIONALE_VERSION:-}"
tmp="$(mktemp "${TMPDIR:-/tmp}/rationale-update.XXXXXX")"
trap 'rm -f "$tmp"' EXIT

if [[ -z "$VERSION" || "$VERSION" == "latest" ]]; then
  if [[ "$CHANNEL" == "preview" ]]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/$REPOSITORY/releases?per_page=100" | awk '
      /"tag_name"[[:space:]]*:/ {
        value = $0
        sub(/^.*"tag_name"[[:space:]]*:[[:space:]]*"/, "", value)
        sub(/".*$/, "", value)
        latest_tag = value
      }
      /"prerelease"[[:space:]]*:[[:space:]]*true/ && latest_tag != "" {
        print latest_tag
        exit
      }
    ' | head -1)"
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
