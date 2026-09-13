import { useEffect, useState, type ReactNode } from "react";
import { api } from "../api/client";
import type { OperationSummary, RecordView, RelationView } from "../api/types";
import { ROLE_COLORS, ROLE_LABELS, STATE_COLORS, STATE_LABELS } from "../graph/palette";
import { clientLabel, type LiveOperation } from "../model/activity";
import { clockTime, formatBytes, formatTokens, plural, relativeTime, shortId } from "../model/format";
import { ROLES, type EdgeState, type Role } from "../model/graph";
import { Pill, Section, Stat } from "./chrome";

const PROVENANCE: Record<string, string> = {
  agent_asserted: "afirmado por agente",
  human_stated: "declarado por humano",
  migrated: "migrado",
};

export function provenanceLabel(value: string): string {
  return PROVENANCE[value] ?? value;
}

function asRole(value: string): Role {
  return (ROLES as readonly string[]).includes(value) ? (value as Role) : "context";
}

function asState(value: string): EdgeState {
  return value === "indirect" || value === "orphaned" || value === "unknown" ? value : "observed";
}

const STATE_HELP: Record<EdgeState, string> = {
  observed: "La relación directa existe en el índice actual.",
  indirect: "Ya no es directa, pero un camino compatible y acotado conecta los extremos.",
  orphaned: "No se puede localizar: la explicación puede haber quedado obsoleta. Nunca se borra.",
  unknown: "El proveedor no pudo verificarla. No implica que haya desaparecido.",
};

export function RecordCard({ record }: { record: RecordView }) {
  return (
    <article className={`record record-${record.status}`}>
      <header>
        <Pill tone="causal">{record.kind}</Pill>
        {record.authority === "pinned" && <Pill tone="accent">fijado</Pill>}
        <Pill>{provenanceLabel(record.provenance)}</Pill>
        {record.status !== "active" && <Pill tone="warn">{record.status}</Pill>}
      </header>
      <p className="statement">{record.statement}</p>
      {record.rationale && (
        <p className="rationale">
          <span className="label">por qué</span>
          {record.rationale}
        </p>
      )}
      <footer>{record.id}</footer>
    </article>
  );
}

interface DetailState<T> {
  data: T | null;
  error: string | null;
  loading: boolean;
}

function useDetail<T>(load: (signal: AbortSignal) => Promise<T>, deps: unknown[]): DetailState<T> {
  const [state, setState] = useState<DetailState<T>>({ data: null, error: null, loading: true });
  useEffect(() => {
    const controller = new AbortController();
    setState((previous) => ({ ...previous, loading: true }));
    load(controller.signal)
      .then((data) => setState({ data, error: null, loading: false }))
      .catch((reason: unknown) => {
        if (controller.signal.aborted) return;
        setState({ data: null, error: reason instanceof Error ? reason.message : String(reason), loading: false });
      });
    return () => controller.abort();
    // `load` es un cierre nuevo en cada render; las dependencias reales son `deps`.
  }, deps); // eslint-disable-line react-hooks/exhaustive-deps
  return state;
}

function CloseButton({ onClose }: { onClose: () => void }) {
  return (
    <button className="close" onClick={onClose} aria-label="Cerrar detalle">
      ×
    </button>
  );
}

export function PanelMessage({
  title,
  message,
  onClose,
}: {
  title: string;
  message: ReactNode;
  onClose?: () => void;
}) {
  return (
    <div className="panel">
      <header className="panel-head">
        <div className="panel-top">
          <span className="eyebrow">{title}</span>
          {onClose && <CloseButton onClose={onClose} />}
        </div>
        <p className="dim small">{message}</p>
      </header>
    </div>
  );
}

function RelationList({
  title,
  items,
  direction,
  onSelectEdge,
  onSelectNode,
}: {
  title: string;
  items: RelationView[];
  direction: "in" | "out";
  onSelectEdge: (key: string) => void;
  onSelectNode: (key: string) => void;
}) {
  if (items.length === 0) return null;
  return (
    <>
      <div className="subhead">
        {title} <span className="count">{items.length}</span>
      </div>
      <ul className="relations">
        {items.map((item) => {
          const state = asState(item.state);
          return (
            <li key={item.key} onClick={() => onSelectEdge(item.key)}>
              <span className="edge-kind">
                {direction === "out" ? "→" : "←"} {item.kind}
              </span>
              <button
                className="link ellipsis"
                onClick={(event) => {
                  event.stopPropagation();
                  onSelectNode(item.node.key);
                }}
                title={item.node.qualified_name}
              >
                {item.node.name ?? shortId(item.node.key)}
              </button>
              <span className="relation-flags">
                {item.record_ids.length > 0 && <span className="causal-dot" title="explicada por el canon" />}
                {state !== "observed" && (
                  <span className="state-text" style={{ color: STATE_COLORS[state] }}>
                    {STATE_LABELS[state]}
                  </span>
                )}
              </span>
            </li>
          );
        })}
      </ul>
    </>
  );
}

