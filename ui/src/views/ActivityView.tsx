import { useMemo, useState } from "react";
import type { ActivityEvent } from "../api/types";
import { kindFamily, Pill, SESSION_STATE_TEXT } from "../components/chrome";
import type { LiveData } from "../live/useLive";
import {
  clientLabel,
  collapseTimeline,
  describeEvent,
  foldOperations,
  sessionState,
  type LiveOperation,
} from "../model/activity";
import { clockTime, formatBytes, formatTokens, plural, relativeTime, shortId } from "../model/format";

const LIVE_WINDOW_MS = 2 * 60_000;

function MetricGroup({ title, lines }: { title: string; lines: [string, string | number | null | undefined][] }) {
  return (
    <div className="metric-group">
      <h4>{title}</h4>
      {lines.map(([label, value]) => (
        <div key={label} className="metric-line">
          <span className="dim">{label}</span>
          <b>{value ?? "—"}</b>
        </div>
      ))}
    </div>
  );
}

function OperationCard({
  operation,
  now,
  onShowGraph,
}: {
  operation: LiveOperation;
  now: number;
  onShowGraph: () => void;
}) {
  const timeline = collapseTimeline(operation.events);
  const packet = operation.packet;
  const provider = operation.provider;
  const finalize = operation.finalize;
  const running = now - Date.parse(operation.lastAt) < LIVE_WINDOW_MS;
  return (
    <div className="operation-card">
      <div className="op-header">
        <span className="op-client">{clientLabel(operation.client)}</span>
        <span className={`state-badge ${running ? "state-active" : "state-idle"}`}>
          {running ? "● activo" : relativeTime(operation.lastAt, now)}
        </span>
      </div>
      <div className="mono dim">{operation.operationId}</div>
      <h2 className="op-intent">{operation.intent ?? <span className="dim">sin intención declarada</span>}</h2>
      <div className="op-target mono">
        <span className="wrap">{operation.qualifiedName ?? operation.target ?? "—"}</span>
        {operation.resolved === false && <Pill tone="warn">no resuelto por el proveedor</Pill>}
      </div>

      <div className="metric-groups">
        <MetricGroup
          title="estructura"
          lines={[
            ["nodos considerados", packet?.consideredNodes],
            ["nodos seleccionados", packet?.selectedNodes],
            ["relaciones consideradas", packet?.consideredRelationships],
            ["relaciones seleccionadas", packet?.selectedRelationships],
          ]}
        />
        <MetricGroup
          title="memoria"
          lines={[
            ["records activos", packet?.consideredRecords],
            ["records servidos", packet?.selectedRecords.length],
          ]}
        />
        <MetricGroup
          title="packet"
          lines={[
            ["tamaño", packet ? formatBytes(packet.bytes) : null],
            ["tokens", packet ? formatTokens(packet.tokens) : null],
            ["overflow", packet ? (packet.overflow ?? "no") : null],
          ]}
        />
        <MetricGroup
          title="proveedor"
          lines={[
            ["proveedor", provider?.name],
            ["estado", provider?.status ? `${provider.status}/${provider.coverage ?? "?"}` : null],
            ["índice", provider?.index],
            ["latencia", provider?.latencyMs != null ? `${provider.latencyMs} ms` : null],
          ]}
        />
        <MetricGroup title="entrega" lines={[["prepare → packet", operation.deliveredMs != null ? `${operation.deliveredMs} ms` : null]]} />
      </div>

      {finalize && (
        <div className="finalize-strip">
          <span className="subhead">captura</span>
          <span className="mono">
            <b>{finalize.candidates}</b> {plural(finalize.candidates, "candidato", "candidatos")}
          </span>
          <span className="mono dim">
            <b>{finalize.discarded.length}</b> {plural(finalize.discarded.length, "descartado", "descartados")}{finalize.discarded.length > 0 && ` (${[...new Set(finalize.discarded)].join(", ")})`}
          </span>
          <span className="mono causal-text">
            <b>{finalize.committed.length}</b> {plural(finalize.committed.length, "escrito", "escritos")}
          </span>
          {finalize.superseded.length > 0 && (
            <span className="mono warn-text">
              <b>{finalize.superseded.length}</b> {plural(finalize.superseded.length, "reemplazado", "reemplazados")}
            </span>
          )}
          {finalize.conflicts.length > 0 && (
            <span className="mono danger-text">
              <b>{finalize.conflicts.length}</b> conflicto con regla fijada
            </span>
          )}
        </div>
      )}

      {operation.alerts.length > 0 && (
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
      )}

      <div className="subhead">línea de tiempo</div>
      <ol className="timeline">
        {timeline.map((entry) => (
          <li key={entry.key}>
            <span className="times">{clockTime(entry.timestamp)}</span>
            <span className={`kind ${kindFamily(entry.kind)}`}>
              {entry.kind}
              {entry.count > 1 && <span className="times-count"> × {entry.count}</span>}
            </span>
            <span className="dim ellipsis">{describeEvent(entry.event)}</span>
          </li>
        ))}
      </ol>
      <button className="btn" onClick={onShowGraph}>
        Ver el subgrafo en el grafo →
      </button>
    </div>
  );
}

