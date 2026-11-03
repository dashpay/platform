#!/usr/bin/env python3
"""Generate immutable SwiftData snapshots from the exact sources that shipped.

V1's accepted historical FREEZES rows remain unchanged. New App Store releases
are recorded in schema-releases.json, using the full Platform SHA and the model
inventory committed at that SHA. Intermediate TestFlight builds only capture
evidence; they do not add released schema versions.

Each release snapshot copies the complete model graph and stored value types
into a separate DashSchemaSnapshotVN namespace. It is not an additional runtime
migration stage: DashSchemaVN stays on live model types until a later shape
change moves it onto the snapshot and introduces a new live version/migration.

--check verifies deterministic generated sources, registry bindings and immutable
fixture digests. --check-inventory rejects missing transitive stored value types
within the supported explicit storage grammar. Standard Swift/Foundation type
names must not be shadowed elsewhere in the SDK. Custom Codable methods and
other helper behavior remain outside this declaration check. The runtime
DashReleasedSchemaTests compares SwiftData's hashes and SQLite indexes against
the captured fixture, after constructing the live schema first. This also
catches inline value types or relationship references accidentally left live.
Do not replace those runtime checks with a static scan of model source.

Usage:
    python3 packages/swift-sdk/scripts/freeze_schema_models.py --check
    python3 packages/swift-sdk/scripts/freeze_schema_models.py \
        --release-manifest build.json --fixture fixture.store

A full-history checkout including swift-schema-source/* tags is required. New
manifests must refer to a commit that already contains schema-models.json.
Historical fixture files are never rebuilt from today's sources. --repo selects
the data repository explicitly when a trusted copy of this generator runs from
outside the checkout.
"""

import argparse
import contextlib
import dataclasses
import hashlib
import json
import pathlib
import plistlib
import sqlite3
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

# Accepted V1 model inventory, excluding the asset lock whose earlier shape
# has a separate unchanged row below.
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


HISTORICAL_V2_SOURCE = "52e8d4ec68f0c772313fa1bbef223fb1eabbf1cc"

FREEZES = [
    # The accepted V1 asset lock: the last commit before
    # `recipientIsExternal` was added to the live model.
    Freeze("DashSchemaV1", "7127c38566", ("PersistentAssetLock",)),
    # The accepted V1 graph at the last commit before the sweep additions.
    Freeze(
        "DashSchemaV1",
        "5f58417079",
        tuple(V1_GRAPH_MODELS),
        TOKEN_TYPES_FILE,
        tuple(TOKEN_VALUE_TYPES),
    ),
    # Reconstructed pre-August-28 V2: all 35 hashes and the model checksum
    # match the observed App Store store. This is reconstruction provenance,
    # not a claim that this exact commit built the released app binary.
    Freeze(
        "DashSchemaV2", HISTORICAL_V2_SOURCE,
        tuple(V1_GRAPH_MODELS + ["PersistentAssetLock", "PersistentTrackedMasternode"]),
        TOKEN_TYPES_FILE, tuple(TOKEN_VALUE_TYPES),
    ),
]

HEADER = "import Foundation\nimport SwiftData\n\n"


def git(root, *args):
    try:
        return subprocess.check_output(
            ["git", *args], text=True, encoding="utf-8", cwd=root
        )
    except subprocess.CalledProcessError as error:
        raise SystemExit(
            f"git {' '.join(args)} failed (exit {error.returncode}); a baseline or "
            "release source commit may not be fetched locally"
        )


def repo_root(directory=None):
    return git(directory or os.getcwd(), "rev-parse", "--show-toplevel").strip()


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


def render_baseline(root):
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


REGISTRY_FILE = "packages/swift-sdk/schema-releases.json"
INVENTORY_FILE = "packages/swift-sdk/schema-models.json"
TEST_REGISTRY_FILE = "packages/swift-sdk/SwiftTests/SwiftDashSDKTests/DashReleasedSchemaRegistry.generated.swift"
FIXTURE_DIR = "packages/swift-sdk/SwiftTests/SwiftDashSDKTests/Fixtures/SchemaStores/releases"


def read_registry(root):
    with open(os.path.join(root, REGISTRY_FILE), encoding="utf-8") as source:
        registry = json.load(source)
    if registry.get("format_version") != 1 or not isinstance(registry.get("schemas"), dict):
        raise SystemExit("unsupported schema release registry")
    return registry


