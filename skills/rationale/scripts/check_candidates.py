#!/usr/bin/env python3
"""Check Rationale candidates before sending them to finalize_change.

The checks mirror Rationale's capture gate (src/canon.rs in the Rationale
repository): the same thresholds, keyword lists, id pattern, and binding rules,
in the same order. The gate stays the authority. A clean result does not
guarantee a write, because the gate also confirms symbols through the
structural provider and re-reads the live canon under a lock.

Usage:
    python3 check_candidates.py FILE [--repo PATH] [--json]
    python3 check_candidates.py - [--repo PATH] [--json] < candidates.json

FILE holds a JSON array of candidates, or the finalize_change arguments object
with a "candidates" array. --repo is the repository root and defaults to the
current directory.

Exit codes:
    0  no candidate would be discarded (warnings may remain)
    1  the input or the repository could not be read
    2  at least one candidate would be discarded
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

try:  # PyYAML is optional; a minimal reader covers Rationale's own YAML.
    import yaml  # type: ignore
except ImportError:  # pragma: no cover - depends on the environment
    yaml = None

MIN_TEXT_CHARS = 16
MIN_TEXT_WORDS = 3
VALID_KINDS = ("constraint", "decision", "risk", "exception")
VALID_SEVERITIES = ("critical", "high", "medium", "low")
# Structural relationship kinds a candidate may explain. The gate ignores
# inferred kinds (semantically_related, similar_to) and "other".
RELATIONSHIP_KINDS = (
    "calls", "uses", "writes", "imports", "defines", "implements",
    "tests", "configures", "depends_on", "http_calls", "contains", "decorates",
)
# .rationale/schemas/record.schema.json
ID_PATTERN = re.compile(r"^[a-z][a-z0-9-]*\.[a-z][a-z0-9.-]*$")
INACTIVE_STATUSES = ("superseded", "revoked")

# Change-log openings. A statement that starts with one is mechanical noise
# unless the statement or the rationale carries a causal marker. A Rust test in
# Rationale keeps both lists identical to src/canon.rs.
MECHANICAL_PREFIXES = (
    "added", "add", "adds", "changed", "change", "changes", "updated",
    "update", "updates", "removed", "remove", "deleted", "delete", "renamed",
    "rename", "moved", "move", "reformatted", "reformat", "formatted",
    "format", "bumped", "bump", "refactored", "refactor", "cleaned",
    "cleanup", "clean", "tidy", "lint", "fixed typo", "fix typo", "typo",
    "wip", "chore", "misc", "se agregó", "se agrego", "agregado", "agregar",
    "añadido", "añadir", "cambiado", "cambiar", "actualizado", "actualizar",
    "eliminado", "eliminar", "renombrado", "renombrar", "movido", "mover",
    "formateado", "formatear", "refactorizado", "limpieza",
)
CAUSAL_MARKERS = (
    "because", "so that", "in order to", "to avoid", "to prevent",
    "otherwise", "must", "never", "always", "should", "only", "required",
    "requires", "invariant", "guarantee", "guarantees", "ensure", "ensures",
    "since", "due to", "without", "porque", "para que", "para evitar",
    "evitar", "de lo contrario", "debe", "deben", "nunca", "siempre", "solo",
    "sólo", "requiere", "garantiza", "invariante", "ya que", "debido a",
    "sin esto", "sin",
)
# Phrases that narrate the change instead of stating what stays true. The gate
# does not check these; they only produce warnings.
CHANGE_NARRATION = (
    "this change", "this pr", "this commit", "this task", "this session",
    "we changed", "we added", "i changed", "i added", "now uses",
    "este cambio", "este pr", "este commit", "esta tarea", "ahora usa",
)


# --- Text rules (src/canon.rs) ---------------------------------------------

def normalize(text: str) -> str:
    """Lowercase, alphanumerics only, single spaces: how the gate compares."""
    kept = "".join(ch if ch.isalnum() else " " for ch in text.lower())
    return " ".join(kept.split())


def is_meaningful(text: str) -> bool:
    stripped = text.strip()
    return len(stripped) >= MIN_TEXT_CHARS and len(stripped.split()) >= MIN_TEXT_WORDS


def contains_marker(normalized: str, marker: str) -> bool:
    return f" {normalize(marker)} " in f" {normalized} "


def is_mechanical_noise(statement: str, rationale: str) -> bool:
    s, r = normalize(statement), normalize(rationale)
    prefixes = (normalize(prefix) for prefix in MECHANICAL_PREFIXES)
    if not any(s == prefix or s.startswith(prefix + " ") for prefix in prefixes):
        return False
    return not any(contains_marker(s, m) or contains_marker(r, m) for m in CAUSAL_MARKERS)


# --- Canon -----------------------------------------------------------------

TOP_LEVEL_KEY = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*):(?:\s(.*))?$")


def unquote(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] == "'":
        return value[1:-1].replace("''", "'")
    if len(value) >= 2 and value[0] == value[-1] == '"':
        try:
            return json.loads(value)
        except ValueError:
            return value[1:-1]
    return value


def read_record_minimal(text: str) -> dict:
    """Reads id, statement, authority, and lifecycle status without PyYAML."""
    fields = {"id": "", "statement": "", "authority": "normal", "status": "active"}
    lines = text.splitlines()
    index, section, status_seen = 0, None, False
    while index < len(lines):
        line = lines[index]
        match = TOP_LEVEL_KEY.match(line)
        if match:
            key, value = match.group(1), match.group(2) or ""
            section = key
            if key in ("id", "statement", "authority"):
                continuation = []
                index += 1
                while index < len(lines) and (
                    lines[index].startswith((" ", "\t")) or not lines[index].strip()
                ):
                    if lines[index].strip():
                        continuation.append(lines[index].strip())
                    index += 1
                head = value.strip()
                parts = continuation if head in ("|", "|-", "|+", ">", ">-", ">+") else [head, *continuation]
                fields[key] = unquote(" ".join(part for part in parts if part))
                continue
        elif section == "lifecycle" and not status_seen:
            status = re.match(r"^\s+status:\s*(\S+)", line)
            if status:
                fields["status"] = unquote(status.group(1))
                status_seen = True
        index += 1
    return fields


def read_record(text: str) -> dict:
    if yaml is not None:
        try:
            data = yaml.safe_load(text)
        except Exception:  # noqa: BLE001 - fall back to the minimal reader
            data = None
        if isinstance(data, dict):
            lifecycle = data.get("lifecycle") if isinstance(data.get("lifecycle"), dict) else {}
            return {
                "id": str(data.get("id") or ""),
                "statement": str(data.get("statement") or ""),
                "authority": str(data.get("authority") or "normal"),
                "status": str(lifecycle.get("status") or "active"),
            }
    return read_record_minimal(text)


def load_canon(repo: Path) -> list[dict]:
    records_dir = repo / ".rationale" / "records"
    if not records_dir.is_dir():
        return []
    records = []
    for path in sorted(records_dir.glob("*.yaml")):
        try:
            record = read_record(path.read_text(encoding="utf-8"))
        except OSError:
            continue
        if record["id"]:
            records.append(record)
    return records


# --- Candidate checks -----------------------------------------------------

class Finding:
    def __init__(self, code: str, detail: str):
        self.code, self.detail = code, detail

    def as_dict(self) -> dict:
        return {"code": self.code, "detail": self.detail}


def shape_error(candidate: dict) -> Finding | None:
    """Type checks, then the gate's validate_shape order."""
    text_fields = ("kind", "statement", "rationale", "durability", "severity", "id")
    for field in text_fields:
        if field in candidate and candidate[field] is not None and not isinstance(candidate[field], str):
            return Finding("malformed_candidate", f"'{field}' must be a string")
    for field in ("bindings", "relationships", "supersedes", "risks", "evidence"):
        if field in candidate and not isinstance(candidate[field], list):
            return Finding("malformed_candidate", f"'{field}' must be an array")
    for binding in candidate.get("bindings", []):
        if isinstance(binding, str):
            continue
        if not (isinstance(binding, dict) and isinstance(binding.get("path"), str)
                and isinstance(binding.get("symbol", ""), (str, type(None)))):
            return Finding("malformed_candidate", "each binding must be \"path::symbol\" or {\"path\", \"symbol\"}")
    for relationship in candidate.get("relationships", []):
        if not (isinstance(relationship, dict)
                and all(isinstance(relationship.get(k), str) for k in ("source", "kind", "target"))):
            return Finding("malformed_candidate", "each relationship needs string source, kind, and target")
    for field in ("supersedes", "risks"):
        if not all(isinstance(item, str) for item in candidate.get(field, [])):
            return Finding("malformed_candidate", f"'{field}' must contain strings")
    for evidence in candidate.get("evidence", []):
        if not (isinstance(evidence, dict) and isinstance(evidence.get("path"), str)):
            return Finding("malformed_candidate", "each evidence item needs a string 'path'")
    subject = candidate.get("subject")
    if subject is not None and not (isinstance(subject, dict)
                                    and isinstance(subject.get("id"), str)
                                    and isinstance(subject.get("title"), str)):
        return Finding("malformed_candidate", "subject needs string 'id' and 'title'")

    kind = candidate.get("kind") or ""
    statement = (candidate.get("statement") or "").strip()
    rationale = (candidate.get("rationale") or "").strip()
    durability = candidate.get("durability")

    if kind not in VALID_KINDS:
        return Finding("invalid_kind", f"kind '{kind}' is not one of: {', '.join(VALID_KINDS)}")
    if durability == "transient":
        return Finding("transient", "declared transient, so no durable memory is written; put it in summary")
    if durability is None:
        return Finding("durability_not_declared",
                       "durability is missing; declare \"durable\" only if it stays true after this change")
    if durability != "durable":
        return Finding("durability_not_declared", f"durability '{durability}' is invalid; use \"durable\" or \"transient\"")
    if not is_meaningful(statement):
        return Finding("statement_not_meaningful",
                       f"the statement needs at least {MIN_TEXT_CHARS} characters and {MIN_TEXT_WORDS} words")
    if not is_meaningful(rationale):
        return Finding("missing_rationale",
                       f"the rationale needs at least {MIN_TEXT_CHARS} characters and {MIN_TEXT_WORDS} words of cause")
    normalized_statement, normalized_rationale = normalize(statement), normalize(rationale)
    if normalized_rationale == normalized_statement or normalized_rationale in normalized_statement:
        return Finding("rationale_restates_statement", "the rationale repeats the statement instead of giving the cause")
    if is_mechanical_noise(statement, rationale):
        return Finding("mechanical_noise", "reads as a change log with no cause; Git already records the change")
    severity = candidate.get("severity")
    if severity is not None and severity not in VALID_SEVERITIES:
        return Finding("invalid_severity", f"severity '{severity}' is not one of: {', '.join(VALID_SEVERITIES)}")
    record_id = candidate.get("id")
    if record_id is not None:
        if not ID_PATTERN.match(record_id) or ".." in record_id:
            return Finding("invalid_id", f"id '{record_id}' must look like '<kind>.<lowercase-slug>'")
        if record_id.split(".", 1)[0] != kind:
            return Finding("id_kind_mismatch", f"id prefix of '{record_id}' does not match kind '{kind}'")
    return None


