/*
 * Adapted from codebase-memory-mcp graph-ui — src/components/EdgeLines.tsx
 * (additively blended line segments tinted per edge; a selection stays bright
 * while everything else dims) and the soft point sprite from NodeCloud.tsx.
 * Copyright (c) 2025 DeusData. MIT License — see THIRD_PARTY.md.
 * Rationale colours relationships by structural state and causal overlay,
 * follows live positions, lets relationships be clicked, and sends pulses
 * along the relationships an agent just received or the canon explains.
 */
import { useFrame, type ThreeEvent } from "@react-three/fiber";
import { useEffect, useMemo, type RefObject } from "react";
import * as THREE from "three";
import { highlightActive, type Highlight, type ModelEdge } from "../model/graph";
import type { Vec3 } from "./layout";
import { CAUSAL, edgeColor } from "./palette";

interface EdgeLinesProps {
  edges: ModelEdge[];
  live: RefObject<Map<string, Vec3>>;
  highlight: Highlight | null;
  selectedEdge: string | null;
  focusNode: string | null;
  hoveredKey: string | null;
  onSelect: (key: string) => void;
}

export function EdgeLines({
  edges,
  live,
  highlight,
  selectedEdge,
  focusNode,
  hoveredKey,
  onSelect,
}: EdgeLinesProps) {
  const geometry = useMemo(() => {
    const capacity = Math.max(1, edges.length) * 6;
    const buffer = new THREE.BufferGeometry();
    buffer.setAttribute("position", new THREE.BufferAttribute(new Float32Array(capacity), 3));
    buffer.setAttribute("color", new THREE.BufferAttribute(new Float32Array(capacity), 3));
    buffer.setDrawRange(0, edges.length * 2);
    return buffer;
  }, [edges]);
  useEffect(() => () => geometry.dispose(), [geometry]);

  const base = useMemo(
    () => edges.map((edge) => new THREE.Color(edgeColor(edge.kind, edge.state, edge.recordIds.length > 0))),
    [edges],
  );

  useFrame(({ clock }) => {
    const positions = geometry.getAttribute("position") as THREE.BufferAttribute;
    const colors = geometry.getAttribute("color") as THREE.BufferAttribute;
    const time = clock.elapsedTime;
    const active = highlightActive(highlight, Date.now()) ? highlight : null;
    const touches = (edge: ModelEdge, key: string | null) =>
      key !== null && (edge.source === key || edge.target === key);

    edges.forEach((edge, index) => {
      const source = live.current.get(edge.source);
      const target = live.current.get(edge.target);
      if (!source || !target) {
        positions.setXYZ(index * 2, 0, 0, 0);
        positions.setXYZ(index * 2 + 1, 0, 0, 0);
        return;
      }
      positions.setXYZ(index * 2, source.x, source.y, source.z);
      positions.setXYZ(index * 2 + 1, target.x, target.y, target.z);

      let intensity = edge.recordIds.length > 0 ? 0.95 : 0.34;
      if (edge.state === "orphaned" || edge.state === "indirect") intensity = 0.9;
      if (!edge.selected) intensity *= 0.45;
      if (active) {
        if (active.edges.has(edge.key)) intensity = 1.45 + 0.35 * Math.sin(time * 5 + index);
        else if (active.phase !== "requested") intensity *= 0.35;
      }
      if (touches(edge, hoveredKey) || touches(edge, focusNode)) intensity = Math.max(intensity, 1.1);
      if (edge.key === selectedEdge) intensity = 2.4;

      const color = base[index];
      colors.setXYZ(index * 2, color.r * intensity, color.g * intensity, color.b * intensity);
      colors.setXYZ(index * 2 + 1, color.r * intensity, color.g * intensity, color.b * intensity);
    });
    positions.needsUpdate = true;
    colors.needsUpdate = true;
    geometry.computeBoundingSphere();
  });

  if (edges.length === 0) return null;

  return (
    <lineSegments
      geometry={geometry}
      frustumCulled={false}
      onClick={(event: ThreeEvent<MouseEvent>) => {
        event.stopPropagation();
        if (event.index === undefined) return;
        // En LineSegments, `index` es el primer vértice del segmento.
        const edge = edges[Math.floor(event.index / 2)];
        if (edge) onSelect(edge.key);
      }}
    >
      <lineBasicMaterial
        vertexColors
        transparent
        depthWrite={false}
        blending={THREE.AdditiveBlending}
        toneMapped={false}
      />
    </lineSegments>
  );
}

