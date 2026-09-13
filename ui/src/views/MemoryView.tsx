import { useEffect, useMemo, useState } from "react";
import { api } from "../api/client";
import type { ConflictView, RecordView } from "../api/types";
import { Pill, Section } from "../components/chrome";
import { provenanceLabel, RecordCard } from "../components/panels";
import type { LiveData } from "../live/useLive";
import { plural, shortName } from "../model/format";

const KINDS = ["all", "constraint", "decision", "risk", "exception"] as const;
const STATUSES = ["active", "superseded", "revoked", "all"] as const;
const KIND_TEXT: Record<string, string> = {
  all: "todos",
  constraint: "constraints",
  decision: "decisiones",
  risk: "riesgos",
  exception: "excepciones",
};
const STATUS_TEXT: Record<string, string> = {
  all: "todos",
  active: "activos",
  superseded: "reemplazados",
  revoked: "revocados",
};
const FRESH_WINDOW_MS = 10 * 60_000;

function Segmented({
  options,
  labels,
  value,
  onChange,
}: {
  options: readonly string[];
  labels: Record<string, string>;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="segmented" role="group">
      {options.map((option) => (
        <button key={option} className={option === value ? "active" : undefined} onClick={() => onChange(option)}>
          {labels[option] ?? option}
        </button>
      ))}
    </div>
  );
}

function RecordDetail({ record }: { record: RecordView }) {
  return (
    <div className="panel">
      <header className="panel-head">
        <span className="eyebrow">
          {record.kind} · severidad {record.severity}
        </span>
        <h2 className="title mono wrap">{record.id}</h2>
      </header>
      <div className="section">
        <RecordCard record={record} />
      </div>
      <Section title="Gobierna" aside={<span className="count">{record.bindings.length}</span>}>
        {record.bindings.length === 0 ? (
          <p className="dim small">Sin bindings de código.</p>
        ) : (
          <ul className="plain-list">
            {record.bindings.map((binding, index) => (
              <li key={`${binding.path}-${binding.structural_id}-${index}`} className="mono small">
                <span className="wrap">{binding.path ?? "—"}</span>
                {binding.structural_id && <span className="dim wrap"> · {shortName(binding.structural_id)}</span>}
                {binding.provisional && <span className="warn-text"> · provisional</span>}
              </li>
            ))}
          </ul>
        )}
      </Section>
      <Section title="Relaciones que explica" aside={<span className="count">{record.relationships.length}</span>}>
        {record.relationships.length === 0 ? (
          <p className="dim small">No explica relaciones entre nodos.</p>
        ) : (
          <ul className="plain-list">
            {record.relationships.map((relationship) => (
              <li key={relationship.key}>
                <span className="mono">{shortName(relationship.source)}</span>
                <span className="edge-kind"> —{relationship.kind}→ </span>
                <span className="mono">{shortName(relationship.target)}</span>
                <div className="mono dim small wrap">
                  {relationship.source_file} → {relationship.target_file}
                </div>
              </li>
            ))}
          </ul>
        )}
      </Section>
      {(record.supersedes.length > 0 || record.superseded_by) && (
        <Section title="Linaje">
          {record.supersedes.length > 0 && (
            <p className="mono small">
              reemplaza a <span className="causal-text">{record.supersedes.join(", ")}</span>
            </p>
          )}
          {record.superseded_by && (
            <p className="mono small">
              reemplazado por <span className="warn-text">{record.superseded_by}</span>
            </p>
          )}
        </Section>
      )}
    </div>
  );
}