def split_spec(spec: str) -> tuple[str, str | None]:
    path, separator, symbol = spec.partition("::")
    return path.strip(), (symbol.strip() or None) if separator else None


def resolve_file(repo: Path, raw_path: str) -> tuple[str | None, str | None]:
    if not raw_path:
        return None, "has an empty path"
    candidate = Path(raw_path)
    full = candidate if candidate.is_absolute() else repo / candidate
    if not full.is_file():
        return None, "does not point to an existing file"
    try:
        relative = full.resolve().relative_to(repo.resolve()).as_posix()
    except (ValueError, OSError):
        return None, "is outside the repository"
    if relative.startswith((".rationale/", ".rationale-local/")):
        return None, "points at Rationale data, not code"
    return relative, None


def symbol_appears(repo: Path, relative: str, symbol: str) -> bool:
    tail = next((part for part in reversed(re.split(r"[:.]", symbol)) if part), symbol)
    try:
        text = (repo / relative).read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return True  # unreadable: leave it to the provider
    return re.search(rf"(?<![A-Za-z0-9_]){re.escape(tail)}(?![A-Za-z0-9_])", text) is not None


def check_anchors(repo: Path, candidate: dict, warnings: list[Finding]) -> Finding | None:
    anchored = False
    for binding in candidate.get("bindings", []):
        if isinstance(binding, str):
            spec = binding
            path, symbol = split_spec(binding)
        else:
            path, symbol = binding["path"].strip(), (binding.get("symbol") or "").strip() or None
            spec = f"{path}::{symbol}" if symbol else path
        relative, problem = resolve_file(repo, path)
        if problem:
            warnings.append(Finding("binding_ignored", f"binding '{spec}' {problem}"))
            continue
        anchored = True
        if symbol and not symbol_appears(repo, relative, symbol):
            warnings.append(Finding("symbol_not_found",
                                    f"'{symbol}' does not appear in {relative}; the file binding still counts, "
                                    "but the provider will likely not confirm the symbol"))
    for relationship in candidate.get("relationships", []):
        kind = relationship["kind"].strip().lower()
        label = f"{relationship['source']} {kind} {relationship['target']}"
        if kind not in RELATIONSHIP_KINDS:
            warnings.append(Finding("relationship_ignored",
                                    f"'{label}': kind must be one of {', '.join(RELATIONSHIP_KINDS)}"))
            continue
        ends_ok = True
        for end in (relationship["source"], relationship["target"]):
            path, symbol = split_spec(end)
            _, problem = resolve_file(repo, path)
            if problem:
                warnings.append(Finding("relationship_ignored", f"'{label}': endpoint '{end}' {problem}"))
                ends_ok = False
            elif symbol is None:
                warnings.append(Finding("relationship_endpoint", f"'{label}': use path::symbol for '{end}'"))
        anchored = anchored or ends_ok
    if not anchored:
        return Finding("no_meaningful_binding",
                       "no binding or relationship resolves to a real file in the repository")
    return None


