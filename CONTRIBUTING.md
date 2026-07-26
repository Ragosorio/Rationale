# Contribuir a Rationale

Gracias por querer mejorar Rationale. El proyecto separa deliberadamente el
código que prepara contexto de las decisiones que adquieren autoridad.

## Antes de empezar

Lee según el cambio:

- Producto y modelo: [`Rationale_v0.5.md`](Rationale_v0.5.md).
- Arquitectura: [`Rationale_Arquitectura_Conceptual_v0.1.md`](Rationale_Arquitectura_Conceptual_v0.1.md).
- Proceso entre agentes: [`Rationale_Proceso_Construccion_Agentes_v0.1.md`](Rationale_Proceso_Construccion_Agentes_v0.1.md).
- Código factual: [`docs/architecture/code-map.md`](docs/architecture/code-map.md).
- Rust: [`docs/rust/`](docs/rust/).
- Decisiones: [`docs/adr/`](docs/adr/).

Para cambios que atraviesan módulos, almacenamiento, MCP, seguridad o
packaging, consulta Codebase Memory antes de editar y declara su cobertura y
warnings en el trabajo.

## Desarrollo local

```bash
git clone https://github.com/Ragosorio/Rationale.git
cd Rationale
export PATH="$HOME/.cargo/bin:$PATH"
cargo build
cargo test
```

Antes de abrir un PR:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo audit
```

Los cambios de código Rust deben incluir tests proporcionales. Los cambios de
schema deben pasar `cargo test --test schema_validation`; los cambios de
almacenamiento deben preservar el round-trip; los cambios MCP deben mantener
stdout exclusivamente para el protocolo.

## Flujo de trabajo

1. Registra el problema o work item y la evidencia disponible.
2. Decide si hace falta ADR antes de tocar la frontera arquitectónica.
3. Usa una rama descriptiva: `feature/...`, `fix/...`, `docs/...` o
   `release/...`.
4. Implementa el cambio mínimo, añade tests y actualiza documentación.
5. Ejecuta la revisión adversarial: busca carreras, drift, falsos positivos,
   problemas cross-platform, tests ausentes y costos operativos.
6. Reindexa Codebase Memory cuando aplique y registra cobertura.
7. Ejecuta el flujo de Rationale correspondiente: una decisión no queda
   aprobada porque un agente la haya redactado.
8. Abre el PR con contexto, evidencia, riesgos y comandos ejecutados.

## Convenciones

- Commits causales y pequeños, por ejemplo:
  `fix(review): reject stale proposal before approval`.
- No mezcles refactors no relacionados.
- No agregues secretos, `.env`, dumps ni datos personales.
- No apruebes automáticamente Records.
- No modifiques internals de Codebase Memory ni leas su SQLite privada.
- No ocultes cobertura parcial: escribe `Unknown`, evidencia, riesgo y próximo
  experimento cuando corresponda.

## Checklist del PR

- [ ] Código y tests actualizados.
- [ ] Formatter y lint limpios.
- [ ] Security check ejecutado cuando aplica.
- [ ] Documentación, ADR o research artifact actualizado.
- [ ] Revisión independiente realizada para cambios críticos.
- [ ] Codebase Memory reindexado o se explicó por qué no aplica.
- [ ] `git status` limpio salvo archivos deliberadamente no relacionados.

## Revisión de decisiones

Las propuestas humanas se revisan con `rationale review`; el lifecycle de un
Record aprobado se revisa con `rationale review-record`. MCP consulta y prepara,
pero nunca sustituye la confirmación humana.

Para dudas de seguridad, sigue [`SECURITY.md`](SECURITY.md). Para preguntas de
uso, sigue [`SUPPORT.md`](SUPPORT.md).
