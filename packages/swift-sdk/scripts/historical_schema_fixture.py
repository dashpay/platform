#!/usr/bin/env python3
"""Verify and reproduce the source-reconstructed fd8 legacy SwiftData fixture.

The store is synthetic evidence for a pinned iOS/Platform source pair. It is not
an extracted App Store database and does not change the accepted frozen V1.
--check validates the committed artifact and deterministic historical model
rendering. --prepare-sdk exports those models and a dedicated capture test into
a disposable copy of the SDK, never into the production source tree.
"""

import argparse
import contextlib
import hashlib
import json
from pathlib import Path
import re
import shutil
import sqlite3

import freeze_schema_models as freeze


SDK = "packages/swift-sdk"
FIXTURE_DIR = f"{SDK}/SwiftTests/SwiftDashSDKTests/Fixtures/SchemaStores/legacy-fd8d8d13e5"
MANIFEST = f"{FIXTURE_DIR}/manifest.json"
CAPTURE_TEST = f"{SDK}/scripts/fixtures/DashHistoricalFixtureCaptureTests.swift"
PLATFORM_SHA = "fd8d8d13e5d7cea17b00df5974934ab1910e8039"
IOS_SHA = "8094751eb2be8d52b57da3589fdd2ae2dcd0ecc6"
CONTAINER = f"{SDK}/Sources/SwiftDashSDK/Persistence/DashModelContainer.swift"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_manifest(root):
    manifest = json.loads(Path(root, MANIFEST).read_text())
    if (manifest.get("format_version") != 1
            or manifest.get("scope") != "source-reconstructed-synthetic"
            or manifest.get("app_store_provenance") != "not-verified"
            or manifest.get("platform_sha") != PLATFORM_SHA
            or manifest.get("wallet_sha") != IOS_SHA):
        raise SystemExit("historical fixture provenance changed")
    return manifest


def render_graph(root, manifest):
    inventory = freeze.validate_inventory(manifest["inventory"])
    # This particular historical factory has a literal modelTypes list. Compare
    # the explicit inventory with that list; do not infer general Swift types.
    factory = freeze.git(root, "show", f"{PLATFORM_SHA}:{CONTAINER}")
    declaration = re.search(
        r"public static var modelTypes:\s*\[any PersistentModel.Type\]\s*\{\s*\[(.*?)\]\s*\}",
        factory, re.S)
    if declaration is None:
        raise SystemExit("historical modelTypes declaration is missing")
    models = re.findall(r"\b(Persistent\w+)\.self\b", declaration.group(1))
    if models != list(inventory["models"]):
        raise SystemExit("historical inventory differs from its pinned factory")
    return freeze.render_snapshot(root, "1.0.0", {
        "platform_sha": PLATFORM_SHA,
        "namespace": "DashSchemaSnapshotV1",
        "schema": manifest["schema"],
    }, inventory=inventory)


def verify(root, manifest=None):
    root = Path(root)
    manifest = read_manifest(root) if manifest is None else manifest
    freeze.validate_schema(manifest["schema"])
    fixture = root / FIXTURE_DIR / "fixture.store"
    if digest(fixture.read_bytes()) != manifest["fixture_sha256"]:
        raise SystemExit("historical fixture checksum mismatch")
    freeze.validate_fixture_description(fixture, manifest["schema"])
    uri = fixture.resolve().as_uri() + "?mode=ro&immutable=1"
    with contextlib.closing(sqlite3.connect(uri, uri=True)) as database:
        if database.execute("PRAGMA quick_check").fetchone() != ("ok",):
            raise SystemExit("historical fixture SQLite integrity check failed")
        counts = {table: database.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0]
                  for table in ("ZPERSISTENTWALLET", "ZPERSISTENTDATACONTRACT",
                                "ZPERSISTENTDOCUMENTTYPE", "ZPERSISTENTINDEX")}
    if counts != manifest["synthetic_record_counts"]:
        raise SystemExit("historical synthetic records changed")
    files = set(manifest["inventory"]["models"].values()) | {
        item["path"] for item in manifest["inventory"]["value_types"]
    } | {CONTAINER}
    sources = {path: digest(freeze.git(root, "show", f"{PLATFORM_SHA}:{path}").encode())
               for path in sorted(files)}
    if sources != manifest["source_file_sha256"]:
        raise SystemExit("historical source file digests differ from the manifest")
    graph = render_graph(root, manifest)
    if {path: digest(text.encode()) for path, text in sorted(graph.items())} != manifest["generated_file_sha256"]:
        raise SystemExit("historical graph regeneration differs from the recorded generator output")
    if digest((root / CAPTURE_TEST).read_bytes()) != manifest["capture_test_sha256"]:
        raise SystemExit("historical capture recipe changed")
    return graph


def prepare_sdk(root, destination):
    root, destination = Path(root).resolve(), Path(destination).resolve()
    if destination.exists() or destination.is_relative_to(root):
        raise SystemExit("capture SDK must be a new directory outside the repository")
    graph = verify(root)
    sdk = root / SDK
    framework = sdk / "DashSDKFFI.xcframework"
    if not framework.exists():
        raise SystemExit("build the current SDK simulator XCFramework before preparing the capture SDK")
    destination.mkdir(parents=True)
    for folder in ("Sources", "SwiftTests"):
        shutil.copytree(sdk / folder, destination / folder)
    shutil.copy2(sdk / "Package.swift", destination / "Package.swift")
    (destination / "DashSDKFFI.xcframework").symlink_to(framework.resolve(), target_is_directory=True)
    for path, text in graph.items():
        output = destination / Path(path).relative_to(SDK)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text)
    shutil.copy2(root / CAPTURE_TEST, destination / "SwiftTests/SwiftDashSDKTests/DashHistoricalFixtureCaptureTests.swift")
    print(f"Prepared disposable SDK at {destination}")
    print("Run only DashHistoricalFixtureCaptureTests on an arm64 iOS simulator, Release configuration.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--prepare-sdk", type=Path, metavar="NEW_DIRECTORY")
    args = parser.parse_args()
    root = Path(freeze.repo_root(args.repo))
    if args.check:
        graph = verify(root)
        print(f"Historical fixture and {len(graph)} generated source files match the pinned source pair")
    else:
        prepare_sdk(root, args.prepare_sdk)


if __name__ == "__main__":
    main()
