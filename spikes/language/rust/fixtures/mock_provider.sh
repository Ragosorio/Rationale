#!/usr/bin/env bash
# Mock structural provider para el spike de lenguaje (operación 3).
# Uso normal: responde rápido con una revisión JSON.
# Uso "slow": duerme 5s para poder probar deadline + cancelación real.
set -euo pipefail
if [ "${1:-}" = "slow" ]; then
  sleep 5
fi
echo '{"provider":"mock","indexed_revision":"def456abc0000000000000000000000000000000","status":"ready"}'
