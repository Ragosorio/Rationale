// Estado vivo de la Control Room: instantáneas REST + stream SSE de
// actividad. El stream no repite historia (empieza al final), así que al
// reconectar se recargan las instantáneas para no perder eventos.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api/client";
import type { ActivityEvent, GraphResponse, Meta } from "../api/types";
import { buildGraphModel, nextHighlight, type GraphModel, type Highlight } from "../model/graph";

export type Connection = "connecting" | "live" | "offline";

export interface LiveData {
  meta: Meta | null;
  graph: GraphResponse | null;
  model: GraphModel | null;
  events: ActivityEvent[];
  highlight: Highlight | null;
  connection: Connection;
  error: string | null;
  /** Aumenta con cada recarga de instantáneas: los paneles de detalle se refrescan. */
  version: number;
}

const MAX_EVENTS = 1500;
const GRAPH_KINDS = new Set([
  "packet.compiled",
  "record.committed",
  "record.superseded",
  "change.finalized",
  "conflict.detected",
  "conflict.resolved",
]);
const META_KINDS = new Set(["session.started", "agent.connected", "session.ended"]);

interface Parts {
  meta: boolean;
  graph: boolean;
  activity: boolean;
}

const eventId = (event: ActivityEvent) => `${event.session_id}:${event.seq}`;

function compareEvents(a: ActivityEvent, b: ActivityEvent): number {
  return a.timestamp.localeCompare(b.timestamp) || a.session_id.localeCompare(b.session_id) || a.seq - b.seq;
}

export function useLive(): LiveData {
  const [meta, setMeta] = useState<Meta | null>(null);
  const [graph, setGraph] = useState<GraphResponse | null>(null);
  const [events, setEvents] = useState<ActivityEvent[]>([]);
  const [highlight, setHighlight] = useState<Highlight | null>(null);
  const [connection, setConnection] = useState<Connection>("connecting");
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const seen = useRef(new Set<string>());
  const pending = useRef<Parts>({ meta: false, graph: false, activity: false });
  const timer = useRef<number | null>(null);

  const merge = useCallback((incoming: ActivityEvent[]) => {
    const fresh = incoming.filter((event) => {
      const id = eventId(event);
      if (seen.current.has(id)) return false;
      seen.current.add(id);
      return true;
    });
    if (fresh.length === 0) return;
    setEvents((previous) => {
      const next = [...previous, ...fresh].sort(compareEvents);
      return next.length > MAX_EVENTS ? next.slice(next.length - MAX_EVENTS) : next;
    });
  }, []);

  const load = useCallback(
    async (parts: Parts) => {
      try {
        const [nextMeta, nextGraph, nextEvents] = await Promise.all([
          parts.meta ? api.meta() : Promise.resolve(null),
          parts.graph ? api.graph({ recent: 6 }) : Promise.resolve(null),
          parts.activity ? api.activity(600) : Promise.resolve(null),
        ]);
        if (nextMeta) setMeta(nextMeta);
        if (nextGraph) setGraph(nextGraph);
        if (nextEvents) merge(nextEvents);
        setError(null);
        setVersion((value) => value + 1);
      } catch (reason) {
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    },
    [merge],
  );

  const schedule = useCallback(
    (parts: Partial<Parts>) => {
      pending.current = {
        meta: pending.current.meta || Boolean(parts.meta),
        graph: pending.current.graph || Boolean(parts.graph),
        activity: pending.current.activity || Boolean(parts.activity),
      };
      if (timer.current !== null) return;
      timer.current = window.setTimeout(() => {
        const request = pending.current;
        pending.current = { meta: false, graph: false, activity: false };
        timer.current = null;
        void load(request);
      }, 300);
    },
    [load],
  );

  useEffect(() => {
    void load({ meta: true, graph: true, activity: true });
  }, [load]);

  useEffect(() => {
    let openedBefore = false;
    const source = new EventSource("/events");
    source.onopen = () => {
      setConnection("live");
      if (openedBefore) schedule({ meta: true, graph: true, activity: true });
      openedBefore = true;
    };
    source.onerror = () => {
      setConnection(source.readyState === EventSource.CLOSED ? "offline" : "connecting");
    };
    const onActivity = (message: MessageEvent<string>) => {
      let event: ActivityEvent;
      try {
        event = JSON.parse(message.data) as ActivityEvent;
      } catch {
        return;
      }
      merge([event]);
      setHighlight((previous) => nextHighlight(previous, event, Date.now()));
      if (GRAPH_KINDS.has(event.kind)) schedule({ meta: true, graph: true });
      else if (META_KINDS.has(event.kind)) schedule({ meta: true });
    };
    source.addEventListener("activity", onActivity as EventListener);
    return () => {
      source.removeEventListener("activity", onActivity as EventListener);
      source.close();
      if (timer.current !== null) window.clearTimeout(timer.current);
      timer.current = null;
    };
  }, [merge, schedule]);

  const model = useMemo(() => (graph ? buildGraphModel(graph) : null), [graph]);
  return { meta, graph, model, events, highlight, connection, error, version };
}

export function useNow(intervalMs = 1000): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(id);
  }, [intervalMs]);
  return now;
}
