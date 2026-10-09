use std::{env, fs, path::PathBuf, process::Command};

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

    // Build the C# agent as a self-contained NativeAOT executable. Embedding that
    // executable avoids unresolved NativeAOT runtime symbols in the Rust linker.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let project = manifest.join("native/csharp/MINUX.Agent.csproj");
        let status = Command::new("dotnet")
            .arg("publish")
            .arg(&project)
            .args(["--configuration", "Release", "--runtime", "win-x64", "--self-contained", "true"])
            .status()
            .expect("Could not start dotnet. Install the .NET 9 SDK to build the C# agent.");

        if !status.success() {
            panic!("C# NativeAOT executable build failed");
        }

        let published_agent = manifest.join("native/csharp/bin/Release/net9.0/win-x64/publish/MINUXAgent.exe");
        if !published_agent.is_file() {
            panic!("C# NativeAOT output was not found: {}", published_agent.display());
        }

        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
        let embedded_agent = out_dir.join("MINUXAgent.exe");
        fs::copy(&published_agent, &embedded_agent)
            .unwrap_or_else(|e| panic!("Could not embed C# agent: {e}"));

        // Fingerprint the embedded binary so updated builds do not reuse an older
        // agent extracted by a previous version of the IDE.
        let bytes = fs::read(&embedded_agent).expect("Could not read the built C# agent");
        let fingerprint = bytes.iter().fold(2166136261u32, |hash, byte| {
            (hash ^ u32::from(*byte)).wrapping_mul(16777619)
        });

        println!("cargo:rustc-env=MINUX_AGENT_EXE_PATH={}", embedded_agent.display());
        println!("cargo:rustc-env=MINUX_AGENT_FINGERPRINT={fingerprint:08x}");
    }
}
