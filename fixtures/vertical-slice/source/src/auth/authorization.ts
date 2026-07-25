// Fixture determinista para la vertical slice de Rationale (Fase D2).
// Mismo caso canónico usado en Rationale_v0.5.md §2, §9, §27.

export interface EntityAssignment {
  entityId: string;
  userId: string;
}

export function resolveEntityRole(entityAssignment: EntityAssignment | null) {
  if (entityAssignment) {
    return resolveEntityPermissions(entityAssignment);
  }
  return denyAccess();
}

function resolveEntityPermissions(assignment: EntityAssignment) {
  return { scope: "entity", entityId: assignment.entityId };
}

function denyAccess() {
  return { scope: "none" };
}
