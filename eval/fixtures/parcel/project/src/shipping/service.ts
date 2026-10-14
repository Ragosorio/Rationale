// Synthetic fixture for Rationale's evaluation. Not real code.
import { quoteRate, applySurcharge } from "./rates";
import { buyLabel } from "./carriers/dhl";

export function createShipment(order: { id: string; idempotencyKey: string }) {
  const price = applySurcharge(quoteRate("GT", "MX", 2), "remote");
  return { id: order.id, key: order.idempotencyKey, price };
}

export function cancelShipment(shipmentId: string, carrierAck: boolean) {
  if (!carrierAck) throw new Error("carrier must acknowledge before cancelling");
  return { shipmentId, cancelled: true };
}

export async function retryLabel(shipmentId: string) {
  for (let attempt = 0; attempt < 5; attempt++) {
    const label = await buyLabel(shipmentId);
    if (label) return label;
    await new Promise((r) => setTimeout(r, 2 ** attempt * 100));
  }
  return null;
}