def narration_warnings(candidate: dict) -> list[Finding]:
    statement = candidate.get("statement") or ""
    warnings = []
    normalized = normalize(statement)
    for phrase in CHANGE_NARRATION:
        if contains_marker(normalized, phrase):
            warnings.append(Finding("narrates_the_change",
                                    f"the statement mentions '{phrase}'; state what stays true, not what this change did"))
            break
    sentences = [s for s in re.split(r"(?<=[.!?])\s+", statement.strip()) if s]
    if ";" in statement or len(sentences) > 1:
        warnings.append(Finding("possibly_several_decisions",
                                "the statement has several clauses; split it if the parts could change independently"))
    if candidate.get("severity") is None:
        warnings.append(Finding("severity_missing", "no severity; the gate writes 'medium'"))
    return warnings


def check(candidates: list, repo: Path, canon: list[dict]) -> list[dict]:
    by_id = {record["id"]: record for record in canon}
    active = [record for record in canon if record["status"] not in INACTIVE_STATUSES]
    results = []
    for index, candidate in enumerate(candidates):
        warnings: list[Finding] = []
        errors: list[Finding] = []
        outcome = "write"
        if not isinstance(candidate, dict):
            errors.append(Finding("malformed_candidate", "a candidate must be a JSON object"))
            results.append(result(index, {}, errors, warnings, "discard"))
            continue

        problem = shape_error(candidate)
        if problem is None:
            problem = check_anchors(repo, candidate, warnings)
        if problem is None:
            normalized = normalize(candidate["statement"])
            duplicate = next((r for r in active if normalize(r["statement"]) == normalized), None)
            if duplicate is not None:
                problem = Finding("duplicate", f"an active Record already states this: {duplicate['id']}")
        if problem is None:
            for target in candidate.get("supersedes", []):
                existing = by_id.get(target)
                if existing is None:
                    warnings.append(Finding("supersedes_unknown", f"'{target}' is not in the canon; both would coexist"))
                elif existing["status"] in INACTIVE_STATUSES:
                    warnings.append(Finding("supersedes_inactive", f"'{target}' is no longer active; it is ignored"))
                elif existing["authority"] == "pinned":
                    warnings.append(Finding("supersedes_pinned",
                                            f"'{target}' is pinned: this candidate becomes a conflict a person decides"))
                    outcome = "conflict"
        requested = candidate.get("id")
        if problem is None and outcome != "conflict" and requested:
            if requested in by_id or (repo / ".rationale" / "records" / f"{requested}.yaml").exists():
                problem = Finding("id_already_exists",
                                  f"'{requested}' already exists; use a new id and supersedes if it replaces it")

        if problem is not None:
            errors.append(problem)
            outcome = "discard"
        if problem is None or problem.code != "malformed_candidate":
            warnings.extend(narration_warnings(candidate))
        if outcome == "write":
            # The gate sees earlier candidates of the same batch as canon.
            active.append({"id": requested or f"(candidate {index})",
                           "statement": candidate["statement"], "authority": "normal", "status": "active"})
            if requested:
                by_id[requested] = active[-1]
        results.append(result(index, candidate, errors, warnings, outcome))
    return results


