# Cache reset

La capa derivada (ADR-0004/0005) vive en `~/.cache/rationale/projects/<ruta-sanitizada>/derived.sqlite3` — **nunca dentro del repo**, y **nunca la única copia de una decisión** (`Arquitectura §11.7`). Borrarla es siempre seguro: se reconstruye sola en la siguiente consulta.

## Encontrar el cache de un proyecto

```bash
rationale health --project-root /ruta/al/proyecto
```

O calcularlo a mano — la ruta sanitiza el path absoluto del proyecto reemplazando `/` por `-`:

```bash
echo "$HOME/.cache/rationale/projects/$(realpath /ruta/al/proyecto | sed 's#^/##; s#/#-#g')"
```

## Borrar el cache de un proyecto

```bash
rm -rf "$HOME/.cache/rationale/projects/<ruta-sanitizada>"
```

## Borrar todo el cache de Rationale (todos los proyectos)

```bash
rm -rf "$HOME/.cache/rationale"
```

## Qué se pierde y qué no

| Se pierde (se recalcula solo) | Nunca se pierde (vive en `.rationale/`, versionado en Git) |
|---|---|
| Assessments cacheados | Records, Subjects, Approvals, Bindings |
| Índice FTS5 de statements/títulos | Propuestas pendientes (`.rationale/proposals/`) |
| — | Propuestas rechazadas (`.rationale/proposals/.rejected/`) |

Verificado con test (`cache::tests::cache_rebuild_from_scratch_never_loses_canonical_data`): borrar el cache y reconstruirlo produce resultados idénticos a partir de los mismos Records reales.

## Cuándo hacerlo

- El cache quedó corrupto (muy raro; SQLite en modo WAL es robusto ante cierres abruptos).
- Sospechas que un assessment quedó con datos obsoletos de una versión anterior del schema.
- Estás depurando y quieres confirmar que un resultado no depende de estado cacheado.
