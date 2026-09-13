//! Assets de la UI embebidos en el binario. `build.rs` genera la tabla desde
//! `ui/dist/` si existe; sin assets, una página explícita dice cómo
//! construirlos. Nunca se sirve un archivo del filesystem por su ruta.

pub struct Asset {
    pub path: &'static str,
    pub content_type: &'static str,
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/ui_assets.rs"));

pub fn has_ui() -> bool {
    !ASSETS.is_empty()
}

pub fn find(path: &str) -> Option<&'static Asset> {
    let wanted = path.trim_start_matches('/');
    let wanted = if wanted.is_empty() {
        "index.html"
    } else {
        wanted
    };
    ASSETS.iter().find(|asset| asset.path == wanted)
}

pub const FALLBACK_HTML: &str = r#"<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Rationale</title>
<style>
  :root { color-scheme: dark; }
  body { margin: 0; min-height: 100vh; display: grid; place-items: center;
         background: #07090d; color: #c9d1dc;
         font: 14px/1.6 ui-monospace, SFMono-Regular, Menlo, monospace; }
  main { max-width: 640px; padding: 32px; }
  h1 { font-size: 13px; letter-spacing: .24em; text-transform: uppercase; color: #7dd3fc; }
  code { color: #e2e8f0; }
  li { margin: 4px 0; }
</style>
</head>
<body>
<main>
  <h1>Rationale · Control Room</h1>
  <p>El servidor está vivo, pero este binario se compiló sin la interfaz web.</p>
  <p>Para incluirla: <code>npm --prefix ui ci &amp;&amp; npm --prefix ui run build</code> y vuelve a compilar Rationale.</p>
  <p>La API local ya responde:</p>
  <ul>
    <li><code>/api/meta</code> · <code>/api/activity</code> · <code>/api/operations</code></li>
    <li><code>/api/graph</code> · <code>/api/records</code> · <code>/api/conflicts</code></li>
    <li><code>/api/node/:key</code> · <code>/api/edge/:key</code> · <code>/events</code> (SSE)</li>
  </ul>
</main>
</body>
</html>
"#;
