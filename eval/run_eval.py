#!/usr/bin/env python3
"""Rationale retrieval evaluation over a synthetic project.

Usage:
    python3 -I eval/run_eval.py --binary v1.1.0=/path/to/rationale \
        --binary dev=target/release/rationale --out eval/results

Every binary runs against a fresh copy of the synthetic `parcel` project with
an isolated HOME and the fixture structural provider, so no installation,
cache or real repository is touched. Results are synthetic: they show whether
a failure mode reproduces and how much context is served, never how a real
team's repository behaves.
"""

from __future__ import annotations

import argparse
import json
import platform
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import fixture  # noqa: E402
import scenarios  # noqa: E402
from mcp_client import McpSession, sandbox_env, estimate_tokens  # noqa: E402

INACTIVE = set(scenarios.SUPERSEDED + scenarios.REVOKED)

AUTHORITY_LISTS = {
    "explain_target": [("governing_records", "id")],
    "prepare_change": [
        ("critical_constraints", "id"),
        ("decisions", "id"),
        ("relationships", "record_id"),
        ("inherited_constraints", "id"),
    ],
}


def packet_of(response: dict) -> dict:
    data = response.get("json") or {}
    if response["tool"] == "prepare_change":
        return data.get("packet") or {}
    return data


def authority_items(response: dict) -> list[dict]:
    """Records the response presents as current authority."""
    packet = packet_of(response)
    items = []
    for field, key in AUTHORITY_LISTS.get(response["tool"], []):
        for item in packet.get(field) or []:
            status = item.get("status", "active")
            if status in ("active", None):
                items.append({"id": item.get(key), "item": item, "field": field})
    return items


def walk(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for child in value.values():
            yield from strings(child)
    elif isinstance(value, list):
        for child in value:
            yield from strings(child)


def flags_of(item: dict) -> set[str]:
    flags = set(item.get("flags") or [])
    if item.get("stale_bindings"):
        flags.add("stale_bindings")
    return flags


def check(name: str, response: dict, project: Path, scenario: dict) -> tuple[bool, str]:
    packet = packet_of(response)
    data = response.get("json") or {}
    if name == "orphaned_exception_flagged":
        target = "exception.legacy-kms-skips-rotation"
        for item in walk(data):
            if item.get("id") == target or item.get("record_id") == target:
                if "orphaned_exception" in flags_of(item):
                    return True, "exception carries the orphaned_exception flag"
        for text in strings(data):
            if target in text and "orphan" in text.lower():
                return True, "a warning names the orphaned exception"
        return False, "the orphaned exception is not flagged"
    if name == "stale_records_flagged":
        stale = [i for i in authority_items(response) if ".comunicaciones-" in (i["id"] or "")]
        if not stale:
            return False, "no Record of the retired module was served or flagged"
        unflagged = [i["id"] for i in stale if "stale_bindings" not in flags_of(i["item"])]
        if unflagged:
            return False, f"served without a stale flag: {unflagged}"
        return True, f"{len(stale)} retired-module Records flagged"
    if name == "no_phantom_nodes":
        nodes = (packet.get("structure") or {}).get("nodes") or []
        phantom = [n["file_path"] for n in nodes if not (project / n["file_path"]).exists()]
        if phantom:
            return False, f"nodes from files that no longer exist: {sorted(set(phantom))}"
        return True, f"{len(nodes)} structural nodes, all present in the working tree"
    if name == "index_incomplete_reported":
        status = (packet.get("retrieval") or {}).get("status")
        return status == "index_incomplete", f"retrieval.status={status!r}"
    if name == "none_found_reported":
        status = (packet.get("retrieval") or {}).get("status")
        return status == "none_found", f"retrieval.status={status!r}"
    if name == "retired_target_explained":
        retrieval = packet.get("retrieval") or {}
        status = retrieval.get("status")
        bound = retrieval.get("records_bound_to_missing_path") or 0
        ok = status == "target_missing" and bound > 0
        return ok, f"retrieval.status={status!r}, records_bound_to_missing_path={bound}"
    if name == "within_budget":
        limit = scenario["args"]["max_tokens"]
        return response["tokens"] <= limit, f"{response['tokens']} estimated tokens for a budget of {limit}"
    if name == "budget_respected":
        # Within budget, or over it only with governing knowledge left: the
        # overflow is declared and nothing trimmable remains.
        limit = scenario["args"]["max_tokens"]
        if response["tokens"] <= limit:
            return True, f"{response['tokens']} estimated tokens for a budget of {limit}"
        trimmable = [k for k in ("structure", "relevant_code", "history", "known_risks", "affected_targets")
                     if packet.get(k)]
        declared = bool(packet.get("budget_overflow"))
        ok = declared and not trimmable
        return ok, (f"{response['tokens']} > {limit}; overflow declared={declared}; "
                    f"trimmable left={trimmable}")
    raise ValueError(name)


def score(scenario: dict, response: dict, project: Path) -> dict:
    served = [i["id"] for i in authority_items(response)]
    served_set = set(served)
    must = set(scenario["must"])
    forbidden = set(scenario["forbidden"])
    acceptable = set(scenario.get("acceptable", []))
    result = {
        "id": scenario["id"],
        "category": scenario["category"],
        "title": scenario["title"],
        "is_error": response["is_error"],
        "served_authority": sorted(served_set),
        "missing": sorted(must - served_set),
        "forbidden_served": sorted(forbidden & served_set),
        "inactive_served": sorted(served_set & INACTIVE),
        "irrelevant_served": sorted(served_set - must - forbidden - acceptable),
        "tokens": response["tokens"],
        "legacy_tokens": response["legacy_tokens"],
        "bytes": response["bytes"],
        "latency_ms": round(response["latency_ms"], 2),
        "checks": {},
    }
    for name in scenario.get("checks", []):
        passed, detail = check(name, response, project, scenario)
        result["checks"][name] = {"passed": passed, "detail": detail}
    result["passed"] = (
        not response["is_error"]
        and not result["missing"]
        and not result["forbidden_served"]
        and all(c["passed"] for c in result["checks"].values())
    )
    return result


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, round(fraction * (len(ordered) - 1))))
    return round(ordered[index], 2)


