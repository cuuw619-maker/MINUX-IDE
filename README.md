# MINUX-IDE

Native desktop IDE foundation built with Rust/egui, a C++ engine module, and a bundled C# NativeAOT agent.

## Build

Install Rust stable and the .NET 9 SDK, then run `cargo build --release`.

The Windows x64 executable is published manually from GitHub Actions as the artifact `MINUX-IDE-Windows-x64`.

## Included libraries

- **egui_extras + Syntect** — memoized grammar-based syntax highlighting using bundled Sublime Text syntax definitions.
- **resvg** (through `egui_extras/svg`) — rasterizes bundled SVG icons into GPU textures.
- **rfd** — native file and folder dialogs.
- **cc** — builds the native C++ engine module.

## Languages and file icons

The editor selects syntax grammars based on the file extension. Included language mappings cover Rust, TypeScript/TSX, JavaScript/JSX, Python, C, C++, C#, Shell/Bash, XML/XSL/XSLT, Makefile, HTML, CSS, JSON, YAML, TOML, Markdown, SQL, Java, Go, Lua, PHP, Ruby, Swift, Kotlin, Perl, and PowerShell. Actual grammar coverage depends on the bundled Syntect syntax set.

The file tree, editor tabs, activity rail, and tool buttons use bundled SVG textures. Language logos are from [Devicon](https://github.com/devicons/devicon) (MIT); UI glyphs are from [Lucide](https://github.com/lucide-icons/lucide) (ISC). Their upstream license texts are included in `assets/icons/DEVICON-LICENSE.txt` and `assets/icons/LUCIDE-LICENSE.txt`.

## Current limitations

- AI provider requests are not connected yet; the C# agent currently acknowledges prompts.
- The output panel is a UI shell; a process terminal/build runner is not implemented.
- Language support here means file association and syntax highlighting, not yet LSP diagnostics, completion, or refactoring.
