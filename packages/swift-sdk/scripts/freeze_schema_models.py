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
optionality and defaults,
`@Attribute`, `@Relationship`, `#Index` and `#Unique` are what the checksum
hashes, and they are copied verbatim.

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

The table is not trusted to be complete. Before anything is generated or
checked, the freeze is validated as a closed graph, and an incomplete one
is refused with the missing names rather than emitted partially:

  - every type a frozen model or frozen value type stores (a relationship
    target, an inline Codable struct or enum) must itself be frozen under
    the same schema, transitively; a bare name in a nested class body that
    has no nested sibling resolves to the live type, which is exactly the
    silent drift the freeze exists to prevent;
  - every `DashSchemaVN.X` that `DashModelContainer.swift` registers must be
    produced by the table, every frozen model must be registered, and no
    released version may register a live model type.

Usage, from anywhere inside the repository:

    scripts/freeze_schema_models.py            # regenerate every frozen file
    scripts/freeze_schema_models.py --check    # exit 1 if any file would change

`--check` is what a reviewer or CI runs (the `swift-sdk-frozen-schema` job
in `.github/workflows/tests.yml`): the frozen files are a pure function of
`FREEZES` and the repository history, so a clean check proves nobody edited
a frozen copy by hand. It needs the commits named in `FREEZES` to be
present locally, so CI runs it on a full-history checkout.

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
CONTAINER_FILE = "packages/swift-sdk/Sources/SwiftDashSDK/Persistence/DashModelContainer.swift"

# Types SwiftData stores natively: they carry no shape of their own, so a
# stored property of one of these adds nothing that has to be frozen.
BUILTIN_TYPES = {
    "Bool",
    "Data",
    "Date",
    "Decimal",
    "Double",
    "Float",
    "Int",
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "String",
    "UInt",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "URL",
    "UUID",
    "Array",
    "Dictionary",
    "Optional",
    "Set",
}

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


ATTRIBUTE = r"@\w+(\((?:[^()]|\([^()]*\))*\))?\s*"
MODIFIER = (
    r"(?:(?:private|fileprivate|internal|public|open|package)(?:\(set\))?"
    r"|static|lazy|weak|unowned|final)\s+"
)
STORED_DECLARATION = re.compile(
    rf"^\s*(?P<attributes>(?:{ATTRIBUTE})*)(?P<modifiers>(?:{MODIFIER})*)(var|let)\s+\w+\s*"
    r"(?::\s*(?P<type>[^={]+))?(?:=\s*(?P<initializer>[^{]*))?"
)
ENUM_CASE = re.compile(r"^\s*(indirect\s+)?case\s+(?P<payload>.+)")
NESTED_TYPE = re.compile(
    rf"^\s*(?:{MODIFIER})*(?:indirect\s+)?(struct|enum|class)\s+(?P<name>\w+)"
)


def type_names(expression):
    """Capitalised identifiers a type expression or enum payload mentions.

    String literals are dropped (an enum raw value is not a type) and so is
    everything after a dot: `Foo.Bar` belongs to `Foo`, and the nested type
    is copied along with it.
    """
    expression = re.sub(r'"[^"]*"', "", expression)
    expression = re.sub(r"\.\w+", "", expression)
    return set(re.findall(r"\b[A-Z]\w*", expression))


def stored_type_names(block):
    """Type names the stored declarations of one comment-stripped type body mention.

    A stored property (`var` / `let` with no getter body and no `@Transient`)
    or an enum case payload is what SwiftData hashes, so those are the
    references that must resolve to frozen copies. Types declared nested in
    the body are copied with it, so their names are not returned, but their
    own stored declarations are walked: a nested struct's field binds a
    name the same way the model's does. Computed properties, initializers,
    methods, static members and the raw-value clause do not affect the
    entity, and a builtin has no shape to freeze, so none of those are
    returned. A property whose type is inferred from its initializer
    contributes the initializer's type names instead.
    """
    names, nested = set(), set()
    depth = 0
    attributes = ""
    for i, raw in enumerate(block):
        line = code_only(raw)
        if depth == 1:
            declared = NESTED_TYPE.match(line)
            if declared:
                nested.add(declared.group("name"))
                names |= stored_type_names(block[i : block_end(block, i) + 1])
            if line.strip().startswith("@") and not re.search(r"\b(var|let)\b", line):
                attributes += line
            else:
                following = next((code_only(l) for l in block[i + 1 :] if l.strip()), "")
                if re.search(r"\b(var|let)\s+\w+\s*:\s*$", line):
                    line += " " + following.strip()
                stored = STORED_DECLARATION.match(line)
                if stored and "static" not in stored.group("modifiers"):
                    observed = re.search(r"\b(didSet|willSet)\b", line + " " + following)
                    computed = "{" in line and not observed
                    transient = "@Transient" in attributes + stored.group("attributes")
                    if not computed and not transient:
                        names |= type_names(
                            stored.group("type") or stored.group("initializer") or ""
                        )
                case = ENUM_CASE.match(line)
                if case:
                    names |= type_names(case.group("payload"))
                attributes = ""
        depth += braces(raw)
    return names - nested - BUILTIN_TYPES


