#!/usr/bin/env python3
"""Tests for freeze_schema_models.py.

Run from the repository root:

    python3 -m unittest discover -s packages/swift-sdk/scripts -p 'test_*.py'

These cover the generator as a generator: that `--check` is an exact byte
comparison of the committed frozen files against what `FREEZES` and the
repository history produce, and that the copy is not truncated by a brace
inside a string or a comment. Whether the freeze is COMPLETE is not a
question this script can answer; see `DashModelMigrationTests`.
"""

import os
import contextlib
import hashlib
import json
import plistlib
import sqlite3
from pathlib import Path
from unittest import mock
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import freeze_schema_models as gen  # noqa: E402

ROOT = gen.repo_root()


class CheckTests(unittest.TestCase):
    """`--check` proves one thing: the committed files are the generator's output."""

    @classmethod
    def setUpClass(cls):
        cls.files = gen.render_all(ROOT)

    def setUp(self):
        # A scratch root holding a copy of the committed frozen files.
        self.scratch = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.scratch)
        shutil.copytree(
            os.path.join(ROOT, gen.OUT_DIR), os.path.join(self.scratch, gen.OUT_DIR)
        )
        target = os.path.join(self.scratch, gen.TEST_REGISTRY_FILE)
        os.makedirs(os.path.dirname(target), exist_ok=True)
        shutil.copyfile(os.path.join(ROOT, gen.TEST_REGISTRY_FILE), target)

    def test_should_find_the_committed_files_are_the_generators_output(self):
        self.assertEqual(len(gen.render_baseline(ROOT)), 71)
        self.assertIn(gen.TEST_REGISTRY_FILE, self.files)
        self.assertEqual(gen.check_problems(ROOT, self.files), [])

    def test_should_report_a_hand_edit_to_a_frozen_file(self):
        path = f"{gen.OUT_DIR}/DashSchemaV1+PersistentWallet.swift"
        with open(os.path.join(self.scratch, path), "a", encoding="utf-8") as f:
            f.write("// edited by hand\n")
        self.assertEqual(gen.check_problems(self.scratch, self.files), [f"differs:  {path}"])

    def test_should_report_a_missing_and_a_stale_file(self):
        missing = f"{gen.OUT_DIR}/DashSchemaV1+PersistentAssetLock.swift"
        stale = f"{gen.OUT_DIR}/DashSchemaV9+PersistentGhost.swift"
        os.rename(
            os.path.join(self.scratch, missing), os.path.join(self.scratch, stale)
        )
        self.assertEqual(
            gen.check_problems(self.scratch, self.files),
            [f"missing:  {missing}", f"stale:    {stale}"],
        )


class BlockEndTests(unittest.TestCase):
    """A brace that is not code must not end the copied block early."""

    def test_should_not_count_braces_in_strings_and_comments(self):
        lines = [
            "final class PersistentThing {",
            '    init() { let brace = "{"; _ = brace }',
            "    // a } in a comment",
            "    var late: Int",
            "}",
            "extension PersistentThing {}",
        ]
        self.assertEqual(gen.block_end(lines, 0), 4)

    def test_should_refuse_a_block_comment_rather_than_misread_it(self):
        lines = ["final class PersistentThing {", "    /* { */", "}"]
        with self.assertRaises(SystemExit):
            gen.block_end(lines, 0)


