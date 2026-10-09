# MINUX IDE

A native desktop IDE built around Rust/egui, with C++ and C# NativeAOT modules. The Windows Release build opens as a GUI application without a console window.

## Build

Install Rust stable and the .NET 9 SDK, then run:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

The GitHub Actions workflow is **manual-only**. Start it from the Actions page with **Run workflow**. The successful run publishes the `MINUX-IDE-Windows-x64` artifact.

## Hugging Face AI agent

1. Open MINUX IDE → Settings.
2. Paste a Hugging Face Access Token with Inference Providers permission. Create/manage tokens at https://huggingface.co/settings/tokens.
3. Load the available models, or type a model ID manually.
4. Save settings, open the Agent panel and send a request.

The client uses the OpenAI-compatible Hugging Face router at `https://router.huggingface.co/v1/chat/completions`. Network requests run on worker threads so the editor remains responsive. Available-model lookup also runs in the background.

The agent supports tool calls for listing the workspace, reading text files, creating directories and files, and editing files. It restricts paths to the currently opened workspace, refuses path traversal and symlink escapes, caps tool file reads/writes at 2 MiB, and does not delete files or execute shell commands. Tool calling requires a model/provider that supports function tools. The reasoning toggle sends the model's thinking flag where supported and displays returned reasoning in a collapsed section when the provider returns it.

The token is stored in the local per-user `MINUX-IDE/settings.json` without encryption. On Windows this is under `%APPDATA%\MINUX-IDE\settings.json`; do not publish or share that file.

## Supported syntax highlighting

The editor uses `egui_extras` + `Syntect` with bundled syntax definitions for:

- Rust
- TypeScript / TSX
- JavaScript / JSX
- Shell / Bash
- XML, XSL and XSLT
- Python
- C and C++
- C#
- Makefile / GNU Make / CMake / Dockerfile
- HTML, CSS, JSON, YAML, TOML, Markdown, SQL, Java, Go, Lua, PHP, Ruby, Swift, Kotlin, Perl and PowerShell
- Clojure, CoffeeScript, D, Diff/Patch, Erlang, F#, Groovy, Haskell, MATLAB, Objective-C, OCaml, R, Scala, Tcl and LaTeX

Grammar coverage depends on the definitions bundled with Syntect. Syntax highlighting is not a substitute for LSP diagnostics, code completion or refactoring; these are not implemented yet.

## Responsiveness and assets

- The workspace is indexed on a cancellable worker thread, with a 30,000-entry cap and ignored generated directories such as `target`, `node_modules`, `bin`, `obj` and `dist`.
- The file explorer and search results use row virtualization; search queries the in-memory index instead of recursively scanning disk on every frame.
- Files larger than 4 MiB are not opened in the text editor, and syntax highlighting switches to a lightweight layout for files over 500 KiB.
- SVG icons are bundled locally and loaded once as textures. Lucide and Devicon license texts are included in `assets/icons/`.

## Main libraries

- `egui` / `eframe` — native desktop UI
- `egui_extras` + `Syntect` — grammar-based syntax highlighting
- `resvg` through `egui_extras/svg` — SVG icon rendering
- `reqwest`, `serde`, `serde_json` — asynchronous-threaded Hugging Face API client and tool-call processing
- `rfd` — native file/folder dialogs
- `cc` — C++ engine compilation
- .NET 9 NativeAOT — bundled C# module
