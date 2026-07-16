from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_sdk_parity_manifest as checker  # noqa: E402


class SdkParityManifestTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.repo_root = Path(self.temp_dir.name)
        (self.repo_root / "src").mkdir()
        (self.repo_root / "tests").mkdir()
        (self.repo_root / "src/api.rs").write_text(
            "pub fn shared_api() {}\n", encoding="utf-8"
        )
        (self.repo_root / "tests/plan.md").write_text(
            "# CAP-01 restart verification\n", encoding="utf-8"
        )
        self.manifest = {
            "schema_version": 1,
            "baseline": "PR #1 @ 0123456789abcdef0123456789abcdef01234567",
            "description": "Minimal valid parity manifest fixture.",
            "declared_persistence_capabilities": ["atomic_changesets"],
            "shared_symbols": {"shared_api": "src/api.rs"},
            "capabilities": [
                {
                    "id": "test.capability",
                    "title": "Fixture capability",
                    "area": "test",
                    "shared_apis": ["shared_api"],
                    "required_persistence_capabilities": ["atomic_changesets"],
                    "hosts": {
                        "swift": {
                            "sdk": "supported",
                            "example_app": "not-applicable",
                            "restart": "tested",
                            "reason": None,
                        },
                        "kotlin": {
                            "sdk": "unsupported",
                            "example_app": "unsupported",
                            "restart": "required",
                            "reason": "The fixture intentionally has no Kotlin implementation.",
                        },
                    },
                    "verification": [
                        {
                            "host": "swift",
                            "kind": "unit",
                            "file": "tests/plan.md",
                            "id": "CAP-01",
                            "command": "run-fixture-test",
                            "covers_restart": True,
                        }
                    ],
                }
            ],
        }

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_valid_manifest_renders_deterministic_counts(self) -> None:
        checker.validate_manifest(self.manifest, self.repo_root)

        summary = checker.render_summary(self.manifest)

        self.assertIn("Audit baseline: `PR #1", summary)
        self.assertIn("Capabilities tracked: **1**", summary)
        self.assertIn("| Swift | SDK | 1 | 0 | 0 | 0 |", summary)
        self.assertIn("| Kotlin | Example app | 0 | 0 | 1 | 0 |", summary)
        self.assertIn("| Swift | 1 | 0 | 0 |", summary)

    def test_partial_or_unsupported_status_requires_reason(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["capabilities"][0]["hosts"]["kotlin"]["reason"] = None

        with self.assertRaisesRegex(checker.ManifestError, "requires a reason"):
            checker.validate_manifest(manifest, self.repo_root)

    def test_schema_version_rejects_boolean(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["schema_version"] = True

        with self.assertRaisesRegex(checker.ManifestError, "expected integer 1"):
            checker.validate_manifest(manifest, self.repo_root)

    def test_declared_shared_symbol_must_exist_in_source(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["shared_symbols"] = {"missing_api": "src/api.rs"}
        manifest["capabilities"][0]["shared_apis"] = ["missing_api"]

        with self.assertRaisesRegex(checker.ManifestError, "symbol is not present"):
            checker.validate_manifest(manifest, self.repo_root)

    def test_verification_id_must_exist_in_referenced_file(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["capabilities"][0]["verification"][0]["id"] = "CAP-99"

        with self.assertRaisesRegex(checker.ManifestError, "is not present"):
            checker.validate_manifest(manifest, self.repo_root)

    def test_restart_tested_requires_restart_verification(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["capabilities"][0]["verification"][0]["covers_restart"] = False

        with self.assertRaisesRegex(checker.ManifestError, "tested requires covers_restart"):
            checker.validate_manifest(manifest, self.repo_root)

    def test_stale_generated_summary_is_rejected(self) -> None:
        summary_path = self.repo_root / "summary.md"
        summary_path.write_text("stale\n", encoding="utf-8")

        with self.assertRaisesRegex(checker.ManifestError, "summary is stale"):
            checker.check_summary(summary_path, checker.render_summary(self.manifest))

    def test_swift_example_app_test_rejects_swift_package_command(self) -> None:
        example_test = self.repo_root / "packages/swift-sdk/SwiftExampleApp/SwiftExampleAppTests"
        example_test.mkdir(parents=True)
        test_file = example_test / "PricingTests.swift"
        test_file.write_text("func testPrice() {}\n", encoding="utf-8")
        verification = self.manifest["capabilities"][0]["verification"][0]
        verification.update(
            {
                "file": str(test_file.relative_to(self.repo_root)),
                "id": "testPrice",
                "command": "swift test --package-path packages/swift-sdk --filter PricingTests",
                "covers_restart": False,
            }
        )
        self.manifest["capabilities"][0]["hosts"]["swift"]["restart"] = "not_applicable"

        with self.assertRaisesRegex(checker.ManifestError, "SwiftExampleAppTests require"):
            checker.validate_manifest(self.manifest, self.repo_root)

    def test_swift_example_app_test_accepts_its_xcode_target(self) -> None:
        example_test = self.repo_root / "packages/swift-sdk/SwiftExampleApp/SwiftExampleAppTests"
        example_test.mkdir(parents=True)
        test_file = example_test / "PricingTests.swift"
        test_file.write_text("func testPrice() {}\n", encoding="utf-8")
        verification = self.manifest["capabilities"][0]["verification"][0]
        verification.update(
            {
                "file": str(test_file.relative_to(self.repo_root)),
                "id": "testPrice",
                "command": "xcodebuild test -project packages/swift-sdk/SwiftExampleApp/SwiftExampleApp.xcodeproj -scheme SwiftExampleApp",
                "covers_restart": False,
            }
        )
        self.manifest["capabilities"][0]["hosts"]["swift"]["restart"] = "not_applicable"

        checker.validate_manifest(self.manifest, self.repo_root)


if __name__ == "__main__":
    unittest.main()
