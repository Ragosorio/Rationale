#!/usr/bin/env bash
# Install lifecycle under a sandbox HOME, with no source checkout on PATH:
# an existing v1.1.0 install and project → upgrade through the shell installer
# from a local release mirror → install-agent → uninstall-agent → uninstall.
#
# Usage: eval/install_lifecycle.sh <workdir> <repo> <old-binary> <packages-dir> <version>
# <packages-dir> holds rationale-<version>-<target>.tar.xz(.sha256) built by
# scripts/package.sh. Nothing outside <workdir> is written.
set -uo pipefail
SP="$1"; REPO="$2"; OLD="$3"; PACKAGES="$4"; VERSION="$5"; PORT="${PORT:-8765}"
CLAUDE_BIN="$(command -v claude || true)"
S="$SP/install-test"; rm -rf "$S"; mkdir -p "$S/home/.local/bin" "$S/mirror/v$VERSION" "$S/proj" "$S/agents"
cp "$PACKAGES"/* "$S/mirror/v$VERSION/"
cp "$REPO/scripts/rationale-update.sh" "$S/mirror/v$VERSION/rationale-update.sh"
[ -n "$CLAUDE_BIN" ] && ln -s "$CLAUDE_BIN" "$S/agents/claude"
export HOME="$S/home"
export PATH="$S/home/.local/bin:$S/agents:/usr/bin:/bin:/usr/sbin:/sbin"
printf '[user]\n\tname = owner\n\temail = owner@example.invalid\n' > "$HOME/.gitconfig"
step() { printf '\n== %s\n' "$*"; }

step "existing install: the previous binary"
cp "$OLD" "$HOME/.local/bin/rationale"
rationale --version
cd "$S/proj" && git init -q . && printf 'export function quote() { return 1; }\n' > rates.ts && git add -A && git commit -qm init
step "project initialized by the previous binary"
rationale init --no-mascot 2>&1 | tail -3
python3 -I - "$S/proj" <<'PY'
import sys, json, subprocess
p = subprocess.Popen(["rationale", "serve", "--client", "claude-code"], cwd=sys.argv[1], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def req(i, m, params):
    p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": i, "method": m, "params": params}) + "\n"); p.stdin.flush()
    return json.loads(p.stdout.readline())
req(1, "initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}})
r = req(2, "tools/call", {"name": "finalize_change", "arguments": {"summary": "x", "candidates": [{"kind": "decision", "severity": "high", "statement": "Quotes are computed in integer cents end to end.", "rationale": "Float rounding double-charged customers once.", "durability": "durable", "bindings": ["rates.ts::quote"]}]}})
print("previous binary wrote:", json.loads(r["result"]["content"][0]["text"])["committed"][0]["id"])
p.stdin.close(); p.wait()
PY
git add -A && git commit -qm "canon from the previous binary"
before=$(cd .rationale && find . -type f -exec shasum {} \; | sort | shasum)

step "upgrade from a release mirror (installer)"
python3 -m http.server --bind 127.0.0.1 --directory "$S/mirror" $PORT >/dev/null 2>&1 &
server=$!; sleep 1
RATIONALE_DOWNLOAD_BASE="http://127.0.0.1:$PORT" RATIONALE_VERSION="$VERSION" RATIONALE_NO_MASCOT=1 bash "$REPO/scripts/rationale-installer.sh" 2>&1 | grep -v '^$' | tail -4
kill $server; wait $server 2>/dev/null
rationale --version
python3 -c "import json;d=json.load(open('$HOME/.claude.json'));print('claude user MCP after install:', d['mcpServers']['rationale'])"
step "install-agent with the new binary"
rationale install-agent --no-mascot 2>&1 | grep -E "hooks|bloque|Agentes" | head -6
after=$(cd .rationale && find . -type f -exec shasum {} \; | sort | shasum)
[ "$before" = "$after" ] && echo "canon unchanged by upgrade: yes" || echo "canon unchanged by upgrade: NO"
python3 -c "import json;d=json.load(open('.claude/settings.local.json'));print('hook commands:', sorted({h['command'] for g in d['hooks'].values() for e in g for h in e['hooks']}))"
step "new binary on the old canon"
rationale doctor --no-baseline 2>&1 | tail -2
rationale prepare rates.ts::quote --intent "use floats" 2>/dev/null | python3 -c "import sys,json;p=json.load(sys.stdin);print('retrieval:',p['retrieval']['status'],'| decisions:',[d['id'] for d in p['decisions']],'| tokens:',p['budget'])"
echo '{"session_id":"s1","cwd":"'"$S/proj"'","source":"startup"}' | rationale hook session-start | python3 -c "import sys,json;print('session-start hook:', json.load(sys.stdin)['hookSpecificOutput']['additionalContext'][:160])"
step "uninstall-agent"
rationale uninstall-agent --no-mascot 2>&1 | grep -iE "hook|CLAUDE|retir|elimin" | head -6
[ -f .claude/settings.local.json ] && echo "settings.local.json still present" || echo "settings.local.json removed"
grep -c "rationale:begin" CLAUDE.md 2>/dev/null || echo "CLAUDE.md block removed"
step "uninstall script"
bash "$REPO/scripts/rationale-uninstall.sh" 2>&1 | tail -3
[ -e "$HOME/.local/bin/rationale" ] && echo "binary still present" || echo "binary removed"
after2=$(cd .rationale && find . -type f -exec shasum {} \; | sort | shasum)
[ "$before" = "$after2" ] && echo "canon untouched by uninstall: yes" || echo "canon untouched by uninstall: NO"
python3 -c "import json,os;p='$HOME/.claude.json';print('claude user MCP after uninstall:', json.load(open(p)).get('mcpServers',{}).get('rationale') if os.path.exists(p) else 'file removed')"