let pulseSprite: THREE.CanvasTexture | null = null;

function getPulseSprite(): THREE.CanvasTexture {
  if (pulseSprite) return pulseSprite;
  const size = 64;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const context = canvas.getContext("2d");
  if (context) {
    const gradient = context.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
    gradient.addColorStop(0, "rgba(255,255,255,1)");
    gradient.addColorStop(0.45, "rgba(255,255,255,0.85)");
    gradient.addColorStop(1, "rgba(255,255,255,0)");
    context.fillStyle = gradient;
    context.fillRect(0, 0, size, size);
  }
  pulseSprite = new THREE.CanvasTexture(canvas);
  return pulseSprite;
}

function phaseOffset(key: string): number {
  let hash = 0;
  for (let index = 0; index < key.length; index += 1) hash = (hash * 31 + key.charCodeAt(index)) | 0;
  return ((hash >>> 0) % 1000) / 1000;
}

const PULSE_CAPACITY = 256;

/** Pulsos que recorren las relaciones vivas y las que el canon explica. */
export function EdgePulses({
  edges,
  live,
  highlight,
}: {
  edges: ModelEdge[];
  live: RefObject<Map<string, Vec3>>;
  highlight: Highlight | null;
}) {
  const geometry = useMemo(() => {
    const buffer = new THREE.BufferGeometry();
    buffer.setAttribute("position", new THREE.BufferAttribute(new Float32Array(PULSE_CAPACITY * 3), 3));
    buffer.setAttribute("color", new THREE.BufferAttribute(new Float32Array(PULSE_CAPACITY * 3), 3));
    buffer.setDrawRange(0, 0);
    return buffer;
  }, []);
  useEffect(() => () => geometry.dispose(), [geometry]);
  const colors = useMemo(() => ({ hot: new THREE.Color("#bfe7ff"), causal: new THREE.Color(CAUSAL) }), []);

  useFrame(({ clock }) => {
    const positions = geometry.getAttribute("position") as THREE.BufferAttribute;
    const tints = geometry.getAttribute("color") as THREE.BufferAttribute;
    const active = highlightActive(highlight, Date.now()) ? highlight : null;
    let count = 0;
    for (const edge of edges) {
      if (count >= PULSE_CAPACITY) break;
      const hot = active?.edges.has(edge.key) ?? false;
      if (!hot && edge.recordIds.length === 0) continue;
      const source = live.current.get(edge.source);
      const target = live.current.get(edge.target);
      if (!source || !target) continue;
      const speed = hot ? 0.6 : 0.16;
      const t = (clock.elapsedTime * speed + phaseOffset(edge.key)) % 1;
      positions.setXYZ(
        count,
        source.x + (target.x - source.x) * t,
        source.y + (target.y - source.y) * t,
        source.z + (target.z - source.z) * t,
      );
      const color = hot ? colors.hot : colors.causal;
      const strength = hot ? 2.2 : 1.1;
      tints.setXYZ(count, color.r * strength, color.g * strength, color.b * strength);
      count += 1;
    }
    geometry.setDrawRange(0, count);
    positions.needsUpdate = true;
    tints.needsUpdate = true;
  });

  return (
    <points geometry={geometry} raycast={() => null} frustumCulled={false}>
      <pointsMaterial
        size={5}
        map={getPulseSprite()}
        vertexColors
        transparent
        depthWrite={false}
        blending={THREE.AdditiveBlending}
        toneMapped={false}
        sizeAttenuation
      />
    </points>
  );
}
