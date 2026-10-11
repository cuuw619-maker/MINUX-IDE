#!/usr/bin/env python3
"""Dependency-free audit of MINUX's polyglot source tree and embedded SVG assets."""
from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path

REQUIRED = (
    "Cargo.toml", "Makefile", "build.rs", "src/main.rs", "src/ai.rs",
    "native/c/engine.c", "web/model_id.ts", "web/model_id.mjs",
    "web/model_id.test.mjs", "scripts/dev.sh", "scripts/generate_themes.py",
    "resources/themes.xml", "resources/themes.xsl", "resources/themes.json",
    "src/runner.rs", "src/theme.rs",
    "kotlin/settings.gradle.kts", "kotlin/build.gradle.kts",
    "kotlin/src/main/kotlin/dev/minux/WorkspaceDoctor.kt",
    "kotlin/src/test/kotlin/dev/minux/WorkspaceDoctorTest.kt",
)

LANGUAGE_EXTENSIONS = {
    "Rust": {".rs"},
    "C": {".c", ".h"},
    "C++": {".cc", ".cpp", ".cxx", ".hpp", ".hh", ".hxx", ".c++"},
    "Assembly": {".s", ".asm"},
    "C#": {".cs"},
    "F#": {".fs", ".fsx", ".fsi"},
    "TypeScript": {".ts", ".tsx", ".mts", ".cts"},
    "JavaScript": {".js", ".jsx", ".mjs", ".cjs"},
    "Python": {".py", ".pyw"},
    "Shell": {".sh", ".bash"},
    "PowerShell": {".ps1", ".psm1", ".psd1"},
    "Kotlin": {".kt", ".kts"},
    "Java": {".java"},
    "Go": {".go"},
    "Swift": {".swift"},
    "Ruby": {".rb", ".rake"},
    "PHP": {".php"},
    "Lua": {".lua"},
    "Perl": {".pl", ".pm"},
    "Dart": {".dart"},
    "Scala": {".scala", ".sc"},
    "Haskell": {".hs", ".lhs"},
    "Clojure": {".clj", ".cljs", ".cljc", ".cljx", ".edn"},
    "Erlang": {".erl", ".hrl"},
    "Elixir": {".ex", ".exs"},
    "R": {".r", ".rmd"},
    "Julia": {".jl"},
    "Zig": {".zig"},
    "Solidity": {".sol"},
    "Terraform": {".tf", ".tfvars", ".hcl"},
    "Nix": {".nix"},
    "Fortran": {".f", ".for", ".f77", ".f90", ".f95", ".f03", ".f08"},
    "OCaml": {".ml", ".mli"},
    "Objective-C": {".mm"},
    "MATLAB": {".m"},
    "Vue": {".vue"},
    "Svelte": {".svelte"},
    "SQL": {".sql"},
    "HTML": {".html", ".htm"},
    "CSS": {".css", ".scss", ".sass", ".less"},
    "XML": {".xml", ".xsd", ".dtd"},
    "JSON": {".json", ".jsonc", ".json5", ".jsonl"},
    "YAML": {".yaml", ".yml"},
    "TOML": {".toml"},
    "Markdown": {".md", ".markdown", ".mdx"},
    "GraphQL": {".graphql", ".gql"},
    "CoffeeScript": {".coffee"},
    "Tcl": {".tcl"},
    "LaTeX": {".tex", ".ltx"},
    "Diff": {".diff", ".patch"},
    "XSLT": {".xsl", ".xslt"},
}
LANGUAGE_FILENAMES = {
    "Make": {"Makefile", "GNUmakefile", "makefile", "gnumakefile"},
    "Docker": {"Dockerfile", "Containerfile", "dockerfile", "containerfile"},
    "CMake": {"CMakeLists.txt", "cmakelists.txt"},
    "Just": {"Justfile", "justfile"},
}
IGNORED_PARTS = {".git", "target", "node_modules", ".gradle", "build", "out", "dist", ".venv", "venv"}


def audit_embedded_icons(root: Path, main_rs: str) -> list[str]:
    icon_paths = re.findall(r'include_str!\("\.\./assets/icons/([^"]+\.svg)"\)', main_rs)
    missing_files = sorted({path for path in icon_paths if not (root / "assets" / "icons" / path).is_file()})
    if missing_files:
        return [f"missing embedded SVG: assets/icons/{path}" for path in missing_files]
    duplicates = sorted({path for path in icon_paths if icon_paths.count(path) > 1})
    if duplicates:
        return [f"SVG embedded more than once: assets/icons/{path}" for path in duplicates]

    mapped_block = main_rs.split("fn file_icon_key_for_path", 1)
    if len(mapped_block) != 2:
        return ["file_icon_key_for_path mapping is missing"]
    mapped_block = mapped_block[1].split("fn syntax_selector_for_path", 1)[0]
    mapped_keys = set(re.findall(r'(?:=>|return)\s*"(lang-[\w-]+)"', mapped_block))
    registered_keys = set(re.findall(r'\("(lang-[\w-]+)",\s*include_str!', main_rs))
    unregistered = sorted(mapped_keys - registered_keys)
    return [f"mapped language icon is not embedded: {key}" for key in unregistered]


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    missing = [item for item in REQUIRED if not (root / item).is_file()]
    if missing:
        print("MINUX source audit failed: missing required files: " + ", ".join(missing), file=sys.stderr)
        return 1

    main_rs = (root / "src/main.rs").read_text(encoding="utf-8")
    icon_errors = audit_embedded_icons(root, main_rs)
    if icon_errors:
        print("MINUX source audit failed: " + "; ".join(icon_errors), file=sys.stderr)
        return 1

    counts: Counter[str] = Counter()
    for path in root.rglob("*"):
        if not path.is_file() or any(part in IGNORED_PARTS for part in path.parts):
            continue
        filename = path.name
        suffix = path.suffix.lower()
        for language, names in LANGUAGE_FILENAMES.items():
            if filename in names:
                counts[language] += 1
        for language, extensions in LANGUAGE_EXTENSIONS.items():
            if suffix in extensions:
                counts[language] += 1

    print(json.dumps(
        {
            "status": "ok",
            "embedded_svg_references": len(re.findall(r'include_str!\("\.\./assets/icons/([^"]+\.svg)"\)', main_rs)),
            "languages_with_source_inventory": sum(count > 0 for count in counts.values()),
            "languages": dict(sorted(counts.items())),
        },
        ensure_ascii=False,
        indent=2,
    ))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
