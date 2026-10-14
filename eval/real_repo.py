#!/usr/bin/env python3
"""Measure Rationale on a real repository with the real structural provider.

Usage:
    python3 -I eval/real_repo.py --binary dev=target/release/rationale \
        --repo /path/to/clone --targets eval/real_targets.txt --out eval/results

The repository is cloned into a temporary directory at its current HEAD and
Codebase Memory runs with an isolated HOME, so the first query pays the cold
index and neither your provider cache nor your checkout is touched. Results
describe that one repository on this machine; they are real measurements, not
customer results.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from mcp_client import McpSession  # noqa: E402
from run_eval import percentile  # noqa: E402


def provider_path() -> str:
    found = shutil.which("codebase-memory-mcp")
    if not found:
        raise SystemExit("codebase-memory-mcp is not on PATH")
    return str(Path(found).parent)


def run(binary: str, repo: Path, targets: list[str], warm: int) -> dict:
    with tempfile.TemporaryDirectory(prefix="rationale-real-") as tmp:
        clone = Path(tmp) / "repo"
        subprocess.run(["git", "clone", "-q", "--local", str(repo), str(clone)], check=True)
        home = Path(tmp) / "home"
        home.mkdir()
        env = {
            "HOME": str(home),
            "PATH": f"{provider_path()}:/usr/bin:/bin:/usr/sbin:/sbin",
            "LANG": "C.UTF-8",
        }
        started = time.perf_counter()
        session = McpSession(binary, clone, env)
        first = session.call("health", {})
        cold_ms = (time.perf_counter() - started) * 1000
        rows, warm_samples = [], []
        try:
            for target in targets:
                response = session.call("prepare_change", {"target": target})
                packet = (response.get("json") or {}).get("packet") or {}
                nodes = (packet.get("structure") or {}).get("nodes") or []
                missing = sorted({n["file_path"] for n in nodes if not (clone / n["file_path"]).exists()})
                rows.append({
                    "target": target,
                    "tokens": response["tokens"],
                    "legacy_tokens": response["legacy_tokens"],
                    "bytes": response["bytes"],
                    "latency_ms": round(response["latency_ms"], 1),
                    "governing": len(packet.get("critical_constraints") or [])
                    + len(packet.get("decisions") or []),
                    "nodes": len(nodes),
                    "nodes_from_missing_files": missing,
                    "retrieval": packet.get("retrieval"),
                    "budget_overflow": packet.get("budget_overflow"),
                })
                for _ in range(warm):
                    warm_samples.append(session.call("prepare_change", {"target": target})["latency_ms"])
        finally:
            session.close()
        tokens = [r["tokens"] for r in rows]
        return {
            "first_call_with_cold_index_ms": round(cold_ms, 1),
            "health_is_error": first["is_error"],
            "targets": rows,
            "tokens_p50": percentile(tokens, 0.5),
            "tokens_p95": percentile(tokens, 0.95),
            "tokens_max": max(tokens) if tokens else 0,
            "over_default_budget": sum(1 for t in tokens if t > 2400),
            "warm_prepare_ms": {"n": len(warm_samples), "p50": percentile(warm_samples, 0.5),
                                "p95": percentile(warm_samples, 0.95)},
        }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", action="append", required=True, help="label=path")
    parser.add_argument("--repo", required=True)
    parser.add_argument("--targets", required=True)
    parser.add_argument("--warm", type=int, default=5)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    targets = [line.strip() for line in Path(args.targets).read_text().splitlines()
               if line.strip() and not line.startswith("#")]
    report = {"repo_head": subprocess.run(["git", "-C", args.repo, "rev-parse", "HEAD"],
                                          capture_output=True, text=True).stdout.strip(),
              "runs": {}}
    for item in args.binary:
        label, binary = item.split("=", 1)
        report["runs"][label] = run(binary, Path(args.repo), targets, args.warm)
        summary = {k: v for k, v in report["runs"][label].items() if k != "targets"}
        print(label, json.dumps(summary), file=sys.stderr)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "real-repo.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
