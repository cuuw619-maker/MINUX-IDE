use std::{env, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    cc::Build::new()
        .file(manifest.join("native/c/engine.c"))
        .warnings(true)
        .compile("minux_engine");

    // Embed the .ico in the Windows executable so Explorer, taskbar and shortcuts
    // use the MINUX brand. Skip resource compilation when cross-compiling from
    // non-Windows hosts; the normal Windows CI build embeds it.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let host = env::var("HOST").unwrap_or_default();
    let target = env::var("TARGET").unwrap_or_default();
    if target_os == "windows" && host == target {
        winres::WindowsResource::new()
            .set_icon(manifest.join("assets/minux-logo.ico").to_string_lossy().as_ref())
            .set("ProductName", "MINUX IDE")
            .set("FileDescription", "MINUX IDE - Native Polyglot Coding Environment")
            .set("CompanyName", "MINUX")
            .compile()
            .expect("Failed to embed MINUX IDE Windows icon resource");
    }

    println!("cargo:rerun-if-changed=native/c/engine.c");
    println!("cargo:rerun-if-changed=resources/themes.json");
    println!("cargo:rerun-if-changed=src/theme.rs");
    println!("cargo:rerun-if-changed=assets/minux-logo.ico");
}