export function NodePanel({
  nodeKey,
  version,
  now,
  onSelectNode,
  onSelectEdge,
  onClose,
}: {
  nodeKey: string;
  version: number;
  now: number;
  onSelectNode: (key: string) => void;
  onSelectEdge: (key: string) => void;
  onClose: () => void;
}) {
  const { data, error, loading } = useDetail((signal) => api.node(nodeKey, signal), [nodeKey, version]);
  if (!data) {
    return <PanelMessage title="nodo" message={loading ? "Cargando…" : (error ?? "Sin datos")} onClose={onClose} />;
  }
  const { node } = data;
  const role = asRole(node.role);
  const records = Object.values(data.records);
  return (
    <div className="panel">
      <header className="panel-head">
        <div className="panel-top">
          <span className="eyebrow">nodo · {node.label}</span>
          <CloseButton onClose={onClose} />
        </div>
        <h2 className="title" style={{ color: ROLE_COLORS[role] }}>
          {node.name}
        </h2>
        <p className="mono dim wrap">{node.qualified_name}</p>
        <p className="mono dim wrap">
          {node.file_path}
          {node.start_line ? `:${node.start_line}` : ""}
        </p>
        <div className="pills">
          <Pill>{ROLE_LABELS[role]}</Pill>
          {node.selected && <Pill tone="accent">entregado en un packet</Pill>}
        </div>
      </header>
      <Section title="Por qué · memoria" aside={<span className="count">{records.length}</span>}>
        {records.length === 0 ? (
          <p className="dim small">Ningún Record explica este nodo todavía. Eso no implica que no importe.</p>
        ) : (
          records.map((record) => <RecordCard key={record.id} record={record} />)
        )}
      </Section>
      <Section title="Relaciones" aside={<span className="count">{data.outgoing.length + data.incoming.length}</span>}>
        <RelationList title="salientes" items={data.outgoing} direction="out" onSelectEdge={onSelectEdge} onSelectNode={onSelectNode} />
        <RelationList title="entrantes" items={data.incoming} direction="in" onSelectEdge={onSelectEdge} onSelectNode={onSelectNode} />
        {data.outgoing.length + data.incoming.length === 0 && (
          <p className="dim small">Sin relaciones en las operaciones recientes.</p>
        )}
      </Section>
      <Section title="Apariciones" aside={<span className="count">{data.appearances.length}</span>}>
        <ol className="appearances">
          {data.appearances.slice(0, 8).map((appearance) => (
            <li key={appearance.operation_id}>
              <span className="ellipsis">
                {appearance.intent ?? <span className="dim">sin intención declarada</span>}
              </span>
              <span className="mono dim">
                {relativeTime(appearance.created_at, now)} · {appearance.role}
                {appearance.selected ? "" : " · solo considerado"}
              </span>
            </li>
          ))}
        </ol>
      </Section>
    </div>
  );
}

export function EdgePanel({
  edgeKey,
  version,
  onSelectNode,
  onClose,
}: {
  edgeKey: string;
  version: number;
  onSelectNode: (key: string) => void;
  onClose: () => void;
}) {
  const { data, error, loading } = useDetail((signal) => api.edge(edgeKey, signal), [edgeKey, version]);
  if (!data) {
    return <PanelMessage title="relación" message={loading ? "Cargando…" : (error ?? "Sin datos")} onClose={onClose} />;
  }
  const { edge } = data;
  const state = asState(edge.state);
  const records = Object.values(data.records);
  const transitions: { state: EdgeState; at: string; count: number }[] = [];
  for (const entry of data.history) {
    const entryState = asState(entry.state);
    const last = transitions[transitions.length - 1];
    if (last && last.state === entryState) last.count += 1;
    else transitions.push({ state: entryState, at: entry.created_at, count: 1 });
  }
  return (
    <div className="panel">
      <header className="panel-head">
        <div className="panel-top">
          <span className="eyebrow">relación</span>
          <CloseButton onClose={onClose} />
        </div>
        <div className="edge-title">
          <button className="link" onClick={() => edge.source && onSelectNode(edge.source.key)}>
            {edge.source?.name ?? shortId(edge.source?.key)}
          </button>
          <span className="edge-kind big">{(edge.kind ?? "?").toUpperCase()}</span>
          <button className="link" onClick={() => edge.target && onSelectNode(edge.target.key)}>
            {edge.target?.name ?? shortId(edge.target?.key)}
          </button>
        </div>
        <p className="mono dim wrap">
          {edge.source?.qualified_name ?? "—"} → {edge.target?.qualified_name ?? "—"}
        </p>
      </header>
      <Section title="Estructura">
        <div className="state-line">
          <span className="state-dot" style={{ background: STATE_COLORS[state] }} />
          <b style={{ color: STATE_COLORS[state] }}>{STATE_LABELS[state]}</b>
        </div>
        <p className="dim small">{STATE_HELP[state]}</p>
      </Section>
      <Section title="Por qué" aside={<span className="count">{records.length}</span>}>
        {records.length === 0 ? (
          <p className="dim small">Ningún Record explica esta relación todavía.</p>
        ) : (
          records.map((record) => <RecordCard key={record.id} record={record} />)
        )}
      </Section>
      <Section title="Historia">
        {transitions.length === 0 ? (
          <p className="dim small">Sin apariciones en operaciones recientes.</p>
        ) : (
          <ol className="history">
            {transitions.map((transition) => (
              <li key={`${transition.at}-${transition.state}`}>
                <span className="state-dot" style={{ background: STATE_COLORS[transition.state] }} />
                <span>{STATE_LABELS[transition.state]}</span>
                <span className="mono dim">
                  desde {clockTime(transition.at)} · {transition.count}×
                </span>
              </li>
            ))}
          </ol>
        )}
      </Section>
    </div>
  );
}

