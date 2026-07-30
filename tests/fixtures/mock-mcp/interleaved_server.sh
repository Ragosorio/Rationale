#!/usr/bin/env bash
# Emite una notificación antes de cada respuesta para verificar correlación
# JSON-RPC por id. Sin correlación, health confundiría la notificación con la
# respuesta de list_projects y degradaría.
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

message_id() {
  echo "$1" | grep -o '"id"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*$'
}

initialize="$(read_message)"
initialize_id="$(message_id "$initialize")"
write_message "{\"jsonrpc\":\"2.0\",\"id\":${initialize_id},\"result\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"serverInfo\":{\"name\":\"mock-interleaved\",\"version\":\"0.0.0\"}}}"

# notifications/initialized
read_message >/dev/null

list_projects="$(read_message)"
list_id="$(message_id "$list_projects")"
write_message '{"jsonrpc":"2.0","method":"notifications/progress","params":{"progress":0.5}}'
write_message "{\"jsonrpc\":\"2.0\",\"id\":${list_id},\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"{\\\"projects\\\":[{\\\"name\\\":\\\"tmp-repo\\\",\\\"root_path\\\":\\\"/tmp/repo\\\"}]}\"}]}}"

index_status="$(read_message)"
status_id="$(message_id "$index_status")"
write_message '{"jsonrpc":"2.0","method":"notifications/progress","params":{"progress":1}}'
write_message "{\"jsonrpc\":\"2.0\",\"id\":${status_id},\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"{\\\"status\\\":\\\"ready\\\"}\"}]}}"
