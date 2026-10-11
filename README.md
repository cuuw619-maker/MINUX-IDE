<div align="center">
  <img src="assets/minux-logo.svg" alt="MINUX IDE logo" width="128" height="128" />
  <h1>MINUX IDE</h1>
  <p>Native polyglot coding environment with an integrated AI coding agent.</p>
</div>

---

MINUX IDE is a native desktop coding environment with a SambaNova Cloud coding agent, configurable appearance, high-resolution language icons and a user-triggered polyglot run command.

## Build

Requirements: Rust stable, JDK 21, Gradle 8.14.3+, Node.js 22+, Python 3.12+, and (for theme generation) dependencies from `requirements-dev.txt`.

```powershell
python -m pip install -r requirements-dev.txt
npm install
python scripts/generate_themes.py
python scripts/project_audit.py .
npm run typecheck
npm run test:js
gradle -p kotlin test
gradle -p kotlin run --args=".."
cargo build --release --target x86_64-pc-windows-msvc
```

GitHub Actions is **manual-only**. Start it from the Actions page with **Run workflow**. It validates TypeScript/JavaScript, regenerates the XSLT theme catalog, audits the polyglot sources, and builds the Windows executable. No workflow is triggered by a push.

## MINUX Agent / SambaNova Cloud API

1. Open Settings and choose a model from the built-in list or refresh the model catalog from the endpoint.
2. Enter a token only if the endpoint requires one; requests without a token are attempted when the field is empty.
3. Save settings and open the Agent panel.

The AI API Base URL defaults to `https://api.sambanova.ai/v1`. The website `cloud.sambanova.ai` is used to manage your account and API key; it is not the API endpoint. MINUX appends `/chat/completions` and `/models` to the configured Base URL and automatically migrates old DuckDuckGo settings to SambaNova on startup, clearing any obsolete provider token and selecting a SambaNova model. Refresh the model list to see the current catalog. Automatic HTTP redirects are disabled so the app reports the original status and `Location` header instead of silently following a redirect to an HTML page. HTTP 410 and HTML responses are explained rather than inserted into chat as model output. Requests run on worker threads; the API key is saved locally in `%APPDATA%\\MINUX-IDE\\settings.json` without encryption.

The agent can inspect a bounded project overview (language mix and build manifests), list workspace files, search text across bounded project trees, read text files, create folders/files, and edit files. For cross-language tasks it is instructed to extend the existing architecture instead of defaulting to Rust for every component. It also has a safe `replace_in_file` tool that only edits a fragment when it occurs exactly once, reducing accidental broad rewrites. Temporary HTTP failures (429/5xx) receive a small bounded backoff. Paths are restricted to the opened project, reads/writes are limited to 2 MiB, secret files are blocked, and agent tools do not delete files or execute shell commands.

An optional API token and appearance/recent-project settings are saved locally in `%APPDATA%\\MINUX-IDE\\settings.json` on Windows, without encryption. Do not publish or share the file.

## Actual polyglot toolchain

The languages listed below are used for project functionality and developer tools, not only shown in the syntax picker:

- **TypeScript and JavaScript** — model-ID guard utilities with type-checking and executable regression tests.
- **Kotlin** — bounded workspace diagnostics in `kotlin/src/main/kotlin/dev/minux/WorkspaceDoctor.kt`, with JUnit tests for language counts, manifest detection, TODO scanning, ignored build folders, secret paths and scan limits.
- **Python** — source-tree audit and theme catalog generator.
- **XSLT + XML** — source of truth for the Ocean, Violet, Emerald and Amber theme palettes; XSLT transforms `resources/themes.xml` into the JSON catalog embedded by the native UI.
- **C** — compiled native core used for model-ID validation and panel easing.
- **Shell + Make** — repeatable developer commands through `scripts/dev.sh` and the root `Makefile`.
- **Run button runtimes** — Rust (standalone `rustc` files), C/C++/Assembly/Fortran (local compiler), Go, C# projects, Java, Kotlin, TypeScript/JavaScript, Python, Shell/PowerShell, Ruby, PHP, Lua, Perl, Swift, Haskell, Scala, Julia, Zig, Clojure, Erlang, OCaml, D, Nix, Elixir, R, Dart, F#, Groovy, CoffeeScript, Tcl, Make, XSLT and Terraform validation where the relevant toolchain is installed. Running code is always a user-initiated action.

Responsibilities are split by purpose: Rust/egui owns the desktop host and state; C owns native search scoring, model-ID validation and animation curves; TypeScript/JavaScript own typed tooling and regression checks; Python audits the source tree and drives XSLT theme generation; XML/XSLT defines the palettes. The AI receives a language inventory before broad changes, so it can extend the existing module in the language already responsible for that job. The editor remains hosted by Rust/egui because it is a native desktop UI.

