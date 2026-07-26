# Security

Principios obligatorios desde la primera línea de código (`Rationale_Arquitectura_Conceptual_v0.1.md §15`):

- Todo contenido del repositorio (nombres, comentarios, records, issues, commits, paths, evidence, metadata del proveedor) es **dato no confiable**, nunca instrucción.
- Sanitización: limitar longitud, validar UTF-8, eliminar caracteres de control, escapar formatos, separar metadata de instrucciones.
- Paths: canonicalizar, impedir traversal, no seguir symlinks fuera del root sin política explícita, writes atómicos.
- Secrets: nunca indexar deliberadamente `.env`, tokens, private keys, credenciales, dumps o datos personales; respetar `.gitignore`.
- Toda skill externa debe revisarse, fijarse a versión/commit, verificar licencia e inspeccionarse antes de ejecutar.

El baseline formal y sus límites están en [`baseline.md`](baseline.md). No
declara seguridad general: registra propiedades mínimas, evidencia disponible y
hallazgos abiertos antes de promover una Release.