def validate_schema(schema):
    if not isinstance(schema, dict) or set(schema) != {
        "schema_version", "model_checksum", "entity_hashes", "indexes"
    }:
        raise SystemExit("invalid captured schema description")
    version = schema["schema_version"]
    if not isinstance(version, str) or not re.fullmatch(r"[1-9][0-9]*\.0\.0", version):
        raise SystemExit("schema versions must be major.0.0")
    if not isinstance(schema["model_checksum"], str) or not schema["model_checksum"]:
        raise SystemExit("missing model checksum")
    hashes = schema["entity_hashes"]
    if not isinstance(hashes, dict) or not hashes or any(
        not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name)
        or not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]+", value)
        for name, value in hashes.items()
    ):
        raise SystemExit("invalid entity hashes")
    indexes = schema["indexes"]
    if not isinstance(indexes, list) or any(not isinstance(item, str) for item in indexes):
        raise SystemExit("invalid index description")
    if indexes != sorted(set(indexes)):
        raise SystemExit("indexes must be sorted and unique")


def read_inventory(root, commit):
    inventory = json.loads(git(root, "show", f"{commit}:{INVENTORY_FILE}"))
    return validate_inventory(inventory)


def validate_inventory(inventory):
    if inventory.get("format_version") != 1:
        raise SystemExit("unsupported historical schema model inventory")
    models = inventory["models"]
    groups = inventory["value_types"]
    names = list(models) + [name for group in groups for name in group["names"]]
    if len(names) != len(set(names)):
        raise SystemExit("duplicate type in model inventory")
    for name in names:
        if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name):
            raise SystemExit("invalid Swift name in model inventory")
    for path in list(models.values()) + [group["path"] for group in groups]:
        if not path.startswith("packages/swift-sdk/Sources/SwiftDashSDK/") or ".." in pathlib.PurePosixPath(path).parts:
            raise SystemExit("model inventory path is outside the Swift SDK")
    return inventory


# This is deliberately a restricted declaration grammar, not a Swift compiler.
# It checks explicit stored type expressions against the copied graph and
# standard types; standard names must not be shadowed elsewhere in the SDK.
# Unsupported storage syntax fails closed; SwiftData's native hash/index tests
# still verify the generated models' semantics and registration order.
SWIFT_SCALARS = set("Bool String Character Int Int8 Int16 Int32 Int64 UInt UInt8 UInt16 UInt32 UInt64 Float Double".split())
FOUNDATION_SCALARS = set("Data Date UUID Decimal URL TimeInterval".split())
STORAGE_CONTAINERS = {"Array": 1, "Set": 1, "Optional": 1, "Dictionary": 2}
STORAGE_PROTOCOLS = set("Codable Decodable Encodable Equatable Hashable Sendable CaseIterable Identifiable".split())


def storage_tokens(lines, owner):
    tokens = []
    for line in lines:
        # The existing source copier supports ordinary single-line strings.
        # Do not guess where a raw/multiline string or backtick identifier ends.
        if re.search(r'#+"|"""|`', line.split("//", 1)[0]):
            raise SystemExit(f"{owner}: raw/multiline strings and escaped identifiers need explicit parser support")
        code = code_only(line)
        tokens.extend(re.findall(r'[A-Za-z_][A-Za-z_0-9]*|""|[^\s]', code))
        tokens.append("\n")
    return tokens


def matching_token(tokens, start, owner):
    opening = tokens[start]
    closing = {"(": ")", "[": "]", "{": "}"}[opening]
    index = start + 1
    while index < len(tokens):
        if tokens[index] == closing:
            return index
        if tokens[index] in ("(", "[", "{"):
            index = matching_token(tokens, index, owner)
        elif tokens[index] in (")", "]", "}"):
            break
        index += 1
    raise SystemExit(f"{owner}: unsupported or unbalanced storage declaration")


