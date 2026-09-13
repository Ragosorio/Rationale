export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatTokens(tokens: number): string {
  if (!Number.isFinite(tokens) || tokens <= 0) return "0";
  return tokens >= 1000 ? `~${(tokens / 1000).toFixed(1)}k` : `~${tokens}`;
}

/** La palabra que acompaña a un conteo: `1 escrito`, `3 escritos`. */
export function plural(count: number, one: string, many: string): string {
  return count === 1 ? one : many;
}

/** Hora local HH:MM:SS de un timestamp RFC3339. */
export function clockTime(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return "--:--:--";
  return date.toLocaleTimeString("es", { hour12: false });
}

export function relativeTime(timestamp: string, now: number): string {
  const then = Date.parse(timestamp);
  if (!Number.isFinite(then)) return "—";
  const seconds = Math.max(0, Math.round((now - then) / 1000));
  if (seconds < 5) return "ahora";
  if (seconds < 60) return `hace ${seconds} s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `hace ${minutes} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `hace ${hours} h`;
  return `hace ${Math.round(hours / 24)} d`;
}

export function shortId(id: string | null | undefined, keep = 12): string {
  if (!id) return "—";
  return id.length > keep + 2 ? `${id.slice(0, keep)}…` : id;
}

/** Los dos últimos segmentos de un nombre calificado: `Tipo.metodo`. */
export function shortName(qualified: string | null | undefined): string {
  if (!qualified) return "—";
  const parts = qualified.split(/[.:]+/).filter(Boolean);
  return parts.slice(-2).join(".");
}
