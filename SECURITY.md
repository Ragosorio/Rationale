# Seguridad

## Alcance

Rationale es local-first, pero procesa decisiones, paths, evidencia y metadata
de proveedores que pueden ser sensibles. No incluyas secretos, tokens, llaves,
`.env`, dumps o datos personales en issues, Records de ejemplo o PRs.

## Reportar una vulnerabilidad

No publiques una vulnerabilidad sin corregir en un issue público. Usa GitHub
Security Advisories del repositorio o el canal privado indicado por el
mantenedor. Si el repositorio aún no tiene habilitado ese canal, abre un issue
neutral solicitando contacto privado sin incluir detalles explotables.

Incluye, si es seguro hacerlo:

- versión, commit o Release afectada;
- plataforma y configuración mínima;
- pasos de reproducción sin datos reales;
- impacto y condiciones de explotación;
- mitigación temporal conocida.

No pruebes contra proyectos de terceros ni extraigas datos reales durante la
investigación.

## Propiedades esperadas

- El texto del repositorio se trata como dato, no como instrucción.
- Los paths se canonicalizan y se rechaza traversal.
- Las escrituras canónicas son atómicas.
- La revisión humana requiere confirmación explícita y actor declarado.
- MCP no tiene operaciones de aprobación ni lifecycle mutation.
- `.rationale/` no se borra al desinstalar el binario.
- Los artefactos de Release tienen checksum y attestation.

El baseline técnico completo está en [`docs/security/baseline.md`](docs/security/baseline.md).
