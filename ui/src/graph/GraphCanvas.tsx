import { Canvas } from "@react-three/fiber";
import { memo, useMemo, useRef } from "react";
import * as THREE from "three";
import type { GraphModel, Highlight } from "../model/graph";
import { EdgeLines, EdgePulses } from "./EdgeLines";
import { computeLayout, type Vec3 } from "./layout";
import { NodeCloud } from "./NodeCloud";
import { NodeLabels } from "./NodeLabels";
import { BACKGROUND } from "./palette";
import { Bloom, CameraRig, Motion, RaycastTuning, Starfield, type CameraTarget } from "./Stage";

export interface FocusRequest {
  keys: string[];
  nonce: number;
}

export interface GraphCanvasProps {
  model: GraphModel;
  highlight: Highlight | null;
  selectedNode: string | null;
  selectedEdge: string | null;
  hoveredKey: string | null;
  focus: FocusRequest | null;
  active: boolean;
  onHover: (key: string | null) => void;
  onSelectNode: (key: string) => void;
  onSelectEdge: (key: string) => void;
  onBackground: () => void;
}

function frameKeys(keys: string[], positions: Map<string, Vec3>): CameraTarget | null {
  let points = keys.map((key) => positions.get(key)).filter((point): point is Vec3 => point !== undefined);
  if (points.length === 0) points = [...positions.values()];
  if (points.length === 0) return null;
  const center = points.reduce(
    (sum, point) => ({
      x: sum.x + point.x / points.length,
      y: sum.y + point.y / points.length,
      z: sum.z + point.z / points.length,
    }),
    { x: 0, y: 0, z: 0 },
  );
  const spread = Math.max(
    0,
    ...points.map((point) => Math.hypot(point.x - center.x, point.y - center.y, point.z - center.z)),
  );
  const distance = Math.max(170, spread * 2.8);
  return {
    lookAt: new THREE.Vector3(center.x, center.y, center.z),
    position: new THREE.Vector3(center.x + distance * 0.28, center.y + distance * 0.22, center.z + distance),
  };
}

export const GraphCanvas = memo(function GraphCanvas(props: GraphCanvasProps) {
  const { model, highlight, selectedNode, selectedEdge, hoveredKey, focus, active } = props;
  const previous = useRef(new Map<string, Vec3>());
  const live = useRef(new Map<string, Vec3>());

  // El layout solo se recalcula cuando cambia la estructura, no el resaltado.
  const signature = `${model.nodes.map((node) => node.key).join(",")}|${model.edges.map((edge) => edge.key).join(",")}`;
  const positions = useMemo(() => {
    const next = computeLayout(
      {
        nodes: model.nodes,
        edges: model.edges.map((edge) => ({
          source: edge.source,
          target: edge.target,
          explained: edge.recordIds.length > 0,
        })),
      },
      previous.current,
    );
    previous.current = next;
    return next;
  }, [signature]); // eslint-disable-line react-hooks/exhaustive-deps

  const cameraTarget = useMemo(() => (focus ? frameKeys(focus.keys, positions) : null), [focus, positions]);
  const focusNode = highlight?.target ?? model.focusKey;

  return (
    <Canvas
      flat
      frameloop={active ? "always" : "never"}
      dpr={[1, 1.75]}
      camera={{ position: [0, 90, 640], fov: 45, near: 0.5, far: 20000 }}
      gl={{ antialias: true, powerPreference: "high-performance" }}
      onPointerMissed={props.onBackground}
    >
      <color attach="background" args={[BACKGROUND]} />
      <fog attach="fog" args={[BACKGROUND, 900, 2800]} />
      <Motion targets={positions} live={live} />
      <RaycastTuning />
      <Starfield />
      <EdgeLines
        edges={model.edges}
        live={live}
        highlight={highlight}
        selectedEdge={selectedEdge}
        focusNode={focusNode}
        hoveredKey={hoveredKey ?? selectedNode}
        onSelect={props.onSelectEdge}
      />
      <EdgePulses edges={model.edges} live={live} highlight={highlight} />
      <NodeCloud
        nodes={model.nodes}
        live={live}
        highlight={highlight}
        selectedKey={selectedNode}
        hoveredKey={hoveredKey}
        onHover={props.onHover}
        onSelect={props.onSelectNode}
      />
      <NodeLabels
        nodes={model.nodes}
        live={live}
        highlight={highlight}
        focusKey={focusNode}
        selectedKey={selectedNode}
        hoveredKey={hoveredKey}
      />
      <CameraRig target={cameraTarget} />
      <Bloom />
    </Canvas>
  );
});
