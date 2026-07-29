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
- **Qué tags se marcan prerelease en GitHub.** Solo `-alpha.`, `-rc.` y
  `-dogfood.`. `beta` y las versiones finales se publican como Release
  completa. La razón es operativa, no cosmética: GitHub solo marca «latest» una
  Release que no sea prerelease, y `releases/latest` es exactamente lo que
  resuelve el canal `stable` de `rationale-installer.sh/.ps1`. Marcar por
  «cualquier tag con guión» dejó las siete alphas como prerelease y, con ellas,
  `stable` sirviendo `v0.0.0-dogfood.7` — un build de dogfood anterior a
  `install-agent` — a cualquiera que instalara por ese canal. Una versión que
  el proyecto recomienda usar debe poder ser «latest».
- Una plataforma que no pase la matriz no se declara soportada en las notas.
- Los binarios de release son una superficie de supply-chain y necesitan
  checksums y attestation antes de publicar.
