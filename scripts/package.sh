#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${RATIONALE_VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -1)}"
TARGET="${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/target/packages}"

case "$VERSION" in
  v*) VERSION="${VERSION#v}" ;;
esac

if [[ -n "${CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER:-}" ]]; then
  export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER
fi

cargo build --release --locked --target "$TARGET"

binary="$ROOT_DIR/target/$TARGET/release/rationale"
if [[ ! -x "$binary" ]]; then
  echo "no se encontró el binario compilado: $binary" >&2
  exit 1
fi

stage="$(mktemp -d "${TMPDIR:-/tmp}/rationale-package.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/rationale-$VERSION-$TARGET"
cp "$binary" "$stage/rationale-$VERSION-$TARGET/rationale"
cp "$ROOT_DIR/LICENSE" "$ROOT_DIR/README.md" "$stage/rationale-$VERSION-$TARGET/"

mkdir -p "$OUT_DIR"
archive="$OUT_DIR/rationale-$VERSION-$TARGET.tar.xz"
tar -C "$stage" -cJf "$archive" "rationale-$VERSION-$TARGET"

if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$archive" > "$archive.sha256"
else
  sha256sum "$archive" > "$archive.sha256"
fi
echo "creado: $archive"
