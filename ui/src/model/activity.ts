// Plegado de la actividad: de eventos a operaciones vivas, línea de tiempo y
// estado de sesiones. Funciones puras: se prueban sin navegador.

import type { ActivityEvent, SessionView } from "../api/types";
import { formatBytes, formatTokens, shortName } from "./format";

export interface LiveOperation {
  operationId: string;
  sessionId: string;
  client: string;
  actorType: string;
  startedAt: string;
  lastAt: string;
  intent: string | null;
  target: string | null;
  targetNode: string | null;
  resolved: boolean | null;
  qualifiedName: string | null;
  provider: {
    name: string | null;
    status: string | null;
    coverage: string | null;
    index: string | null;
    latencyMs: number | null;
  } | null;
  packet: {
    consideredNodes: number;
    selectedNodes: number;
    consideredRelationships: number;
    selectedRelationships: number;
    consideredRecords: number;
    selectedRecords: string[];
    bytes: number;
    tokens: number;
    overflow: string | null;
  } | null;
  deliveredMs: number | null;
  finalize: {
    candidates: number;
    changedFiles: number;
    committed: string[];
    discarded: string[];
    superseded: string[];
    conflicts: string[];
  } | null;
  alerts: { state: string; from: string; to: string; recordId: string }[];
  events: ActivityEvent[];
}

const num = (value: unknown): number =>
  typeof value === "number" && Number.isFinite(value) ? value : 0;
const str = (value: unknown): string | null => (typeof value === "string" ? value : null);
const list = (value: unknown): string[] =>
  Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];

function finalizeOf(operation: LiveOperation): NonNullable<LiveOperation["finalize"]> {
  operation.finalize ??= {
    candidates: 0,
    changedFiles: 0,
    committed: [],
    discarded: [],
    superseded: [],
    conflicts: [],
  };
  return operation.finalize;
}

/** Operaciones reconstruidas desde sus eventos, la más reciente primero. */
export function foldOperations(events: ActivityEvent[]): LiveOperation[] {
  const byId = new Map<string, LiveOperation>();
  for (const event of events) {
    const id = event.operation_id;
    if (!id) continue;
    let operation = byId.get(id);
    if (!operation) {
      operation = {
        operationId: id,
        sessionId: event.session_id,
        client: event.actor.client,
        actorType: event.actor.type,
        startedAt: event.timestamp,
        lastAt: event.timestamp,
        intent: null,
        target: null,
        targetNode: null,
        resolved: null,
        qualifiedName: null,
        provider: null,
        packet: null,
        deliveredMs: null,
        finalize: null,
        alerts: [],
        events: [],
      };
      byId.set(id, operation);
    }
    operation.lastAt = event.timestamp;
    operation.events.push(event);
    const payload = event.payload ?? {};
    switch (event.kind) {
      case "context.requested":
        operation.intent = str(payload.intent);
        operation.target = str(payload.target);
        break;
      case "target.resolved":
        operation.resolved = payload.resolved === true;
        operation.targetNode = str(payload.node_key);
        operation.qualifiedName = str(payload.qualified_name);
        break;
      case "provider.started":
        operation.provider = {
          name: str(payload.provider),
          status: null,
          coverage: null,
          index: null,
          latencyMs: null,
        };
        break;
      case "provider.finished":
        operation.provider = {
          name: str(payload.provider),
          status: str(payload.status),
          coverage: str(payload.coverage),
          index: str(payload.index_state),
          latencyMs: num(payload.latency_ms),
        };
        break;
      case "packet.compiled":
        operation.packet = {
          consideredNodes: num(payload.considered_nodes),
          selectedNodes: list(payload.selected_node_keys).length,
          consideredRelationships: num(payload.considered_relationships),
          selectedRelationships: list(payload.selected_edge_keys).length,
          consideredRecords: num(payload.considered_records),
          selectedRecords: list(payload.selected_records),
          bytes: num(payload.packet_bytes),
          tokens: num(payload.estimated_tokens),
          overflow: str(payload.budget_overflow),
        };
        operation.targetNode ??= str(payload.target_node);
        break;
      case "packet.delivered":
        operation.deliveredMs = num(payload.latency_ms);
        break;
      case "change.finalized": {
        const finalize = finalizeOf(operation);
        finalize.candidates = num(payload.candidates);
        finalize.changedFiles = num(payload.changed_files);
        break;
      }
      case "record.committed":
        finalizeOf(operation).committed.push(str(payload.record_id) ?? "?");
        break;
      case "capture.discarded":
        finalizeOf(operation).discarded.push(str(payload.reason) ?? "discarded");
        break;
      case "record.superseded":
        finalizeOf(operation).superseded.push(str(payload.record_id) ?? "?");
        break;
      case "conflict.detected":
        finalizeOf(operation).conflicts.push(str(payload.conflict_id) ?? "?");
        break;
      case "relationship.indirect":
      case "relationship.orphaned":
        operation.alerts.push({
          state: str(payload.state) ?? event.kind.slice("relationship.".length),
          from: str(payload.from) ?? "?",
          to: str(payload.to) ?? "?",
          recordId: str(payload.record_id) ?? "?",
        });
        break;
      default:
        break;
    }
  }
  return [...byId.values()].sort((a, b) => b.startedAt.localeCompare(a.startedAt));
}

