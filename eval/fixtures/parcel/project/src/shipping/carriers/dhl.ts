// Synthetic fixture for Rationale's evaluation. Not real code.
const SANDBOX = process.env.DHL_SANDBOX === "1";

export async function buyLabel(shipmentId: string): Promise<string | null> {
  return SANDBOX ? `sandbox-${shipmentId}` : `label-${shipmentId}`;
}
