"""Builds the synthetic `parcel` project used by Rationale's evaluation.

Everything here is invented. The project reproduces, at small scale, the
failure modes reported by a team that uses Rationale (superseded Records shown
as current, whole-file context for a symbol, a retired module that Records
still describe, an orphaned exception, a stale structural index). Numbers
measured on it are synthetic and must never be presented as customer results.

History of the fixture repository:

1. commit "initial": every source file and the canon in `.rationale/`.
2. commit "retire comunicaciones": `src/comunicaciones/` is deleted; the
   Records that describe it are left untouched, as happened in the report.
3. working tree: `splitShipment` is added to `src/shipping/service.ts` and is
   not committed, so the structural index (the fixture graph) does not know it.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path

FIXTURE_DIR = Path(__file__).resolve().parent / "fixtures" / "parcel"
PROJECT_TEMPLATE = FIXTURE_DIR / "project"

# Structural index of the fixture provider: what the code looked like at the
# first commit. It deliberately still lists the retired module and a phantom
# file (`legacy.ts`) and does not know `splitShipment`.
SYMBOLS = [
    ("src/shipping/rates.ts", "toCents"),
    ("src/shipping/rates.ts", "quoteRate"),
    ("src/shipping/rates.ts", "applySurcharge"),
    ("src/shipping/service.ts", "createShipment"),
    ("src/shipping/service.ts", "cancelShipment"),
    ("src/shipping/service.ts", "retryLabel"),
    ("src/shipping/carriers/dhl.ts", "buyLabel"),
    ("src/billing/invoice.ts", "issueInvoice"),
    ("src/billing/refunds.ts", "refund"),
    ("src/secrets/rotation.ts", "rotateKeys"),
    ("src/shared/cache.ts", "cacheGet"),
    ("src/shared/cache.ts", "cacheSet"),
    ("src/notifications/email.ts", "sendEmail"),
    ("src/notifications/email.ts", "sendSms"),
    ("src/comunicaciones/inbox.ts", "inboxHandler"),
    ("src/comunicaciones/outbox.ts", "outboxHandler"),
    ("src/comunicaciones/templates.ts", "templatesHandler"),
]
PHANTOM_NODES = [
    {
        "file_path": "src/shipping/legacy.ts",
        "qualified_name": "src.shipping.legacy.oldQuote",
        "label": "function",
        "start_line": 3,
        "end_line": 9,
        "source": "export function oldQuote() { return 0; }",
    }
]
CALLS = [
    ("src.shipping.service.createShipment", "src.shipping.rates.quoteRate"),
    ("src.shipping.service.createShipment", "src.shipping.rates.applySurcharge"),
    ("src.shipping.rates.quoteRate", "src.shipping.rates.toCents"),
    ("src.shipping.rates.quoteRate", "src.shared.cache.cacheGet"),
    ("src.shipping.rates.quoteRate", "src.shipping.legacy.oldQuote"),
    ("src.shipping.service.retryLabel", "src.shipping.carriers.dhl.buyLabel"),
    ("src.billing.invoice.issueInvoice", "src.shipping.rates.toCents"),
    ("src.notifications.email.sendEmail", "src.comunicaciones.outbox.outboxHandler"),
]

NEW_SYMBOL_SOURCE = """
export function splitShipment(shipmentId: string, parts: number) {
  return Array.from({ length: parts }, (_, i) => `${shipmentId}-${i + 1}`);
}
"""


def qualified(path: str, symbol: str) -> str:
    return path.rsplit(".", 1)[0].replace("/", ".") + "." + symbol


def _symbol_lines(text: str, symbol: str) -> tuple[int, int]:
    lines = text.splitlines()
    pattern = re.compile(rf"\bfunction\s+{re.escape(symbol)}\b")
    for index, line in enumerate(lines):
        if pattern.search(line):
            end = index
            while end + 1 < len(lines) and not lines[end].startswith("}"):
                end += 1
            return index + 1, end + 1
    raise ValueError(f"{symbol} not found")


def build_graph(template: Path) -> dict:
    """Regenerates `fixtures/parcel/graph.json` (committed, also read by the
    Rust tests): `python3 -I -c 'import fixture, json; ...'` — see README."""
    nodes = []
    for path, symbol in SYMBOLS:
        text = (template / path).read_text()
        start, end = _symbol_lines(text, symbol)
        source = "\n".join(text.splitlines()[start - 1 : end])
        nodes.append(
            {
                "file_path": path,
                "qualified_name": qualified(path, symbol),
                "label": "function",
                "start_line": start,
                "end_line": end,
                "source": source,
            }
        )
    nodes.extend(PHANTOM_NODES)
    relationships = [{"source": s, "kind": "calls", "target": t} for s, t in CALLS]
    return {"nodes": nodes, "relationships": relationships}


def _git(cwd: Path, *args: str) -> None:
    subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env={
            "GIT_AUTHOR_NAME": "owner",
            "GIT_AUTHOR_EMAIL": "owner@example.invalid",
            "GIT_COMMITTER_NAME": "owner",
            "GIT_COMMITTER_EMAIL": "owner@example.invalid",
            "GIT_AUTHOR_DATE": "2026-09-01T10:00:00Z",
            "GIT_COMMITTER_DATE": "2026-09-01T10:00:00Z",
            "PATH": "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin",
            "HOME": str(cwd),
        },
    )


def build(workdir: Path) -> dict:
    """Creates the fixture repository and its graph under `workdir`.

    Returns paths: `project` (the Git repository) and `graph` (fixture
    provider JSON).
    """
    if workdir.exists():
        shutil.rmtree(workdir)
    project = workdir / "parcel"
    shutil.copytree(PROJECT_TEMPLATE, project)
    graph_path = workdir / "graph.json"
    shutil.copyfile(FIXTURE_DIR / "graph.json", graph_path)

    _git(project, "init", "-q", "-b", "main")
    _git(project, "config", "user.name", "owner")
    _git(project, "config", "user.email", "owner@example.invalid")
    _git(project, "add", "-A")
    _git(project, "commit", "-q", "-m", "initial")
    shutil.rmtree(project / "src" / "comunicaciones")
    _git(project, "add", "-A")
    _git(project, "commit", "-q", "-m", "retire comunicaciones")
    service = project / "src" / "shipping" / "service.ts"
    service.write_text(service.read_text() + NEW_SYMBOL_SOURCE)
    return {"project": project, "graph": graph_path}
