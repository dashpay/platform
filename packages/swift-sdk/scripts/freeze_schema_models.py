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
fixture digests. It cannot prove hash-relevant graph completeness: the runtime
DashReleasedSchemaTests compares SwiftData's hashes and SQLite indexes against
the captured fixture, after constructing the live schema first. This also
catches inline value types or relationship references accidentally left live.
Do not replace those runtime checks with a static scan of model source.

Usage:
    python3 packages/swift-sdk/scripts/freeze_schema_models.py --check
    python3 packages/swift-sdk/scripts/freeze_schema_models.py \
        --release-manifest build.json --fixture fixture.store

A full-history checkout is required. New manifests must refer to a commit that
already contains schema-models.json. Historical fixture files are never rebuilt
from today's sources.
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
    if inventory.get("format_version") != 1:
        raise SystemExit("unsupported historical schema model inventory")
    models = inventory["models"]
    groups = inventory["value_types"]
    for name in list(models) + [name for group in groups for name in group["names"]]:
        if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name):
            raise SystemExit("invalid Swift name in model inventory")
    for path in list(models.values()) + [group["path"] for group in groups]:
        if not path.startswith("packages/swift-sdk/Sources/SwiftDashSDK/") or ".." in pathlib.PurePosixPath(path).parts:
            raise SystemExit("model inventory path is outside the Swift SDK")
    return inventory


def render_snapshot(root, version, entry):
    validate_schema(entry["schema"])
    if entry["schema"]["schema_version"] != version:
        raise SystemExit("registry key does not match captured schema version")
    commit = entry["platform_sha"]
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise SystemExit("release source must be a full Git SHA")
    namespace = "DashSchemaSnapshotV" + version.split(".")[0]
    if entry["namespace"] != namespace:
        raise SystemExit("unexpected snapshot namespace")
    inventory = read_inventory(root, commit)
    models = inventory["models"]
    if set(models) != set(entry["schema"]["entity_hashes"]):
        raise SystemExit("historical inventory differs from captured model membership")
    names = set(models) | {name for group in inventory["value_types"] for name in group["names"]}
    sibling = re.compile(r"(?<![\w.])(" + "|".join(re.escape(name) for name in sorted(names)) + r")\b")
    freeze = Freeze(namespace, commit, tuple(models))
    files = {}
    for name, path in models.items():
        files[f"{OUT_DIR}/{namespace}+{name}.swift"] = render_model(
            freeze, commit[:10], git(root, "show", f"{commit}:{path}"), name, sibling)
    for group in inventory["value_types"]:
        value_freeze = Freeze(namespace, commit, (), group["path"], tuple(group["names"]))
        path = f"{OUT_DIR}/{namespace}+{os.path.basename(group['path'])}"
        if path in files:
            raise SystemExit("duplicate snapshot output")
        files[path] = render_value_types(value_freeze, commit[:10], git(root, "show", f"{commit}:{group['path']}"))
    files[f"{OUT_DIR}/{namespace}+Schema.swift"] = (
        HEADER + "// Generated release snapshot; never add alongside its live version in the migration plan.\n"
        + f"enum {namespace}: VersionedSchema {{\n"
        + f"    static var versionIdentifier: Schema.Version {{ Schema.Version({version.replace('.', ', ')}) }}\n"
        + "    static var models: [any PersistentModel.Type] {\n        [\n"
        + ",\n".join(f"            {name}.self" for name in models)
        + "\n        ]\n    }\n}\n"
    )
    return files


def render_all(root, registry=None):
    files = render_baseline(root)
    registry = read_registry(root) if registry is None else registry
    fixtures = []
    checksums = set()
    for version, entry in sorted(registry["schemas"].items(), key=lambda item: tuple(map(int, item[0].split('.')))):
        if version == "1.0.0":
            raise SystemExit("V1 is the unchanged accepted baseline, not a new release snapshot")
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
    commit = manifest["platform_sha"]
    if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise SystemExit("release source must be a full Git SHA")
    fixture_bytes = pathlib.Path(fixture).read_bytes()
    digest = hashlib.sha256(fixture_bytes).hexdigest()
    if manifest.get("fixture_sha256") != digest or manifest.get("fixture_path") != f"stores/{digest}.store":
        raise SystemExit("fixture does not match build manifest")
    validate_fixture_description(fixture, schema)
    registry = read_registry(root)
    existing = registry["schemas"].get(version)
    if existing:
        if existing["schema"] != schema:
            raise SystemExit("published schema version already has a different immutable shape")
        # Source commits and SQLite file bytes may differ while the schema is identical.
        return registry
    if any(entry["schema"]["model_checksum"] == schema["model_checksum"] for entry in registry["schemas"].values()):
        raise SystemExit("a new schema version cannot reuse a published model checksum")
    entry = {
        "platform_sha": commit, "schema": schema, "fixture_sha256": digest,
        "fixture_path": f"{FIXTURE_DIR}/{digest}.store",
        "namespace": "DashSchemaSnapshotV" + version.split(".")[0],
    }
    # Resolve and render everything before changing the registry or copying evidence.
    render_snapshot(root, version, entry)
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
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare with the files on disk instead of writing; exit 1 on any difference",
    )
    parser.add_argument("--release-manifest", help="verified build manifest from the released archive")
    parser.add_argument("--fixture", help="captured SQLite store matching the manifest")
    args = parser.parse_args()
    if bool(args.release_manifest) != bool(args.fixture) or (args.check and args.release_manifest):
        parser.error("--release-manifest and --fixture are required together and cannot use --check")
    root = repo_root()
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