def validate_stored_type(tokens, names, owner):
    tokens = [token for token in tokens if token != "\n"]
    index = 0

    def consume():
        nonlocal index
        if index == len(tokens):
            raise SystemExit(f"{owner}: missing stored type")
        token = tokens[index]
        index += 1
        if token == "[":
            consume()
            if index < len(tokens) and tokens[index] == ":":
                index += 1
                consume()
            if index == len(tokens) or tokens[index] != "]":
                raise SystemExit(f"{owner}: unsupported collection type")
            index += 1
        elif re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", token):
            name = token
            if index < len(tokens) and tokens[index] == ".":
                index += 1
                if index == len(tokens):
                    raise SystemExit(f"{owner}: incomplete qualified type")
                name = tokens[index]
                index += 1
                permitted = SWIFT_SCALARS | set(STORAGE_CONTAINERS) if token == "Swift" else FOUNDATION_SCALARS if token == "Foundation" else set()
                if name not in permitted:
                    raise SystemExit(f"{owner}: module-qualified or nested stored type {token}.{name} is not isolated")
            if name in STORAGE_CONTAINERS:
                if index == len(tokens) or tokens[index] != "<":
                    raise SystemExit(f"{owner}: collection requires explicit type arguments")
                index += 1
                for argument in range(STORAGE_CONTAINERS[name]):
                    if argument:
                        if index == len(tokens) or tokens[index] != ",":
                            raise SystemExit(f"{owner}: invalid generic collection arguments")
                        index += 1
                    consume()
                if index == len(tokens) or tokens[index] != ">":
                    raise SystemExit(f"{owner}: invalid generic collection arguments")
                index += 1
            elif name not in names | SWIFT_SCALARS | FOUNDATION_SCALARS:
                raise SystemExit(f"{owner}: stored type {name} is absent from schema-models.json; include its transitive value graph")
        else:
            raise SystemExit(f"{owner}: unsupported stored type expression {' '.join(tokens)}")
        while index < len(tokens) and tokens[index] == "?":
            index += 1

    consume()
    if index != len(tokens):
        raise SystemExit(f"{owner}: unsupported stored type expression {' '.join(tokens)}")


