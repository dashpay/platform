#!/usr/bin/env python3
"""Generate the frozen SwiftData model copies for released schema versions.

A `VersionedSchema` identifies a store by the checksum of the entities it
declares, so a released version may only reference model types whose shape
never changes again. Pointing a released version at a live `@Model` type
means the next property added to that type silently changes the released
checksum: a store written by the previously shipped build then matches no
registered version and fails to open with Cocoa error 134504 ("Cannot use
staged migration with an unknown model version") instead of migrating.

This script copies each live `@Model` class as it existed at a given commit
into a nested type of the schema enum that version registers
(`extension DashSchemaV1 { final class PersistentX { ... } }`), one file per
model, under `Persistence/FrozenSchemas/`. SwiftData derives the entity name
from the unqualified type name, so `DashSchemaV1.PersistentX` and the live
`PersistentX` describe the same entity, which is what lets a migration stage
map one onto the other.

The class body and the extensions declared in the model's own file are
copied; doc comments, `public` modifiers and top-level enums are dropped.
Extensions add no stored properties (so they are not part of the entity)
but the class body may call into them. The stored properties, their
optionality and defaults, `@Attribute`, `@Relationship` and `#Unique` are
what the checksum hashes, and they are copied verbatim. `#Index` is copied
verbatim too but is NOT part of the hash (Core Data leaves indexes out of
entity version hashes), which is why index drift needs its own check.

Value types a model stores inline (Codable structs and raw enums SwiftData
expands into composite attributes, such as `ChangeControlRules` on
`PersistentToken`) are entity-hash inputs too, so they are frozen the same
way, into one nested file per schema, and every frozen model body then
resolves those names to the nested copies. Names are qualified per schema:
a model frozen only under `DashSchemaV2` that mentioned a `DashSchemaV1`
model by bare name in an extension would still bind to the live type.

Every released version is frozen as a whole graph, never partially: a
relationship binds its destination by entity name, and SwiftData resolves
that name to whichever Swift type claimed it first in the process, so a
frozen model whose relationship pointed at a live type could be hashed with
the live type's current shape.

`FREEZES` below is the record of what each released version registers and
the commit its shapes are taken from. Rows are append-only: retiring a
version means adding rows for it (normally one row listing every model at
the last commit before the change), adding the new `DashSchemaVN` and a
migration stage, and rerunning this script. Never edit an existing row; a
released checksum cannot move.

Nothing here checks that the table is COMPLETE. That is deliberate. A
frozen model whose relationship target or stored value type is missing
from the table binds that bare name to the live type, and the released
checksum then moves with the live type's next change; but whether a given
Swift reference feeds the entity hash is decided by SwiftData (a struct
stored directly on a model does, an array of structs nested inside one does
not), and a text scan of Swift source cannot know that, nor keep up with
optionals, generics, extensions, nested types and enum payloads. Every
reference such a scan misses is a silent failure in the field, and every
one it wrongly flags is a false alarm. The authority for
hash-relevant completeness (properties, relationships, `#Unique`) is
`DashModelMigrationTests.testFrozenVersionsBuiltAfterTheLiveSchemaHashLikeTheStoresTheyShipped`,
which builds each released version after the live schema and compares the
hashes SwiftData computes against a store the shipping build wrote; its
sibling `testFixturesAndMigratedStoresCarryTheIndexesFreshStoresHave`
covers `#Index`, which the hash cannot see, by comparing SQLite indexes.
Do not add static validation here; extend those tests (and their
fixtures) instead.

Usage, from anywhere inside the repository:

    scripts/freeze_schema_models.py            # regenerate every frozen file
    scripts/freeze_schema_models.py --check    # exit 1 if any file would change

`--check` is a regeneration check and nothing more: the frozen files are a
pure function of `FREEZES` and the repository history, so a clean check
proves that the committed files are exactly this generator's output, byte
for byte, and that no one edited a frozen copy by hand. It says nothing
about whether the freeze is complete. CI runs it (the
`swift-sdk-frozen-schema` job in `.github/workflows/tests.yml`) on a
full-history checkout, because it needs the commits named in `FREEZES`.

The generator's own tests, from the repository root:

    python3 -m unittest discover -s packages/swift-sdk/scripts -p 'test_*.py'
"""

import argparse
import dataclasses
import os
import re
import subprocess
import sys

MODELS_DIR = "packages/swift-sdk/Sources/SwiftDashSDK/Persistence/Models"
OUT_DIR = "packages/swift-sdk/Sources/SwiftDashSDK/Persistence/FrozenSchemas"
TOKEN_TYPES_FILE = "packages/swift-sdk/Sources/SwiftDashSDK/Persistence/Types/TokenTypes.swift"
# The value types `PersistentToken` stores inline, plus the enums their
# initializers reference.
TOKEN_VALUE_TYPES = [
    "ChangeControlRules",
    "AuthorizedActionTakers",
    "TokenPerpetualDistribution",
    "TokenPreProgrammedDistribution",
    "DistributionEvent",
    "TokenDistributionChangeRules",
    "TokenTradeMode",
    "TokenLocalization",
]

