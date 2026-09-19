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
        self.assertEqual(len(gen.render_baseline(ROOT)), 35)
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
            if args == ("show", "a" * 40 + ":" + gen.INVENTORY_FILE):
                return json.dumps(self.inventory)
            if args == ("show", "a" * 40 + ":" + gen.MODELS_DIR + "/PersistentThing.swift"):
                return "@Model\npublic final class PersistentThing {\n    var old: String = \"released\"\n}\n"
            raise AssertionError(f"unexpected historical read: {args}")

        self.addCleanup(mock.patch.stopall)
        mock.patch.object(gen, "git", side_effect=git).start()

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
