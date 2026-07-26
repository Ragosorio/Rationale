# Schemas

Los 7 schemas JSON formales (`record`, `subject`, `binding`, `evidence`, `approval`, `assessment`, `context-packet.schema.json`) ya existen — escritos en Fase E2, consistentes campo por campo con los structs Rust reales (`src/storage.rs`, `src/subjects.rs`, `src/assessment.rs`, `src/retrieval.rs`), no como diseño aspiracional aislado del código.

## Estado real de la validación

**No hay un validador JSON Schema corriendo en tiempo de ejecución todavía.** Se evaluó el crate `jsonschema` (0.49.1) y se descartó por ahora: sus *features* por defecto (`resolve-http`, `tls-aws-lc-rs`) traen capacidad de red y TLS para resolver `$ref` remotos — contradice directamente `Rationale_Arquitectura_Conceptual_v0.1.md §4.1` ("no deberá requerir obligatoriamente... llamadas de red"). Añadirlo correctamente requeriría `default-features = false, features = ["resolve-file"]` y verificación de que ningún camino de red quede alcanzable — no se hizo en esta pasada por presupuesto de tiempo, no por decisión final.

**La validación determinista real hoy** (`Rationale_v0.5.md §9.2`: "la validación básica debe ser determinista y basada en schema") vive en Rust: `storage::read_record` y `subjects::read_subject` rechazan explícitamente campos obligatorios ausentes (`StorageError::MissingRequiredField`, `SubjectError::MissingRequiredField`), verificado con tests (`storage::tests::rejects_record_missing_statement`). No es heurística ni un LLM decidiendo si un campo "parece" completo.

## Decisión tomada (Fase E6): no se añadió un validador JSON Schema

Fase E6 (`tests/schema_validation.rs`) resolvió esta brecha de otra forma: en vez de agregar `jsonschema`, un test verifica que los campos `required` de los 7 schemas coinciden con los campos no-`Option` de sus structs Rust correspondientes — detecta divergencia schema/struct sin necesitar el crate. La validación de datos reales (Records/Subjects/ContextPacket) sigue viviendo en `storage::read_record`/`subjects::read_subject` como se describe arriba.

**Revisit trigger:** reabrir esta decisión si aparece una necesidad real de validar contra el schema JSON completo (no solo campos requeridos) — por ejemplo, tipos de datos o formatos específicos (`format: date-time`) — o si `jsonschema` publica una variante sin `resolve-http`/`tls-aws-lc-rs` en sus features por defecto.