def model_names_at(root, commit):
    """Every `@Model` class under MODELS_DIR as of `commit`."""
    grep = subprocess.run(
        ["git", "grep", "-l", "@Model", commit, "--", MODELS_DIR],
        text=True,
        encoding="utf-8",
        cwd=root,
        capture_output=True,
    )
    if grep.returncode > 1:
        raise SystemExit(f"git grep at {commit} failed: {grep.stderr.strip()}")
    return {
        os.path.basename(line.split(":", 1)[1])[: -len(".swift")]
        for line in grep.stdout.splitlines()
    }


def closure_problems(frozen, bodies, models_at):
    """Why the freeze is not closed under "stores", as human-readable lines.

    `frozen` maps a schema to every name frozen under it, `bodies` maps
    `(schema, name)` to the comment-stripped body of that frozen type plus
    the names of the types its own extensions declare (those are copied
    with it), and `models_at` maps a schema to the live model names at the
    commits its rows are taken from. A stored reference must resolve to a
    frozen sibling; a bare name with no sibling binds to the live type
    instead, and its next change would move the released checksum.
    """
    problems = []
    for (schema, owner), (body, declared) in sorted(bodies.items()):
        for name in sorted(stored_type_names(body) - frozen[schema] - declared):
            what = "a model" if name in models_at[schema] else "a value type"
            problems.append(
                f"{schema}.{owner} stores {name}, which is not frozen under {schema}: "
                f"add it to a {schema} row of FREEZES as {what}"
            )
    return problems


def registration_problems(container_lines, frozen_models, live_models):
    """Why DashModelContainer.swift and FREEZES disagree, as human-readable lines.

    Every `DashSchemaVN.X` the container names must be produced by the
    table, every frozen model must be named somewhere (else it is a copy no
    version registers), a live model type may appear only inside the live
    list `modelTypes`, and only the newest `DashSchemaVN` may use that
    list: a released version that registered a live shape would hash it
    into its checksum.

    This is a presence check, not per-version membership: a version that
    registered the wrong frozen copy (say V2 taking V3's asset lock) passes
    here and is caught by `DashModelMigrationTests` opening real stores.
    """
    # Comments are blanked line by line rather than dropped so that a
    # reported line number is the file's own.
    lines = [code_only(line) for line in container_lines]
    # The live list is found by its exact declaration; a reformat that
    # hides it makes the check refuse rather than pass.
    live_list = range(0)
    versions = {}
    for i, line in enumerate(lines):
        if re.match(r"^\s*(public )?static var modelTypes: \[any PersistentModel\.Type\] \{", line):
            live_list = range(i, block_end(lines, i) + 1)
        version = re.match(r"^\s*(public )?enum DashSchemaV(\d+)\b", line)
        if version:
            versions[int(version.group(2))] = range(i, block_end(lines, i) + 1)
    problems = []
    if not live_list:
        problems.append(f"{CONTAINER_FILE}: the live model list `modelTypes` was not found")
    # Only the newest version may be the live one; an older version that
    # aliased `modelTypes` would register every live shape at once.
    live_version = versions[max(versions)] if versions else range(0)
    registered = set()
    for i, line in enumerate(lines):
        for schema, name in re.findall(r"\b(DashSchemaV\d+)\.([A-Z]\w*)\b", line):
            registered.add((schema, name))
        if i in live_list:
            continue
        aliased = versions and re.search(r"\bmodelTypes\b", line)
        if aliased and i not in live_version:
            problems.append(
                f"{CONTAINER_FILE} line {i + 1} uses the live model list `modelTypes` "
                f"outside DashSchemaV{max(versions)}; a released version may only "
                "register frozen copies"
            )
        for name in re.findall(r"(?<![\w.])(\w+)\.self", line):
            if name in live_models:
                problems.append(
                    f"{CONTAINER_FILE} line {i + 1} registers the live {name} outside "
                    "`modelTypes`; a released version may only register frozen copies"
                )
    for schema, name in sorted(registered - frozen_models):
        problems.append(
            f"{CONTAINER_FILE} registers {schema}.{name}, which no FREEZES row produces"
        )
    for schema, name in sorted(frozen_models - registered):
        problems.append(
            f"FREEZES freezes {schema}.{name}, which {CONTAINER_FILE} never registers"
        )
    return problems


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
        "// every @Attribute / @Relationship / #Index / #Unique here is an input to\n"
        "// that version's checksum, and changing any of them re-breaks the stores\n"
        "// this copy exists to keep openable. See the live model for what each\n"
        "// column means.\n"
        f"extension {freeze.schema} {{\n"
        f"{indent(extract_class(source, model))}\n"
        "}\n"
        + extensions
    )


