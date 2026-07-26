use std::env;

fn main() {
    let version = env::var("RATIONALE_VERSION")
        .or_else(|_| env::var("CARGO_PKG_VERSION"))
        .expect("Cargo siempre debe proporcionar una versión de paquete");
    println!("cargo:rustc-env=RATIONALE_BUILD_VERSION={version}");
    println!("cargo:rerun-if-env-changed=RATIONALE_VERSION");
}
