#!/usr/bin/env bash
set -euo pipefail

REPOSITORY="${RATIONALE_REPOSITORY:-Ragosorio/Rationale}"
VERSION="${RATIONALE_VERSION:-latest}"
CHANNEL="${RATIONALE_CHANNEL:-stable}"
PREFIX="${RATIONALE_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) TARGET="aarch64-apple-darwin" ;;
  Darwin:x86_64) TARGET="x86_64-apple-darwin" ;;
  Linux:x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  Linux:aarch64|Linux:arm64) TARGET="aarch64-unknown-linux-gnu" ;;
  *) echo "plataforma no soportada: $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac

command -v curl >/dev/null || { echo "se necesita curl" >&2; exit 1; }
if [[ -z "$VERSION" || "$VERSION" == "latest" ]]; then
  case "$CHANNEL" in
    stable)
      tag="$(curl -fsSL "https://api.github.com/repos/$REPOSITORY/releases/latest" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"
      ;;
    preview)
      tag="$(curl -fsSL "https://api.github.com/repos/$REPOSITORY/releases?per_page=100" | awk '
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
      ;;
    *)
      echo "RATIONALE_CHANNEL debe ser stable o preview" >&2
      exit 1
      ;;
  esac
else
  tag="$VERSION"
fi
tag="${tag#v}"
[[ -n "$tag" && "$tag" != "latest" ]] || {
  echo "no se pudo resolver una Release publicada para $REPOSITORY" >&2
  exit 1
}
base="https://github.com/$REPOSITORY/releases/download/v$tag"
archive="rationale-$tag-$TARGET.tar.xz"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/rationale-install.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

curl --fail --location --silent --show-error "$base/$archive" -o "$tmp/$archive"
curl --fail --location --silent --show-error "$base/$archive.sha256" -o "$tmp/$archive.sha256"
curl --fail --location --silent --show-error "$base/rationale-update.sh" -o "$tmp/rationale-update.sh"
(cd "$tmp" && shasum -a 256 -c "$archive.sha256" >/dev/null 2>&1) || {
  echo "checksum inválido para $archive" >&2
  exit 1
}
tar -xJf "$tmp/$archive" -C "$tmp"
mkdir -p "$PREFIX"
install -m 0755 "$tmp/rationale-$tag-$TARGET/rationale" "$PREFIX/rationale"
install -m 0755 "$tmp/rationale-update.sh" "$PREFIX/rationale-update"

if [[ "${RATIONALE_SKIP_AGENT_CONFIG:-0}" != "1" ]]; then
  "$PREFIX/rationale" install-agent --global-only || true
fi

echo "Rationale $tag instalado en $PREFIX/rationale"
echo "Versión verificada: $("$PREFIX/rationale" --version)"
echo "Ejecuta 'rationale init' dentro de un proyecto: crea .rationale/ y avisa"
echo "automáticamente a los agentes de código presentes (usa --skip-agent-config para omitirlo)."
