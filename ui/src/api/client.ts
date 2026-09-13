import type {
  ActivityEvent,
  ConflictView,
  EdgeDetail,
  GraphResponse,
  Meta,
  NodeDetail,
  OperationSummary,
  RecordView,
} from "./types";

export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function getJson<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(path, { signal, headers: { Accept: "application/json" } });
  const body: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    const message =
      body && typeof body === "object" && "error" in body && typeof body.error === "string"
        ? body.error
        : response.statusText;
    throw new ApiError(response.status, message);
  }
  return body as T;
}

export const api = {
  meta: (signal?: AbortSignal) => getJson<Meta>("/api/meta", signal),
  graph: (options: { operation?: string; recent?: number } = {}, signal?: AbortSignal) => {
    const params = new URLSearchParams();
    if (options.operation) params.set("operation", options.operation);
    if (options.recent) params.set("recent", String(options.recent));
    const query = params.toString();
    return getJson<GraphResponse>(`/api/graph${query ? `?${query}` : ""}`, signal);
  },
  activity: (limit = 400, signal?: AbortSignal) =>
    getJson<ActivityEvent[]>(`/api/activity?limit=${limit}`, signal),
  operations: (limit = 50, signal?: AbortSignal) =>
    getJson<OperationSummary[]>(`/api/operations?limit=${limit}`, signal),
  records: (signal?: AbortSignal) => getJson<RecordView[]>("/api/records", signal),
  conflicts: (signal?: AbortSignal) => getJson<ConflictView[]>("/api/conflicts", signal),
  node: (key: string, signal?: AbortSignal) =>
    getJson<NodeDetail>(`/api/node/${encodeURIComponent(key)}`, signal),
  edge: (key: string, signal?: AbortSignal) =>
    getJson<EdgeDetail>(`/api/edge/${encodeURIComponent(key)}`, signal),
};
