#!/usr/bin/env python3
"""
Generate the wasm entity conversion inventory for wasm-dpp2.

Scans src/ for `pub struct *Wasm` definitions and records which wasm-bindgen
`to*` / `from*` helpers each entity implements. Results are written to
WASM_ENTITY_CONVERSIONS.md in the package root.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Dict, List, Set, Tuple

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = PACKAGE_ROOT.parents[1]
SRC_DIR = PACKAGE_ROOT / "src"
OUTPUT_FILE = PACKAGE_ROOT / "WASM_ENTITY_CONVERSIONS.md"


HEADER = """# wasm-dpp2 entity conversion map

This file is generated on demand. It lists every exported `*Wasm` entity in `packages/wasm-dpp2` and the wasm-bindgen `to*`/`from*` helpers they expose as of the current workspace state.

## Task context

We want a uniform interface across all wasm-dpp2 entities (each `*Wasm` wrapper maps 1:1 to an `rs-dpp` type). The expectations:

- `toObject` returns a `JsValue` containing a plain JS object; binary data should be emitted as `Uint8Array`.
- `fromObject` accepts a plain JS object (not an existing wasm wrapper) and instantiates the entity.
- `toJSON` returns a `JsValue` representing the JSON form, using string encodings for binary fields; `fromJSON` reverses it.
- `toBytes`/`fromBytes` are mandatory alongside the object/JSON helpers so callers can move raw binary data without extra conversions.
- `toBase64`, `toHex`, `toBase58` (and similar conversions) remain optional and can be added per-entity when useful.
- Prefer reusing the underlying `rs-dpp` serialization helpers (e.g., `to_object`, `from_object`, serde Platform serialization). Platform Value already round-trips to/from JSON, so lean on those helpers rather than reimplementing logic in wasm.

This catalog helps track which entities already expose these conversions and which still need work.

| Entity | Source file | `to*` methods | `from*` methods |
| --- | --- | --- | --- |
"""


def gather_structs() -> Dict[str, Dict[str, Set[str]]]:
    """Return mapping of entity name -> metadata."""
    struct_map: Dict[str, Dict[str, Set[str]]] = {}

    impl_pattern = re.compile(r"\s*impl\s+(\w+Wasm)\s*\{")
    struct_pattern = re.compile(r"pub\s+struct\s+(\w+Wasm)")
    js_name_pattern = re.compile(r'js_name\s*=\s*"([^"]+)"')

    for path in SRC_DIR.rglob("*.rs"):
        rel_path = path.relative_to(REPO_ROOT)
        text = path.read_text()

        for match in struct_pattern.finditer(text):
            name = match.group(1)
            entry = struct_map.setdefault(
                name, {"file": str(rel_path), "to": set(), "from": set()}
            )
            entry["file"] = str(rel_path)

        lines = text.splitlines()
        current_impl = None
        brace_depth = 0

        for line in lines:
            if current_impl is None:
                match = impl_pattern.match(line)
                if match:
                    current_impl = match.group(1)
                    brace_depth = line.count("{") - line.count("}")
                continue

            brace_depth += line.count("{") - line.count("}")

            if "#[wasm_bindgen" in line and "js_name" in line:
                js_match = js_name_pattern.search(line)
                if js_match:
                    method = js_match.group(1)
                    entry = struct_map.setdefault(
                        current_impl, {"file": str(rel_path), "to": set(), "from": set()}
                    )
                    if method.startswith("to") and len(method) > 2 and method[2].isupper():
                        entry["to"].add(method)
                    elif (
                        method.startswith("from")
                        and len(method) > 4
                        and method[4].isupper()
                    ):
                        entry["from"].add(method)

            if brace_depth <= 0:
                current_impl = None
                brace_depth = 0

    return struct_map


def format_rows(struct_map: Dict[str, Dict[str, Set[str]]]) -> List[str]:
    rows: List[str] = []
    for name in sorted(struct_map):
        info = struct_map[name]
        to_methods = ", ".join(sorted(info["to"])) if info["to"] else "—"
        from_methods = ", ".join(sorted(info["from"])) if info["from"] else "—"
        rows.append(
            f"| `{name}` | `{info['file']}` | {to_methods} | {from_methods} |\n"
        )
    return rows


def main() -> None:
    struct_map = gather_structs()
    rows = format_rows(struct_map)
    OUTPUT_FILE.write_text(HEADER + "".join(rows))
    print(f"Wrote {len(rows)} entries to {OUTPUT_FILE}")


if __name__ == "__main__":
    main()
