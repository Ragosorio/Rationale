#!/usr/bin/env python3
"""Every `rationale <command> --flag` written in the documentation must exist.

Usage: python3 -I scripts/check-doc-commands.py <path-to-rationale-binary>

Scans code spans and code blocks in the user-facing documentation (README,
docs/ except the historical ADRs, work items and research notes, the skill,
and the website's documentation pages), and checks each invocation against the
binary: the command must appear in `rationale --help` and each long flag in
`rationale <command> --help`. Exits 1 and lists every mismatch.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = [
    "README.md",
    "docs",
    "skills/rationale",
    "site/src/content/docs",
]
HISTORICAL = ("docs/adr/", "docs/work-items/", "docs/research/", "docs/experiments/")
GLOBAL_FLAGS = {"--help", "--version", "--project-root", "--no-mascot"}
INVOCATION = re.compile(r"(?<![\w$/.\-])rationale\s+([a-z][a-z-]*)((?:[ \t]+[^\s`|]+)*)")
CODE_SPAN = re.compile(r"`([^`\n]+)`")


def usage_commands(binary: str) -> set[str]:
    out = subprocess.run([binary, "--help"], capture_output=True, text=True, check=True).stdout
    first = out.splitlines()[0]
    inside = first.split("<", 1)[1].split(">", 1)[0]
    return set(inside.split("|"))


def command_flags(binary: str, command: str, cache: dict[str, set[str]]) -> set[str]:
    if command not in cache:
        out = subprocess.run([binary, command, "--help"], capture_output=True, text=True).stdout
        cache[command] = set(re.findall(r"--[a-z][a-z-]*", out))
    return cache[command]


def code_fragments(text: str):
    in_block = False
    for number, line in enumerate(text.splitlines(), start=1):
        if line.strip().startswith("```"):
            in_block = not in_block
            continue
        if in_block:
            yield number, line
        else:
            for span in CODE_SPAN.findall(line):
                yield number, span


def files():
    for source in SOURCES:
        path = ROOT / source
        candidates = [path] if path.is_file() else sorted(path.rglob("*.md*"))
        for file in candidates:
            relative = file.relative_to(ROOT).as_posix()
            if not relative.startswith(HISTORICAL) and "/evals/" not in relative:
                yield file, relative


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    binary = sys.argv[1]
    known = usage_commands(binary)
    cache: dict[str, set[str]] = {}
    problems = []
    checked = 0
    for file, relative in files():
        for number, fragment in code_fragments(file.read_text(encoding="utf-8")):
            for match in INVOCATION.finditer(fragment):
                command, rest = match.group(1), match.group(2)
                if command in ("skill", "skills", "is", "keeps", "and", "binary", "server", "mcp"):
                    continue
                checked += 1
                if command not in known:
                    problems.append(f"{relative}:{number}: unknown command `rationale {command}`")
                    continue
                flags = {flag.rstrip(".,;:)]}'\"") for flag in rest.split() if flag.startswith("--")}
                missing = sorted(f for f in flags if f and f not in GLOBAL_FLAGS
                                 and f not in command_flags(binary, command, cache))
                for flag in missing:
                    problems.append(f"{relative}:{number}: `rationale {command}` has no flag {flag}")
    for problem in problems:
        print(problem)
    print(f"{checked} documented invocation(s) checked against {binary}; {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
