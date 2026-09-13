/*
 * Adapted from codebase-memory-mcp graph-ui — src/components/NodeLabels.tsx
 * (canvas-texture sprites with a dark stroke, binary-search text fitting,
 * device-pixel-ratio-aware textures, a hard cap on visible labels).
 * Copyright (c) 2025 DeusData. MIT License — see THIRD_PARTY.md.
 * Rationale picks labels by relevance to the current work (selection, focus,
 * target, explained nodes) and keeps them attached to moving nodes.
 */
import { useFrame } from "@react-three/fiber";
import { useEffect, useMemo, useRef, type RefObject } from "react";
import * as THREE from "three";
import type { Highlight, ModelNode } from "../model/graph";
import type { Vec3 } from "./layout";
import { nodeRadius } from "./NodeCloud";
import { ROLE_COLORS } from "./palette";

const FONT_SIZE = 56;
const FONT = `500 ${FONT_SIZE}px Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif`;
const MAX_TEXT_WIDTH = 640;
const PADDING_X = 20;
const PADDING_Y = 12;
const STROKE = 8;
const MAX_LABELS = 36;

interface LabelTexture {
  texture: THREE.CanvasTexture;
  width: number;
  height: number;
}

function fitText(context: CanvasRenderingContext2D, text: string, maxWidth: number): string {
  if (context.measureText(text).width <= maxWidth) return text;
  let low = 0;
  let high = text.length;
  while (low < high) {
    const middle = Math.ceil((low + high) / 2);
    if (context.measureText(`${text.slice(0, middle)}…`).width <= maxWidth) low = middle;
    else high = middle - 1;
  }
  return `${text.slice(0, Math.max(1, low))}…`;
}

function createLabelTexture(name: string, color: string): LabelTexture | null {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (!context) return null;
  context.font = FONT;
  const text = fitText(context, name, MAX_TEXT_WIDTH);
  const width = Math.max(1, Math.ceil(context.measureText(text).width) + PADDING_X * 2 + STROKE * 2);
  const height = FONT_SIZE + PADDING_Y * 2 + STROKE * 2;
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  canvas.width = Math.ceil(width * ratio);
  canvas.height = Math.ceil(height * ratio);
  context.scale(ratio, ratio);
  context.font = FONT;
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.lineJoin = "round";
  context.lineWidth = STROKE;
  context.strokeStyle = "rgba(2, 4, 8, 0.92)";
  context.fillStyle = color;
  context.strokeText(text, width / 2, height / 2);
  context.fillText(text, width / 2, height / 2);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.minFilter = THREE.LinearFilter;
  texture.magFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;
  return { texture, width, height };
}

function LabelSprite({
  node,
  live,
  strong,
}: {
  node: ModelNode;
  live: RefObject<Map<string, Vec3>>;
  strong: boolean;
}) {
  const sprite = useRef<THREE.Sprite>(null);
  const label = useMemo(() => createLabelTexture(node.name, ROLE_COLORS[node.role]), [node.name, node.role]);
  useEffect(() => () => label?.texture.dispose(), [label]);
  const worldHeight = strong ? 6.4 : 4.6;

  useFrame(() => {
    const position = live.current.get(node.key);
    const target = sprite.current;
    if (!position || !target) return;
    target.position.set(position.x, position.y + nodeRadius(node) * 1.6 + worldHeight * 0.6, position.z);
  });

  if (!label) return null;
  const worldWidth = worldHeight * (label.width / label.height);
  return (
    <sprite ref={sprite} scale={[worldWidth, worldHeight, 1]} renderOrder={20} frustumCulled={false}>
      <spriteMaterial
        map={label.texture}
        transparent
        depthWrite={false}
        depthTest={false}
        toneMapped={false}
        opacity={strong ? 1 : 0.7}
      />
    </sprite>
  );
}

export function NodeLabels({
  nodes,
  live,
  highlight,
  focusKey,
  selectedKey,
  hoveredKey,
}: {
  nodes: ModelNode[];
  live: RefObject<Map<string, Vec3>>;
  highlight: Highlight | null;
  focusKey: string | null;
  selectedKey: string | null;
  hoveredKey: string | null;
}) {
  const chosen = useMemo(() => {
    const priority = (node: ModelNode): number => {
      if (node.key === selectedKey || node.key === hoveredKey) return 0;
      if (node.key === focusKey) return 1;
      if (node.role === "target") return 2;
      if (node.recordIds.length > 0 || node.role === "explained") return 3;
      if (highlight?.selected.has(node.key)) return 4;
      if (node.selected && node.degree > 1) return 5;
      return 9;
    };
    return nodes
      .filter((node) => priority(node) < 9)
      .sort((a, b) => priority(a) - priority(b) || b.degree - a.degree)
      .slice(0, MAX_LABELS);
  }, [nodes, highlight, focusKey, selectedKey, hoveredKey]);

  return (
    <group>
      {chosen.map((node) => (
        <LabelSprite
          key={node.key}
          node={node}
          live={live}
          strong={node.key === selectedKey || node.key === hoveredKey || node.key === focusKey}
        />
      ))}
    </group>
  );
}
