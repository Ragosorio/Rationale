import { describe, expect, it } from "vitest";
import type { ActivityEvent, SessionView } from "../api/types";
import {
  ACTIVE_WINDOW_MS,
  collapseTimeline,
  describeEvent,
  foldOperations,
  sessionState,
} from "./activity";

let seq = 0;
function event(kind: string, payload: Record<string, unknown>, operation_id?: string, second = seq): ActivityEvent {
  seq += 1;
  return {
    schema_version: "rationale/activity/1",
    session_id: "session_a",
    seq,
    operation_id,
    timestamp: `2026-09-13T05:00:${String(second).padStart(2, "0")}.000Z`,
    actor: { type: "agent", client: "claude-code", client_source: "flag" },
    project: "p",
    kind,
    payload,
  };
}

const lifecycle: ActivityEvent[] = [
  event("session.started", { pid: 1, version: "0.0.0" }),
  event("context.requested", { target: "src/pay.rs::create_link", intent: "configurable expiry" }, "op_1", 1),
  event("provider.started", { provider: "codebase-memory" }, "op_1", 1),
  event("target.resolved", { resolved: true, node_key: "n_1", qualified_name: "src.pay.create_link" }, "op_1", 1),
  event("provider.finished", { provider: "codebase-memory", status: "successful", coverage: "complete", index_state: "ready", latency_ms: 42 }, "op_1", 1),
  event("packet.compiled", { considered_nodes: 42, selected_node_keys: ["n_1", "n_2"], considered_relationships: 71, selected_edge_keys: ["e_1"], considered_records: 23, selected_records: ["decision.x"], packet_bytes: 8214, estimated_tokens: 2104, budget_overflow: null, target_node: "n_1" }, "op_1", 1),
  event("packet.delivered", { latency_ms: 183, packet_bytes: 8214 }, "op_1", 1),
  event("change.finalized", { candidates: 3, changed_files: 2 }, "op_1", 20),
  event("capture.candidate", { index: 0, kind: "decision" }, "op_1", 20),
  event("capture.candidate", { index: 1, kind: "constraint" }, "op_1", 20),
  event("capture.candidate", { index: 2, kind: "constraint" }, "op_1", 20),
  event("capture.discarded", { index: 2, reason: "mechanical_noise" }, "op_1", 20),
  event("record.committed", { record_id: "decision.expiry", authority: "normal" }, "op_1", 20),
  event("record.committed", { record_id: "constraint.expiry-ceiling", authority: "normal" }, "op_1", 20),
  event("relationship.orphaned", { record_id: "decision.old", from: "src.a.f", to: "src.b.g", kind: "uses", state: "orphaned" }, "op_1", 21),
  event("context.requested", { target: "src/other.rs" }, "op_2", 30),
];

describe("foldOperations", () => {
  it("rebuilds the operation card from its events, newest operation first", () => {
    const [latest, operation] = foldOperations(lifecycle);
    expect(latest.operationId).toBe("op_2");
    expect(operation.intent).toBe("configurable expiry");
    expect(operation.resolved).toBe(true);
    expect(operation.provider).toEqual({ name: "codebase-memory", status: "successful", coverage: "complete", index: "ready", latencyMs: 42 });
    expect(operation.packet).toMatchObject({ consideredNodes: 42, selectedNodes: 2, consideredRelationships: 71, selectedRelationships: 1, consideredRecords: 23, bytes: 8214, tokens: 2104 });
    expect(operation.deliveredMs).toBe(183);
    expect(operation.finalize).toEqual({
      candidates: 3,
      changedFiles: 2,
      committed: ["decision.expiry", "constraint.expiry-ceiling"],
      discarded: ["mechanical_noise"],
      superseded: [],
      conflicts: [],
    });
    expect(operation.alerts).toEqual([{ state: "orphaned", from: "src.a.f", to: "src.b.g", recordId: "decision.old" }]);
    expect(operation.events).toHaveLength(14);
  });
});

describe("collapseTimeline", () => {
  it("groups consecutive events of the same kind", () => {
    const timeline = collapseTimeline(lifecycle.filter((e) => e.operation_id === "op_1"));
    const candidates = timeline.find((entry) => entry.kind === "capture.candidate");
    expect(candidates?.count).toBe(3);
    expect(timeline.filter((entry) => entry.kind === "record.committed")[0].count).toBe(2);
  });
});

describe("sessionState", () => {
  const session = (overrides: Partial<SessionView>): SessionView => ({
    session_id: "s",
    actor: { type: "agent", client: "codex", client_source: "flag" },
    started_at: "2026-09-13T05:00:00.000Z",
    last_event_at: "2026-09-13T05:00:00.000Z",
    last_kind: "agent.connected",
    ended: false,
    events: 2,
    operations: 0,
    last_operation_id: null,
    ...overrides,
  });
  const lastAt = Date.parse("2026-09-13T05:00:00.000Z");

  it("distinguishes active, idle and ended sessions", () => {
    expect(sessionState(session({}), lastAt + 1000)).toBe("active");
    expect(sessionState(session({}), lastAt + ACTIVE_WINDOW_MS + 1)).toBe("idle");
    expect(sessionState(session({ ended: true }), lastAt)).toBe("ended");
  });
});

describe("describeEvent", () => {
  it("summarizes events with what they carry and nothing else", () => {
    expect(describeEvent(lifecycle[5])).toBe("2/42 nodos · 1/71 relaciones · ~2.1k tokens");
    expect(describeEvent(lifecycle[6])).toBe("183 ms · 8.0 KB");
    expect(describeEvent(lifecycle[11])).toBe("#2 mechanical_noise");
    expect(describeEvent(lifecycle[14])).toBe("a.f —uses→ b.g");
  });
});
