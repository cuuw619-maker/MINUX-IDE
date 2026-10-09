use std::{env, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    cc::Build::new()
        .file(manifest.join("native/c/engine.c"))
        .warnings(true)
        .compile("minux_engine");
    println!("cargo:rerun-if-changed=native/c/engine.c");
    println!("cargo:rerun-if-changed=resources/themes.json");
    println!("cargo:rerun-if-changed=src/theme.rs");
}
