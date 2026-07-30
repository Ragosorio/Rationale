#!/usr/bin/env bash
set -euo pipefail

PREFIX="${RATIONALE_INSTALL_DIR:-$HOME/.local/bin}"
if [[ -f "$PREFIX/rationale" ]]; then
  "$PREFIX/rationale" uninstall-agent --global-only || {
    echo "aviso: no se pudieron revertir todos los registros globales de agentes" >&2
  }
  rm -f "$PREFIX/rationale"
  echo "eliminado: $PREFIX/rationale"
else
  echo "no existe: $PREFIX/rationale"
fi
rm -f "$PREFIX/rationale-update"

echo "No se modificó ningún directorio .rationale/."
echo "Si 'rationale init' avisó a algún agente en un proyecto, corre"
echo "'rationale uninstall-agent' dentro de ese proyecto para revertirlo."
