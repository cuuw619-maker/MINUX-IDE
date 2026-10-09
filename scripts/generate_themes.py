#!/usr/bin/env python3
"""Generate the Rust-embedded palette JSON from XML through XSLT."""
from __future__ import annotations
import argparse
import json
import re
from pathlib import Path
from lxml import etree

ROOT = Path(__file__).resolve().parents[1]

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--xml", type=Path, default=ROOT / "resources" / "themes.xml")
    parser.add_argument("--xsl", type=Path, default=ROOT / "resources" / "themes.xsl")
    parser.add_argument("--output", type=Path, default=ROOT / "resources" / "themes.json")
    args = parser.parse_args()
    safe_xml = etree.XMLParser(resolve_entities=False, no_network=True, load_dtd=False)
    source = etree.parse(str(args.xml), safe_xml)
    style = etree.parse(str(args.xsl), safe_xml)
    result = json.loads(str(etree.XSLT(style)(source)))
    ids = {theme["id"] for theme in result.get("themes", [])}
    if not ids or result.get("default") not in ids:
        raise SystemExit("Invalid theme catalog: missing default or palette entries.")
    for theme in result["themes"]:
        for key in ("accent", "accent_bg"):
            if not re.fullmatch(r"#[0-9A-Fa-f]{6}", theme[key]):
                raise SystemExit(f"Invalid {key} in theme {theme.get('id')}")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"Wrote {len(ids)} themes to {args.output}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