class StorageGraphTests(unittest.TestCase):
    def setUp(self):
        self.model_path = gen.MODELS_DIR + "/PersistentThing.swift"
        self.values_path = "packages/swift-sdk/Sources/SwiftDashSDK/Persistence/Types/Values.swift"
        self.inventory = {"format_version": 1, "models": {"PersistentThing": self.model_path}, "value_types": []}
        self.sources = {self.model_path: "@Model\npublic final class PersistentThing {\n var payload: Payload?\n}\n"}

    def include_values(self, names, source):
        self.inventory["value_types"] = [{"path": self.values_path, "names": names}]
        self.sources[self.values_path] = source

    def check(self):
        gen.validate_storage_graph(gen.validate_inventory(self.inventory), self.sources)

    def test_should_refuse_a_real_omitted_stored_codable_type(self):
        self.sources[self.values_path] = "public struct Payload: Codable {\n var value: String\n}\n"
        with self.assertRaisesRegex(SystemExit, "PersistentThing.payload: stored type Payload is absent"):
            self.check()
        self.include_values(["Payload"], self.sources[self.values_path])
        self.check()

    def test_should_follow_transitive_struct_fields_and_enum_payloads_through_containers(self):
        source = ("public struct Payload: Codable {\n var values: [String: Array<Optional<Event>>]\n}\n"
                  "public enum Event: Codable {\n case changed(detail: [String: Leaf?]), deleted\n}\n"
                  "public struct Leaf: Codable {\n var values: Swift.Set<Foundation.UUID>\n}\n")
        self.include_values(["Payload", "Event"], source)
        with self.assertRaisesRegex(SystemExit, "Event enum payload: stored type Leaf is absent"):
            self.check()
        self.inventory["value_types"][0]["names"].append("Leaf")
        self.check()
        self.sources[self.values_path] = source.replace("Swift.Set<Foundation.UUID>", "Dictionary<String, Missing>")
        with self.assertRaisesRegex(SystemExit, "Leaf.values: stored type Missing is absent"):
            self.check()

    def test_should_ignore_computed_static_and_transient_helpers_but_not_observed_storage(self):
        self.sources[self.model_path] = """@Model
public final class PersistentThing {
    @Transient var helper: LiveHelper?
    static var cache = LiveHelper()
    var computed: LiveHelper { LiveHelper() }
    var nextLine: LiveHelper
    { LiveHelper() }
    var stored: String = "var phantom: Missing { // }"
    // var omitted: Missing
    func helper(defaultValue: () -> LiveHelper = { LiveHelper() }) { }
}
"""
        self.check()
        self.sources[self.model_path] = self.sources[self.model_path].replace(
            'var stored: String = "var phantom: Missing { // }"',
            "var stored: Missing { didSet { print(stored) } }")
        with self.assertRaisesRegex(SystemExit, "PersistentThing.stored: stored type Missing is absent"):
            self.check()

    def test_should_fail_closed_for_unsupported_storage_forms(self):
        forms = {
            "var payload = Payload()": "inferred stored types",
            "var payload: External.Payload?": "not isolated",
            "typealias Alias = Payload\nvar payload: Alias": "aliases",
            "var payload: Alias": "stored type Alias is absent",
            "var payload: (String, Int)": "unsupported stored type",
            "var payload: Wrapper<String>": "stored type Wrapper is absent",
            "var a: String, b: Missing": "unsupported stored type",
            "#if DEBUG\nvar payload: String\n#endif": "conditional declarations",
            "@Attribute(.transformable(by: LiveTransformer.self)) var payload: Data": "transformable",
            "@MyStorage var payload: String": "unsupported property macro",
            "enum Swift { }\nvar payload: Swift.String": "shadows a stored type",
        }
        for declaration, error in forms.items():
            self.sources[self.model_path] = "@Model\npublic final class PersistentThing {\n" + declaration + "\n}\n"
            with self.subTest(declaration=declaration), self.assertRaisesRegex(SystemExit, error):
                self.check()

    def test_should_refuse_to_guess_unsupported_lexical_syntax(self):
        for declaration, error in (("var `payload`: String", "escaped identifiers"),
                                   ('var payload: String = #"raw"#', "raw/multiline"),
                                   ("/* hidden } */ var payload: Missing", "block comments")):
            self.sources[self.model_path] = "@Model\npublic final class PersistentThing {\n" + declaration + "\n}\n"
            with self.subTest(declaration=declaration), self.assertRaisesRegex(SystemExit, error):
                self.check()

    def test_should_check_the_current_inventory_and_detect_a_real_transitive_omission(self):
        inventory = gen.validate_inventory(json.loads(Path(ROOT, gen.INVENTORY_FILE).read_text()))
        sources = gen.inventory_sources(ROOT, inventory)
        gen.validate_storage_graph(inventory, sources)
        inventory["value_types"][0]["names"].remove("DistributionEvent")
        with self.assertRaisesRegex(SystemExit, "TokenPreProgrammedDistribution.distributionSchedule: stored type DistributionEvent is absent"):
            gen.validate_storage_graph(inventory, sources)

    def test_should_reject_omitted_values_before_rendering_a_snapshot(self):
        schema = {"schema_version": "2.0.0", "model_checksum": "checksum", "entity_hashes": {"PersistentThing": "ab"}, "indexes": []}
        entry = {"platform_sha": "a" * 40, "namespace": "DashSchemaSnapshotV2", "schema": schema}
        def read(root, operation, object_path):
            self.assertEqual(operation, "show")
            path = object_path.split(":", 1)[1]
            return json.dumps(self.inventory) if path == gen.INVENTORY_FILE else self.sources[path]
        with mock.patch.object(gen, "git", side_effect=read):
            with self.assertRaisesRegex(SystemExit, "stored type Payload is absent"):
                gen.render_snapshot(ROOT, "2.0.0", entry)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.root)
        registry = Path(self.root, gen.REGISTRY_FILE)
        registry.parent.mkdir(parents=True)
        registry.write_text(json.dumps({"format_version": 1, "schemas": {}, "releases": {"apple": {"build_id": "existing"}}}))
        self.fixture = Path(self.root, "capture.store")
        self.schema = {
            "schema_version": "2.0.0", "model_checksum": "checksum",
            "entity_hashes": {"PersistentThing": "abcd"}, "indexes": []}
        with contextlib.closing(sqlite3.connect(self.fixture)) as database, database:
            database.execute("CREATE TABLE Z_METADATA (Z_PLIST BLOB)")
            metadata = {"NSStoreModelVersionIdentifiers": ["2.0.0"], "NSStoreModelVersionChecksumKey": "checksum", "NSStoreModelVersionHashes": {"PersistentThing": bytes.fromhex("abcd")}}
            database.execute("INSERT INTO Z_METADATA VALUES (?)", (plistlib.dumps(metadata, fmt=plistlib.FMT_BINARY),))
        digest = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
        self.manifest = {
            "format_version": 1, "platform_sha": "a" * 40, "schema": self.schema,
            "fixture_sha256": digest, "fixture_path": f"stores/{digest}.store"}
        self.inventory = {
            "format_version": 1,
            "models": {"PersistentThing": gen.MODELS_DIR + "/PersistentThing.swift"},
            "value_types": []}
        self.git_calls = []

        def git(root, *args):
            self.git_calls.append(args)
            if args[0] == "show" and args[1].split(":", 1)[1] == gen.INVENTORY_FILE:
                return json.dumps(self.inventory)
            if args[0] == "show" and args[1].split(":", 1)[1] == gen.MODELS_DIR + "/PersistentThing.swift":
                return "@Model\npublic final class PersistentThing {\n    var old: String = \"released\"\n}\n"
            raise AssertionError(f"unexpected historical read: {args}")

        self.addCleanup(mock.patch.stopall)
        mock.patch.object(gen, "git", side_effect=git).start()

    def historical_registry(self):
        path = "packages/swift-sdk/SwiftTests/SwiftDashSDKTests/Fixtures/SchemaStores/historical-v2.store"
        target = Path(self.root, path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(self.fixture.read_bytes())
        registry = gen.read_registry(self.root)
        registry["historical_schemas"] = {"2.0.0": {
            "schema": dict(self.schema), "fixture_path": path,
            "fixture_sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
            "source_sha": gen.HISTORICAL_V2_SOURCE, "provenance": "reconstructed-model-match",
            "app_store_baseline": {
                "bundle_id": "org.dashfoundation.dash", "app_id": "1206647026", "app_version": "9.0.2",
                "release_id": "679060c3-2e64-49d6-b28e-baa307c817be"}}}
        Path(self.root, gen.REGISTRY_FILE).write_text(json.dumps(registry))
        return registry

    def test_should_validate_the_historical_app_store_baseline_the_ios_gate_reads(self):
        self.assertEqual(gen.validate_historical_schemas(self.root, self.historical_registry()).keys(), {"2.0.0"})
        mutations = {
            "missing": lambda entry: entry.pop("app_store_baseline"),
            "not an object": lambda entry: entry.__setitem__("app_store_baseline", "9.0.2"),
            "missing field": lambda entry: entry["app_store_baseline"].pop("release_id"),
            "extra field": lambda entry: entry["app_store_baseline"].__setitem__("build_number", "30"),
            "empty version": lambda entry: entry["app_store_baseline"].__setitem__("app_version", ""),
            "non-string app id": lambda entry: entry["app_store_baseline"].__setitem__("app_id", 1206647026),
            "malformed release id": lambda entry: entry["app_store_baseline"].__setitem__("release_id", "release-30"),
            "malformed bundle id": lambda entry: entry["app_store_baseline"].__setitem__("bundle_id", "dash"),
        }
        for name, mutate in mutations.items():
            registry = self.historical_registry()
            mutate(registry["historical_schemas"]["2.0.0"])
            with self.subTest(case=name), self.assertRaisesRegex(SystemExit, "App Store baseline"):
                gen.validate_historical_schemas(self.root, registry)

    def test_should_reserve_historical_v2_without_writing_snapshot(self):
        self.historical_registry()
        before = Path(self.root, gen.REGISTRY_FILE).read_bytes()
        with self.assertRaisesRegex(SystemExit, "Historical schema version is reserved"):
            gen.add_release(self.root, self.manifest, self.fixture)
        self.assertEqual(Path(self.root, gen.REGISTRY_FILE).read_bytes(), before)
        self.assertFalse(Path(self.root, gen.FIXTURE_DIR).exists())

    def test_should_reject_modified_historical_fixture(self):
        registry = self.historical_registry()
        Path(self.root, registry["historical_schemas"]["2.0.0"]["fixture_path"]).write_bytes(b"changed")
        with self.assertRaisesRegex(SystemExit, "immutable historical fixture"):
            gen.validate_historical_schemas(self.root, registry)

    def test_should_reject_changed_historical_schema_or_reconstruction_source(self):
        for field in ("source_sha", "schema"):
            registry = self.historical_registry()
            entry = registry["historical_schemas"]["2.0.0"]
            if field == "source_sha":
                entry[field] = "f" * 40
            else:
                entry[field]["model_checksum"] = "changed"
            with self.subTest(field=field), self.assertRaises(SystemExit):
                gen.validate_historical_schemas(self.root, registry)

    def test_should_copy_exact_captured_commit_and_preserve_release_metadata(self):
        registry = gen.add_release(self.root, self.manifest, self.fixture)
        entry = registry["schemas"]["2.0.0"]
        self.assertEqual(registry["releases"], {"apple": {"build_id": "existing"}})
        self.assertEqual(Path(self.root, entry["fixture_path"]).read_bytes(), self.fixture.read_bytes())
        rendered = gen.render_snapshot(self.root, "2.0.0", entry)
        self.assertIn('var old: String = "released"', rendered[f"{gen.OUT_DIR}/DashSchemaSnapshotV2+PersistentThing.swift"])
        self.assertIn("enum DashSchemaSnapshotV2", rendered[f"{gen.OUT_DIR}/DashSchemaSnapshotV2+Schema.swift"])
        self.assertTrue(all("a" * 40 + ":" in args[-1] for args in self.git_calls))

    def test_should_close_fixture_connection_after_success_or_validation_failure(self):
        for version in ("2.0.0", "3.0.0"):
            database = sqlite3.connect(self.fixture)
            schema = dict(self.schema, schema_version=version)
            with mock.patch.object(gen.sqlite3, "connect", return_value=database):
                if version == "2.0.0":
                    gen.validate_fixture_description(self.fixture, schema)
                else:
                    with self.assertRaisesRegex(SystemExit, "schema version does not match"):
                        gen.validate_fixture_description(self.fixture, schema)
            with self.assertRaises(sqlite3.ProgrammingError):
                database.execute("SELECT 1")

    def test_should_reuse_same_shape_even_when_a_later_build_has_different_bytes_and_sha(self):
        first = gen.add_release(self.root, self.manifest, self.fixture)
        with contextlib.closing(sqlite3.connect(self.fixture)) as database, database:
            database.execute("CREATE TABLE unimportant (value INTEGER)")
        digest = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
        next_manifest = {**self.manifest, "platform_sha": "b" * 40,
                         "fixture_sha256": digest, "fixture_path": f"stores/{digest}.store"}
        self.assertEqual(gen.add_release(self.root, next_manifest, self.fixture), first)

    def test_should_reject_missing_value_types_before_admitting_new_or_same_shape_evidence(self):
        original_read = gen.git

        def omitted_value(root, *args):
            if args[0] == "show" and args[1].endswith("/PersistentThing.swift"):
                return "@Model\npublic final class PersistentThing {\n var payload: MissingCodable?\n}\n"
            return original_read(root, *args)

        for already_registered in (False, True):
            if already_registered:
                gen.add_release(self.root, self.manifest, self.fixture)
            before = Path(self.root, gen.REGISTRY_FILE).read_bytes()
            with self.subTest(already_registered=already_registered), mock.patch.object(gen, "git", side_effect=omitted_value):
                with self.assertRaisesRegex(SystemExit, "stored type MissingCodable is absent"):
                    gen.add_release(self.root, self.manifest, self.fixture)
            self.assertEqual(Path(self.root, gen.REGISTRY_FILE).read_bytes(), before)
            if not already_registered:
                self.assertFalse(Path(self.root, gen.FIXTURE_DIR).exists())

    def test_should_reject_changed_shape_under_published_version_including_indexes(self):
        gen.add_release(self.root, self.manifest, self.fixture)
        original = self.fixture.read_bytes()
        for field in ["model_checksum", "indexes"]:
            self.fixture.write_bytes(original)
            schema = dict(self.schema)
            with contextlib.closing(sqlite3.connect(self.fixture)) as database, database:
                if field == "model_checksum":
                    metadata = plistlib.loads(database.execute("SELECT Z_PLIST FROM Z_METADATA").fetchone()[0])
                    metadata["NSStoreModelVersionChecksumKey"] = "changed"
                    database.execute("UPDATE Z_METADATA SET Z_PLIST = ?", (plistlib.dumps(metadata),))
                    schema[field] = "changed"
                else:
                    database.execute("CREATE INDEX fixture_index ON Z_METADATA (Z_PLIST)")
                    schema[field] = ["Z_METADATA fixture_index: CREATE INDEX fixture_index ON Z_METADATA (Z_PLIST)"]
            digest = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
            manifest = {**self.manifest, "schema": schema,
                        "fixture_sha256": digest, "fixture_path": f"stores/{digest}.store"}
            with self.subTest(field=field), self.assertRaisesRegex(SystemExit, "immutable shape"):
                gen.add_release(self.root, manifest, self.fixture)

    def test_should_reject_duplicate_checksum_under_another_version(self):
        gen.add_release(self.root, self.manifest, self.fixture)
        with contextlib.closing(sqlite3.connect(self.fixture)) as database, database:
            metadata = plistlib.loads(database.execute("SELECT Z_PLIST FROM Z_METADATA").fetchone()[0])
            metadata["NSStoreModelVersionIdentifiers"] = ["3.0.0"]
            database.execute("UPDATE Z_METADATA SET Z_PLIST = ?", (plistlib.dumps(metadata),))
        digest = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
        manifest = {**self.manifest, "schema": {**self.schema, "schema_version": "3.0.0"},
                    "fixture_sha256": digest, "fixture_path": f"stores/{digest}.store"}
        with self.assertRaisesRegex(SystemExit, "reuse a published model checksum"):
            gen.add_release(self.root, manifest, self.fixture)

    def test_should_reject_mismatched_evidence_before_writing_registry(self):
        self.fixture.write_bytes(b"modified")
        with self.assertRaisesRegex(SystemExit, "fixture does not match"):
            gen.add_release(self.root, self.manifest, self.fixture)
        self.assertEqual(gen.read_registry(self.root)["schemas"], {})

    def test_should_reject_correct_digest_with_fabricated_schema_description(self):
        manifest = {**self.manifest, "schema": {**self.schema, "entity_hashes": {"PersistentThing": "beef"}}}
        with self.assertRaisesRegex(SystemExit, "does not match SQLite fixture metadata"):
            gen.add_release(self.root, manifest, self.fixture)
        self.assertEqual(gen.read_registry(self.root)["schemas"], {})

    def test_should_reject_an_incorrect_store_version(self):
        manifest = {**self.manifest, "schema": {**self.schema, "schema_version": "3.0.0"}}
        with self.assertRaisesRegex(SystemExit, "schema version does not match"):
            gen.add_release(self.root, manifest, self.fixture)

    def test_should_reject_inventory_that_omits_a_captured_entity(self):
        self.inventory["models"] = {}
        with self.assertRaisesRegex(SystemExit, "model membership"):
            gen.add_release(self.root, self.manifest, self.fixture)
        self.assertEqual(gen.read_registry(self.root)["schemas"], {})

    def test_should_never_accept_a_new_v1_snapshot(self):
        manifest = {**self.manifest, "schema": {**self.schema, "schema_version": "1.0.0"}}
        with self.assertRaisesRegex(SystemExit, "V1 must remain unchanged"):
            gen.add_release(self.root, manifest, self.fixture)

    def test_should_check_generated_registry_and_immutable_fixture(self):
        registry = gen.add_release(self.root, self.manifest, self.fixture)
        with mock.patch.object(gen, "render_baseline", return_value={}):
            files = gen.render_all(self.root)
            self.assertIn("DashSchemaSnapshotV2.self", files[gen.TEST_REGISTRY_FILE])
            Path(self.root, registry["schemas"]["2.0.0"]["fixture_path"]).write_bytes(b"edited")
            with self.assertRaisesRegex(SystemExit, "immutable release fixture"):
                gen.render_all(self.root)


if __name__ == "__main__":
    unittest.main()
