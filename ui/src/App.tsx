import { useEffect, useState } from "react";
import { Ticker, TopBar, type Tab } from "./components/chrome";
import { useLive, useNow } from "./live/useLive";
import { ActivityView } from "./views/ActivityView";
import { GraphView } from "./views/GraphView";
import { MemoryView } from "./views/MemoryView";
import { SystemView } from "./views/SystemView";

const TABS: Tab[] = ["graph", "activity", "memory", "system"];
const SHORTCUTS: Record<string, Tab> = { g: "graph", a: "activity", m: "memory", s: "system" };

function initialTab(): Tab {
  const hash = window.location.hash.slice(1);
  return (TABS as string[]).includes(hash) ? (hash as Tab) : "graph";
}

export function App() {
  const live = useLive();
  const now = useNow(1000);
  const [tab, setTab] = useState<Tab>(initialTab);

  useEffect(() => {
    window.history.replaceState(null, "", `#${tab}`);
  }, [tab]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement
      ) {
        return;
      }
      const next = SHORTCUTS[event.key.toLowerCase()];
      if (next) setTab(next);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="shell">
      <TopBar meta={live.meta} tab={tab} onTab={setTab} connection={live.connection} now={now} />
      <main className="stage">
        {/* El grafo sigue montado fuera de su pestaña: conserva cámara y layout
            y se congela (frameloop "never") mientras no se ve. */}
        <div className="view" hidden={tab !== "graph"}>
          <GraphView live={live} now={now} active={tab === "graph"} onOpenActivity={() => setTab("activity")} />
        </div>
        {tab === "activity" && (
          <div className="view">
            <ActivityView live={live} now={now} onShowGraph={() => setTab("graph")} />
          </div>
        )}
        {tab === "memory" && (
          <div className="view">
            <MemoryView live={live} />
          </div>
        )}
        {tab === "system" && (
          <div className="view">
            <SystemView live={live} now={now} />
          </div>
        )}
        {live.error && <div className="toast">API local: {live.error}</div>}
      </main>
      <Ticker events={live.events} onOpen={() => setTab("activity")} />
    </div>
  );
}
