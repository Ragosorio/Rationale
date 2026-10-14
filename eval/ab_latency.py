#!/usr/bin/env python3
"""Interleaved A/B latency of `prepare_change` on a real repository.

Usage:
    python3 -I eval/ab_latency.py --a v1.1.0=/path/a --b dev=target/release/rationale \
        --repo . --targets eval/real_targets.txt --rounds 8

Both servers run at the same time on one temporary clone, with the real
Codebase Memory and an isolated HOME, and calls alternate A, B, A, B so that
machine load affects both equally. Sequential runs on a busy laptop differed by
more than the change being measured; interleaving is what makes the comparison
fair.
"""

from __future__ import annotations

import argparse
import json
import shutil
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from mcp_client import McpSession  # noqa: E402


def quantiles(values: list[float]) -> dict:
    ordered = sorted(values)
    return {
        "n": len(values),
        "p50": round(statistics.median(values), 1),
        "p95": round(ordered[int(0.95 * (len(ordered) - 1))], 1),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--a", required=True, help="label=path")
    parser.add_argument("--b", required=True, help="label=path")
    parser.add_argument("--repo", required=True)
    parser.add_argument("--targets", required=True)
    parser.add_argument("--rounds", type=int, default=8)
    args = parser.parse_args()
    provider = shutil.which("codebase-memory-mcp")
    if not provider:
        raise SystemExit("codebase-memory-mcp is not on PATH")
    targets = [t.strip() for t in Path(args.targets).read_text().splitlines()
               if t.strip() and not t.startswith("#")]
    (label_a, path_a), (label_b, path_b) = (x.split("=", 1) for x in (args.a, args.b))
    with tempfile.TemporaryDirectory(prefix="rationale-ab-") as tmp:
        clone = Path(tmp) / "repo"
        subprocess.run(["git", "clone", "-q", "--local", args.repo, str(clone)], check=True)
        home = Path(tmp) / "home"
        home.mkdir()
        env = {"HOME": str(home), "PATH": f"{Path(provider).parent}:/usr/bin:/bin", "LANG": "C.UTF-8"}
        a = McpSession(str(Path(path_a).resolve()), clone, env)
        b = McpSession(str(Path(path_b).resolve()), clone, env)
        a.call("health", {})
        b.call("health", {})
        samples = {label_a: [], label_b: []}
        try:
            for _ in range(args.rounds):
                for target in targets:
                    samples[label_a].append(a.call("prepare_change", {"target": target})["latency_ms"])
                    samples[label_b].append(b.call("prepare_change", {"target": target})["latency_ms"])
        finally:
            a.close()
            b.close()
    print(json.dumps({label: quantiles(values) for label, values in samples.items()}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
