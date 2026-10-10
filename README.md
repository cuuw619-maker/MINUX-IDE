# MINUX IDE

MINUX IDE is a native desktop coding environment with a DuckDuckGo Chat API agent, configurable appearance, language-specific icons and a user-triggered polyglot run command.

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

## MINUX Agent / DuckDuckGo Chat API

1. Open Settings and choose a model from the built-in list or refresh the model catalog from the endpoint.
2. Enter a token only if the endpoint requires one; requests without a token are attempted when the field is empty.
3. Save settings and open the Agent panel.

The client sends OpenAI-compatible chat requests to `https://duckduckgo.com/duckduckgo-html-api/v1/chat/completions`. It requests the model catalog from the sibling `/v1/models` path; if that catalog endpoint is unavailable, a built-in picker and manual model ID input remain available. Requests run on worker threads. The API token is optional and is sent as a Bearer token only when one is entered. IDs are checked for safe syntax, while the endpoint determines which models it actually supports.

The agent can inspect a bounded project overview (language mix and build manifests), list workspace files, search text across bounded project trees, read text files, create folders/files, and edit files. For cross-language tasks it is instructed to extend the existing architecture instead of defaulting to Rust for every component. It also has a safe `replace_in_file` tool that only edits a fragment when it occurs exactly once, reducing accidental broad rewrites. Temporary HTTP failures (429/5xx) receive a small bounded backoff. Paths are restricted to the opened project, reads/writes are limited to 2 MiB, secret files are blocked, and agent tools do not delete files or execute shell commands.

An optional API token and appearance/recent-project settings are saved locally in `%APPDATA%\\MINUX-IDE\\settings.json` on Windows, without encryption. Do not publish or share the file.

## Actual polyglot toolchain

The languages listed below are used for project functionality and developer tools, not only shown in the syntax picker:

- **TypeScript and JavaScript** — model-ID guard utilities with type-checking and executable regression tests.
- **Kotlin** — bounded workspace diagnostics in `kotlin/src/main/kotlin/dev/minux/WorkspaceDoctor.kt`, with JUnit tests for language counts, manifest detection, TODO scanning, ignored build folders, secret paths and scan limits.
- **Python** — source-tree audit and theme catalog generator.
- **XSLT + XML** — source of truth for the Ocean, Violet, Emerald and Amber theme palettes; XSLT transforms `resources/themes.xml` into the JSON catalog embedded by the native UI.
- **C** — compiled native core used for model-ID validation and panel easing.
- **Kotlin** — a real `WorkspaceDoctor` CLI scans language/build-manifest counts and TODO/FIXME/HACK markers under safety limits. It has automated tests, is run in manual CI, and can be launched from the editor when selecting a `.kt` file.
- **Shell + Make** — repeatable developer commands through `scripts/dev.sh` and the root `Makefile`.
- **TypeScript, JavaScript, Python, Shell, C, Make, XSLT** — supported by the Run button through their local runtimes/tools when installed. Running code is always a user-initiated action.

Responsibilities are split by purpose: Rust/egui owns the desktop host and state; C owns native search scoring, model-ID validation and animation curves; TypeScript/JavaScript own typed tooling and regression checks; Python audits the source tree and drives XSLT theme generation; XML/XSLT defines the palettes. The AI receives a language inventory before broad changes, so it can extend the existing module in the language already responsible for that job. The editor remains hosted by Rust/egui because it is a native desktop UI.

## Editor and customization

- Initial screen uses a project-launch hero, visual language badges, recent-project entries and quick access to appearance settings.
- The AI sidebar, activity rail buttons and home/editor transitions use native C easing; switching the left workspace pane animates its width. Transition speed is configurable. UI zoom ranges from 75% to 150%.
- Settings include four built-in palettes, a custom color picker, corner radius, editor font size up to 30 px, UI zoom, sidebar width from 220–420 px, icon size from 14–28 px, animation speed, selected model and response token limit. Appearance settings persist with the rest of the local settings.
- C v4 handles ranked exact/prefix/substring/fuzzy scoring for filenames and relative paths, model-ID checks, and bounded easing curves. The workspace search gives basename matches priority while still finding files by directory name.
- SVG icon assets are bundled locally with thinner stroke rendering. Language-specific icons cover Go, Java, PHP, Ruby, Swift, Kotlin, Lua, SQL, Dart, Perl, R, Scala, Haskell, Clojure, Erlang, Elixir, Vue, Svelte, PowerShell, GraphQL, Docker, CMake and D.
- Workspace indexing runs in a cancellable worker; generated directories are ignored and tree/search rows are virtualized.

## Run button

The **Run** toolbar button runs the currently selected and saved file on an explicit user click. The command is passed as an argument list (not through a shell), and the selected file must be inside the workspace.

| File type | Runtime |
|---|---|
| `.ts` / `.tsx` | Node type stripping or local `tsx` |
| `.js` / `.mjs` / `.cjs` | Node.js |
| `.py` | `py -3` or `python` |
| `.sh` | Bash or sh |
| `.ps1` | PowerShell |
| `.c` | GCC/Clang (`cc`) compile, then run |
| `.kt` | Kotlin compiler (`kotlinc`) → temporary JAR → Java |
| `.kts` | Kotlin script runner (`kotlinc -script`) |
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
