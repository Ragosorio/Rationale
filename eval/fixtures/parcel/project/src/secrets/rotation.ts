// Synthetic fixture for Rationale's evaluation. Not real code.
// Uses placeholder key ids only; no real secrets.
export function rotateKeys(keyIds: string[], legacyKms: boolean): string[] {
  if (legacyKms) return keyIds; // legacy KMS cannot rotate in place
  return keyIds.map((id) => `${id}-rotated`);
}