def run_cli(binary: str, project: Path, env: dict, *args: str, stdin: str = "") -> dict:
    started = time.perf_counter()
    process = subprocess.run(
        [binary, *args],
        cwd=project,
        env=env,
        input=stdin,
        capture_output=True,
        text=True,
        timeout=60,
    )
    return {
        "code": process.returncode,
        "stdout": process.stdout,
        "stderr": process.stderr,
        "latency_ms": round((time.perf_counter() - started) * 1000, 2),
    }


def fresh(work: Path, name: str) -> tuple[Path, dict]:
    paths = fixture.build(work / name)
    home = work / name / "home"
    home.mkdir(parents=True, exist_ok=True)
    (home / ".gitconfig").write_text("[user]\n\tname = owner\n\temail = owner@example.invalid\n")
    return paths["project"], sandbox_env(home, paths["graph"])


def retrieval_suite(binary: str, work: Path, warm: int) -> dict:
    project, env = fresh(work, "retrieval")
    session = McpSession(binary, project, env)
    results, samples = [], {"prepare_change": [], "explain_target": []}
    try:
        for scenario in scenarios.RETRIEVAL:
            response = session.call(scenario["tool"], scenario["args"])
            results.append(score(scenario, response, project))
            for _ in range(warm):
                again = session.call(scenario["tool"], scenario["args"])
                samples[scenario["tool"]].append(again["latency_ms"])
    finally:
        session.close()
    return {"scenarios": results, "warm_latency_ms": {
        tool: {"n": len(v), "p50": percentile(v, 0.5), "p95": percentile(v, 0.95)}
        for tool, v in samples.items()
    }}


def cold_suite(binary: str, work: Path, runs: int) -> dict:
    project, env = fresh(work, "cold")
    samples = []
    for _ in range(runs):
        started = time.perf_counter()
        session = McpSession(binary, project, env)
        session.call("prepare_change", {"target": "src/shipping/service.ts::cancelShipment"})
        session.close()
        samples.append((time.perf_counter() - started) * 1000)
    return {"n": runs, "p50": percentile(samples, 0.5), "p95": percentile(samples, 0.95)}


