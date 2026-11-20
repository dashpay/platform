"""Keep the CI image cleanup from deleting an image a test job can still pull.

The cleanup deletes versions of the images that the Tests workflow pushes to
GHCR. It must leave recent images alone, never take a part of an image before
its tag, cover exactly the images the workflow builds, and never fail the run.
"""
from datetime import datetime, timedelta, timezone
import importlib.util
from pathlib import Path
import re
import unittest
import urllib.error


ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("cleanup_ci_images", ROOT / ".github/scripts/cleanup-ci-images.py")
cleanup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cleanup)

NOW = datetime(2026, 11, 20, 12, 0, tzinfo=timezone.utc)


def version(version_id, age, tags=()):
    pushed = (NOW - age).strftime("%Y-%m-%dT%H:%M:%SZ")
    return {"id": version_id, "updated_at": pushed, "metadata": {"container": {"tags": list(tags)}}}


class FakeGitHub:
    """Serve package versions from memory and record what gets deleted."""

    def __init__(self, packages, owner_type="Organization", refuse=()):
        self.packages = packages
        self.owner_type = owner_type
        self.refuse = set(refuse)
        self.deleted = []

    def versions_path(self, owner, package):
        return package

    def versions(self, path):
        if path not in self.packages:
            raise urllib.error.HTTPError(path, 404, "Not Found", None, None)
        return self.packages[path]

    def request(self, method, path):
        package, version_id = path.rsplit("/", 1)
        if int(version_id) in self.refuse:
            raise urllib.error.HTTPError(path, 400, "Bad Request", None, None)
        self.deleted.append((package, int(version_id)))


class ExpiryTests(unittest.TestCase):
    def test_should_keep_images_younger_than_the_retention(self):
        versions = [
            version(1, timedelta(hours=1), ["sha-new"]),
            version(2, timedelta(days=cleanup.RETENTION_DAYS) - timedelta(minutes=1), ["sha-almost"]),
            version(3, timedelta(days=cleanup.RETENTION_DAYS) + timedelta(minutes=1), ["sha-old"]),
        ]
        self.assertEqual([item["id"] for item in cleanup.expired(versions, NOW)], [3])

    def test_should_keep_the_parts_of_an_image_longer_than_its_tag(self):
        # An image index and the untagged manifests it points to are pushed
        # together. Deleting a manifest first would leave a tag that cannot be
        # pulled.
        age = timedelta(days=cleanup.RETENTION_DAYS) + timedelta(hours=1)
        versions = [version(1, age, ["sha-old"]), version(2, age), version(3, age)]
        self.assertEqual([item["id"] for item in cleanup.expired(versions, NOW)], [1])
        later = NOW + timedelta(days=cleanup.UNTAGGED_GRACE_DAYS)
        self.assertEqual({item["id"] for item in cleanup.expired(versions, later)}, {1, 2, 3})

    def test_should_delete_the_oldest_first(self):
        versions = [version(1, timedelta(days=5), ["a"]), version(2, timedelta(days=9), ["b"]),
                    version(3, timedelta(days=7), ["c"])]
        self.assertEqual([item["id"] for item in cleanup.expired(versions, NOW)], [2, 3, 1])


class CleanTests(unittest.TestCase):
    def packages(self, count=1):
        old = timedelta(days=30)
        return {
            "platform/ci/drive": [version(10 + index, old, [f"sha-{index}"]) for index in range(count)],
            "platform/ci/rs-dapi": [version(20, old, ["sha-a"]), version(21, timedelta(hours=2), ["sha-b"])],
        }

    def test_should_delete_expired_versions_and_skip_a_package_that_does_not_exist_yet(self):
        github = FakeGitHub(self.packages())
        log = []
        self.assertEqual(cleanup.clean(github, "dashpay/platform", NOW, log=log.append), 2)
        self.assertEqual(github.deleted, [("platform/ci/drive", 10), ("platform/ci/rs-dapi", 20)])
        self.assertIn("platform/ci/dashmate-helper: no such package yet", log)

    def test_should_delete_nothing_in_a_dry_run(self):
        github = FakeGitHub(self.packages())
        self.assertEqual(cleanup.clean(github, "dashpay/platform", NOW, dry_run=True, log=lambda _: None), 2)
        self.assertEqual(github.deleted, [])

    def test_should_carry_on_when_a_version_cannot_be_deleted(self):
        github = FakeGitHub(self.packages(), refuse=[10])
        log = []
        self.assertEqual(cleanup.clean(github, "dashpay/platform", NOW, log=log.append), 1)
        self.assertEqual(github.deleted, [("platform/ci/rs-dapi", 20)])
        self.assertTrue(any(line.startswith("::warning::") for line in log))

    def test_should_stop_at_the_deletion_limit(self):
        github = FakeGitHub(self.packages(count=cleanup.MAX_DELETIONS + 5))
        self.assertEqual(cleanup.clean(github, "dashpay/platform", NOW, log=lambda _: None), cleanup.MAX_DELETIONS)
        self.assertEqual(len(github.deleted), cleanup.MAX_DELETIONS)


class WorkflowTests(unittest.TestCase):
    def test_should_name_packages_the_way_the_workflow_pushes_them(self):
        self.assertEqual(cleanup.package_name("dashpay/platform", "drive"), "platform/ci/drive")
        # Docker repository names are lowercase, so the login action lowercases them.
        self.assertEqual(cleanup.package_name("Someone/Platform", "rs-dapi"), "platform/ci/rs-dapi")
        action = (ROOT / ".github/actions/ghcr_login/action.yaml").read_text()
        self.assertIn('image_org=ghcr.io/${GITHUB_REPOSITORY,,}/ci"', action)

    def test_should_clean_every_image_the_workflow_builds(self):
        workflow = (ROOT / ".github/workflows/tests.yml").read_text()
        matrix = workflow.split("  build-images:\n", 1)[1].split("    uses: ./.github/workflows/tests-build-image.yml", 1)[0]
        self.assertEqual(sorted(re.findall(r"image_name: (\S+)", matrix)), sorted(cleanup.IMAGES))

    def test_should_address_a_package_whose_name_has_slashes(self):
        github = cleanup.GitHub("token", "https://api.github.invalid/")
        github.request = lambda method, path: {"type": "Organization"}
        self.assertEqual(github.versions_path("dashpay", "platform/ci/drive"),
                         "/orgs/dashpay/packages/container/platform%2Fci%2Fdrive/versions")
        github.request = lambda method, path: {"type": "User"}
        self.assertEqual(github.versions_path("someone", "platform/ci/drive"),
                         "/users/someone/packages/container/platform%2Fci%2Fdrive/versions")


if __name__ == "__main__":
    unittest.main()