export function MemoryView({ live }: { live: LiveData }) {
  const [records, setRecords] = useState<RecordView[] | null>(null);
  const [conflicts, setConflicts] = useState<ConflictView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<string>("all");
  const [status, setStatus] = useState<string>("active");
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([api.records(controller.signal), api.conflicts(controller.signal)])
      .then(([nextRecords, nextConflicts]) => {
        setRecords(nextRecords);
        setConflicts(nextConflicts);
        setError(null);
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
      });
    return () => controller.abort();
  }, [live.version]);

  const fresh = useMemo(() => {
    const cutoff = Date.now() - FRESH_WINDOW_MS;
    return new Set(
      live.events
        .filter((event) => event.kind === "record.committed" && Date.parse(event.timestamp) > cutoff)
        .map((event) => String(event.payload.record_id ?? "")),
    );
  }, [live.events]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (records ?? [])
      .filter(
        (record) =>
          (kind === "all" || record.kind === kind) &&
          (status === "all" || record.status === status) &&
          (!pinnedOnly || record.authority === "pinned") &&
          (!needle || `${record.id} ${record.statement} ${record.rationale ?? ""}`.toLowerCase().includes(needle)),
      )
      .sort(
        (a, b) =>
          Number(b.authority === "pinned") - Number(a.authority === "pinned") ||
          Number(fresh.has(b.id)) - Number(fresh.has(a.id)) ||
          a.id.localeCompare(b.id),
      );
  }, [records, query, kind, status, pinnedOnly, fresh]);

  const current = (records ?? []).find((record) => record.id === selected) ?? null;

  return (
    <div className="memory-view">
      <div className="memory-main">
        <div className="toolbar">
          <input
            className="input"
            type="search"
            placeholder="buscar en ids, statements y rationale…"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <Segmented options={KINDS} labels={KIND_TEXT} value={kind} onChange={setKind} />
          <Segmented options={STATUSES} labels={STATUS_TEXT} value={status} onChange={setStatus} />
          <label className="toggle">
            <input type="checkbox" checked={pinnedOnly} onChange={(event) => setPinnedOnly(event.target.checked)} />
            fijados
          </label>
          <span className="count">
            {filtered.length} / {records?.length ?? 0}
          </span>
        </div>

        {conflicts.length > 0 && (
          <div className="conflicts">
            {conflicts.map((conflict) => (
              <article key={conflict.conflict_id} className="conflict">
                <header>
                  <Pill tone="danger">conflicto con una regla fijada</Pill>
                  <span className="mono dim">{conflict.conflict_id}</span>
                </header>
                <div className="conflict-body">
                  <div>
                    <span className="label">fijada · {conflict.pinned_record_id}</span>
                    <p>{conflict.pinned_statement}</p>
                  </div>
                  <div>
                    <span className="label">nueva afirmación</span>
                    <p>{conflict.candidate_statement}</p>
                  </div>
                </div>
                <p className="small dim">
                  El agente debe preguntarte cuál gobierna y aplicar tu respuesta con <code>resolve_conflict</code>.
                  Desde la terminal: <code>rationale resolve {conflict.conflict_id} keep-pinned</code> o{" "}
                  <code>adopt-new</code>.
                </p>
              </article>
            ))}
          </div>
        )}

        {error && <p className="danger-text pad">{error}</p>}
        <div className="record-list">
          {records === null && !error && <p className="dim pad">Leyendo el canon…</p>}
          {records !== null && filtered.length === 0 && <p className="dim pad">Ningún Record coincide con el filtro.</p>}
          {filtered.map((record) => (
            <button
              key={record.id}
              className={`record-row${record.id === selected ? " active" : ""}${fresh.has(record.id) ? " fresh" : ""}`}
              onClick={() => setSelected(record.id)}
            >
              <span className="record-row-top">
                <Pill tone="causal">{record.kind}</Pill>
                {record.authority === "pinned" && <Pill tone="accent">fijado</Pill>}
                {record.status !== "active" && <Pill tone="warn">{STATUS_TEXT[record.status] ?? record.status}</Pill>}
                {fresh.has(record.id) && <Pill tone="ok">nuevo</Pill>}
                <span className="mono dim ellipsis">{record.id}</span>
              </span>
              <span className="record-row-statement">{record.statement}</span>
              <span className="mono dim small">
                {provenanceLabel(record.provenance)} · {record.bindings.length}{" "}
                {plural(record.bindings.length, "binding", "bindings")} · {record.relationships.length}{" "}
                {plural(record.relationships.length, "relación", "relaciones")}
              </span>
            </button>
          ))}
        </div>
      </div>
      <aside className="side-panel">
        {current ? (
          <RecordDetail record={current} />
        ) : (
          <div className="panel">
            <header className="panel-head">
              <span className="eyebrow">memoria causal</span>
              <p className="dim small">
                Selecciona un Record para ver su porqué, el código que gobierna y las relaciones que explica.
              </p>
            </header>
          </div>
        )}
      </aside>
    </div>
  );
}
