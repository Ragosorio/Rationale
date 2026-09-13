// Contratos de la API local de `rationale ui` (src/ui/api.rs). La UI observa:
// ningún endpoint escribe.

export interface Actor {
  type: "agent" | "cli" | string;
  client: string;
  client_source: string;
}

export interface ActorContext {
  client: string;
  client_source: string;
  session_id: string | null;
  operation_id: string | null;
}

export interface ActivityEvent {
  schema_version: string;
  session_id: string;
  seq: number;
  trace_id?: string;
  operation_id?: string;
  timestamp: string;
  actor: Actor;
  project: string;
  kind: string;
  payload: Record<string, unknown>;
}

export interface OperationTarget {
  spec: string;
  file_path: string | null;
  symbol: string | null;
  node_key: string | null;
  qualified_name: string | null;
}

export interface Selection {
  considered_nodes: number;
  selected_nodes: number;
  considered_relationships: number;
  selected_relationships: number;
  considered_records: number;
  selected_records: string[];
  packet_bytes: number;
  estimated_tokens: number;
}

export interface FinalizeSummary {
  finalized_at: string;
  committed: string[];
  discarded: number;
  conflicts: string[];
}

export interface OperationSummary {
  operation_id: string;
  created_at: string;
  actor: ActorContext;
  target: OperationTarget;
  intent: string | null;
  selection: Selection;
  finalized: FinalizeSummary | null;
}

export interface RecordRelationship {
  key: string;
  kind: string;
  source: string;
  source_file: string;
  source_key: string;
  target: string;
  target_file: string;
  target_key: string;
}

export interface RecordView {
  id: string;
  kind: string;
  severity: string;
  statement: string;
  rationale: string | null;
  authority: string;
  provenance: string;
  status: "active" | "superseded" | "revoked" | string;
  supersedes: string[];
  superseded_by: string | null;
  bindings: {
    kind: string;
    path: string | null;
    structural_id: string | null;
    provisional: boolean;
  }[];
  relationships: RecordRelationship[];
}

export interface GraphNodeView {
  key: string;
  name: string;
  label: string;
  file_path: string;
  qualified_name: string;
  role: string;
  start_line: number | null;
  record_ids: string[];
  selected: boolean;
  operations: string[];
  last_seen: string;
}

export interface GraphEdgeView {
  key: string;
  source: string;
  target: string;
  kind: string;
  state: string;
  record_ids: string[];
  selected: boolean;
  operations: string[];
}

export interface GraphResponse {
  project: string;
  focus: { operation_id: string; node_key: string | null; created_at: string } | null;
  operations: OperationSummary[];
  nodes: GraphNodeView[];
  edges: GraphEdgeView[];
  records: Record<string, RecordView>;
}

export interface SessionView {
  session_id: string;
  actor: Actor;
  started_at: string;
  last_event_at: string;
  last_kind: string;
  ended: boolean;
  events: number;
  operations: number;
  last_operation_id: string | null;
}

export interface Meta {
  name: string;
  version: string;
  project: { id: string; root: string };
  git: { revision: string | null; dirty: boolean };
  canon: {
    records: number;
    active: number;
    superseded: number;
    revoked: number;
    pinned: number;
  };
  conflicts_pending: number;
  latest_operation: OperationSummary | null;
  sessions: SessionView[];
}

export interface NodeRef {
  key: string;
  name?: string;
  qualified_name?: string;
  file_path?: string;
  label?: string;
}

export interface RelationView {
  key: string;
  kind: string;
  state: string;
  record_ids: string[];
  node: NodeRef;
}

export interface NodeDetail {
  node: {
    key: string;
    name: string;
    label: string;
    file_path: string;
    qualified_name: string;
    role: string;
    selected: boolean;
    record_ids: string[];
    start_line: number | null;
  };
  incoming: RelationView[];
  outgoing: RelationView[];
  records: Record<string, RecordView>;
  appearances: {
    operation_id: string;
    created_at: string;
    intent: string | null;
    role: string;
    selected: boolean;
  }[];
}

export interface EdgeDetail {
  edge: {
    key: string;
    kind?: string;
    state: string;
    source?: NodeRef;
    target?: NodeRef;
  };
  history: { operation_id: string; created_at: string; state: string; selected: boolean }[];
  records: Record<string, RecordView>;
}

export interface ConflictView {
  conflict_id: string;
  pinned_record_id: string;
  pinned_statement: string;
  candidate_statement: string;
  question: string;
  options: { decision: string; meaning: string }[];
}
