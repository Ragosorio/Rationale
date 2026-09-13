// Modelo del subgrafo de trabajo: transforma la respuesta de /api/graph en
// lo que dibuja el renderer y deriva el resaltado en vivo desde la actividad.
// Funciones puras: se prueban sin navegador.

import type { ActivityEvent, GraphResponse } from "../api/types";

export const ROLES = [
  "target",
  "explained",
  "caller",
  "callee",
  "dependency",
  "dependent",
  "test",
  "context",
] as const;
export type Role = (typeof ROLES)[number];

export type EdgeState = "observed" | "indirect" | "orphaned" | "unknown";

export interface ModelNode {
  key: string;
  name: string;
  label: string;
  file: string;
  qualifiedName: string;
  role: Role;
  selected: boolean;
  recordIds: string[];
  operations: string[];
  lastSeen: string;
  startLine: number | null;
  degree: number;
}

export interface ModelEdge {
  key: string;
  source: string;
  target: string;
  kind: string;
  state: EdgeState;
  recordIds: string[];
  selected: boolean;
}

export interface GraphModel {
  nodes: ModelNode[];
  edges: ModelEdge[];
  byKey: Map<string, ModelNode>;
  focusKey: string | null;
  focusOperation: string | null;
}

function asRole(role: string): Role {
  return (ROLES as readonly string[]).includes(role) ? (role as Role) : "context";
}

function asState(state: string): EdgeState {
  return state === "indirect" || state === "orphaned" || state === "unknown" ? state : "observed";
}

export function buildGraphModel(response: GraphResponse): GraphModel {
  const byKey = new Map<string, ModelNode>();
  for (const node of response.nodes) {
    byKey.set(node.key, {
      key: node.key,
      name: node.name,
      label: node.label,
      file: node.file_path,
      qualifiedName: node.qualified_name,
      role: asRole(node.role),
      selected: node.selected,
      recordIds: node.record_ids ?? [],
      operations: node.operations ?? [],
      lastSeen: node.last_seen,
      startLine: node.start_line ?? null,
      degree: 0,
    });
  }
  const edges: ModelEdge[] = [];
  for (const edge of response.edges) {
    const source = byKey.get(edge.source);
    const target = byKey.get(edge.target);
    // Una arista sin sus dos extremos no se puede dibujar honestamente.
    if (!source || !target || source === target) continue;
    source.degree += 1;
    target.degree += 1;
    edges.push({
      key: edge.key,
      source: edge.source,
      target: edge.target,
      kind: edge.kind,
      state: asState(edge.state),
      recordIds: edge.record_ids ?? [],
      selected: edge.selected,
    });
  }
  return {
    nodes: [...byKey.values()],
    edges,
    byKey,
    focusKey: response.focus?.node_key ?? null,
    focusOperation: response.focus?.operation_id ?? null,
  };
}

/** Lo que la actividad reciente enciende sobre el grafo. */
export interface Highlight {
  operationId: string | null;
  target: string | null;
  considered: Set<string>;
  selected: Set<string>;
  edges: Set<string>;
  records: Set<string>;
  startedAt: number;
  phase: "requested" | "compiled" | "committed";
}

export const HIGHLIGHT_TTL_MS = 60_000;

function stringList(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

function stringOr(value: unknown, fallback: string | null): string | null {
  return typeof value === "string" ? value : fallback;
}

export function nextHighlight(
  previous: Highlight | null,
  event: ActivityEvent,
  now: number,
): Highlight | null {
  const payload = event.payload ?? {};
  const operationId = event.operation_id ?? null;
  switch (event.kind) {
    case "context.requested":
      return {
        operationId,
        target: null,
        considered: new Set(),
        selected: new Set(),
        edges: new Set(),
        records: new Set(),
        startedAt: now,
        phase: "requested",
      };
    case "target.resolved":
      if (!previous || previous.operationId !== operationId) return previous;
      return { ...previous, target: stringOr(payload.node_key, previous.target) };
    case "packet.compiled":
      return {
        operationId,
        target: stringOr(payload.target_node, previous?.operationId === operationId ? previous.target : null),
        considered: new Set(stringList(payload.considered_node_keys)),
        selected: new Set(stringList(payload.selected_node_keys)),
        edges: new Set(stringList(payload.selected_edge_keys)),
        records: new Set(stringList(payload.selected_records)),
        startedAt: now,
        phase: "compiled",
      };
    case "record.committed": {
      if (!previous) return previous;
      const recordId = stringOr(payload.record_id, null);
      return {
        ...previous,
        records: new Set(recordId ? [...previous.records, recordId] : previous.records),
        startedAt: now,
        phase: "committed",
      };
    }
    default:
      return previous;
  }
}

export function highlightActive(highlight: Highlight | null, now: number): highlight is Highlight {
  return highlight !== null && now - highlight.startedAt < HIGHLIGHT_TTL_MS;
}

export type NodeEmphasis = "focus" | "selected" | "considered" | "dim" | "normal";

export function nodeEmphasis(key: string, highlight: Highlight | null, now: number): NodeEmphasis {
  if (!highlightActive(highlight, now)) return "normal";
  if (highlight.target === key) return "focus";
  if (highlight.selected.has(key)) return "selected";
  if (highlight.considered.has(key)) return "considered";
  return highlight.phase === "requested" ? "normal" : "dim";
}
