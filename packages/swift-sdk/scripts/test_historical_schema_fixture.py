#!/usr/bin/env python3
"""Regression checks for the pinned, source-reconstructed legacy fixture."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import historical_schema_fixture as historical


ROOT = Path(historical.freeze.repo_root())


class HistoricalFixtureTests(unittest.TestCase):
    def setUp(self):
        self.manifest = historical.read_manifest(ROOT)

    def test_should_reproduce_the_committed_artifacts_from_the_pinned_source_pair(self):
        graph = historical.verify(ROOT)
        self.assertEqual(len(graph), 36)
        self.assertEqual(len(self.manifest["schema"]["entity_hashes"]), 34)
        self.assertNotIn("PersistentTrackedMasternode", self.manifest["inventory"]["models"])

    def test_should_reject_provenance_that_claims_a_different_source_or_app_store_release(self):
        with tempfile.TemporaryDirectory() as scratch:
            path = Path(scratch, historical.MANIFEST)
            path.parent.mkdir(parents=True)
            for key, value in (("platform_sha", "a" * 40), ("wallet_sha", "b" * 40),
                               ("app_store_provenance", "verified")):
                path.write_text(json.dumps(dict(self.manifest, **{key: value})))
                with self.subTest(key=key), self.assertRaisesRegex(SystemExit, "provenance changed"):
                    historical.read_manifest(scratch)

    def test_should_reject_changed_fixture_bytes_before_reading_history(self):
        with tempfile.TemporaryDirectory() as scratch:
            fixture = Path(scratch, historical.FIXTURE_DIR, "fixture.store")
            fixture.parent.mkdir(parents=True)
            fixture.write_bytes(b"not the recorded fixture")
            with mock.patch.object(historical.freeze, "git") as git:
                with self.assertRaisesRegex(SystemExit, "checksum mismatch"):
                    historical.verify(scratch, self.manifest)
                git.assert_not_called()

    def test_should_reject_a_schema_description_that_does_not_match_the_sqlite_store(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["schema"]["entity_hashes"]["PersistentDocumentType"] = "00" * 32
        with self.assertRaisesRegex(SystemExit, "SQLite fixture metadata"):
            historical.verify(ROOT, manifest)

    def test_should_reject_changes_to_the_recorded_synthetic_rows(self):
        self.manifest["synthetic_record_counts"]["ZPERSISTENTWALLET"] = 2
        with self.assertRaisesRegex(SystemExit, "synthetic records changed"):
            historical.verify(ROOT, self.manifest)

    def test_should_reject_an_inventory_that_differs_from_the_historical_factory(self):
        self.manifest["inventory"]["models"].pop("PersistentWallet")
        with self.assertRaisesRegex(SystemExit, "pinned factory"):
            historical.render_graph(ROOT, self.manifest)

    def test_should_detect_source_digest_and_generator_output_drift(self):
        for field, error in (("source_file_sha256", "source file digests"),
                             ("generated_file_sha256", "graph regeneration")):
            manifest = copy.deepcopy(self.manifest)
            manifest[field][next(iter(manifest[field]))] = "0" * 64
            with self.subTest(field=field), self.assertRaisesRegex(SystemExit, error):
                historical.verify(ROOT, manifest)

    def test_should_detect_a_changed_capture_recipe(self):
        self.manifest["capture_test_sha256"] = "0" * 64
        with self.assertRaisesRegex(SystemExit, "capture recipe changed"):
            historical.verify(ROOT, self.manifest)


class CapturePreparationTests(unittest.TestCase):
    def test_should_refuse_to_export_into_the_repository_or_an_existing_directory(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch, "repository")
            root.mkdir()
            for destination in (root / "new-sdk", Path(scratch)):
                with self.subTest(destination=destination), self.assertRaisesRegex(SystemExit, "new directory outside"):
                    historical.prepare_sdk(root, destination)
            self.assertEqual(list(root.iterdir()), [])

    def test_should_export_historical_models_only_into_a_disposable_sdk_copy(self):
        with tempfile.TemporaryDirectory() as scratch:
            root, destination = Path(scratch, "repository"), Path(scratch, "capture-sdk")
            sdk = root / historical.SDK
            for folder in ("Sources/SwiftDashSDK", "SwiftTests/SwiftDashSDKTests", "DashSDKFFI.xcframework"):
                (sdk / folder).mkdir(parents=True)
            (sdk / "Package.swift").write_text("// package\n")
            (sdk / "Sources/SwiftDashSDK/Current.swift").write_text("// current\n")
            template = root / historical.CAPTURE_TEST
            template.parent.mkdir(parents=True)
            template.write_text("// capture test\n")
            generated = f"{historical.freeze.OUT_DIR}/DashSchemaSnapshotV1+Schema.swift"
            with mock.patch.object(historical, "verify", return_value={generated: "// historical\n"}):
                historical.prepare_sdk(root, destination)
            self.assertFalse((root / generated).exists())
            self.assertEqual((destination / Path(generated).relative_to(historical.SDK)).read_text(), "// historical\n")
            self.assertEqual((destination / "Sources/SwiftDashSDK/Current.swift").read_text(), "// current\n")
            self.assertEqual((destination / "SwiftTests/SwiftDashSDKTests/DashHistoricalFixtureCaptureTests.swift").read_bytes(), template.read_bytes())
            self.assertEqual((destination / "DashSDKFFI.xcframework").resolve(), (sdk / "DashSDKFFI.xcframework").resolve())


if __name__ == "__main__":
    unittest.main()
