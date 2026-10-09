use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));

    cc::Build::new()
        .cpp(true)
        .file(manifest.join("native/cpp/engine.cpp"))
        .warnings(true)
        .compile("minux_engine");

    println!("cargo:rerun-if-changed=native/cpp/engine.cpp");
    println!("cargo:rerun-if-changed=native/csharp/MINUX.Agent.csproj");
    println!("cargo:rerun-if-changed=native/csharp/Agent.cs");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let project = manifest.join("native/csharp/MINUX.Agent.csproj");
        let status = Command::new("dotnet")
            .arg("publish")
            .arg(&project)
            .args(["--configuration", "Release", "--runtime", "win-x64", "--self-contained", "true", "-p:NativeLib=Static"])
            .status()
            .expect("Could not start dotnet. Install the .NET 9 SDK to build the C# agent.");

        if !status.success() {
            panic!("C# NativeAOT static-library build failed");
        }

        let native_dir = manifest.join("native/csharp/bin/Release/net9.0/win-x64/native");
        if !native_dir.join("MINUXAgent.lib").exists() {
            panic!("C# NativeAOT output MINUXAgent.lib was not found in {}", native_dir.display());
        }

        println!("cargo:rustc-link-search=native={}", native_dir.display());
        println!("cargo:rustc-link-lib=static=MINUXAgent");
    }
}