export function OperationPanel({
  operation,
  summary,
  now,
  onOpenActivity,
}: {
  operation: LiveOperation | null;
  summary: OperationSummary | null;
  now: number;
  onOpenActivity: () => void;
}) {
  if (!operation && !summary) {
    return (
      <PanelMessage
        title="operación"
        message="Cuando un agente llame prepare_change, su contexto aparecerá aquí: el target, el subgrafo que Rationale seleccionó y la memoria que lo explica."
      />
    );
  }
  const packet = operation?.packet ?? null;
  const selection = summary?.selection ?? null;
  const client = operation?.client ?? summary?.actor.client ?? "unknown";
  const started = operation?.startedAt ?? summary?.created_at ?? "";
  const target =
    operation?.qualifiedName ?? summary?.target.qualified_name ?? operation?.target ?? summary?.target.spec ?? "—";
  const intent = operation?.intent ?? summary?.intent ?? null;
  const finalize = operation?.finalize ?? null;
  const committed = finalize?.committed ?? summary?.finalized?.committed ?? [];
  return (
    <div className="panel">
      <header className="panel-head">
        <span className="eyebrow">operación · {clientLabel(client)}</span>
        <h2 className="title">{intent ?? <span className="dim">sin intención declarada</span>}</h2>
        <p className="mono dim wrap">{target}</p>
        <p className="mono dim">
          {shortId(operation?.operationId ?? summary?.operation_id, 24)} · {relativeTime(started, now)}
        </p>
      </header>
      <div className="stats-grid">
        <Stat
          label="nodos"
          value={packet?.selectedNodes ?? selection?.selected_nodes ?? 0}
          sub={`de ${packet?.consideredNodes ?? selection?.considered_nodes ?? 0} considerados`}
        />
        <Stat
          label="relaciones"
          value={packet?.selectedRelationships ?? selection?.selected_relationships ?? 0}
          sub={`de ${packet?.consideredRelationships ?? selection?.considered_relationships ?? 0}`}
        />
        <Stat
          label="records"
          value={packet?.selectedRecords.length ?? selection?.selected_records.length ?? 0}
          sub={`de ${packet?.consideredRecords ?? selection?.considered_records ?? 0} activos`}
        />
        <Stat
          label="packet"
          value={formatBytes(packet?.bytes ?? selection?.packet_bytes ?? 0)}
          sub={`${formatTokens(packet?.tokens ?? selection?.estimated_tokens ?? 0)} tokens`}
        />
      </div>
      {operation?.provider && (
        <Section title="Proveedor estructural">
          <dl className="kv">
            <dt>proveedor</dt>
            <dd>{operation.provider.name ?? "—"}</dd>
            <dt>estado</dt>
            <dd>
              {operation.provider.status ?? "…"} / {operation.provider.coverage ?? "…"}
            </dd>
            <dt>índice</dt>
            <dd>{operation.provider.index ?? "—"}</dd>
            <dt>latencia</dt>
            <dd>{operation.provider.latencyMs !== null ? `${operation.provider.latencyMs} ms` : "—"}</dd>
          </dl>
        </Section>
      )}
      {(finalize || committed.length > 0) && (
        <Section title="Captura">
          {finalize && (
            <p className="mono small">
              {finalize.candidates} {plural(finalize.candidates, "candidato", "candidatos")} · {finalize.discarded.length}{" "}
              {plural(finalize.discarded.length, "descartado", "descartados")} · {finalize.committed.length}{" "}
              {plural(finalize.committed.length, "escrito", "escritos")}
              {finalize.conflicts.length > 0 && <span className="danger-text"> · {finalize.conflicts.length} conflicto</span>}
            </p>
          )}
          <div className="pills">
            {committed.map((id) => (
              <Pill key={id} tone="causal">
                {id}
              </Pill>
            ))}
          </div>
        </Section>
      )}
      {operation && operation.alerts.length > 0 && (
        <Section title="Explicaciones en riesgo">
          <ul className="alerts">
            {operation.alerts.map((alert) => (
              <li key={`${alert.recordId}-${alert.from}-${alert.to}`}>
                <span className="warn-text">{alert.state}</span>
                <span className="mono dim ellipsis">
                  {alert.from} → {alert.to}
                </span>
                <span className="mono dim">{alert.recordId}</span>
              </li>
            ))}
          </ul>
        </Section>
      )}
      <button className="btn" onClick={onOpenActivity}>
        Ver la actividad de la operación →
      </button>
    </div>
  );
}
