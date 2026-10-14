"""Retrieval scenarios over the synthetic `parcel` project.

Each scenario names the Records that must be served as current authority, the
ones that must never be served as authority (superseded, revoked, or bound to
a different symbol), and the ones that are acceptable but not required.
Anything else served is counted as irrelevant context.
"""

SHIPPING = "src/shipping/"

SUPERSEDED = [
    "decision.rate-cache-ttl-5m",
    "decision.label-retry-fixed-3x",
    "constraint.rotate-secrets-90d",
]
REVOKED = ["constraint.no-weekend-pickups"]

RETRIEVAL = [
    {
        "id": "Q1",
        "category": "superseded",
        "title": "explain a symbol whose decision was superseded (both sides marked)",
        "tool": "explain_target",
        "args": {"target": SHIPPING + "rates.ts::quoteRate"},
        "must": ["decision.rate-cache-ttl-60s"],
        "forbidden": ["decision.rate-cache-ttl-5m"],
        "acceptable": [],
    },
    {
        "id": "Q2",
        "category": "superseded",
        "title": "explain a symbol whose old decision is replaced only through `supersedes`",
        "tool": "explain_target",
        "args": {"target": SHIPPING + "service.ts::retryLabel"},
        "must": ["decision.label-retry-exponential-backoff"],
        "forbidden": ["decision.label-retry-fixed-3x", "constraint.no-weekend-pickups"],
        "acceptable": [],
    },
    {
        "id": "Q3",
        "category": "symbol",
        "title": "prepare a symbol in a file with Records bound to other symbols",
        "tool": "prepare_change",
        "args": {
            "target": SHIPPING + "service.ts::cancelShipment",
            "intent": "Allow cancelling a shipment locally when the carrier API is down",
        },
        "must": ["decision.cancel-requires-carrier-ack"],
        "forbidden": ["decision.label-retry-fixed-3x", "constraint.no-weekend-pickups"],
        "acceptable": [],
    },
    {
        "id": "Q4",
        "category": "folder",
        "title": "prepare a folder",
        "tool": "prepare_change",
        "args": {"target": "src/shipping"},
        "must": [
            "constraint.money-in-integer-cents",
            "decision.rate-cache-ttl-60s",
            "decision.label-retry-exponential-backoff",
            "decision.cancel-requires-carrier-ack",
            "constraint.shipment-idempotency-key",
            "decision.dhl-sandbox-env-flag",
            "decision.remote-zone-flat-surcharge",
        ],
        "forbidden": SUPERSEDED[:2] + REVOKED,
        "acceptable": [],
    },
    {
        "id": "Q5",
        "category": "cross-module",
        "title": "prepare a billing symbol that calls the shipping invariant",
        "tool": "prepare_change",
        "args": {
            "target": "src/billing/invoice.ts::issueInvoice",
            "intent": "Sum invoice lines as floats and round once at the end",
        },
        "must": ["decision.invoice-rounding-bankers", "constraint.money-in-integer-cents"],
        "forbidden": [],
        "acceptable": [],
    },
    {
        "id": "Q6",
        "category": "exception",
        "title": "prepare the secret rotation symbol with an orphaned exception",
        "tool": "prepare_change",
        "args": {
            "target": "src/secrets/rotation.ts::rotateKeys",
            "intent": "Rotate legacy KMS keys in place",
        },
        "must": ["constraint.rotate-secrets-30d"],
        "forbidden": ["constraint.rotate-secrets-90d"],
        "acceptable": ["exception.legacy-kms-skips-rotation"],
        "checks": ["orphaned_exception_flagged"],
    },
    {
        "id": "Q7",
        "category": "retired-module",
        "title": "prepare a symbol that Records of a retired module still bind",
        "tool": "prepare_change",
        "args": {
            "target": "src/notifications/email.ts::sendEmail",
            "intent": "Send email directly through the provider SDK",
        },
        "must": ["decision.notifications-replaces-comunicaciones"],
        "forbidden": [],
        "acceptable": [
            "decision.comunicaciones-email-via-outbox",
            "constraint.comunicaciones-template-ids-stable",
        ],
        "checks": ["stale_records_flagged", "no_phantom_nodes"],
    },
    {
        "id": "Q8",
        "category": "folder",
        "title": "explain a folder",
        "tool": "explain_target",
        "args": {"target": "src/notifications"},
        "must": ["decision.notifications-replaces-comunicaciones", "decision.sms-max-160-chars"],
        "forbidden": [],
        "acceptable": [
            "decision.comunicaciones-email-via-outbox",
            "constraint.comunicaciones-template-ids-stable",
        ],
    },
    {
        "id": "Q9",
        "category": "stale-index",
        "title": "prepare a symbol created on the branch and missing from the index",
        "tool": "prepare_change",
        "args": {
            "target": SHIPPING + "service.ts::splitShipment",
            "intent": "Split a shipment into parts",
        },
        "must": [],
        "forbidden": [
            "constraint.shipment-idempotency-key",
            "decision.cancel-requires-carrier-ack",
            "decision.label-retry-exponential-backoff",
            "decision.label-retry-fixed-3x",
            "constraint.no-weekend-pickups",
        ],
        "acceptable": [],
        "checks": ["index_incomplete_reported"],
    },
    {
        "id": "Q10",
        "category": "budget",
        "title": "prepare with a tight budget",
        "tool": "prepare_change",
        "args": {"target": SHIPPING + "rates.ts::quoteRate", "max_tokens": 700},
        "must": ["decision.rate-cache-ttl-60s"],
        "forbidden": ["decision.rate-cache-ttl-5m"],
        "acceptable": ["constraint.money-in-integer-cents"],
        "checks": ["within_budget"],
    },
    {
        "id": "Q11",
        "category": "retired-module",
        "title": "prepare a path of the retired module",
        "tool": "prepare_change",
        "args": {"target": "src/comunicaciones/outbox.ts"},
        "must": [],
        "forbidden": [],
        "acceptable": [],
        "checks": ["retired_target_explained"],
    },
    {
        "id": "Q12",
        "category": "file",
        "title": "prepare a whole file",
        "tool": "prepare_change",
        "args": {"target": "src/billing/refunds.ts", "intent": "Rename a local variable"},
        "must": ["decision.refund-floor-rounding"],
        "forbidden": [],
        "acceptable": [],
    },
    {
        "id": "Q13",
        "category": "superseded",
        "title": "prepare a symbol whose pinned rule superseded an older rule",
        "tool": "explain_target",
        "args": {"target": "src/secrets/rotation.ts::rotateKeys"},
        "must": ["constraint.rotate-secrets-30d"],
        "forbidden": ["constraint.rotate-secrets-90d"],
        "acceptable": ["exception.legacy-kms-skips-rotation"],
        "checks": ["orphaned_exception_flagged"],
    },
    {
        "id": "Q14",
        "category": "abstention",
        "title": "prepare a symbol no Record governs",
        "tool": "prepare_change",
        "args": {"target": "src/shared/cache.ts::cacheGet", "intent": "Add a size limit"},
        "must": [],
        "forbidden": [],
        "acceptable": [],
        "checks": ["none_found_reported"],
    },
]

# A Record captured for a symbol the index does not know yet. It must govern
# that symbol, not every symbol in the file.
CAPTURE_CANDIDATE = {
    "kind": "decision",
    "severity": "medium",
    "statement": "A split shipment keeps the parent shipment id as the prefix of every part.",
    "rationale": "Carriers reject multi-parcel labels whose parts do not share the parent reference.",
    "durability": "durable",
    "bindings": [SHIPPING + "service.ts::splitShipment"],
}
