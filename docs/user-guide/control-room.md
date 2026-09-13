# Control Room

`rationale ui` abre un centro de control local y de solo lectura: el subgrafo de
trabajo de los agentes, la memoria que lo explica y la actividad de cada sesión
en vivo.

```bash
rationale ui
rationale ui --port 9800 --no-open
```

Escucha solo en `127.0.0.1` (puerto `9748` por defecto; sin `--port` prueba los
siguientes si está ocupado). La interfaz va embebida en el binario. Observa el
mismo estado local que la CLI y el servidor MCP y nunca escribe:
`rationale serve` sigue siendo la frontera con los agentes.

## Vistas

- **Grafo.** El subgrafo de las operaciones recientes en 3D. El color del nodo
  es su rol en el cambio (target, caller, callee, dependencia, dependiente,
  test, contexto); el de la arista, su estado estructural. Un anillo marca lo
  que explica el canon. Selecciona un nodo o una relación para leer sus Records,
  sus relaciones y su historia.
- **Actividad.** Cada sesión — Claude Code, Codex, Cursor o la CLI — con sus
  operaciones, la latencia del proveedor, el tamaño del packet, lo capturado, lo
  descartado, los conflictos y las explicaciones en riesgo. Llega por
  Server-Sent Events.
- **Memoria.** El canon con filtros por tipo, estado y autoridad, y los
  conflictos pendientes.
- **Sistema.** Proyecto, revisión Git, conteos del canon, estado del stream y
  tabla de sesiones.

## Datos que muestra

- Actividad: `.rationale-local/activity/<session>.ndjson`.
- Operaciones: `.rationale-local/operations/`.
- Canon: `.rationale/records/`; conflictos: `.rationale-local/conflicts/`.

La actividad guarda identificadores, la intención declarada (una línea, 280
caracteres como máximo) y referencias a Records — nunca contenido de Records,
código ni conversaciones (ADR-0017). `RATIONALE_ACTIVITY=off` la desactiva por
completo.

## Seguridad

Solo `GET` y `HEAD`; validación de la cabecera `Host` contra nombres de loopback
(defensa contra DNS rebinding); cabeceras acotadas en tamaño y tiempo;
Content-Security-Policy restrictiva; solo assets embebidos, nunca rutas del
sistema de archivos.

## Desde el código fuente

Sin `ui/dist`, el binario sirve una página que explica cómo construir la
interfaz. Los binarios de Release siempre la incluyen.

```bash
npm --prefix ui ci
npm --prefix ui run build
cargo build --release
```

`build.rs` vigila `ui/dist` (o `ui/` si todavía no existe). Si un directorio
`target/` se compiló antes de que existiera `ui/`, toca `build.rs` una vez para
que vuelva a embeber los assets.