def validate_storage_declaration(lines, name, names):
    tokens = storage_tokens(lines, name)
    start = tokens.index("{")
    header = [token for token in tokens[:start] if token != "\n"]
    if any(header[index + 1] != "Model" for index, token in enumerate(header) if token == "@"):
        raise SystemExit(f"{name}: unsupported type declaration macro")
    kind = next(token for token in header if token in ("class", "struct", "enum"))
    inherited = header[header.index(name) + 1:]
    if inherited:
        if inherited[0] != ":" or any(token not in STORAGE_PROTOCOLS | SWIFT_SCALARS | {":", ","} for token in inherited):
            raise SystemExit(f"{name}: generic types, custom conformances and inherited storage need explicit parser support")
    end = matching_token(tokens, start, name)
    index = start + 1
    prefix = []
    attributes = []
    while index < end:
        token = tokens[index]
        if token in ("\n", ";"):
            index += 1
            continue
        if token == "@":
            attribute = tokens[index + 1]
            index += 2
            if index < end and tokens[index] == "(":
                close = matching_token(tokens, index, name)
                arguments = tokens[index + 1:close]
                if attribute == "Attribute" and "transformable" in arguments:
                    raise SystemExit(f"{name}: transformable storage requires explicit isolation support")
                index = close + 1
            if attribute not in ("Attribute", "Relationship", "Transient"):
                raise SystemExit(f"{name}: unsupported property macro @{attribute}")
            attributes.append(attribute)
            continue
        if token in ("#", "typealias", "associatedtype"):
            if token == "#" and tokens[index + 1] in ("Index", "Unique"):
                opening = index + 2
                if tokens[opening:opening + 3] == ["<", name, ">"]:
                    opening += 3
                if tokens[opening] != "(":
                    raise SystemExit(f"{name}: unsupported index/unique declaration")
                index = matching_token(tokens, opening, name) + 1
                prefix, attributes = [], []
                continue
            raise SystemExit(f"{name}: conditional declarations, aliases and declaration macros need explicit isolation support")
        if token == "class" and tokens[index + 1] in ("var", "func"):
            prefix.append(token)
            index += 1
            continue
        if token in ("struct", "class", "enum", "actor", "protocol"):
            nested_name = tokens[index + 1]
            if nested_name in names | SWIFT_SCALARS | FOUNDATION_SCALARS | set(STORAGE_CONTAINERS) | {"Swift", "Foundation"}:
                raise SystemExit(f"{name}: nested type {nested_name} shadows a stored type")
            # Unused nested helpers are copied with their enclosing type. Any
            # stored reference to them is rejected by validate_stored_type.
            opening = tokens.index("{", index)
            index = matching_token(tokens, opening, name) + 1
            prefix, attributes = [], []
            continue
        if token in ("var", "let"):
            field = tokens[index + 1]
            if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", field):
                raise SystemExit(f"{name}: destructured stored declarations are unsupported")
            cursor = index + 2
            expression = []
            while cursor < end and tokens[cursor] not in ("=", "{", ";", "\n"):
                if tokens[cursor] in ("[", "("):
                    close = matching_token(tokens, cursor, name)
                    expression.extend(tokens[cursor:close + 1])
                    cursor = close + 1
                else:
                    expression.append(tokens[cursor])
                    cursor += 1
            # An accessor body is computed, except willSet/didSet observers.
            # A closure initializer follows '=' and is still stored.
            next_code = cursor
            while tokens[next_code] == "\n":
                next_code += 1
            computed = tokens[next_code] == "{"
            if computed:
                close = matching_token(tokens, next_code, name)
                body = tokens[next_code + 1:close]
                computed = "willSet" not in body and "didSet" not in body
            if not (computed or "static" in prefix or "class" in prefix or "Transient" in attributes):
                if not expression or expression[0] != ":":
                    raise SystemExit(f"{name}.{field}: inferred stored types need an explicit type annotation")
                validate_stored_type(expression[1:], names, f"{name}.{field}")
            # Skip the initializer/accessors, not just the type. A comma at
            # declaration level could start another binding and is refused.
            index = cursor
            while index < end and tokens[index] not in ("\n", ";"):
                if tokens[index] in ("{", "(", "["):
                    index = matching_token(tokens, index, name) + 1
                elif tokens[index] == ",":
                    raise SystemExit(f"{name}.{field}: multiple property bindings need separate declarations")
                else:
                    index += 1
            if next_code < end and tokens[next_code] == "{" and cursor != next_code:
                index = matching_token(tokens, next_code, name) + 1
            prefix, attributes = [], []
            continue
        if token == "case" and kind == "enum":
            index += 1
            while index < end and tokens[index] not in ("\n", ";"):
                if tokens[index] == "(":
                    close = matching_token(tokens, index, name)
                    payload = tokens[index + 1:close]
                    # Split only outer commas; generic dictionary arguments
                    # and nested bracket syntax belong to the same payload.
                    groups, group, depth = [], [], 0
                    for part in payload + [","]:
                        if part == "," and depth == 0:
                            groups.append(group)
                            group = []
                        else:
                            group.append(part)
                            depth += int(part in ("[", "<", "(")) - int(part in ("]", ">", ")"))
                    for group in groups:
                        if ":" in group and group.index(":") < 2:
                            group = group[group.index(":") + 1:]
                        validate_stored_type(group, names, f"{name} enum payload")
                    index = close + 1
                else:
                    index += 1
            prefix, attributes = [], []
            continue
        if token in ("func", "init", "deinit", "subscript"):
            opening = index + 1
            while opening < end and tokens[opening] != "{":
                if tokens[opening] in ("(", "["):
                    opening = matching_token(tokens, opening, name)
                opening += 1
            if opening == end:
                raise SystemExit(f"{name}: declaration without a body needs explicit parser support")
            index = matching_token(tokens, opening, name) + 1
            prefix, attributes = [], []
            continue
        if token == "(":
            if tokens[index:index + 3] != ["(", "set", ")"]:
                raise SystemExit(f"{name}: unsupported declaration continuation")
            index = matching_token(tokens, index, name) + 1
            continue
        if token not in {"public", "private", "fileprivate", "internal", "package", "open", "static",
                         "final", "override", "required", "convenience", "mutating", "nonmutating",
                         "lazy", "weak", "unowned", "dynamic", "nonisolated"}:
            raise SystemExit(f"{name}: unsupported declaration token {token!r}")
        prefix.append(token)
        index += 1


