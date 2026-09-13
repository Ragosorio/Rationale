/*
 * Adapted from codebase-memory-mcp graph-ui — src/components/NodeCloud.tsx
 * (instanced spheres; per-instance colour pushed above 1.0 so bloom turns the
 * excess into a glow corona; dimming everything outside a selection).
 * Copyright (c) 2025 DeusData. MIT License — see THIRD_PARTY.md.
 * Rationale adds eased motion toward the layout, a birth ramp for nodes that
 * just joined the working set, emphasis driven by live activity, a focus pulse
 * and an amber halo for every node the canon explains.
 */
import { useFrame, type ThreeEvent } from "@react-three/fiber";
import { useLayoutEffect, useMemo, useRef, type RefObject } from "react";
import * as THREE from "three";
import { nodeEmphasis, type Highlight, type ModelNode, type NodeEmphasis } from "../model/graph";
import type { Vec3 } from "./layout";
import { CAUSAL, ROLE_COLORS } from "./palette";

export interface NodeCloudProps {
  nodes: ModelNode[];
  live: RefObject<Map<string, Vec3>>;
  highlight: Highlight | null;
  selectedKey: string | null;
  hoveredKey: string | null;
  onHover: (key: string | null) => void;
  onSelect: (key: string) => void;
}

const GLOW: Record<NodeEmphasis, number> = {
  focus: 2.9,
  selected: 1.95,
  considered: 1.05,
  dim: 0.24,
  normal: 1.3,
};

const SCALE: Record<NodeEmphasis, number> = {
  focus: 1.55,
  selected: 1.18,
  considered: 1,
  dim: 0.72,
  normal: 1,
};

export function nodeRadius(node: ModelNode): number {
  const base = 2.4 + Math.min(4.2, Math.sqrt(node.degree) * 0.95);
  return node.role === "target" ? base * 1.5 : base;
}

function initializeColors(mesh: THREE.InstancedMesh | null, color: THREE.Color) {
  if (!mesh) return;
  // `setColorAt` crea el atributo por instancia; el material debe recompilarse
  // sabiendo que existe o ignoraría los colores.
  for (let index = 0; index < mesh.count; index += 1) {
    mesh.setColorAt(index, color.set("#ffffff"));
  }
  if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
  (mesh.material as THREE.Material).needsUpdate = true;
}

export function NodeCloud({
  nodes,
  live,
  highlight,
  selectedKey,
  hoveredKey,
  onHover,
  onSelect,
}: NodeCloudProps) {
  const mesh = useRef<THREE.InstancedMesh>(null);
  const halo = useRef<THREE.InstancedMesh>(null);
  const born = useRef(new Map<string, number>());
  const explained = useMemo(() => nodes.filter((node) => node.recordIds.length > 0), [nodes]);
  const temp = useMemo(() => ({ object: new THREE.Object3D(), color: new THREE.Color() }), []);

  useLayoutEffect(() => {
    initializeColors(mesh.current, temp.color);
    initializeColors(halo.current, temp.color);
  }, [nodes, explained, temp]);

  useFrame(({ clock }) => {
    const time = clock.elapsedTime;
    const now = Date.now();
    const cloud = mesh.current;
    if (cloud) {
      nodes.forEach((node, index) => {
        const position = live.current.get(node.key);
        if (!position) return;
        if (!born.current.has(node.key)) born.current.set(node.key, time);
        const age = Math.min(1, (time - (born.current.get(node.key) ?? time)) / 0.7);
        const emphasis = nodeEmphasis(node.key, highlight, now);

        let scale = nodeRadius(node) * SCALE[emphasis] * (1 - Math.pow(1 - age, 3));
        if (emphasis === "focus") scale *= 1 + 0.12 * Math.sin(time * 4.2);
        if (node.key === selectedKey) scale *= 1.35;
        if (node.key === hoveredKey) scale *= 1.2;
        temp.object.position.set(position.x, position.y, position.z);
        temp.object.scale.setScalar(Math.max(0.001, scale));
        temp.object.updateMatrix();
        cloud.setMatrixAt(index, temp.object.matrix);

        let glow = GLOW[emphasis] * (node.selected ? 1 : 0.55);
        if (node.key === selectedKey) glow = Math.max(glow, 2.5);
        if (node.key === hoveredKey) glow += 0.5;
        temp.color.set(ROLE_COLORS[node.role]).multiplyScalar(glow);
        cloud.setColorAt(index, temp.color);
      });
      cloud.instanceMatrix.needsUpdate = true;
      if (cloud.instanceColor) cloud.instanceColor.needsUpdate = true;
      cloud.computeBoundingSphere();
    }

    const ring = halo.current;
    if (ring) {
      explained.forEach((node, index) => {
        const position = live.current.get(node.key);
        if (!position) return;
        const breathe = 1 + 0.08 * Math.sin(time * 1.6 + index);
        temp.object.position.set(position.x, position.y, position.z);
        temp.object.scale.setScalar(nodeRadius(node) * 2.3 * breathe);
        temp.object.updateMatrix();
        ring.setMatrixAt(index, temp.object.matrix);
        const fresh = highlight !== null && node.recordIds.some((id) => highlight.records.has(id));
        temp.color.set(CAUSAL).multiplyScalar(fresh ? 0.6 : 0.26);
        ring.setColorAt(index, temp.color);
      });
      ring.instanceMatrix.needsUpdate = true;
      if (ring.instanceColor) ring.instanceColor.needsUpdate = true;
    }
  });

  if (nodes.length === 0) return null;

  return (
    <group>
      <instancedMesh
        key={`nodes-${nodes.length}`}
        ref={mesh}
        args={[undefined, undefined, nodes.length]}
        frustumCulled={false}
        onPointerMove={(event: ThreeEvent<PointerEvent>) => {
          event.stopPropagation();
          const node = event.instanceId !== undefined ? nodes[event.instanceId] : undefined;
          onHover(node ? node.key : null);
        }}
        onPointerOut={() => onHover(null)}
        onClick={(event: ThreeEvent<MouseEvent>) => {
          event.stopPropagation();
          const node = event.instanceId !== undefined ? nodes[event.instanceId] : undefined;
          if (node) onSelect(node.key);
        }}
      >
        <sphereGeometry args={[1, 28, 20]} />
        <meshBasicMaterial toneMapped={false} />
      </instancedMesh>
      {explained.length > 0 && (
        <instancedMesh
          key={`halo-${explained.length}`}
          ref={halo}
          args={[undefined, undefined, explained.length]}
          frustumCulled={false}
          raycast={() => null}
        >
          <sphereGeometry args={[1, 24, 16]} />
          <meshBasicMaterial
            toneMapped={false}
            transparent
            depthWrite={false}
            blending={THREE.AdditiveBlending}
          />
        </instancedMesh>
      )}
    </group>
  );
}
