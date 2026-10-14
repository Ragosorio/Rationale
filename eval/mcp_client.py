"""Minimal MCP stdio client for `rationale serve` (newline-delimited JSON-RPC)."""

from __future__ import annotations

import json
import math
import os
import subprocess
import time
from pathlib import Path


PROSE_CHARS_PER_TOKEN = 2.7
CODE_CHARS_PER_TOKEN = 2.0


def estimate_tokens(text: str, code: str = "") -> int:
    """Conservative token estimate, identical to `src/tokens.rs`.

    ASCII characters count at 2.7 per token (JSON and English measured at
    3.0–3.3 with Claude's current tokenizer); the part of the text that is
    source code is re-counted at 2.0 (code measured at 2.2–2.45); every
    non-ASCII character counts as one token.
    """
    ascii_chars = sum(1 for ch in text if ord(ch) < 128)
    other = len(text) - ascii_chars
    code_ascii = sum(1 for ch in code if ord(ch) < 128)
    code_extra = math.ceil(code_ascii / CODE_CHARS_PER_TOKEN) - math.floor(
        code_ascii / PROSE_CHARS_PER_TOKEN
    )
    return math.ceil(ascii_chars / PROSE_CHARS_PER_TOKEN) + other + max(code_extra, 0)


def code_in(parsed) -> str:
    """Source code a Rationale response carries (`relevant_code.source`)."""
    if not isinstance(parsed, dict):
        return ""
    packet = parsed.get("packet") if isinstance(parsed.get("packet"), dict) else parsed
    code = packet.get("relevant_code") if isinstance(packet, dict) else None
    if isinstance(code, dict):
        return code.get("source") or ""
    return ""


def legacy_estimate(text: str) -> int:
    """The proxy Rationale used up to v1.1: UTF-8 bytes / 4."""
    return math.ceil(len(text.encode("utf-8")) / 4)


class McpSession:
    def __init__(self, binary: str, cwd: Path, env: dict[str, str]):
        self.process = subprocess.Popen(
            [binary, "serve", "--client", "claude-code"],
            cwd=cwd,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        self.next_id = 1
        self.request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "rationale-eval", "version": "1"},
        })
        self.notify("notifications/initialized")

    def notify(self, method: str) -> None:
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method}) + "\n")
        self.process.stdin.flush()

    def request(self, method: str, params: dict) -> dict:
        request_id = self.next_id
        self.next_id += 1
        self.process.stdin.write(
            json.dumps({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}) + "\n"
        )
        self.process.stdin.flush()
        while True:
            line = self.process.stdout.readline()
            if not line:
                raise RuntimeError(f"server closed while waiting for {method}")
            message = json.loads(line)
            if message.get("id") == request_id:
                return message

    def call(self, tool: str, arguments: dict) -> dict:
        started = time.perf_counter()
        message = self.request("tools/call", {"name": tool, "arguments": arguments})
        latency_ms = (time.perf_counter() - started) * 1000
        result = message.get("result", {})
        text = ""
        content = result.get("content") or []
        if content:
            text = content[0].get("text", "")
        try:
            parsed = json.loads(text)
        except json.JSONDecodeError:
            parsed = None
        return {
            "tool": tool,
            "is_error": bool(result.get("isError")) or "error" in message,
            "text": text,
            "json": parsed,
            "latency_ms": latency_ms,
            "bytes": len(text.encode("utf-8")),
            "tokens": estimate_tokens(text, code_in(parsed)),
            "legacy_tokens": legacy_estimate(text),
        }

    def close(self) -> None:
        try:
            self.process.stdin.close()
            self.process.wait(timeout=10)
        except Exception:
            self.process.kill()


def sandbox_env(home: Path, graph: Path | None) -> dict[str, str]:
    env = {
        "HOME": str(home),
        "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
        "LANG": "C.UTF-8",
        "GIT_CONFIG_GLOBAL": str(home / ".gitconfig"),
    }
    if graph is not None:
        env["RATIONALE_PROVIDER"] = f"fixture:{graph}"
    for key in ("TMPDIR",):
        if key in os.environ:
            env[key] = os.environ[key]
    return env
