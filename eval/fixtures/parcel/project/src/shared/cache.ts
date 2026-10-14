// Synthetic fixture for Rationale's evaluation. Not real code.
const store = new Map<string, number>();

export function cacheGet(key: string): number | undefined {
  return store.get(key);
}

export function cacheSet(key: string, value: number): void {
  store.set(key, value);
}
