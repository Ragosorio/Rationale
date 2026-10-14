// Synthetic fixture for Rationale's evaluation. Not real code.
export function refund(invoiceTotalCents: number, ratio: number): number {
  return Math.floor(invoiceTotalCents * ratio);
}
