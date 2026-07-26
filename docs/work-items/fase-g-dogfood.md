# Fase G — dogfood formal en Rationale

Fecha de ejecución: 2026-07-26. Este work item registra evidencia reproducible
del dogfood de Fase G y separa lo que el binario ya demostró de lo que todavía
requiere una acción humana o una sesión nueva del agente.

## G1 — conexión MCP

`.mcp.json` está versionado y apunta al servidor real `cargo run --quiet
--release -- serve`. La sesión de Codex que ejecutó F8 ya estaba abierta antes
de que este servidor se añadiera al contexto, por lo que las herramientas MCP
nativas de Rationale no aparecen en esta sesión; cargar `.mcp.json` requiere
reiniciar la sesión del agente. Como evidencia no sustitutiva, se ejecutó el
binario real `target/release/rationale serve` con framing `Content-Length` y un
cliente de transporte efímero, no un mock:

- `initialize` confirmó `2024-11-05`.
- `tools/list` devolvió exactamente `prepare_change`, `explain_target`,
  `health` y `finalize_change`.
- `health` devolvió `provider_status=successful`, `provider_coverage=complete`
  y `working_tree_dirty=true` con el caché local disponible.

El transporte efímero valida el servidor, pero no se presenta como evidencia
de que la sesión de Codex ya haya cargado la integración nativa. Esa parte queda
pendiente de reiniciar la sesión.

## G2 — `prepare_change` antes de cambiar

Antes de cerrar F8 se ejecutó `prepare_change` sobre `src/review.rs`, tanto por
CLI como por el servidor MCP real, con la intención de verificar la revisión
humana y el claim atómico. El packet entregó:

- `consistency=working-tree-ahead`, consistente con cambios locales no
  confirmados;
- proveedor `successful`, pero cobertura `unknown` para el símbolo solicitado;
- el Subject y el Record de frontera del proveedor, con autoridad
  `unreviewed`;
- una advertencia explícita de que el símbolo no estaba en la cobertura
  disponible.

La degradación es honesta: el contexto fue útil para confirmar la frontera del
proveedor y sus riesgos, pero no fingió cobertura estructural completa del
target.

## G3 — captura de decisiones reales

Se ejecutaron tres llamadas `finalize_change` reales contra este repositorio,
usando como `base_revision` el commit previo a F8:

| Record pendiente | Decisión capturada | Resultado |
|---|---|---|
| `constraint.f8-roundtrip-fidelity` | fidelidad de round-trip del Record canónico | propuesta escrita |
| `constraint.f8-atomic-proposal-claim` | claim atómico de una propuesta | propuesta escrita |
| `constraint.f8-project-authority` | autoridad declarada por el proyecto | propuesta escrita |

Las tres propuestas viven en `.rationale/proposals/`, tienen `status: pending`
y no tienen aprobaciones. La segunda ejecución de `rationale review` mostró las
tres una por pantalla, resolvió al actor Git como
`user:ragosorio <ragosorio777@gmail.com>` y mostró `architecture-owner` desde
`.rationale/config.yaml`. Se introdujo `skip` para las tres: ninguna decisión
se convirtió en Record aprobado.

Esto satisface la captura mecánica de G3 sin violar G4. La aprobación humana de
estas decisiones, y especialmente de los siete Subjects fundacionales y los
nueve ADRs que siguen `unreviewed`/`proposed`, queda deliberadamente pendiente.

## G5 — medición honesta y límites

- El packet fue accionable para la frontera del proveedor y el estado de
  revisión, pero la cobertura de `src/review.rs` fue `unknown`; no se cuenta
  como cobertura completa.
- El entorno sandbox no permite abrir siempre el SQLite derivado; con acceso al
  caché local autorizado, `health` se mantuvo `successful/complete`.
- CI para Linux y macOS está versionado en `.github/workflows/ci.yml`, pero su
  ejecución remota aún requiere GitHub.
- No se autoaprobaron Records. `review_record`, embeddings, calibración de
  Jaccard, Windows y el piloto de monorepo permanecen fuera de alcance.

## Verificación del grafo

Después de los cambios se reindexó Codebase Memory en modo `fast`. El estado
quedó `ready` con 1.644 nodos y 3.500 aristas; búsquedas posteriores
encontraron `src/review.rs::claim_proposal` y
`src/configuration.rs::ResolvedConfig.authority_for_actor`. La cobertura del
índice es parcial por diseño del modo rápido y no sustituye la revisión directa
del código fuente.
