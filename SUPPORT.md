# Soporte

## Antes de pedir ayuda

Ejecuta:

```bash
rationale health
rationale --help
```

Luego consulta [`docs/runbooks/diagnostics.md`](docs/runbooks/diagnostics.md)
y [`docs/runbooks/provider-failure.md`](docs/runbooks/provider-failure.md).

## Qué incluir en un issue

- sistema operativo y arquitectura;
- versión de Rationale (`git describe --tags` o Release instalada);
- comando ejecutado;
- salida de `rationale health` sin secretos;
- si Codebase Memory estaba disponible;
- reproducción mínima y resultado esperado.

Redacta tokens, URLs privadas, nombres de clientes y contenido sensible antes
de publicar. Para vulnerabilidades, usa [`SECURITY.md`](SECURITY.md), no un
issue público.

## Tipos de ayuda

- Bug reproducible: issue con pasos mínimos.
- Duda de uso: discusión o issue etiquetado `question`.
- Mejora: propuesta con problema, alternativas y costo.
- Documentación: PR directo si el cambio es autocontenido.
