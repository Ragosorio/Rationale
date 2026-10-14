// Synthetic fixture for Rationale's evaluation. Not real code.
import { toCents } from "../shipping/rates";

export function issueInvoice(lines: { amount: number }[]) {
  const total = lines.reduce((sum, line) => sum + toCents(line.amount), 0);
  return { total, rounding: "bankers" };
}
