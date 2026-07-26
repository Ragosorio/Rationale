# ADR-0010: GitHub Release packaging for the local alpha

**Status:** proposed
**Date:** 2026-07-26
**Deciders:** dueño humano del proyecto + revisión cruzada

## Context

El MVP no se considera producto terminado mientras dependa de clonar el
repositorio e instalar Rust. El usuario necesita instalar Rationale desde
GitHub, conectarlo a sus agentes y actualizarlo o quitarlo sin perder el canon.
La instalación debe poder actualizar un binario ya instalado con un comando,
sin obligar al usuario cleanroom a borrar manualmente el paquete anterior.

## Options considered

### A — `cargo-dist`

Usar la metadata `[package.metadata.dist]` y un workflow generado por
`cargo-dist` fijado a una versión exacta. Ventaja: convenciones maduras para
tarballs, instaladores y tags. Riesgo: el workflow generado debe verificarse
en todos los targets antes de declarar soporte.

### B — workflow propio

Compilar con Cargo, producir archives y checksums, y publicar con `gh`.
Ventaja: control total y poca magia. Riesgo: más mantenimiento de la matriz
cross-platform y de los instaladores.

## Decision proposal

Adoptar el contrato de `cargo-dist` y sus targets como interfaz de release,
manteniendo scripts deterministas versionados (`scripts/package.*`) para que
la CI pueda auditar y probar cada archivo. Se publicarán tarballs/ZIP,
checksums SHA-256, instaladores shell/PowerShell y provenance de build.

`publish = false` se mantiene: la alfa se distribuye por GitHub Releases, no
por crates.io.

## Consequences

- Tier objetivo: macOS ARM64/x86_64, Linux x86_64/ARM64 y Windows x86_64.
- La alfa requiere una prueba de máquina limpia, update y rollback.
- El instalador distribuye también un helper `rationale-update`; el comando
  `rationale update` lo ejecuta y conserva `.rationale/` y la configuración del
  proyecto.
- Mientras Rationale sea pre-1.0 se distinguen los canales `stable` y
  `preview`: la landing no puede usar `releases/latest` si la versión visible
  es una prerelease; debe fijar el tag preview explícito.
- Una plataforma que no pase la matriz no se declara soportada en las notas.
- Los binarios de release son una superficie de supply-chain y necesitan
  checksums y attestation antes de publicar.
