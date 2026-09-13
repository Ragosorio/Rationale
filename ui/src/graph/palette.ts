// Paleta de la Control Room. La codificación de aristas por tipo y el
// fondo profundo siguen la capa de render de codebase-memory-mcp
// (graph-ui/src/components/EdgeLines.tsx, graph-ui/src/lib/colors.ts;
// Copyright (c) 2025 DeusData, MIT — ver THIRD_PARTY.md), reinterpretada
// para roles de contexto y memoria causal.

import type { EdgeState, Role } from "../model/graph";

export const BACKGROUND = "#04060a";

/** Ámbar: todo lo que el canon explica (memoria causal). */
export const CAUSAL = "#f2b544";
export const WARNING = "#f59e0b";
export const DANGER = "#f0575f";
export const MUTED = "#5b6678";

export const ROLE_COLORS: Record<Role, string> = {
  target: "#e6f6ff",
  explained: CAUSAL,
  caller: "#58a6ff",
  callee: "#3dd6c6",
  dependency: "#a391ff",
  dependent: "#7383ff",
  test: "#6fe3a1",
  context: "#7c8799",
};

export const ROLE_LABELS: Record<Role, string> = {
  target: "target",
  explained: "extremo explicado",
  caller: "caller",
  callee: "callee",
  dependency: "dependencia",
  dependent: "dependiente",
  test: "test",
  context: "contexto",
};

const KIND_COLORS: Record<string, string> = {
  calls: "#27a693",
  uses: "#8a7cf6",
  writes: "#e8718c",
  imports: "#4d8cf6",
  tests: "#53c987",
  defines: "#a78bfa",
  implements: "#f59e0b",
  configures: "#d9b23a",
  depends_on: "#5fa3f6",
  http_calls: "#f0708a",
};

export const STATE_COLORS: Record<EdgeState, string> = {
  observed: "#27a693",
  indirect: WARNING,
  orphaned: DANGER,
  unknown: MUTED,
};

export const STATE_LABELS: Record<EdgeState, string> = {
  observed: "observada",
  indirect: "indirecta",
  orphaned: "huérfana",
  unknown: "desconocida",
};

export function kindColor(kind: string): string {
  return KIND_COLORS[kind] ?? "#2f8c8c";
}

/** El estado estructural manda sobre el tipo: una explicación en riesgo se ve. */
export function edgeColor(kind: string, state: EdgeState, explained: boolean): string {
  if (state !== "observed") return STATE_COLORS[state];
  return explained ? CAUSAL : kindColor(kind);
}
