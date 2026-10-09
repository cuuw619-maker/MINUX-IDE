# MINUX IDE

MINUX IDE is a native desktop coding environment with a Hugging Face coding agent, a project start screen, live appearance settings, and a user-triggered polyglot run command.

## Build

Requirements: Rust stable, Node.js 22+, Python 3.12+, and (for theme generation) dependencies from `requirements-dev.txt`.

```powershell
python -m pip install -r requirements-dev.txt
npm install
python scripts/generate_themes.py
python scripts/project_audit.py .
npm run typecheck
npm run test:js
cargo build --release --target x86_64-pc-windows-msvc
```

GitHub Actions is **manual-only**. Start it from the Actions page with **Run workflow**. It validates TypeScript/JavaScript, regenerates the XSLT theme catalog, audits the polyglot sources, and builds the Windows executable. No workflow is triggered by a push.

## MINUX Agent / Hugging Face

1. Open Settings and enter a Hugging Face Access Token with Inference Providers permission.
2. Select a model from the picker or enter a Hugging Face repository ID in `owner/model` form.
3. Save settings and open the Agent panel.

The client uses the OpenAI-compatible router at `https://router.huggingface.co/v1/chat/completions`. Requests run on worker threads. The model ID is validated before sending. If settings contain the previous concatenated value `zai-org/GLM-5.3Qwen/Qwen2.5-Coder-32B-Instruct`, MINUX strips the known appended default and preserves the valid prefix `zai-org/GLM-5.3`. If the cleaned ID is not available to Hugging Face, the agent gives a model-not-found explanation and asks you to select an ID from the available-model list. A genuine `model_not_found` error now explains that the chosen ID/provider is unavailable.

The agent can list workspace files, read text files, create folders and files, and edit files. Paths are restricted to the opened project, reads/writes are limited to 2 MiB, secret files are blocked, and agent tools do not delete files or execute shell commands.

The token and appearance/recent-project settings are saved locally in `%APPDATA%\\MINUX-IDE\\settings.json` on Windows, without encryption. Do not publish or share the file.

## Actual polyglot toolchain

The languages listed below are used for project functionality and developer tools, not only shown in the syntax picker:

- **TypeScript and JavaScript** — model-ID guard utilities with type-checking and executable regression tests.
- **Python** — source-tree audit and theme catalog generator.
- **XSLT + XML** — source of truth for the Ocean, Violet, Emerald and Amber theme palettes; XSLT transforms `resources/themes.xml` into the JSON catalog embedded by the native UI.
- **C** — compiled native core used for model-ID validation and panel easing.
- **Shell + Make** — repeatable developer commands through `scripts/dev.sh` and the root `Makefile`.
- **TypeScript, JavaScript, Python, Shell, C, Make, XSLT** — supported by the Run button through their local runtimes/tools when installed. Running code is always a user-initiated action.

The native desktop shell and file editor are still hosted by Rust/egui; C is the native core, while the additional language modules provide real build, validation, theme-generation and project-run functionality.

## Editor and customization

- Initial screen asks which project folder to open and shows recent folders instead of immediately opening the current working directory.
- The AI sidebar animates open/closed using the native C easing function; SVG stroke widths are reduced for a lighter icon weight.
- Settings include accent palettes, corner radius, editor font size, animations, Hugging Face model, thinking and response token limit.
- SVG icon assets are bundled locally.
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
| `Makefile` / `.mk` | GNU Make |
| `.xsl` / `.xslt` | `xsltproc`, with an adjacent XML input |

Runtimes are optional and need to be installed separately. Output and errors appear in the output panel. The run command is intended for files you explicitly choose; do not run untrusted source code.

## Developer commands

```sh
make build      # release build
make dev        # launch from source
make themes     # regenerate resources/themes.json from XML + XSLT
make audit      # check the polyglot source tree
make test       # source audit + Rust tests + TypeScript checks + JavaScript tests
# Or: scripts/dev.sh test
```
