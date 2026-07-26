#!/usr/bin/env bash
set -euo pipefail

PREFIX="${RATIONALE_INSTALL_DIR:-$HOME/.local/bin}"
if [[ -f "$PREFIX/rationale" ]]; then
  rm -f "$PREFIX/rationale"
  echo "eliminado: $PREFIX/rationale"
else
  echo "no existe: $PREFIX/rationale"
fi

if [[ "${RATIONALE_REMOVE_AGENT_CONFIG:-0}" == "1" ]] && command -v codex >/dev/null 2>&1; then
  codex mcp remove rationale >/dev/null 2>&1 || true
fi
echo "No se modificó ningún directorio .rationale/."