def capture_suite(binary: str, work: Path) -> dict:
    """A Record bound to a symbol the index does not know must govern that symbol only."""
    project, env = fresh(work, "capture")
    session = McpSession(binary, project, env)
    try:
        finalize = session.call("finalize_change", {
            "summary": "Add splitShipment",
            "candidates": [scenarios.CAPTURE_CANDIDATE],
        })
        committed = [c.get("id") for c in (finalize.get("json") or {}).get("committed", [])]
        new_id = committed[0] if committed else None
        other = session.call("explain_target", {"target": "src/shipping/service.ts::createShipment"})
        own = session.call("explain_target", {"target": "src/shipping/service.ts::splitShipment"})
    finally:
        session.close()
    leaked = new_id in [i["id"] for i in authority_items(other)]
    found = new_id in [i["id"] for i in authority_items(own)]
    bindings = []
    if new_id:
        record = (project / ".rationale" / "records" / f"{new_id}.yaml").read_text()
        bindings = [line.strip() for line in record.splitlines() if "type:" in line or "symbol" in line]
    return {
        "committed": committed,
        "governs_its_symbol": found,
        "leaks_to_other_symbols": leaked,
        "passed": bool(new_id) and found and not leaked,
        "binding_lines": bindings,
        "warnings": (finalize.get("json") or {}).get("warnings", []),
    }


def operations_suite(binary: str, work: Path) -> dict:
    project, env = fresh(work, "operations")
    session = McpSession(binary, project, env)
    targets = [
        "src/shipping/service.ts::cancelShipment",
        "src/shipping/rates.ts::quoteRate",
        "src/billing/invoice.ts::issueInvoice",
        "src/secrets/rotation.ts::rotateKeys",
        "src/notifications/email.ts::sendSms",
    ]
    result = {}
    try:
        ids = []
        for target in targets:
            response = session.call("prepare_change", {"target": target, "intent": "evaluation"})
            ids.append((response.get("json") or {}).get("operation_id"))
        for operation_id in ids[:3]:
            session.call("finalize_change", {"operation_id": operation_id, "summary": "no durable knowledge"})
        health = session.call("health", {})
        health_json = health.get("json") or {}
        open_info = health_json.get("open_operations")
        result["health_reports_open"] = isinstance(open_info, dict) and open_info.get("count") == 2
        abandon = session.call("finalize_change", {
            "operation_id": ids[3],
            "abandon": True,
            "summary": "Exploration only; no change was made.",
        })
        stored = project / ".rationale-local" / "operations" / f"{ids[3]}.json"
        stored_json = json.loads(stored.read_text()) if stored.exists() else {}
        result["abandon_supported"] = bool(stored_json.get("abandoned")) and not stored_json.get("finalized")
        result["abandon_response_error"] = abandon["is_error"]
    finally:
        session.close()
    listing = run_cli(binary, project, env, "operations", "--json")
    try:
        listed = json.loads(listing["stdout"])
        open_ids = [op["operation_id"] for op in listed.get("operations", []) if op.get("state") == "open"]
    except (json.JSONDecodeError, AttributeError, KeyError):
        open_ids = None
    result["cli_lists_open"] = open_ids is not None and open_ids == [ids[4]]
    result["passed"] = result["health_reports_open"] and result["abandon_supported"] and result["cli_lists_open"]
    return result


def doctor_suite(binary: str, work: Path) -> dict:
    project, env = fresh(work, "doctor")
    out = run_cli(binary, project, env, "doctor", "--json")
    try:
        report = json.loads(out["stdout"])
    except json.JSONDecodeError:
        report = {"findings": []}
    text = json.dumps(report)
    expected = {
        "orphaned_exception": "exception.legacy-kms-skips-rotation" in text and "orphan" in text.lower(),
        "inconsistent_supersession": "decision.label-retry-fixed-3x" in text,
        "retired_scope_grouped": any(
            (f.get("kind") in ("retired_scope", "retired-scope") and f.get("path") == "src/comunicaciones")
            for f in report.get("findings", []) if isinstance(f, dict)
        ),
        "missing_binding_paths": "src/comunicaciones" in text,
    }
    check_run = run_cli(binary, project, env, "doctor", "--check")
    return {
        "findings": len(report.get("findings", [])),
        "detected": expected,
        "detected_count": sum(expected.values()),
        "expected_count": len(expected),
        "check_exit_code": check_run["code"],
        "latency_ms": out["latency_ms"],
    }