def validate(root, sources):
    """Refuse FREEZES unless it is a closed, registered freeze.

    `sources` maps `(commit, path)` to file contents at that commit. Returns
    every name frozen under each schema.
    """
    frozen, bodies, models_at = {}, {}, {}
    for freeze in FREEZES:
        frozen.setdefault(freeze.schema, set()).update(freeze.models, freeze.value_types)
        models_at.setdefault(freeze.schema, set()).update(
            model_names_at(root, freeze.commit)
        )
        for model in freeze.models:
            source = sources[(freeze.commit, f"{MODELS_DIR}/{model}.swift")]
            declared = {
                NESTED_TYPE.match(line).group("name")
                for _, inner in extract_extensions(source, model)
                for line in inner
                if NESTED_TYPE.match(line)
            }
            bodies[(freeze.schema, model)] = (extract_class(source, model), declared)
        for name in freeze.value_types:
            source = sources[(freeze.commit, freeze.value_types_file)]
            bodies[(freeze.schema, name)] = (extract_value_type(source, name), set())

    with open(os.path.join(root, CONTAINER_FILE), encoding="utf-8") as f:
        container_lines = f.read().splitlines()
    live_models = {
        name[: -len(".swift")]
        for name in os.listdir(os.path.join(root, MODELS_DIR))
        if name.endswith(".swift")
    }
    frozen_models = {
        (freeze.schema, model) for freeze in FREEZES for model in freeze.models
    }

    problems = closure_problems(frozen, bodies, models_at) + registration_problems(
        container_lines, frozen_models, live_models
    )
    if problems:
        raise SystemExit(
            "FREEZES is not a complete freeze; nothing was generated:\n  "
            + "\n  ".join(problems)
        )
    return frozen


def render_all(root):
    """Every frozen file as {relative path: text}."""
    sources = {}
    for freeze in FREEZES:
        paths = [f"{MODELS_DIR}/{model}.swift" for model in freeze.models]
        if freeze.value_types:
            paths.append(freeze.value_types_file)
        for path in paths:
            sources.setdefault(
                (freeze.commit, path), git(root, "show", f"{freeze.commit}:{path}")
            )

    # Names frozen under a schema, across all of its rows: any of them
    # mentioned inside an extension body must resolve to the nested copy.
    per_schema = validate(root, sources)

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
            source = sources[(freeze.commit, freeze.value_types_file)]
            emit(
                f"{OUT_DIR}/{freeze.schema}+{os.path.basename(freeze.value_types_file)}",
                render_value_types(freeze, sha, source),
            )
        for model in freeze.models:
            source = sources[(freeze.commit, f"{MODELS_DIR}/{model}.swift")]
            emit(
                f"{OUT_DIR}/{freeze.schema}+{model}.swift",
                render_model(freeze, sha, source, model, sibling),
            )
    return files


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

    out_dir = os.path.join(root, OUT_DIR)
    on_disk = {
        f"{OUT_DIR}/{name}"
        for name in (os.listdir(out_dir) if os.path.isdir(out_dir) else [])
        if name.startswith("DashSchema") and name.endswith(".swift")
    }

    if args.check:
        problems = []
        for path, text in sorted(files.items()):
            full = os.path.join(root, path)
            if not os.path.exists(full):
                problems.append(f"missing:  {path}")
                continue
            with open(full, encoding="utf-8", newline="") as f:
                if f.read() != text:
                    problems.append(f"differs:  {path}")
        for path in sorted(on_disk - files.keys()):
            problems.append(f"stale:    {path}")
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

    os.makedirs(out_dir, exist_ok=True)
    for path in sorted(on_disk - files.keys()):
        os.remove(os.path.join(root, path))
        print(f"removed {path}")
    for path, text in sorted(files.items()):
        with open(os.path.join(root, path), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(path)


if __name__ == "__main__":
    main()