function StreamRow({ event, now }: { event: ActivityEvent; now: number }) {
  const fresh = now - Date.parse(event.timestamp) < 4000;
  return (
    <li className={fresh ? "fresh" : undefined}>
      <span className="mono dim">{clockTime(event.timestamp)}</span>
      <span className="stream-body">
        <span className={`kind ${kindFamily(event.kind)}`}>{event.kind}</span>
        <span className="dim ellipsis">{describeEvent(event)}</span>
      </span>
    </li>
  );
}

export function ActivityView({
  live,
  now,
  onShowGraph,
}: {
  live: LiveData;
  now: number;
  onShowGraph: () => void;
}) {
  const operations = useMemo(() => foldOperations(live.events), [live.events]);
  const [pinned, setPinned] = useState<string | null>(null);
  const operation = operations.find((candidate) => candidate.operationId === pinned) ?? operations[0] ?? null;
  const sessions = live.meta?.sessions ?? [];
  const stream = useMemo(() => live.events.slice(-250).reverse(), [live.events]);

  return (
    <div className="activity-view">
      <div className="column">
        <div className="column-head">
          <span>agentes</span>
          <span className="count">{sessions.length}</span>
        </div>
        {sessions.length === 0 && <p className="dim small pad">Sin sesiones registradas.</p>}
        {sessions.slice(0, 24).map((session) => {
          const state = sessionState(session, now);
          return (
            <div key={session.session_id} className={`session session-${state}`}>
              <div className="session-top">
                <span className="session-client">
                  {state === "active" && <span className="pulse-dot" />}
                  {clientLabel(session.actor.client)}
                </span>
                <span className={`state-badge state-${state}`}>{SESSION_STATE_TEXT[state]}</span>
              </div>
              <div className="session-meta mono dim">
                <span>{relativeTime(session.last_event_at, now)}</span>
                <span>
                  {session.operations} op · {session.events} ev
                </span>
              </div>
              <div className="mono dim ellipsis">{shortId(session.session_id, 22)}</div>
            </div>
          );
        })}
        <div className="column-head">
          <span>operaciones</span>
          <span className="count">{operations.length}</span>
        </div>
        <ul className="op-list">
          {operations.slice(0, 40).map((candidate) => (
            <li
              key={candidate.operationId}
              className={candidate === operation ? "active" : undefined}
              onClick={() => setPinned(candidate.operationId)}
            >
              <span className="ellipsis">{candidate.intent ?? candidate.target ?? candidate.operationId}</span>
              <span className="mono dim">
                {clientLabel(candidate.client)} · {relativeTime(candidate.startedAt, now)}
              </span>
            </li>
          ))}
        </ul>
      </div>

      <div className="column column-main">
        {operation ? (
          <OperationCard operation={operation} now={now} onShowGraph={onShowGraph} />
        ) : (
          <div className="empty-activity">
            <p className="empty-title">Sin operaciones todavía</p>
            <p className="dim small">
              Cada <code>prepare_change</code> abre una operación: su intención, el target, lo que Rationale consideró y
              seleccionó, y lo que la captura escribió al cerrar.
            </p>
          </div>
        )}
      </div>

      <div className="column">
        <div className="column-head">
          <span>stream</span>
          <span className={`conn conn-${live.connection}`}>
            <span className="dot" />
            {live.events.length} eventos
          </span>
        </div>
        <ul className="stream">
          {stream.map((event) => (
            <StreamRow key={`${event.session_id}:${event.seq}`} event={event} now={now} />
          ))}
        </ul>
      </div>
    </div>
  );
}