def validate_storage_graph(inventory, sources):
    names = set(inventory["models"]) | {name for group in inventory["value_types"] for name in group["names"]}
    if names & (SWIFT_SCALARS | FOUNDATION_SCALARS | set(STORAGE_CONTAINERS) | {"Swift", "Foundation"}):
        raise SystemExit("inventory shadows a standard stored type")
    for name, path in inventory["models"].items():
        validate_storage_declaration(extract_class(sources[path], name), name, names)
    for group in inventory["value_types"]:
        for name in group["names"]:
            validate_storage_declaration(extract_value_type(sources[group["path"]], name), name, names)


def inventory_sources(root, inventory, commit=None):
    paths = set(inventory["models"].values()) | {group["path"] for group in inventory["value_types"]}
    return {path: git(root, "show", f"{commit}:{path}") if commit else pathlib.Path(root, path).read_text(encoding="utf-8")
            for path in sorted(paths)}


def render_snapshot(root, version, entry, *, inventory=None):
    validate_schema(entry["schema"])
    if entry["schema"]["schema_version"] != version:
        raise SystemExit("registry key does not match captured schema version")
    commit = entry["platform_sha"]
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise SystemExit("release source must be a full Git SHA")
    namespace = "DashSchemaSnapshotV" + version.split(".")[0]
    if entry["namespace"] != namespace:
        raise SystemExit("unexpected snapshot namespace")
    inventory = read_inventory(root, commit) if inventory is None else validate_inventory(inventory)
    models = inventory["models"]
    if set(models) != set(entry["schema"]["entity_hashes"]):
        raise SystemExit("historical inventory differs from captured model membership")
    sources = inventory_sources(root, inventory, commit)
    validate_storage_graph(inventory, sources)
    names = set(models) | {name for group in inventory["value_types"] for name in group["names"]}
    sibling = re.compile(r"(?<![\w.])(" + "|".join(re.escape(name) for name in sorted(names)) + r")\b")
    freeze = Freeze(namespace, commit, tuple(models))
    files = {}
    for name, path in models.items():
        files[f"{OUT_DIR}/{namespace}+{name}.swift"] = render_model(
            freeze, commit[:10], sources[path], name, sibling)
    for group in inventory["value_types"]:
        value_freeze = Freeze(namespace, commit, (), group["path"], tuple(group["names"]))
        path = f"{OUT_DIR}/{namespace}+{os.path.basename(group['path'])}"
        if path in files:
            raise SystemExit("duplicate snapshot output")
        files[path] = render_value_types(value_freeze, commit[:10], sources[group["path"]])
    files[f"{OUT_DIR}/{namespace}+Schema.swift"] = (
        HEADER + "// Generated release snapshot; never add alongside its live version in the migration plan.\n"
        + f"enum {namespace}: VersionedSchema {{\n"
        + f"    static var versionIdentifier: Schema.Version {{ Schema.Version({version.replace('.', ', ')}) }}\n"
        + "    static var models: [any PersistentModel.Type] {\n        [\n"
        + ",\n".join(f"            {name}.self" for name in models)
        + "\n        ]\n    }\n}\n"
    )
    return files


APP_STORE_BASELINE_FIELDS = {
    "bundle_id": r"[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+",
    "app_id": r"[0-9]+",
    "app_version": r"[0-9]+(\.[0-9]+){1,2}",
    "release_id": r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
}


def validate_app_store_baseline(binding):
    """The published release a historical schema is bound to. The iOS release
    gate reads exactly this object, so its shape is part of the contract."""
    if not isinstance(binding, dict) or set(binding) != set(APP_STORE_BASELINE_FIELDS):
        raise SystemExit("historical schema must bind exactly one App Store baseline")
    for field, pattern in APP_STORE_BASELINE_FIELDS.items():
        value = binding[field]
        if not isinstance(value, str) or not re.fullmatch(pattern, value):
            raise SystemExit(f"invalid App Store baseline {field}")