## Editor and customization

- Initial screen uses a project-launch hero, visual language badges, recent-project entries and quick access to appearance settings.
- The AI sidebar fades and slides open/closed, the output panel expands/collapses, hover states respect the animation toggle/speed, and one shared capsule-shaped selection indicator glides between activity-rail buttons with a smooth eased transition. The initial screen has an animated accent glow and floating logo, icon-first project actions and readable recent-project rows. Explorer and AI panel boundaries have wide drag targets and clearer resize grips. The workspace sidebar stays mounted when switching Explorer/Search/Languages, preventing scrollbars from abruptly vanishing. Transition speed is configurable. UI zoom ranges from 75% to 150%.
- Settings include four built-in palettes, custom accent color, corner radius (bounded to avoid distorting compact controls), editor font size up to 30 px, UI zoom, Explorer width (220–520 px), AI panel width (240–520 px), output panel height (120–480 px), icon size (14–28 px), animation speed, selected model and response token limit. Explorer and AI widths can be resized by dragging the foreground grip centered on each panel boundary; the grip stays interactive above scrollbars and child widgets, and the new width is saved when dragging stops. The output panel has a draggable split bar. These dimensions persist with the rest of the local settings.
- C v4 handles ranked exact/prefix/substring/fuzzy scoring for filenames and relative paths, model-ID checks, and bounded easing curves. The workspace search gives basename matches priority while still finding files by directory name.
- SVG icon assets are bundled locally and rasterized at 128×128 before display to keep large home-screen symbols sharp; normalization fixes `currentColor`/black-only glyphs while preserving native Devicon brand colors. Hex color literals (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`) in the editor get a color-matched foreground, subtle swatch and underline on top of the normal syntax theme. Missing or invalid textures fall back to rounded tiles with readable ASCII monograms instead of blank squares or missing-glyph symbols. The language catalog includes distinct SVGs for more than 50 languages and tools, including Go, Java, Kotlin, Swift, Julia, Zig, Solidity, Terraform/HCL, Nix, Fortran, OCaml, Objective-C, MATLAB, Docker, GraphQL and CMake.
- Workspace indexing runs in a cancellable worker; generated directories are ignored and tree/search rows are virtualized.

## Run button

The **Run** toolbar button runs the currently selected and saved file on an explicit user click. The command is passed as an argument list (not through a shell), and the selected file must be inside the workspace.

| File type | Runtime |
|---|---|
| `.ts` / `.mts` / `.cts` / `.tsx` | Node type stripping or local `tsx` |
| `.js` / `.mjs` / `.cjs` | Node.js |
| `.py` | `py -3` or `python` |
| `.sh` / `.bash` | Bash or sh |
| `.ps1` | PowerShell |
| `.rs` | `rustc` compile to a temporary binary, then run (standalone file) |
| `.c` / `.cpp` / `.cc` / `.cxx` | GCC/Clang compile to a temporary binary, then run |
| `.s` / `.asm` | GCC/Clang assembler/linker, then run |
| `.f` / `.f90` / `.f95` / `.f03` / `.f08` | `gfortran`, then run |
| `.go` | `go run` |
| `.cs` | `dotnet run` for the nearest `.csproj` |
| `.java` | Java source launcher |
| `.kt` | Kotlin compiler (`kotlinc`) → temporary JAR → Java |
| `.kts` | Kotlin script runner (`kotlinc -script`) |
| `.rb` / `.php` / `.lua` / `.pl` | Ruby / PHP / Lua / Perl runtimes |
| `.swift` / `.ex` / `.exs` / `.r` | Swift / Elixir / R runtimes |
| `.hs` / `.lhs` / `.scala` / `.jl` / `.zig` | Haskell / Scala / Julia / Zig runtimes |
| `.clj` / `.cljc` / `.erl` / `.ml` | Clojure / Erlang / OCaml runtimes |
| `.fsx` / `.groovy` / `.coffee` / `.tcl` / `.d` / `.nix` | Corresponding local language runtime |
| `.tf` | `terraform validate` for the containing project |
| `Makefile` / `.mk` | GNU Make |
| `.xsl` / `.xslt` | `xsltproc`, with an adjacent XML input |

Runtimes are optional and need to be installed separately. Output and errors appear in the output panel. The run command is intended for files you explicitly choose; do not run untrusted source code.

## Developer commands

```sh
make build      # release build
make dev        # launch from source
make themes     # regenerate resources/themes.json from XML + XSLT
make audit      # check the polyglot source tree
make test       # source audit + Rust/Kotlin tests + TypeScript checks + JavaScript tests
make kotlin-test # run Kotlin workspace diagnostic tests
# Or: scripts/dev.sh test
```
