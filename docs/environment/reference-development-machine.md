# Máquina de referencia de desarrollo

Generado con `scripts/dev/collect-environment.sh`. Esta es la versión anonimizada y versionable — el JSON completo vive en `.rationale-local/environment.json` (ignorado por Git, regenerable en cualquier máquina).

Deliberadamente **no** se registran aquí: número de serie, hardware UUID, provisioning UDID, hostname real ni ninguna ruta personal (`Rationale_Arquitectura_Conceptual_v0.1.md §5`).

## Perfil

| Campo | Valor |
|---|---|
| Equipo | MacBook Air |
| Chip | Apple M4 |
| Núcleos | 10 (4 Performance + 6 Efficiency) |
| Memoria | 16 GB |
| Arquitectura | arm64 (Apple Silicon) |
| Sistema operativo | macOS 26.5.2 (build 25F84) |
| Git | 2.50.1 |
| Clang | Apple clang 21.0.0 |
| Xcode Command Line Tools | instaladas |

## Implicaciones de diseño (`Rationale_Arquitectura_Conceptual_v0.1.md §5.2`)

- Evitar procesos residentes innecesarios; ningún daemon obligatorio.
- No duplicar índices completos en memoria.
- Las pruebas de gran escala deben tener límites explícitos.
- Los benchmarks deben registrar memoria pico.
- Un daemon, si llega a existir, debe ser opcional y austero.
- Las operaciones frecuentes deben usar caché local.
- Soporte de Apple Silicon desde el inicio; el desarrollo inicial puede priorizar macOS arm64.
- El núcleo no debe usar APIs exclusivas de macOS.

## Reproducir

```bash
bash scripts/dev/collect-environment.sh
cat .rationale-local/environment.json
```