def validate_historical_schemas(root, registry):
    """Historical reconstruction is separate from archive-captured releases."""
    entries = registry.get("historical_schemas", {})
    if not isinstance(entries, dict):
        raise SystemExit("invalid historical schema registry")
    for version, entry in entries.items():
        if version != "2.0.0" or not isinstance(entry, dict):
            raise SystemExit("unsupported historical schema")
        schema = entry["schema"]
        validate_schema(schema)
        if schema["schema_version"] != version or entry.get("source_sha") != HISTORICAL_V2_SOURCE:
            raise SystemExit("historical schema reconstruction provenance differs")
        if entry.get("provenance") != "reconstructed-model-match":
            raise SystemExit("historical schema must identify reconstruction provenance")
        validate_app_store_baseline(entry.get("app_store_baseline"))
        digest = entry["fixture_sha256"]
        path = entry["fixture_path"]
        if path != "packages/swift-sdk/SwiftTests/SwiftDashSDKTests/Fixtures/SchemaStores/historical-v2.store":
            raise SystemExit("invalid historical fixture path")
        fixture = pathlib.Path(root, path)
        if not re.fullmatch(r"[0-9a-f]{64}", digest) or not fixture.is_file() or hashlib.sha256(fixture.read_bytes()).hexdigest() != digest:
            raise SystemExit("missing or modified immutable historical fixture")
        validate_fixture_description(fixture, schema)
    return entries


def render_all(root, registry=None):
    files = render_baseline(root)
    registry = read_registry(root) if registry is None else registry
    historical = validate_historical_schemas(root, registry)
    fixtures = []
    checksums = {entry["schema"]["model_checksum"] for entry in historical.values()}
    for version, entry in sorted(registry["schemas"].items(), key=lambda item: tuple(map(int, item[0].split('.')))):
        if version == "1.0.0" or version in historical:
            raise SystemExit("Historical schema versions are reserved, not new release snapshots")
        checksum = entry["schema"]["model_checksum"]
        if checksum in checksums:
            raise SystemExit("two registered schema versions have the same model checksum")
        checksums.add(checksum)
        files.update(render_snapshot(root, version, entry))
        digest = entry["fixture_sha256"]
        expected_path = f"{FIXTURE_DIR}/{digest}.store"
        if not re.fullmatch(r"[0-9a-f]{64}", digest) or entry["fixture_path"] != expected_path:
            raise SystemExit("invalid release fixture path")
        fixture = pathlib.Path(root, expected_path)
        if not fixture.is_file() or hashlib.sha256(fixture.read_bytes()).hexdigest() != digest:
            raise SystemExit("missing or modified immutable release fixture")
        validate_fixture_description(fixture, entry["schema"])
        fixtures.append(f'        DashReleasedSchemaFixture(version: {entry["namespace"]}.self, resourceName: "{digest}")')
    files[TEST_REGISTRY_FILE] = (
        "// Generated by scripts/freeze_schema_models.py. Do not edit.\n"
        "@testable import SwiftDashSDK\n\n"
        "enum DashReleasedSchemaRegistry {\n"
        "    static let fixtures: [DashReleasedSchemaFixture] = [\n"
        + ",\n".join(fixtures) + "\n    ]\n}\n"
    )
    return files


def validate_fixture_description(path, schema):
    """Check captured metadata without opening or migrating the store in SwiftData."""
    uri = pathlib.Path(path).resolve().as_uri() + "?mode=ro&immutable=1"
    try:
        with contextlib.closing(sqlite3.connect(uri, uri=True)) as database:
            row = database.execute("SELECT Z_PLIST FROM Z_METADATA").fetchone()
            metadata = plistlib.loads(row[0])
            if metadata.get("NSStoreModelVersionIdentifiers") != [schema["schema_version"]]:
                raise SystemExit("captured schema version does not match SQLite fixture metadata")
            indexes = sorted(
                f"{table} {name}: {sql if sql is not None else '(auto)'}"
                for table, name, sql in database.execute(
                    "SELECT tbl_name, name, sql FROM sqlite_master WHERE type = 'index'")
            )
            captured = {
                "schema_version": schema["schema_version"],
                "model_checksum": metadata["NSStoreModelVersionChecksumKey"],
                "entity_hashes": {name: value.hex() for name, value in metadata["NSStoreModelVersionHashes"].items()},
                "indexes": indexes,
            }
    except (sqlite3.Error, ValueError, TypeError, KeyError, AttributeError) as error:
        raise SystemExit(f"invalid captured SQLite fixture: {error}") from error
    if captured != schema:
        raise SystemExit("captured schema description does not match SQLite fixture metadata")


