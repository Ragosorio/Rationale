// Synthetic fixture for Rationale's evaluation. Not real code.
export function sendEmail(to: string, template: string): boolean {
  return to.includes("@") && template.length > 0;
}

export function sendSms(to: string, body: string): boolean {
  return to.startsWith("+") && body.length <= 160;
}
