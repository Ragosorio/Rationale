import { SESSION_STATE_TEXT } from "../components/chrome";
import type { LiveData } from "../live/useLive";
import { clientLabel, sessionState } from "../model/activity";
import { relativeTime, shortId } from "../model/format";

export function SystemView({ live, now }: { live: LiveData; now: number }) {
  const meta = live.meta;
  if (!meta) {
    return (
      <div className="system-view">
        <p className="dim">{live.error ?? "Leyendo el estado local…"}</p>
      </div>
    );
  }
  const lastEvent = live.events[live.events.length - 1];
  return (
    <div className="system-view">
      <div className="system-grid">
        <section className="card">
          <h3 className="card-title">proyecto</h3>
          <dl className="kv">
            <dt>id</dt>
            <dd>{meta.project.id}</dd>
            <dt>raíz</dt>
            <dd className="wrap">{meta.project.root}</dd>
            <dt>revisión</dt>
            <dd className="wrap">{meta.git.revision ?? "sin repositorio Git"}</dd>
            <dt>working tree</dt>
            <dd>{meta.git.dirty ? "con cambios sin commitear" : "limpio"}</dd>
            <dt>rationale</dt>
            <dd>{meta.version}</dd>
          </dl>
        </section>

        <section className="card">
          <h3 className="card-title">canon</h3>
          <dl className="kv">
            <dt>records</dt>
            <dd>{meta.canon.records}</dd>
            <dt>activos</dt>
            <dd>{meta.canon.active}</dd>
            <dt>fijados</dt>
            <dd>{meta.canon.pinned}</dd>
            <dt>reemplazados</dt>
            <dd>{meta.canon.superseded}</dd>
            <dt>revocados</dt>
            <dd>{meta.canon.revoked}</dd>
            <dt>conflictos</dt>
            <dd className={meta.conflicts_pending > 0 ? "danger-text" : undefined}>{meta.conflicts_pending}</dd>
          </dl>
        </section>

        <section className="card">
          <h3 className="card-title">stream</h3>
          <dl className="kv">
            <dt>conexión</dt>
            <dd>{live.connection}</dd>
            <dt>eventos cargados</dt>
            <dd>{live.events.length}</dd>
            <dt>último evento</dt>
            <dd>{lastEvent ? `${lastEvent.kind} · ${relativeTime(lastEvent.timestamp, now)}` : "—"}</dd>
            <dt>última operación</dt>
            <dd className="wrap">{meta.latest_operation ? shortId(meta.latest_operation.operation_id, 26) : "—"}</dd>
          </dl>
        </section>

        <section className="card card-wide">
          <h3 className="card-title">sesiones</h3>
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>cliente</th>
                  <th>estado</th>
                  <th>identidad</th>
                  <th>inicio</th>
                  <th>último evento</th>
                  <th>operaciones</th>
                  <th>sesión</th>
                </tr>
              </thead>
              <tbody>
                {meta.sessions.slice(0, 40).map((session) => {
                  const state = sessionState(session, now);
                  return (
                    <tr key={session.session_id}>
                      <td>{clientLabel(session.actor.client)}</td>
                      <td className={`state-${state}`}>{SESSION_STATE_TEXT[state]}</td>
                      <td className="dim">{session.actor.client_source}</td>
                      <td className="dim">{relativeTime(session.started_at, now)}</td>
                      <td>
                        {session.last_kind} <span className="dim">· {relativeTime(session.last_event_at, now)}</span>
                      </td>
                      <td>{session.operations}</td>
                      <td className="mono dim">{shortId(session.session_id, 20)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </section>

        <section className="card">
          <h3 className="card-title">privacidad</h3>
          <p className="small">
            La actividad y los snapshots de operación viven en <code>.rationale-local/</code>, excluido de Git, y nunca
            salen de esta máquina.
          </p>
          <p className="small dim">
            Guardan identificadores y la intención declarada (una línea, ≤280 caracteres); el contenido de los Records
            viaja por referencia. <code>RATIONALE_ACTIVITY=off</code> desactiva ambos (ADR-0017).
          </p>
        </section>
      </div>
    </div>
  );
}
