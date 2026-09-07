#!/usr/bin/env python3
"""Freeze SwiftData model definitions for a released schema version.

A `VersionedSchema` identifies a store by the checksum of the entities it
declares, so a released version may only reference model types whose shape
never changes again. This script copies each live `@Model` class as it
existed at a given commit into a nested type of the requested schema
enum (`extension DashSchemaV1 { final class PersistentX { ... } }`), one
file per model, under `Persistence/FrozenSchemas/`.

The class body and the model's own extensions are copied; doc comments,
`public` modifiers and top-level enums are dropped. Extensions add no
stored properties (so they are not part of the entity) but the class body
may call into them, and the live definitions of the shared non-model types
(for example `MasternodeStatus`) stay the single source. The stored
properties, their optionality and defaults, `@Attribute`, `@Relationship`,
`#Index` and `#Unique` are what the checksum hashes, and they are copied
verbatim.

Value types a model stores inline (Codable structs and raw enums SwiftData
expands into composite attributes, such as `ChangeControlRules` on
`PersistentToken`) are entity-hash inputs too, so they are frozen the same
way: `--value-types-file` names the live file they come from and
`--value-types` the types to copy into one nested file, and every frozen
model body then resolves those names to the nested copies.

Usage:
    freeze_schema_models.py --commit <sha> --schema DashSchemaV1 \
        [--value-types-file Persistence/Types/TokenTypes.swift \
         --value-types ChangeControlRules ...] \
        PersistentWallet PersistentAccount ...

Run it from anywhere inside the repository. Regenerating a frozen file from
the same commit is a no-op, which is how a reviewer checks one.
"""

import argparse
import os
import re
import subprocess
import sys

MODELS_DIR = "packages/swift-sdk/Sources/SwiftDashSDK/Persistence/Models"
OUT_DIR = "packages/swift-sdk/Sources/SwiftDashSDK/Persistence/FrozenSchemas"


def repo_root():
    return subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True
    ).strip()


def source_at(commit, model):
    path = f"{MODELS_DIR}/{model}.swift"
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], text=True)


def strip_comments(lines):
    """Drop comment lines and `public`, collapsing the blank runs left behind."""
    out = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("///") or stripped.startswith("//"):
            continue
        line = re.sub(r"\bpublic(\(set\))? ", "", line)
        if line.strip() == "" and out and out[-1].strip() == "":
            continue
        out.append(line)
    while out and out[-1].strip() == "":
        out.pop()
    return out


def block_end(lines, start):
    """Index of the line closing the brace block that opens at `start`.

    Braces inside comment lines are ignored so a `// { ... }` note inside a
    body cannot truncate the copy.
    """
    depth = 0
    opened = False
    for i in range(start, len(lines)):
        stripped = lines[i].strip()
        if stripped.startswith("//"):
            continue
        depth += lines[i].count("{") - lines[i].count("}")
        opened = opened or "{" in lines[i]
        if depth == 0 and opened:
            return i
    raise SystemExit("unbalanced braces")


def extract_value_type(source, name):
    """A top-level `struct <name>` / `enum <name>` block, comments stripped."""
    lines = source.splitlines()
    for i, line in enumerate(lines):
        if re.match(rf"^(public )?(struct|enum) {name}\b", line):
            return strip_comments(lines[i : block_end(lines, i) + 1])
    raise SystemExit(f"{name}: no top-level struct or enum found")


def extract_class(source, model):
    """The `@Model ... final class <model> { ... }` block, comments stripped."""
    lines = source.splitlines()
    start = None
    for i, line in enumerate(lines):
        if re.match(rf"^(public )?final class {model}\b", line):
            start = i
            break
    if start is None:
        raise SystemExit(f"{model}: no top-level class found")
    # Include the macro attributes directly above the class (`@Model`).
    while start > 0 and lines[start - 1].startswith("@"):
        start -= 1
    end = block_end(lines, start)
    return strip_comments(lines[start : end + 1])


