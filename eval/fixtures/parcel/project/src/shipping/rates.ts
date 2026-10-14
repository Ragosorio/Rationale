// Synthetic fixture for Rationale's evaluation. Not real code.
import { cacheGet, cacheSet } from "../shared/cache";

export function toCents(amount: number): number {
  return Math.round(amount * 100);
}

export function quoteRate(origin: string, destination: string, weightKg: number): number {
  const key = `${origin}:${destination}:${weightKg}`;
  const cached = cacheGet(key); // entries expire after 60 seconds
  if (cached !== undefined) return cached;
  const base = 4.5 + weightKg * 1.2;
  const cents = toCents(base);
  cacheSet(key, cents);
  return cents;
}

export function applySurcharge(cents: number, zone: string): number {
  return zone === "remote" ? cents + 250 : cents;
}
