// Layout 3D del subgrafo de trabajo con d3-force-3d, en el navegador.
// Determinista: las semillas salen de la clave del nodo. Las posiciones
// previas se conservan para que el grafo no "salte" cuando llega una
// operación nueva: solo se acomodan los nodos que aparecen.

import {
  forceCenter,
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  type SimLink,
  type SimNode,
} from "d3-force-3d";

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export interface LayoutInput {
  nodes: { key: string }[];
  edges: { source: string; target: string; explained: boolean }[];
}

function hashUnit(text: string, salt: number): number {
  let hash = 2166136261 ^ salt;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return ((hash >>> 0) % 100_000) / 100_000;
}

function seed(key: string, radius: number): Vec3 {
  const theta = hashUnit(key, 1) * Math.PI * 2;
  const phi = Math.acos(2 * hashUnit(key, 2) - 1);
  const r = radius * (0.35 + 0.65 * hashUnit(key, 3));
  return {
    x: r * Math.sin(phi) * Math.cos(theta),
    y: r * Math.sin(phi) * Math.sin(theta),
    z: r * Math.cos(phi),
  };
}

/** Un nodo nuevo nace junto a un vecino ya colocado, si lo tiene. */
function neighborSeed(key: string, input: LayoutInput, previous: Map<string, Vec3>): Vec3 | null {
  for (const edge of input.edges) {
    const other = edge.source === key ? edge.target : edge.target === key ? edge.source : null;
    const placed = other ? previous.get(other) : undefined;
    if (placed) {
      const jitter = seed(key, 16);
      return { x: placed.x + jitter.x, y: placed.y + jitter.y, z: placed.z + jitter.z };
    }
  }
  return null;
}

export function computeLayout(input: LayoutInput, previous: Map<string, Vec3>): Map<string, Vec3> {
  if (input.nodes.length === 0) return new Map();
  const radius = 40 + Math.sqrt(input.nodes.length) * 22;
  const nodes: SimNode[] = input.nodes.map((node) => {
    const start = previous.get(node.key) ?? neighborSeed(node.key, input, previous) ?? seed(node.key, radius);
    return { id: node.key, x: start.x, y: start.y, z: start.z };
  });
  const known = input.nodes.filter((node) => previous.has(node.key)).length;
  const links: SimLink[] = input.edges.map((edge) => ({
    source: edge.source,
    target: edge.target,
    explained: edge.explained,
  }));

  const simulation = forceSimulation(nodes, 3)
    .force(
      "link",
      forceLink(links)
        .id((node) => node.id)
        .distance((link) => (link.explained ? 34 : 54))
        .strength(0.5),
    )
    .force("charge", forceManyBody().strength(-110).distanceMax(700))
    .force("center", forceCenter(0, 0, 0))
    .force("collide", forceCollide(10))
    .stop();

  const warm = known / input.nodes.length > 0.6;
  simulation.alpha(warm ? 0.3 : 1);
  simulation.tick(warm ? 90 : 280);

  return new Map(nodes.map((node) => [node.id, { x: node.x ?? 0, y: node.y ?? 0, z: node.z ?? 0 }]));
}