def extract_extensions(source, model):
    """Every top-level `extension <model> { ... }` body, comments stripped."""
    lines = source.splitlines()
    bodies = []
    for i, line in enumerate(lines):
        if re.match(rf"^(public )?extension {model}\s*(:[^{{]*)?\{{", line):
            end = block_end(lines, i)
            inner = strip_comments(lines[i + 1 : end])
            conformance = re.search(r"extension \w+\s*(:[^{]*)\{", line)
            bodies.append(((conformance.group(1).strip() if conformance else ""), inner))
    return bodies


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--commit", required=True, help="commit the live shapes are taken from")
    parser.add_argument("--schema", required=True, help="e.g. DashSchemaV1")
    parser.add_argument(
        "--value-types-file",
        help="repo-relative file the inline value types are copied from",
    )
    parser.add_argument("--value-types", nargs="*", default=[])
    parser.add_argument("models", nargs="+")
    args = parser.parse_args()
    root = repo_root()
    sha = subprocess.check_output(
        ["git", "rev-parse", "--short=10", args.commit], text=True, cwd=root
    ).strip()
    header = (
        "import Foundation\n"
        "import SwiftData\n"
        "\n"
    )
    if args.value_types:
        source = subprocess.check_output(
            ["git", "show", f"{args.commit}:{args.value_types_file}"], text=True
        )
        bodies = []
        for name in args.value_types:
            body = extract_value_type(source, name)
            bodies.append("\n".join(("    " + line) if line.strip() else "" for line in body))
        text = (
            header
            + f"// Inline value types exactly as schema {args.schema} stored them, generated\n"
            f"// by scripts/freeze_schema_models.py from {os.path.basename(args.value_types_file)}\n"
            f"// at commit {sha}. SwiftData expands a stored Codable struct into composite\n"
            "// attributes of the owning entity, so these shapes are inputs to that\n"
            "// version's checksum just like the model's own properties. Do not edit.\n"
            f"extension {args.schema} {{\n"
            + "\n\n".join(bodies)
            + "\n}\n"
        )
        out = os.path.join(
            root, OUT_DIR, f"{args.schema}+{os.path.basename(args.value_types_file)}"
        )
        with open(out, "w") as f:
            f.write(text)
        print(out)
    qualified = list(args.models) + list(args.value_types)
    for model in args.models:
        source = source_at(args.commit, model)
        body = extract_class(source, model)
        indented = "\n".join(("    " + line) if line.strip() else "" for line in body)
        extensions = ""
        # An extension of a nested type does not see its sibling nested
        # types by bare name (that lookup lands on the live top-level type),
        # so model and frozen value-type names inside extension bodies are
        # qualified.
        sibling = re.compile(
            r"(?<![\w.])(" + "|".join(re.escape(m) for m in qualified) + r")\b"
        )
        for conformance, inner in extract_extensions(source, model):
            inner = [sibling.sub(rf"{args.schema}.\1", line) for line in inner]
            extensions += (
                f"\nextension {args.schema}.{model}{' ' + conformance if conformance else ''} {{\n"
                + "\n".join(inner)
                + "\n}\n"
            )
        text = (
            header
            + f"// `{model}` exactly as schema {args.schema} registered it, generated by\n"
            f"// scripts/freeze_schema_models.py from the live model at commit {sha}.\n"
            "// Do not edit: every stored property, its optionality and default, and\n"
            "// every @Attribute / @Relationship / #Index / #Unique here is an input to\n"
            "// that version's checksum, and changing any of them re-breaks the stores\n"
            "// this copy exists to keep openable. See the live model for what each\n"
            "// column means.\n"
            f"extension {args.schema} {{\n"
            f"{indented}\n"
            "}\n"
            + extensions
        )
        out = os.path.join(root, OUT_DIR, f"{args.schema}+{model}.swift")
        with open(out, "w") as f:
            f.write(text)
        print(out)


if __name__ == "__main__":
    main()
