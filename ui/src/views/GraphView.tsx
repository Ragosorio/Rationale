import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { EdgePanel, NodePanel, OperationPanel } from "../components/panels";
import { GraphCanvas, type FocusRequest } from "../graph/GraphCanvas";
import { CAUSAL, kindColor, ROLE_COLORS, ROLE_LABELS, STATE_COLORS, STATE_LABELS } from "../graph/palette";
import type { LiveData } from "../live/useLive";
import { foldOperations } from "../model/activity";
import { highlightActive, ROLES, type EdgeState } from "../model/graph";

const LEGEND_STATES: EdgeState[] = ["observed", "indirect", "orphaned", "unknown"];

function Legend() {
  return (
    <div className="legend">
      <div className="legend-row">
        {ROLES.map((role) => (
          <span key={role} className="legend-item">
            <span className="swatch" style={{ background: ROLE_COLORS[role], boxShadow: `0 0 6px ${ROLE_COLORS[role]}` }} />
            {ROLE_LABELS[role]}
          </span>
        ))}
      </div>
      <div className="legend-row">
        <span className="legend-item">
          <span className="swatch halo" />
          explicado por el canon
        </span>
        <span className="legend-item">
          <span className="swatch line" style={{ background: CAUSAL }} />
          relación explicada
        </span>
        {LEGEND_STATES.map((state) => (
          <span key={state} className="legend-item">
            <span
              className="swatch line"
              style={{ background: state === "observed" ? kindColor("calls") : STATE_COLORS[state] }}
            />
            {STATE_LABELS[state]}
          </span>
        ))}
      </div>
    </div>
  );
}

function EmptyGraph({ loading }: { loading: boolean }) {
  return (
    <div className="empty-graph">
      <div>
        <div className="empty-glyph" aria-hidden="true" />
        <p className="empty-title">{loading ? "Leyendo el estado local…" : "Todavía no hay subgrafo de trabajo"}</p>
        {!loading && (
          <p className="dim small">
            Cuando un agente llame <code>prepare_change</code> (o ejecutes <code>rationale prepare &lt;target&gt;</code>),
            <br />
            el vecindario que Rationale le entregue se encenderá aquí.
          </p>
        )}
      </div>
    </div>
  );
}