def repair_suite(binary: str, work: Path, old_binary: str | None) -> dict:
    """Retire the deleted module in one reviewable, reversible step."""
    project, env = fresh(work, "repair")
    records_dir = project / ".rationale" / "records"
    before = {p.name: p.read_text() for p in records_dir.glob("*.yaml")}
    plan = run_cli(binary, project, env, "retire", "src/comunicaciones", "--reason",
                   "Module removed in favour of notifications", "--json")
    try:
        plan_json = json.loads(plan["stdout"])
    except json.JSONDecodeError:
        return {"supported": False, "passed": False, "detail": (plan["stderr"] or plan["stdout"])[:300]}
    dry_unchanged = before == {p.name: p.read_text() for p in records_dir.glob("*.yaml")}
    applied = run_cli(binary, project, env, "retire", "src/comunicaciones", "--reason",
                      "Module removed in favour of notifications", "--apply", "--yes", "--json")
    applied_json = json.loads(applied["stdout"]) if applied["stdout"].strip().startswith("{") else {}
    batch = applied_json.get("batch_id")
    doctor_after = run_cli(binary, project, env, "doctor", "--json")
    session = McpSession(binary, project, env)
    try:
        email = session.call("prepare_change", {"target": "src/notifications/email.ts::sendEmail"})
    finally:
        session.close()
    retired_still_served = [
        i["id"] for i in authority_items(email)
        if i["id"] in {a["record_id"] for a in plan_json.get("actions", []) if a.get("action") == "retire"}
    ]
    old_view = None
    if old_binary:
        old_session = McpSession(old_binary, project, env)
        try:
            old_explain = old_session.call("explain_target", {"target": "src/notifications/email.ts::sendEmail"})
        finally:
            old_session.close()
        old_view = sorted({i["id"] for i in authority_items(old_explain)})
    undo = run_cli(binary, project, env, "retire", "--undo", str(batch), "--json") if batch else {"code": 1}
    after_undo = {p.name: p.read_text() for p in records_dir.glob("*.yaml")}
    return {
        "supported": True,
        "plan_actions": {
            kind: sum(1 for a in plan_json.get("actions", []) if a.get("action") == kind)
            for kind in ("retire", "unbind", "needs_human")
        },
        "dry_run_left_canon_unchanged": dry_unchanged,
        "apply_exit_code": applied["code"],
        "retired_records_served_after_apply": retired_still_served,
        "doctor_findings_after_apply": len(json.loads(doctor_after["stdout"]).get("findings", []))
        if doctor_after["stdout"].strip().startswith("{") else None,
        "old_binary_authority_on_email_after_apply": old_view,
        "undo_exit_code": undo["code"],
        "undo_restores_bytes": after_undo == before,
        "passed": dry_unchanged and applied["code"] == 0 and not retired_still_served
        and undo["code"] == 0 and after_undo == before,
    }


