#!/usr/bin/env bash
# Mock MCP server para probar el timeout real del cliente (D5, contract test
# "provider timeout"). Responde correctamente a `initialize`, luego nunca
# vuelve a responder a nada — simula un proveedor colgado, no un error.
set -euo pipefail

read_message() {
  local content_length=0
  local line
  while IFS= read -r line; do
    line="${line%$'\r'}"
    [ -z "$line" ] && break
    if [[ "$line" =~ ^Content-Length:\ *([0-9]+)$ ]]; then
      content_length="${BASH_REMATCH[1]}"
    fi
  done
  if [ "$content_length" -gt 0 ]; then
    dd bs=1 count="$content_length" 2>/dev/null
  fi
}

write_message() {
  local body="$1"
  printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body"
}

# Primera petición: initialize. Responder correctamente y rápido.
first_msg="$(read_message)"
id="$(echo "$first_msg" | grep -o '"id"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*$' || echo 1)"
write_message "{\"jsonrpc\":\"2.0\",\"id\":${id},\"result\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"serverInfo\":{\"name\":\"mock-slow\",\"version\":\"0.0.0\"}}}"

# A partir de aquí: nunca más responde a nada (simula colgado).
sleep 3600