def add_release(root, manifest, fixture):
    if manifest.get("format_version") != 1:
        raise SystemExit("unsupported build manifest")
    schema = manifest["schema"]
    validate_schema(schema)
    version = schema["schema_version"]
    if version == "1.0.0":
        raise SystemExit("V1 must remain unchanged")
    registry = read_registry(root)
    historical = validate_historical_schemas(root, registry)
    if version in historical:
        raise SystemExit("Historical schema version is reserved; publish the current V3 or a later schema")
    if any(entry["schema"]["model_checksum"] == schema["model_checksum"] for entry in historical.values()):
        raise SystemExit("a new schema version cannot reuse a historical model checksum")
    commit = manifest["platform_sha"]
    if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise SystemExit("release source must be a full Git SHA")
    fixture_bytes = pathlib.Path(fixture).read_bytes()
    digest = hashlib.sha256(fixture_bytes).hexdigest()
    if manifest.get("fixture_sha256") != digest or manifest.get("fixture_path") != f"stores/{digest}.store":
        raise SystemExit("fixture does not match build manifest")
    validate_fixture_description(fixture, schema)
    # Check every incoming source graph, including another release with the
    # same checksum. A matching live shape alone cannot prove its copy closes
    # over all stored value types.
    entry = {
        "platform_sha": commit, "schema": schema, "fixture_sha256": digest,
        "fixture_path": f"{FIXTURE_DIR}/{digest}.store",
        "namespace": "DashSchemaSnapshotV" + version.split(".")[0],
    }
    render_snapshot(root, version, entry)
    registry = read_registry(root)
    existing = registry["schemas"].get(version)
    if existing:
        if existing["schema"] != schema:
            raise SystemExit("published schema version already has a different immutable shape")
        # Source commits and SQLite file bytes may differ while the schema is identical.
        return registry
    if any(entry["schema"]["model_checksum"] == schema["model_checksum"] for entry in registry["schemas"].values()):
        raise SystemExit("a new schema version cannot reuse a published model checksum")
    destination = pathlib.Path(root, entry["fixture_path"])
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists() and destination.read_bytes() != fixture_bytes:
        raise SystemExit("release fixture destination already has different bytes")
    destination.write_bytes(fixture_bytes)
    registry["schemas"][version] = entry
    pathlib.Path(root, REGISTRY_FILE).write_text(json.dumps(registry, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return registry


def frozen_files_on_disk(root):
    out_dir = os.path.join(root, OUT_DIR)
    return {TEST_REGISTRY_FILE} | {
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
    parser.add_argument("--repo", help="repository containing the schema data; defaults to the current directory")
    parser.add_argument("--check-inventory", action="store_true",
                        help="verify the working source inventory's stored type graph without generating files")
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare with the files on disk instead of writing; exit 1 on any difference",
    )
    parser.add_argument("--release-manifest", help="verified build manifest from the released archive")
    parser.add_argument("--fixture", help="captured SQLite store matching the manifest")
    args = parser.parse_args()
    if args.check_inventory and (args.check or args.release_manifest or args.fixture):
        parser.error("--check-inventory cannot be combined with --check, --release-manifest or --fixture")
    if bool(args.release_manifest) != bool(args.fixture) or (args.check and args.release_manifest):
        parser.error("--release-manifest and --fixture are required together and cannot use --check")
    root = repo_root(args.repo)
    if args.check_inventory:
        inventory = validate_inventory(json.loads(pathlib.Path(root, INVENTORY_FILE).read_text(encoding="utf-8")))
        validate_storage_graph(inventory, inventory_sources(root, inventory))
        print("Supported stored declarations reference only inventoried or standard types; native schema checks remain required")
        return
    if args.release_manifest:
        with open(args.release_manifest, encoding="utf-8") as source:
            add_release(root, json.load(source), args.fixture)
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
        print(f"{len(files)} generated files match the baseline and release registry")
        return

    os.makedirs(os.path.join(root, OUT_DIR), exist_ok=True)
    for path in sorted(frozen_files_on_disk(root) - files.keys()):
        os.remove(os.path.join(root, path))
        print(f"removed {path}")
    for path, text in sorted(files.items()):
        os.makedirs(os.path.dirname(os.path.join(root, path)), exist_ok=True)
        with open(os.path.join(root, path), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(path)


if __name__ == "__main__":
    main()