def compat_suite(binary: str, old_binary: str, work: Path) -> dict:
    """A canon written by the old binary is read by the new one and vice versa."""
    project, env = fresh(work, "compat")
    old = McpSession(old_binary, project, env)
    try:
        written_old = old.call("finalize_change", {"summary": "old", "candidates": [{
            "kind": "decision", "severity": "low",
            "statement": "Refund receipts are emailed only after the payment provider confirms the refund.",
            "rationale": "Receipts sent before confirmation had to be retracted when refunds failed.",
            "durability": "durable", "bindings": ["src/billing/refunds.ts::refund"]}]})
    finally:
        old.close()
    new = McpSession(binary, project, env)
    try:
        read_new = new.call("explain_target", {"target": "src/billing/refunds.ts::refund"})
        written_new = new.call("finalize_change", {"summary": "new", "candidates": [{
            "kind": "decision", "severity": "low",
            "statement": "Cache keys for rate quotes include the destination postal zone.",
            "rationale": "Two zones in one city have different surcharges; a shared key served the wrong price.",
            "durability": "durable", "bindings": ["src/shipping/rates.ts::quoteRate"]}]})
    finally:
        new.close()
    old = McpSession(old_binary, project, env)
    try:
        read_old = old.call("explain_target", {"target": "src/shipping/rates.ts::quoteRate"})
    finally:
        old.close()
    old_id = ((written_old.get("json") or {}).get("committed") or [{}])[0].get("id")
    new_id = ((written_new.get("json") or {}).get("committed") or [{}])[0].get("id")
    new_reads_old = old_id in [i["id"] for i in authority_items(read_new)]
    old_reads_new = new_id in [i["id"] for i in authority_items(read_old)]
    old_doctor = run_cli(old_binary, project, env, "doctor", "--json")
    return {
        "old_record": old_id,
        "new_record": new_id,
        "new_binary_reads_old_record": new_reads_old,
        "old_binary_reads_new_record": old_reads_new,
        "old_doctor_exit_code": old_doctor["code"],
        "passed": bool(old_id and new_id and new_reads_old and old_reads_new),
    }