def result(index: int, candidate: dict, errors, warnings, outcome: str) -> dict:
    statement = candidate.get("statement") if isinstance(candidate.get("statement"), str) else ""
    return {
        "index": index,
        "kind": candidate.get("kind") if isinstance(candidate.get("kind"), str) else "",
        "statement": statement,
        "outcome": outcome,
        "errors": [finding.as_dict() for finding in errors],
        "warnings": [finding.as_dict() for finding in warnings],
    }


# --- Entry point -----------------------------------------------------------

def read_candidates(source: str) -> list:
    raw = sys.stdin.read() if source == "-" else Path(source).read_text(encoding="utf-8")
    data = json.loads(raw)
    if isinstance(data, dict) and isinstance(data.get("candidates"), list):
        return data["candidates"]
    if isinstance(data, list):
        return data
    raise ValueError("expected a JSON array of candidates or an object with a 'candidates' array")


def summarize(results: list[dict]) -> dict:
    return {
        "candidates": len(results),
        "write": sum(r["outcome"] == "write" for r in results),
        "discard": sum(r["outcome"] == "discard" for r in results),
        "conflict": sum(r["outcome"] == "conflict" for r in results),
        "warnings": sum(len(r["warnings"]) for r in results),
    }


def print_text(results: list[dict], summary: dict, canon_size: int) -> None:
    if not results:
        print("No candidates: finalize_change will write no memory.")
        return
    labels = {"write": "would be written", "discard": "would be discarded",
              "conflict": "would become a conflict for a person"}
    for item in results:
        preview = item["statement"] if len(item["statement"]) <= 72 else item["statement"][:69] + "..."
        print(f"candidate {item['index']} · {item['kind'] or '?'} · \"{preview}\"")
        print(f"  {labels[item['outcome']]}")
        for finding in item["errors"]:
            print(f"  error {finding['code']}: {finding['detail']}")
        for finding in item["warnings"]:
            print(f"  warning {finding['code']}: {finding['detail']}")
    print(f"\nSummary: {summary['candidates']} candidate(s) · {summary['write']} written · "
          f"{summary['discard']} discarded · {summary['conflict']} conflict(s) · "
          f"{summary['warnings']} warning(s) · checked against {canon_size} Record(s)")
    print("The capture gate stays the authority; this check only mirrors its rules.")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check Rationale candidates before finalize_change.")
    parser.add_argument("source", help="JSON file with candidates, or - to read stdin")
    parser.add_argument("--repo", default=".", help="repository root (default: current directory)")
    parser.add_argument("--json", action="store_true", help="print a JSON report")
    args = parser.parse_args(argv)

    repo = Path(args.repo)
    if not repo.is_dir():
        print(f"error: repository '{args.repo}' is not a directory", file=sys.stderr)
        return 1
    try:
        candidates = read_candidates(args.source)
    except (OSError, ValueError) as error:
        print(f"error: could not read candidates: {error}", file=sys.stderr)
        return 1

    canon = load_canon(repo)
    results = check(candidates, repo, canon)
    summary = summarize(results)
    if args.json:
        print(json.dumps({"repo": str(repo.resolve()), "canon_records": len(canon),
                          "candidates": results, "summary": summary}, ensure_ascii=False, indent=2))
    else:
        print_text(results, summary, len(canon))
    return 2 if summary["discard"] else 0


if __name__ == "__main__":
    sys.exit(main())
