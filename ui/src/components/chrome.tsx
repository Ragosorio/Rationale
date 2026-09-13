import type { ReactNode } from "react";
import type { ActivityEvent, Meta } from "../api/types";
import type { Connection } from "../live/useLive";
import { clientLabel, describeEvent, sessionState, type SessionState } from "../model/activity";
import { clockTime } from "../model/format";

export type Tab = "graph" | "activity" | "memory" | "system";

const TABS: { id: Tab; label: string; key: string }[] = [
  { id: "graph", label: "Grafo", key: "G" },
  { id: "activity", label: "Actividad", key: "A" },
  { id: "memory", label: "Memoria", key: "M" },
  { id: "system", label: "Sistema", key: "S" },
];

const CONNECTION_TEXT: Record<Connection, string> = {
  live: "en vivo",
  connecting: "conectando",
  offline: "sin conexión",
};

export const SESSION_STATE_TEXT: Record<SessionState, string> = {
  active: "activo",
  idle: "inactivo",
  ended: "terminado",
};

export function TopBar({
  meta,
  tab,
  onTab,
  connection,
  now,
}: {
  meta: Meta | null;
  tab: Tab;
  onTab: (tab: Tab) => void;
  connection: Connection;
  now: number;
}) {
  const agents = (meta?.sessions ?? []).filter(
    (session) => session.actor.type === "agent" && sessionState(session, now) === "active",
  );
  return (
    <header className="topbar">
      <div className="brand">
        <span className="glyph" aria-hidden="true" />
        <span className="wordmark">RATIONALE</span>
        <span className="brand-sub">control room</span>
      </div>
      <div className="project mono">
        <span>{meta?.project.id ?? "…"}</span>
        <span className="sep">/</span>
        <span>{meta?.git.revision ? meta.git.revision.slice(0, 7) : "sin git"}</span>
        {meta?.git.dirty && (
          <span className="dirty" title="working tree con cambios sin commitear">
            ●
          </span>
        )}
      </div>
      <nav className="tabs" role="tablist" aria-label="Vistas">
        {TABS.map((item) => (
          <button
            key={item.id}
            role="tab"
            aria-selected={tab === item.id}
            className={tab === item.id ? "tab active" : "tab"}
            onClick={() => onTab(item.id)}
            title={`${item.label} (${item.key})`}
          >
            {item.label}
          </button>
        ))}
      </nav>
      <div className="topbar-right">
        {meta && meta.conflicts_pending > 0 && (
          <span className="pill danger">
            {meta.conflicts_pending} conflicto{meta.conflicts_pending === 1 ? "" : "s"} con reglas fijadas
          </span>
        )}
        {meta && (
          <span className="canon-count mono" title="Records activos y fijados del canon">
            <b>{meta.canon.active}</b> records
            {meta.canon.pinned > 0 && (
              <>
                {" · "}
                <b>{meta.canon.pinned}</b> fijados
              </>
            )}
          </span>
        )}
        <div className="agents">
          {agents.length === 0 ? (
            <span className="dim small">sin agentes activos</span>
          ) : (
            agents.slice(0, 3).map((agent) => (
              <span key={agent.session_id} className="agent-chip">
                <span className="pulse-dot" />
                {clientLabel(agent.actor.client)}
              </span>
            ))
          )}
        </div>
        <span className={`conn conn-${connection}`} title="stream de actividad">
          <span className="dot" />
          {CONNECTION_TEXT[connection]}
        </span>
      </div>
    </header>
  );
}

export function kindFamily(kind: string): string {
  return `k-${kind.split(".")[0]}`;
}

export function Ticker({ events, onOpen }: { events: ActivityEvent[]; onOpen: () => void }) {
  const recent = events.slice(-8).reverse();
  return (
    <footer
      className="ticker"
      onClick={onOpen}
      onKeyDown={(event) => {
        if (event.key === "Enter") onOpen();
      }}
      role="button"
      tabIndex={0}
      aria-label="Abrir la vista de actividad"
    >
      <span className="ticker-label">actividad</span>
      <div className="ticker-items">
        {recent.length === 0 ? (
          <span className="dim">esperando eventos…</span>
        ) : (
          recent.map((event) => (
            <span key={`${event.session_id}:${event.seq}`} className="ticker-item">
              <span className="mono dim">{clockTime(event.timestamp)}</span>
              <span className={`mono ${kindFamily(event.kind)}`}>{event.kind}</span>
              <span className="dim ellipsis">{describeEvent(event)}</span>
            </span>
          ))
        )}
      </div>
    </footer>
  );
}

export function Stat({ label, value, sub }: { label: string; value: ReactNode; sub?: ReactNode }) {
  return (
    <div className="stat">
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
      {sub !== undefined && <span className="stat-sub">{sub}</span>}
    </div>
  );
}

export function Section({
  title,
  aside,
  children,
}: {
  title: string;
  aside?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="section">
      <header className="section-head">
        <h3>{title}</h3>
        {aside}
      </header>
      {children}
    </section>
  );
}

export function Pill({ tone = "neutral", children }: { tone?: string; children: ReactNode }) {
  return <span className={`pill ${tone}`}>{children}</span>;
}