def summarize(results: list[dict]) -> dict:
    tokens = [r["tokens"] for r in results]
    return {
        "scenarios": len(results),
        "passed": sum(r["passed"] for r in results),
        "inactive_served_as_authority": sum(len(r["inactive_served"]) for r in results),
        "out_of_scope_served": sum(len(set(r["forbidden_served"]) - INACTIVE) for r in results),
        "missing_required": sum(len(r["missing"]) for r in results),
        "irrelevant_served": sum(len(r["irrelevant_served"]) for r in results),
        "tokens_p50": percentile(tokens, 0.5),
        "tokens_p95": percentile(tokens, 0.95),
        "tokens_max": max(tokens) if tokens else 0,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--binary", action="append", required=True, help="label=path")
    parser.add_argument("--out", default=str(Path(__file__).resolve().parent / "results"))
    parser.add_argument("--warm", type=int, default=20, help="warm repetitions per scenario")
    parser.add_argument("--cold", type=int, default=5, help="cold process starts")
    parser.add_argument("--check", action="store_true",
                        help="exit 1 unless the last binary passes every scenario and suite (CI gate)")
    args = parser.parse_args()

    binaries = [
        (label, str(Path(path).resolve()))
        for label, path in (item.split("=", 1) for item in args.binary)
    ]
    old_binary = binaries[0][1] if len(binaries) > 1 else None
    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    report = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "machine": {"platform": platform.platform(), "python": platform.python_version()},
        "synthetic": True,
        "estimator": "ASCII chars/2.7 + code chars/2.0 + 1 per non-ASCII char (see eval/README.md)",
        "runs": {},
    }
    with tempfile.TemporaryDirectory(prefix="rationale-eval-") as tmp:
        for label, binary in binaries:
            version = subprocess.run([binary, "--version"], capture_output=True, text=True).stdout.strip()
            work = Path(tmp) / label
            retrieval = retrieval_suite(binary, work, args.warm)
            run = {
                "binary_version": version,
                "summary": summarize(retrieval["scenarios"]),
                "retrieval": retrieval,
                "cold_latency_ms": cold_suite(binary, work, args.cold),
                "capture": capture_suite(binary, work),
                "operations": operations_suite(binary, work),
                "doctor": doctor_suite(binary, work),
                "repair": repair_suite(binary, work, old_binary if binary != old_binary else None),
            }
            if old_binary and binary != old_binary:
                run["compatibility"] = compat_suite(binary, old_binary, work)
            report["runs"][label] = run
            print(f"{label}: {run['summary']}", file=sys.stderr)
    (out_dir / "report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    (out_dir / "report.md").write_text(render_markdown(report))
    print(out_dir / "report.md")
    if args.check:
        last = report["runs"][binaries[-1][0]]
        failures = [r["id"] for r in last["retrieval"]["scenarios"] if not r["passed"]]
        failures += [suite for suite in ("capture", "operations", "repair")
                     if not last[suite].get("passed")]
        if last["doctor"]["detected_count"] < last["doctor"]["expected_count"]:
            failures.append("doctor")
        if "compatibility" in last and not last["compatibility"]["passed"]:
            failures.append("compatibility")
        if failures:
            print("evaluation gate failed: " + ", ".join(failures), file=sys.stderr)
            return 1
        print("evaluation gate passed", file=sys.stderr)
    return 0


def render_markdown(report: dict) -> str:
    labels = list(report["runs"])
    lines = [
        "# Rationale evaluation — synthetic `parcel` project",
        "",
        f"Generated {report['generated_at']} on `{report['machine']['platform']}`.",
        "**Synthetic data.** The project, its Records and its structural index are invented to",
        "reproduce reported failure modes; these numbers are not results from any real repository.",
        f"Token estimator: {report['estimator']}.",
        "",
        "## Summary",
        "",
        "| Metric | " + " | ".join(labels) + " |",
        "|---|" + "---|" * len(labels),
    ]
    def row(name, getter):
        lines.append(f"| {name} | " + " | ".join(str(getter(report["runs"][l])) for l in labels) + " |")
    row("Binary", lambda r: r["binary_version"])
    row("Retrieval scenarios passed", lambda r: f"{r['summary']['passed']}/{r['summary']['scenarios']}")
    row("Superseded/revoked Records served as authority", lambda r: r["summary"]["inactive_served_as_authority"])
    row("Records of another symbol served as authority", lambda r: r["summary"]["out_of_scope_served"])
    row("Required Records missing", lambda r: r["summary"]["missing_required"])
    row("Irrelevant Records served", lambda r: r["summary"]["irrelevant_served"])
    row("Response tokens p50 / p95 / max", lambda r: f"{r['summary']['tokens_p50']} / {r['summary']['tokens_p95']} / {r['summary']['tokens_max']}")
    row("Warm prepare_change ms p50 / p95", lambda r: f"{r['retrieval']['warm_latency_ms']['prepare_change']['p50']} / {r['retrieval']['warm_latency_ms']['prepare_change']['p95']}")
    row("Warm explain_target ms p50 / p95", lambda r: f"{r['retrieval']['warm_latency_ms']['explain_target']['p50']} / {r['retrieval']['warm_latency_ms']['explain_target']['p95']}")
    row("Cold start + prepare ms p50 / p95", lambda r: f"{r['cold_latency_ms']['p50']} / {r['cold_latency_ms']['p95']}")
    row("New-symbol capture governs only its symbol", lambda r: r["capture"]["passed"])
    row("Open operations visible and abandonable", lambda r: r["operations"]["passed"])
    row("Doctor detects (of 4)", lambda r: r["doctor"]["detected_count"])
    row("Retired module repaired reversibly", lambda r: r["repair"]["passed"])
    row("Compatible with the other binary's canon", lambda r: r.get("compatibility", {}).get("passed", "—"))
    lines += ["", "## Scenarios", ""]
    lines.append("| Scenario | " + " | ".join(labels) + " |")
    lines.append("|---|" + "---|" * len(labels))
    for index, scenario in enumerate(scenarios.RETRIEVAL):
        cells = []
        for label in labels:
            result = report["runs"][label]["retrieval"]["scenarios"][index]
            notes = []
            if result["forbidden_served"]:
                notes.append("forbidden: " + ", ".join(result["forbidden_served"]))
            if result["missing"]:
                notes.append("missing: " + ", ".join(result["missing"]))
            if result["irrelevant_served"]:
                notes.append(f"{len(result['irrelevant_served'])} irrelevant")
            for name, outcome in result["checks"].items():
                if not outcome["passed"]:
                    notes.append(f"{name}: {outcome['detail']}")
            mark = "pass" if result["passed"] else "FAIL"
            cells.append(f"{mark}, {result['tokens']} tok" + (" — " + "; ".join(notes) if notes else ""))
        lines.append(f"| {scenario['id']} {scenario['title']} | " + " | ".join(cells) + " |")
    lines.append("")
    return "\n".join(lines)


if __name__ == "__main__":
    sys.exit(main())
