# Schemas

Los 7 schemas JSON formales (`record`, `subject`, `binding`, `evidence`, `approval`, `assessment`, `context-packet.schema.json`) ya existen — escritos en Fase E2, consistentes campo por campo con los structs Rust reales (`src/storage.rs`, `src/subjects.rs`, `src/assessment.rs`, `src/retrieval.rs`), no como diseño aspiracional aislado del código.

## Estado real de la validación

**No hay un validador JSON Schema corriendo en tiempo de ejecución todavía.** Se evaluó el crate `jsonschema` (0.49.1) y se descartó por ahora: sus *features* por defecto (`resolve-http`, `tls-aws-lc-rs`) traen capacidad de red y TLS para resolver `$ref` remotos — contradice directamente `Rationale_Arquitectura_Conceptual_v0.1.md §4.1` ("no deberá requerir obligatoriamente... llamadas de red"). Añadirlo correctamente requeriría `default-features = false, features = ["resolve-file"]` y verificación de que ningún camino de red quede alcanzable — no se hizo en esta pasada por presupuesto de tiempo, no por decisión final.

**La validación determinista real hoy** (`Rationale_v0.5.md §9.2`: "la validación básica debe ser determinista y basada en schema") vive en Rust: `storage::read_record` y `subjects::read_subject` rechazan explícitamente campos obligatorios ausentes (`StorageError::MissingRequiredField`, `SubjectError::MissingRequiredField`), verificado con tests (`storage::tests::rejects_record_missing_statement`). No es heurística ni un LLM decidiendo si un campo "parece" completo.

## Próximo paso (Fase E3/E6)

Añadir `jsonschema` con features acotadas y validar los 7 schemas contra datos reales (`.rationale/records/*.yaml`, `.rationale/subjects/*.yaml`, la salida real de `ContextPacket`) como parte del endurecimiento de la capa derivada — cerraría la brecha entre "el schema existe" y "el schema se aplica en cada lectura/escritura".