# Every model registered by the versions that share the V1 graph, in the
# order `DashModelContainer` lists them, minus the two that have their own
# rows below (`PersistentAssetLock`, whose shape differs between V2 and V3,
# and `PersistentTrackedMasternode`, which V2 added).
V1_GRAPH_MODELS = [
    "PersistentIdentity",
    "PersistentDPNSName",
    "PersistentDashpayProfile",
    "PersistentDashpayContactProfile",
    "PersistentDashpayContactRequest",
    "PersistentDashpayPayment",
    "PersistentDashpayIgnoredSender",
    "PersistentDocument",
    "PersistentDataContract",
    "PersistentPublicKey",
    "PersistentTokenBalance",
    "PersistentKeyword",
    "PersistentToken",
    "PersistentDocumentType",
    "PersistentIndex",
    "PersistentProperty",
    "PersistentTokenHistoryEvent",
    "PersistentPlatformAddress",
    "PersistentPlatformAddressesSyncState",
    "PersistentWallet",
    "PersistentAccount",
    "PersistentCoreAddress",
    "PersistentTransaction",
    "PersistentTxo",
    "PersistentPendingInput",
    "PersistentWalletManagerMetadata",
    "PersistentShieldedNote",
    "PersistentShieldedOutgoingNote",
    "PersistentShieldedSyncState",
    "PersistentShieldedActivity",
    "PersistentShieldedViewingKey",
    "PersistentInvitation",
    "PersistentMasternode",
]


@dataclasses.dataclass(frozen=True)
class Freeze:
    """One schema's copy of some models, taken from one commit."""

    schema: str
    commit: str
    models: tuple
    value_types_file: str = ""
    value_types: tuple = ()


FREEZES = [
    # The asset lock as V1 and V2 shipped it: the last commit before
    # `recipientIsExternal` was added to the live model.
    Freeze("DashSchemaV1", "7127c38566", ("PersistentAssetLock",)),
    # The rest of the graph, shared by V1, V2 and V3, at the last commit
    # before V4 widened the wallet transaction models.
    Freeze(
        "DashSchemaV1",
        "5f58417079",
        tuple(V1_GRAPH_MODELS),
        TOKEN_TYPES_FILE,
        tuple(TOKEN_VALUE_TYPES),
    ),
    # V2 adds the tracked-masternode registry.
    Freeze("DashSchemaV2", "5f58417079", ("PersistentTrackedMasternode",)),
    # V3 replaces the asset lock with the shape that has `recipientIsExternal`.
    Freeze("DashSchemaV3", "5f58417079", ("PersistentAssetLock",)),
]

HEADER = "import Foundation\nimport SwiftData\n\n"


def git(root, *args):
    try:
        return subprocess.check_output(
            ["git", *args], text=True, encoding="utf-8", cwd=root
        )
    except subprocess.CalledProcessError as error:
        raise SystemExit(
            f"git {' '.join(args)} failed (exit {error.returncode}); a commit named in "
            "FREEZES may not be fetched locally"
        )


def repo_root():
    return git(os.getcwd(), "rev-parse", "--show-toplevel").strip()


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


def code_only(line):
    """`line` with string literal contents and any trailing `//` comment removed.

    What remains is what brace counting and declaration matching may look
    at: a `{` or a type name inside a string or a comment is not code.
    Block comments are not parsed, so one is refused rather than risk a
    silently truncated copy.
    """
    if "/*" in line:
        raise SystemExit(f"block comments are not supported: {line.strip()}")
    line = re.sub(r'"(?:\\.|[^"\\])*"', '""', line)
    return line.split("//", 1)[0]


def braces(line):
    """Net brace depth change of one line of code."""
    line = code_only(line)
    return line.count("{") - line.count("}")


def block_end(lines, start):
    """Index of the line closing the brace block that opens at `start`."""
    depth = 0
    opened = False
    for i in range(start, len(lines)):
        depth += braces(lines[i])
        opened = opened or "{" in code_only(lines[i])
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
    # Include the macro attributes directly above the class (`@Model`),
    # looking past doc comments between them and the class.
    while start > 0 and (
        lines[start - 1].startswith("@") or lines[start - 1].startswith("///")
    ):
        start -= 1
    end = block_end(lines, start)
    return strip_comments(lines[start : end + 1])


def extract_extensions(source, model):
    """Every top-level `extension <model> { ... }` body, comments stripped."""
    lines = source.splitlines()
    bodies = []
    for i, line in enumerate(lines):
        if not re.match(rf"^(public )?extension {model}\b", line):
            continue
        if not re.match(rf"^(public )?extension {model}\s*(:[^{{]*)?\{{", line):
            raise SystemExit(
                f"{model}: line {i + 1}: an extension header must open its brace "
                "on the same line"
            )
        end = block_end(lines, i)
        inner = strip_comments(lines[i + 1 : end])
        conformance = re.search(r"extension \w+\s*(:[^{]*)\{", line)
        bodies.append(((conformance.group(1).strip() if conformance else ""), inner))
    return bodies


