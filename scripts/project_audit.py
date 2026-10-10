#!/usr/bin/env python3
"""Dependency-free audit of the MINUX polyglot source tree."""
from __future__ import annotations
import json
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
    "TypeScript": {".ts", ".tsx"}, "JavaScript": {".js", ".mjs", ".cjs"},
    "Python": {".py"}, "Shell": {".sh", ".bash"}, "C": {".c", ".h"},
    "XSLT": {".xsl", ".xslt"}, "Make": {".mk"},
    "Kotlin": {".kt", ".kts"},
}

def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    missing = [item for item in REQUIRED if not (root / item).is_file()]
    if missing:
        print("MINUX source audit failed: " + ", ".join(missing), file=sys.stderr)
        return 1
    counts: Counter[str] = Counter()
    for path in root.rglob("*"):
        if not path.is_file() or any(part in {".git", "target", "node_modules", ".gradle", "build", "out", "dist"} for part in path.parts):
            continue
        for language, extensions in LANGUAGE_EXTENSIONS.items():
            if path.suffix.lower() in extensions or (language == "Make" and path.name == "Makefile"):
                counts[language] += 1
    print(json.dumps({"status": "ok", "languages": dict(sorted(counts.items()))}, ensure_ascii=False, indent=2))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