export interface TimelineEntry {
  key: string;
  timestamp: string;
  kind: string;
  count: number;
  event: ActivityEvent;
}

/** Eventos consecutivos del mismo tipo y sesión se agrupan: `capture.candidate × 3`. */
export function collapseTimeline(events: ActivityEvent[]): TimelineEntry[] {
  const entries: TimelineEntry[] = [];
  for (const event of events) {
    const last = entries[entries.length - 1];
    if (last && last.kind === event.kind && last.event.session_id === event.session_id) {
      last.count += 1;
      continue;
    }
    entries.push({
      key: `${event.session_id}:${event.seq}`,
      timestamp: event.timestamp,
      kind: event.kind,
      count: 1,
      event,
    });
  }
  return entries;
}

/** Una sesión sin eventos en esta ventana se muestra inactiva, no conectada. */
export const ACTIVE_WINDOW_MS = 15 * 60_000;

export type SessionState = "active" | "idle" | "ended";

export function sessionState(session: SessionView, now: number): SessionState {
  if (session.ended) return "ended";
  const last = Date.parse(session.last_event_at);
  return Number.isFinite(last) && now - last < ACTIVE_WINDOW_MS ? "active" : "idle";
}

export const CLIENT_LABELS: Record<string, string> = {
  "claude-code": "Claude Code",
  codex: "Codex",
  cursor: "Cursor",
  cli: "CLI",
  unknown: "Cliente desconocido",
};

export function clientLabel(client: string): string {
  return CLIENT_LABELS[client] ?? client;
}

/** Una línea legible por evento, solo con lo que el evento trae. */
export function describeEvent(event: ActivityEvent): string {
  const p = event.payload ?? {};
  switch (event.kind) {
    case "session.started":
      return `pid ${num(p.pid)} · v${str(p.version) ?? "?"}`;
    case "agent.connected":
      return `${clientLabel(str(p.client) ?? "unknown")} (${str(p.client_source) ?? "?"})`;
    case "context.requested":
      return [str(p.target), str(p.intent)].filter(Boolean).join(" — ");
    case "provider.started":
      return str(p.provider) ?? "sin proveedor";
    case "target.resolved":
      return p.resolved === true ? shortName(str(p.qualified_name)) : "no resuelto por el proveedor";
    case "provider.finished":
      return `${str(p.provider) ?? "?"} ${str(p.status) ?? "?"}/${str(p.coverage) ?? "?"} · ${num(p.latency_ms)} ms`;
    case "packet.compiled":
      return `${list(p.selected_node_keys).length}/${num(p.considered_nodes)} nodos · ${list(p.selected_edge_keys).length}/${num(p.considered_relationships)} relaciones · ${formatTokens(num(p.estimated_tokens))} tokens`;
    case "packet.delivered":
      return `${num(p.latency_ms)} ms · ${formatBytes(num(p.packet_bytes))}`;
    case "change.finalized":
      return `${num(p.candidates)} candidatos · ${num(p.changed_files)} archivos`;
    case "capture.candidate":
      return `#${num(p.index)} ${str(p.kind) ?? "mal formado"}`;
    case "capture.discarded":
      return `#${num(p.index)} ${str(p.reason) ?? "?"}`;
    case "record.committed":
      return `${str(p.record_id) ?? "?"} · ${str(p.authority) ?? "normal"}`;
    case "record.superseded":
      return `${str(p.record_id) ?? "?"} → ${str(p.superseded_by) ?? "?"}`;
    case "relationship.indirect":
    case "relationship.orphaned":
      return `${shortName(str(p.from))} —${str(p.kind) ?? "?"}→ ${shortName(str(p.to))}`;
    case "conflict.detected":
      return `${str(p.conflict_id) ?? "?"} contra ${str(p.pinned_record_id) ?? "?"}`;
    case "conflict.resolved":
      return `${str(p.conflict_id) ?? "?"}: ${str(p.decision) ?? "?"}`;
    default:
      return "";
  }
}