def indent(body):
    return "\n".join(("    " + line) if line.strip() else "" for line in body)


def render_value_types(freeze, sha, source):
    bodies = [indent(extract_value_type(source, name)) for name in freeze.value_types]
    return (
        HEADER
        + f"// Inline value types exactly as schema {freeze.schema} stored them, generated\n"
        f"// by scripts/freeze_schema_models.py from {os.path.basename(freeze.value_types_file)}\n"
        f"// at commit {sha}. SwiftData expands a stored Codable struct into composite\n"
        "// attributes of the owning entity, so these shapes are inputs to that\n"
        "// version's checksum just like the model's own properties. Do not edit.\n"
        f"extension {freeze.schema} {{\n"
        + "\n\n".join(bodies)
        + "\n}\n"
    )


def render_model(freeze, sha, source, model, sibling):
    # An extension of a nested type does not see its sibling nested types
    # by bare name (that lookup lands on the live top-level type), so model
    # and frozen value-type names inside extension bodies are qualified.
    extensions = ""
    for conformance, inner in extract_extensions(source, model):
        inner = [sibling.sub(rf"{freeze.schema}.\1", line) for line in inner]
        extensions += (
            f"\nextension {freeze.schema}.{model}{' ' + conformance if conformance else ''} {{\n"
            + "\n".join(inner)
            + "\n}\n"
        )
    return (
        HEADER
        + f"// `{model}` exactly as schema {freeze.schema} registered it, generated by\n"
        f"// scripts/freeze_schema_models.py from the live model at commit {sha}.\n"
        "// Do not edit: every stored property, its optionality and default, and\n"
        "// every @Attribute / @Relationship / #Unique here is an input to that\n"
        "// version's checksum, and every #Index to the store's SQLite indexes;\n"
        "// changing any of them re-breaks the stores this copy exists to keep\n"
        "// openable. See the live model for what each column means.\n"
        f"extension {freeze.schema} {{\n"
        f"{indent(extract_class(source, model))}\n"
        "}\n"
        + extensions
    )


def render_all(root):
    """Every frozen file as {relative path: text}."""
    # Names frozen under a schema, across all of its rows: any of them
    # mentioned inside an extension body must resolve to the nested copy.
    per_schema = {}
    for freeze in FREEZES:
        per_schema.setdefault(freeze.schema, set()).update(freeze.models)
        per_schema[freeze.schema].update(freeze.value_types)

    files = {}

    def emit(path, text):
        if path in files:
            raise SystemExit(f"{path}: produced by two FREEZES rows")
        files[path] = text

    for freeze in FREEZES:
        sha = git(root, "rev-parse", "--verify", freeze.commit).strip()[:10]
        sibling = re.compile(
            r"(?<![\w.])("
            + "|".join(re.escape(name) for name in sorted(per_schema[freeze.schema]))
            + r")\b"
        )
        if freeze.value_types:
            source = git(root, "show", f"{freeze.commit}:{freeze.value_types_file}")
            emit(
                f"{OUT_DIR}/{freeze.schema}+{os.path.basename(freeze.value_types_file)}",
                render_value_types(freeze, sha, source),
            )
        for model in freeze.models:
            source = git(root, "show", f"{freeze.commit}:{MODELS_DIR}/{model}.swift")
            emit(
                f"{OUT_DIR}/{freeze.schema}+{model}.swift",
                render_model(freeze, sha, source, model, sibling),
            )
    return files


def frozen_files_on_disk(root):
    out_dir = os.path.join(root, OUT_DIR)
    return {
        f"{OUT_DIR}/{name}"
        for name in (os.listdir(out_dir) if os.path.isdir(out_dir) else [])
        if name.startswith("DashSchema") and name.endswith(".swift")
    }


def check_problems(root, files):
    """How the frozen files under `root` differ from `files`, byte for byte."""
    problems = []
    for path, text in sorted(files.items()):
        full = os.path.join(root, path)
        if not os.path.exists(full):
            problems.append(f"missing:  {path}")
            continue
        with open(full, encoding="utf-8", newline="") as f:
            if f.read() != text:
                problems.append(f"differs:  {path}")
    for path in sorted(frozen_files_on_disk(root) - files.keys()):
        problems.append(f"stale:    {path}")
    return problems


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare with the files on disk instead of writing; exit 1 on any difference",
    )
    args = parser.parse_args()
    root = repo_root()
    files = render_all(root)

    if args.check:
        problems = check_problems(root, files)
        if problems:
            print("\n".join(problems), file=sys.stderr)
            print(
                f"{len(problems)} frozen file(s) out of date; rerun "
                "scripts/freeze_schema_models.py",
                file=sys.stderr,
            )
            sys.exit(1)
        print(f"{len(files)} frozen files match FREEZES")
        return

    os.makedirs(os.path.join(root, OUT_DIR), exist_ok=True)
    for path in sorted(frozen_files_on_disk(root) - files.keys()):
        os.remove(os.path.join(root, path))
        print(f"removed {path}")
    for path, text in sorted(files.items()):
        with open(os.path.join(root, path), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(path)


if __name__ == "__main__":
    main()
