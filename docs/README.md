# Documentación de Rationale

Elige el recorrido según tu objetivo. El README raíz resume el proyecto; esta
página organiza el detalle sin duplicarlo.

## Quiero usar Rationale

1. [Quickstart](quickstart.md)
2. [Conceptos](user-guide/concepts.md)
3. [Flujo diario](user-guide/daily-workflow.md)
4. [Referencia CLI](user-guide/cli-reference.md)
5. [Agentes y MCP](user-guide/agents-and-mcp.md)
6. [Configuración, archivos y privacidad](user-guide/configuration.md)
7. [Diagnóstico](runbooks/diagnostics.md)

## Quiero contribuir

1. [CONTRIBUTING.md](../CONTRIBUTING.md)
2. [Mapa de arquitectura](architecture/code-map.md)
3. [Guías Rust](rust/)
4. [Proceso de construcción](../Rationale_Proceso_Construccion_Agentes_v0.1.md)
5. [ADRs](adr/)
6. [Work items y evidencia](work-items/)
7. [Build y tests](runbooks/build-and-test.md)

## Quiero operar una instalación

- [Instalación y actualización](runbooks/install.md)
- [Diagnóstico](runbooks/diagnostics.md)
- [Fallo del proveedor](runbooks/provider-failure.md)
- [Reset de cache](runbooks/cache-reset.md)
- [Desinstalación](runbooks/uninstall.md)
- [Release](runbooks/release.md)
- [Security baseline](security/baseline.md)

## Quiero entender el diseño

- [Contrato de producto](../Rationale_v0.5.md)
- [Arquitectura conceptual](../Rationale_Arquitectura_Conceptual_v0.1.md)
- [Mapa factual del código](architecture/code-map.md)
- [Investigación Codebase Memory](research/codebase-memory/)
- [Índice de ADRs](adr/index.md)

## Convenciones

La documentación debe indicar su audiencia, enlazar la fuente de verdad en vez
de duplicarla y usar comandos que hayan sido verificados contra el binario o
los scripts actuales. Si una afirmación es incierta, debe marcarse como
`Unknown` con evidencia, riesgo y siguiente experimento.
