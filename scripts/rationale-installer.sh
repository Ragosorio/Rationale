#!/usr/bin/env bash
set -euo pipefail

REPOSITORY="${RATIONALE_REPOSITORY:-Ragosorio/Rationale}"
VERSION="${RATIONALE_VERSION:-latest}"
PREFIX="${RATIONALE_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) TARGET="aarch64-apple-darwin" ;;
  Darwin:x86_64) TARGET="x86_64-apple-darwin" ;;
  Linux:x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  Linux:aarch64|Linux:arm64) TARGET="aarch64-unknown-linux-gnu" ;;
  *) echo "plataforma no soportada: $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac

command -v curl >/dev/null || { echo "se necesita curl" >&2; exit 1; }
if [[ "$VERSION" == "latest" ]]; then
  tag="$(curl -fsSL "https://api.github.com/repos/$REPOSITORY/releases/latest" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"
else
  tag="$VERSION"
fi
tag="${tag#v}"
base="https://github.com/$REPOSITORY/releases/download/v$tag"
archive="rationale-$tag-$TARGET.tar.xz"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/rationale-install.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

curl --fail --location --silent --show-error "$base/$archive" -o "$tmp/$archive"
curl --fail --location --silent --show-error "$base/$archive.sha256" -o "$tmp/$archive.sha256"
(cd "$tmp" && shasum -a 256 -c "$archive.sha256" >/dev/null 2>&1) || {
  echo "checksum inválido para $archive" >&2
  exit 1
}
tar -xJf "$tmp/$archive" -C "$tmp"
mkdir -p "$PREFIX"
install -m 0755 "$tmp/rationale-$tag-$TARGET/rationale" "$PREFIX/rationale"

if [[ "${RATIONALE_SKIP_AGENT_CONFIG:-0}" != "1" ]] && command -v codex >/dev/null 2>&1; then
  codex mcp list 2>/dev/null | grep -q '[[:space:]]rationale[[:space:]]' || \
    codex mcp add rationale -- "$PREFIX/rationale" serve >/dev/null
fi

echo "Rationale $tag instalado en $PREFIX/rationale"
echo "Ejecuta 'rationale init' dentro de un proyecto para crear .rationale/."
