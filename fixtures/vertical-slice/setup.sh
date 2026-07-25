#!/usr/bin/env bash
#
# Genera el repo git del fixture de la vertical slice (Fase D2) de forma
# determinista: mismo contenido, mismas fechas y autoría fijas -> siempre el
# mismo SHA en cualquier máquina. El repo generado (`repo/`) no se versiona
# en Git (está en .gitignore) porque es 100% regenerable desde `source/` +
# este script; lo que sí se versiona es la fuente y este script.
#
# Uso:
#   bash fixtures/vertical-slice/setup.sh
#
# Salida: fixtures/vertical-slice/repo/ (repo git de un solo commit)

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_DIR="${HERE}/source"
REPO_DIR="${HERE}/repo"

rm -rf "${REPO_DIR}"
mkdir -p "${REPO_DIR}"
cp -R "${SOURCE_DIR}/." "${REPO_DIR}/"

cd "${REPO_DIR}"
git init -q -b main

export GIT_AUTHOR_NAME="rationale-fixture"
export GIT_AUTHOR_EMAIL="fixture@rationale.local"
export GIT_AUTHOR_DATE="2026-07-23T18:20:00Z"
export GIT_COMMITTER_NAME="rationale-fixture"
export GIT_COMMITTER_EMAIL="fixture@rationale.local"
export GIT_COMMITTER_DATE="2026-07-23T18:20:00Z"

git add -A
git commit -q -m "feat(auth): entity-scoped staff authorization"

SHA="$(git rev-parse HEAD)"
echo "Fixture repo generado en: ${REPO_DIR}"
echo "Revisión determinista: ${SHA}"

EXPECTED_SHA="cb878c9d598e54a2a9aa3993395513f7ccfff325"
if [ "${SHA}" != "${EXPECTED_SHA}" ]; then
  echo
  echo "ADVERTENCIA: el SHA generado (${SHA}) no coincide con el SHA" >&2
  echo "esperado registrado en fixtures/vertical-slice/.rationale/ (${EXPECTED_SHA})." >&2
  echo "Si el contenido de source/ cambió intencionalmente, actualizar" >&2
  echo "bound_revision en fixtures/vertical-slice/.rationale/records/ y" >&2
  echo "el EXPECTED_SHA de este script." >&2
fi
