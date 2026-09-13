import { describe, expect, it } from "vitest";
import type { ActivityEvent, GraphResponse } from "../api/types";
import { buildGraphModel, nextHighlight, nodeEmphasis, HIGHLIGHT_TTL_MS } from "./graph";

function node(key: string, role: string, record_ids: string[] = []) {
  return {
    key,
    name: key,
    label: "function",
    file_path: "src/a.rs",
    qualified_name: `src.a.${key}`,
    role,
    start_line: 1,
    record_ids,
    selected: true,
    operations: ["op_1"],
    last_seen: "2026-09-13T05:00:00.000Z",
  };
}

const response: GraphResponse = {
  project: "p",
  focus: { operation_id: "op_1", node_key: "n_target", created_at: "2026-09-13T05:00:00.000Z" },
  operations: [],
  nodes: [node("n_target", "target"), node("n_config", "explained", ["decision.x"]), node("n_odd", "wizard")],
  edges: [
    { key: "e_1", source: "n_target", target: "n_config", kind: "uses", state: "orphaned", record_ids: ["decision.x"], selected: true, operations: ["op_1"] },
    { key: "e_dangling", source: "n_target", target: "n_missing", kind: "calls", state: "observed", record_ids: [], selected: true, operations: ["op_1"] },
    { key: "e_self", source: "n_odd", target: "n_odd", kind: "calls", state: "weird", record_ids: [], selected: false, operations: [] },
  ],
  records: {},
};

function event(kind: string, payload: Record<string, unknown>, operation_id = "op_2"): ActivityEvent {
  return {
    schema_version: "rationale/activity/1",
    session_id: "session_a",
    seq: 1,
    operation_id,
    timestamp: "2026-09-13T05:00:00.000Z",
    actor: { type: "agent", client: "claude-code", client_source: "flag" },
    project: "p",
    kind,
    payload,
  };
}

describe("buildGraphModel", () => {
  it("keeps only drawable edges and normalizes unknown roles and states", () => {
    const model = buildGraphModel(response);
    expect(model.edges.map((e) => e.key)).toEqual(["e_1"]);
    expect(model.edges[0].state).toBe("orphaned");
    expect(model.byKey.get("n_odd")?.role).toBe("context");
    expect(model.byKey.get("n_target")?.degree).toBe(1);
    expect(model.byKey.get("n_config")?.recordIds).toEqual(["decision.x"]);
    expect(model.focusKey).toBe("n_target");
  });
});

describe("live highlight", () => {
  it("follows an operation from request to compiled packet to committed memory", () => {
    let highlight = nextHighlight(null, event("context.requested", { target: "src/a.rs::f" }), 1000);
    expect(highlight?.phase).toBe("requested");
    expect(nodeEmphasis("n_other", highlight, 1000)).toBe("normal");

    highlight = nextHighlight(highlight, event("target.resolved", { node_key: "n_target" }), 1100);
    expect(highlight?.target).toBe("n_target");

    highlight = nextHighlight(
      highlight,
      event("packet.compiled", {
        target_node: "n_target",
        considered_node_keys: ["n_target", "n_a", "n_b"],
        selected_node_keys: ["n_target", "n_a"],
        selected_edge_keys: ["e_1"],
        selected_records: ["decision.x"],
      }),
      1200,
    );
    expect(nodeEmphasis("n_target", highlight, 1200)).toBe("focus");
    expect(nodeEmphasis("n_a", highlight, 1200)).toBe("selected");
    expect(nodeEmphasis("n_b", highlight, 1200)).toBe("considered");
    expect(nodeEmphasis("n_elsewhere", highlight, 1200)).toBe("dim");
    expect(highlight?.edges.has("e_1")).toBe(true);

    highlight = nextHighlight(highlight, event("record.committed", { record_id: "decision.y" }), 5000);
    expect(highlight?.phase).toBe("committed");
    expect([...(highlight?.records ?? [])]).toEqual(["decision.x", "decision.y"]);

    expect(nodeEmphasis("n_target", highlight, 5000 + HIGHLIGHT_TTL_MS)).toBe("normal");
  });

  it("ignores a resolution that belongs to another operation", () => {
    const requested = nextHighlight(null, event("context.requested", {}, "op_2"), 0);
    const other = nextHighlight(requested, event("target.resolved", { node_key: "n_x" }, "op_9"), 10);
    expect(other?.target).toBeNull();
  });
});
