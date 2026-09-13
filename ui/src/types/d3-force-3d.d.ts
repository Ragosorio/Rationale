// Tipos mínimos de d3-force-3d (el paquete no publica los suyos): solo la
// superficie que usa el layout de la Control Room.
declare module "d3-force-3d" {
  export interface SimNode {
    id: string;
    x?: number;
    y?: number;
    z?: number;
    vx?: number;
    vy?: number;
    vz?: number;
    fx?: number | null;
    fy?: number | null;
    fz?: number | null;
    index?: number;
  }

  export interface SimLink {
    source: string | SimNode;
    target: string | SimNode;
    explained?: boolean;
  }

  export interface LinkForce {
    id(accessor: (node: SimNode) => string): LinkForce;
    distance(distance: number | ((link: SimLink) => number)): LinkForce;
    strength(strength: number | ((link: SimLink) => number)): LinkForce;
  }

  export interface ManyBodyForce {
    strength(strength: number): ManyBodyForce;
    distanceMax(distance: number): ManyBodyForce;
  }

  export interface CenterForce {
    strength(strength: number): CenterForce;
  }

  export interface CollideForce {
    strength(strength: number): CollideForce;
  }

  export interface Simulation {
    force(name: string, force: LinkForce | ManyBodyForce | CenterForce | CollideForce | null): Simulation;
    tick(iterations?: number): Simulation;
    stop(): Simulation;
    alpha(alpha: number): Simulation;
    nodes(): SimNode[];
  }

  export function forceSimulation(nodes?: SimNode[], numDimensions?: number): Simulation;
  export function forceLink(links?: SimLink[]): LinkForce;
  export function forceManyBody(): ManyBodyForce;
  export function forceCenter(x?: number, y?: number, z?: number): CenterForce;
  export function forceCollide(radius?: number): CollideForce;
}
