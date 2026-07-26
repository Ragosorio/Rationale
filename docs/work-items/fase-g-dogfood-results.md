# Evidencia de dogfood interno — Fase G

Fecha: 2026-07-26. Binario: commit `fabeb92400b485080278276105bd60b0a3e295c5`.
Repositorio: Rationale. Modo: `rationale prepare`, sin escribir Records ni
propuestas.

## Casos ejecutados

| # | Target | Proveedor | Cobertura | Warnings | tokens |
|---:|---|---|---|---:|---:|
| 1 | `src/review.rs::approve` | successful | complete | 0 | 207 |
| 2 | `src/review.rs::mutate_record` | successful | complete | 0 | 209 |
| 3 | `src/storage.rs::write_record` | successful | complete | 0 | 209 |
| 4 | `src/mcp/server.rs::call_prepare_change` | successful | complete | 0 | 211 |
| 5 | `src/subjects.rs::resolve` | successful | complete | 0 | 247 |
| 6 | `src/pipeline.rs::prepare` | successful | complete | 0 | 218 |
| 7 | `src/capture.rs::capture` | successful | complete | 0 | 220 |
| 8 | `src/configuration.rs::ResolvedConfig.authority_for_actor` | successful | unknown | 1 | 194 |
| 9 | `src/retrieval.rs::compile_packet` | successful | complete | 0 | 231 |
| 10 | `Cargo.toml` | successful | unknown | 1 | 194 |

Consistencia observada: `working-tree-ahead` en todos los casos, reportada
honestamente porque la rama contiene documentación, packaging y artefactos de
esta misma ejecución. No se presentó como revisión exacta.

## Resultado

- 10/10 procesos terminaron con código 0.
- 10/10 produjeron un ContextPacket.
- 8/10 tuvieron cobertura estructural completa; los dos `unknown` corresponden
  a un método que el proveedor no expuso y a un manifiesto no simbólico.
- 8/10 no tuvieron warnings; los 2 warnings fueron explícitos y no se
  convirtieron en un falso "no existe".
- Mediana del proxy de tokens: 210; máximo: 247.
- Los logs locales muestran latencias recientes de 48–80 ms con caché/provider
  disponibles (`.rationale-local/runs/vertical-slice.ndjson`).

La primera pasada dentro del sandbox produjo warnings de SQLite por una ruta de
caché no escribible. Se repitió fuera del sandbox, como entorno local real, y
la caché derivada abrió correctamente; ese incidente no se cuenta como defecto
del producto, pero queda como requisito de smoke test de instalación.

## Gate

El dogfood interno sostiene el núcleo para el tag dogfood. No demuestra todavía
valor comparativo frente a Codebase Memory ni autoriza captura asistida sobre
repositorios laborales. Eso queda para [`fase-h-piloto.md`](fase-h-piloto.md).
