use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let version = env::var("RATIONALE_VERSION")
        .or_else(|_| env::var("CARGO_PKG_VERSION"))
        .expect("Cargo siempre debe proporcionar una versión de paquete");
    println!("cargo:rustc-env=RATIONALE_BUILD_VERSION={version}");
    println!("cargo:rerun-if-env-changed=RATIONALE_VERSION");
    embed_ui_assets();
}

/// Tabla de assets de `rationale ui` generada desde `ui/dist/`, para que el
/// binario siga siendo uno solo. Sin build de la UI la tabla queda vacía y el
/// servidor sirve una página que explica cómo construirla.
fn embed_ui_assets() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let ui = manifest.join("ui");
    let dist = ui.join("dist");
    // Vigilar una ruta que existe: `rerun-if-changed` sobre una ruta
    // inexistente haría correr este script, y recompilar el crate, siempre.
    let watched = if dist.is_dir() {
        dist.clone()
    } else if ui.is_dir() {
        ui
    } else {
        manifest.join("build.rs")
    };
    println!("cargo:rerun-if-changed={}", watched.display());

    let mut files = Vec::new();
    if dist.is_dir() {
        collect(&dist, &mut files);
    }
    files.sort();
    let mut table = String::from("pub static ASSETS: &[Asset] = &[\n");
    for file in &files {
        let relative = file
            .strip_prefix(&dist)
            .expect("cada asset vive bajo ui/dist")
            .to_string_lossy()
            .replace('\\', "/");
        let _ = writeln!(
            table,
            "    Asset {{ path: {:?}, content_type: {:?}, bytes: include_bytes!({:?}) }},",
            relative,
            content_type(file),
            file.display().to_string()
        );
    }
    table.push_str("];\n");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("ui_assets.rs");
    std::fs::write(out, table).expect("no se pudo escribir ui_assets.rs");
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let hidden = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'));
        if hidden {
            continue;
        }
        if path.is_dir() {
            collect(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}