export function GraphView({
  live,
  now,
  active,
  onOpenActivity,
}: {
  live: LiveData;
  now: number;
  active: boolean;
  onOpenActivity: () => void;
}) {
  const model = live.model;
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [selectedEdge, setSelectedEdge] = useState<string | null>(null);
  const [hovered, setHovered] = useState<string | null>(null);
  const [focus, setFocus] = useState<FocusRequest | null>(null);

  const operations = useMemo(() => foldOperations(live.events), [live.events]);
  const focusOperationId = model?.focusOperation ?? null;
  const liveOperation = operations.find((operation) => operation.operationId === focusOperationId) ?? null;
  const summary =
    live.graph?.operations.find((operation) => operation.operation_id === focusOperationId) ??
    live.graph?.operations[0] ??
    null;

  // Cada operación nueva encuadra su vecindario una sola vez.
  const framed = useRef<string | null>(null);
  useEffect(() => {
    if (!model || !focusOperationId || framed.current === focusOperationId) return;
    framed.current = focusOperationId;
    const keys = model.nodes
      .filter((node) => node.selected && node.operations[0] === focusOperationId)
      .map((node) => node.key);
    setFocus({ keys: model.focusKey ? [model.focusKey, ...keys] : keys, nonce: Date.now() });
  }, [model, focusOperationId]);

  const selectNode = useCallback(
    (key: string | null) => {
      setSelectedEdge(null);
      setSelectedNode(key);
      if (key && model) {
        const neighbors = model.edges
          .filter((edge) => edge.source === key || edge.target === key)
          .flatMap((edge) => [edge.source, edge.target]);
        setFocus({ keys: [key, ...neighbors], nonce: Date.now() });
      }
    },
    [model],
  );
  const selectEdge = useCallback(
    (key: string) => {
      setSelectedNode(null);
      setSelectedEdge(key);
      const edge = model?.edges.find((candidate) => candidate.key === key);
      if (edge) setFocus({ keys: [edge.source, edge.target], nonce: Date.now() });
    },
    [model],
  );
  const clearSelection = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
  }, []);

  const hoveredNode = hovered && model ? (model.byKey.get(hovered) ?? null) : null;
  const explained = model ? model.edges.filter((edge) => edge.recordIds.length > 0).length : 0;
  const atRisk = model ? model.edges.filter((edge) => edge.state === "orphaned" || edge.state === "indirect").length : 0;
  const running = highlightActive(live.highlight, now);
  const target = liveOperation?.qualifiedName ?? summary?.target.qualified_name ?? summary?.target.spec ?? null;

  return (
    <div className="graph-view">
      <div className="graph-stage">
        {model && model.nodes.length > 0 ? (
          <GraphCanvas
            model={model}
            highlight={live.highlight}
            selectedNode={selectedNode}
            selectedEdge={selectedEdge}
            hoveredKey={hovered}
            focus={focus}
            active={active}
            onHover={setHovered}
            onSelectNode={selectNode}
            onSelectEdge={selectEdge}
            onBackground={clearSelection}
          />
        ) : (
          <EmptyGraph loading={!live.graph} />
        )}

        {model && model.nodes.length > 0 && (
          <div className="hud hud-top-left">
            <div className="eyebrow">
              {running ? <span className="live-tag">● operación en curso</span> : "subgrafo de trabajo"}
            </div>
            <div className="hud-title">{liveOperation?.intent ?? summary?.intent ?? "Unión de las operaciones recientes"}</div>
            {target && <div className="mono dim">{target}</div>}
            <div className="hud-metrics">
              <span>
                <b>{model.nodes.length}</b> nodos
              </span>
              <span>
                <b>{model.nodes.filter((node) => node.selected).length}</b> entregados
              </span>
              <span>
                <b>{model.edges.length}</b> relaciones
              </span>
              <span className="causal-text">
                <b>{explained}</b> explicadas
              </span>
              {atRisk > 0 && (
                <span className="warn-text">
                  <b>{atRisk}</b> en riesgo
                </span>
              )}
              <span>
                <b>{live.graph?.operations.length ?? 0}</b> operaciones
              </span>
            </div>
          </div>
        )}

        {model && model.nodes.length > 0 && <Legend />}

        {hoveredNode && (
          <div className="hud hud-hover">
            <div>
              <span className="swatch" style={{ background: ROLE_COLORS[hoveredNode.role] }} />
              <b>{hoveredNode.name}</b>
              <span className="dim">
                {" "}
                · {hoveredNode.label} · {ROLE_LABELS[hoveredNode.role]}
                {hoveredNode.recordIds.length > 0 && <span className="causal-text"> · explicado</span>}
              </span>
            </div>
            <div className="mono dim">
              {hoveredNode.file}
              {hoveredNode.startLine ? `:${hoveredNode.startLine}` : ""}
            </div>
          </div>
        )}
      </div>

      <aside className="side-panel">
        {selectedNode ? (
          <NodePanel
            key={selectedNode}
            nodeKey={selectedNode}
            version={live.version}
            now={now}
            onSelectNode={selectNode}
            onSelectEdge={selectEdge}
            onClose={clearSelection}
          />
        ) : selectedEdge ? (
          <EdgePanel
            key={selectedEdge}
            edgeKey={selectedEdge}
            version={live.version}
            onSelectNode={selectNode}
            onClose={clearSelection}
          />
        ) : (
          <OperationPanel operation={liveOperation} summary={summary} now={now} onOpenActivity={onOpenActivity} />
        )}
      </aside>
    </div>
  );
}
