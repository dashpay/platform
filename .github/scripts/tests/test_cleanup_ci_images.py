"""Keep the CI image cleanup from deleting an image a test job can still pull.

The cleanup deletes versions of the images that the Tests workflow pushes to
GHCR. It must leave recent images alone, never take a part of an image that
still has its tag, cover exactly the images the workflow builds, bound what it
spends, and never fail a run.
"""
from datetime import datetime, timedelta, timezone
import importlib.util
import io
from pathlib import Path
import re
import time
import unittest
from unittest import mock
import urllib.error


ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("cleanup_ci_images", ROOT / ".github/scripts/cleanup-ci-images.py")
cleanup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cleanup)

NOW = datetime(2026, 11, 20, 12, 0, tzinfo=timezone.utc)
OLD = timedelta(days=30)
DRIVE = "platform/ci/drive"


def version(version_id, age, tags=()):
    pushed = (NOW - age).strftime("%Y-%m-%dT%H:%M:%SZ")
    return {"id": version_id, "name": f"sha256:{version_id}", "updated_at": pushed,
            "metadata": {"container": {"tags": list(tags)}}}


class FakeGitHub:
    """Serve package versions from memory and record what gets deleted."""

    def __init__(self, packages, refuse=(), rate_limit_after=None):
        self.packages = packages
        self.refuse = set(refuse)
        self.rate_limit_after = rate_limit_after
        self.attempted = []
        self.deleted = []

    def versions_path(self, owner, package):
        return package

    def versions(self, path):
        if path not in self.packages:
            raise urllib.error.HTTPError(path, 404, "Not Found", None, None)
        return self.packages[path]

    def request(self, method, path):
        package, version_id = path.rsplit("/", 1)
        if self.rate_limit_after is not None and len(self.attempted) >= self.rate_limit_after:
            raise cleanup.Stop("rate limited (HTTP 429)")
        self.attempted.append((package, int(version_id)))
        if int(version_id) in self.refuse:
            raise urllib.error.HTTPError(path, 400, "Bad Request", None, None)
        self.deleted.append((package, int(version_id)))


class FakeRegistry:
    """Say which versions each image index points to; `None` makes the read fail."""

    def __init__(self, indexes=None):
        self.indexes = indexes or {}
        self.read = []

    def references(self, repository, digest):
        self.read.append((repository, digest))
        children = self.indexes.get(int(digest.split(":")[1]), ())
        if children is None:
            raise urllib.error.HTTPError(digest, 500, "Server Error", None, None)
        return {f"sha256:{child}" for child in children}


def clean(github, registry=None, **kwargs):
    log = []
    count = cleanup.clean(github, registry or FakeRegistry(), "dashpay/platform", NOW, log=log.append, **kwargs)
    return count, log


class ExpiryTests(unittest.TestCase):
    def test_should_keep_images_younger_than_the_retention(self):
        versions = [
            version(1, timedelta(hours=1), ["sha-new"]),
            version(2, timedelta(days=cleanup.RETENTION_DAYS) - timedelta(minutes=1), ["sha-almost"]),
            version(3, timedelta(days=cleanup.RETENTION_DAYS) + timedelta(minutes=1), ["sha-old"]),
        ]
        self.assertEqual([item["id"] for item in cleanup.expired(versions, NOW)], [3])

    def test_should_delete_the_oldest_first(self):
        versions = [version(1, timedelta(days=5), ["a"]), version(2, timedelta(days=9), ["b"]),
                    version(3, timedelta(days=7), ["c"])]
        self.assertEqual([item["id"] for item in cleanup.expired(versions, NOW)], [2, 3, 1])

    def test_should_not_take_a_version_without_tag_information_for_untagged(self):
        self.assertIsNone(cleanup.tags({"metadata": None}))
        self.assertIsNone(cleanup.tags({"metadata": {"container": None}}))
        self.assertIsNone(cleanup.tags({"metadata": {"container": {"tags": None}}}))
        self.assertEqual(cleanup.tags({"metadata": {"container": {"tags": []}}}), [])


class ImagePartsTests(unittest.TestCase):
    """An image is a tagged index (1) pointing to untagged manifests (2, 3), pushed just before it."""

    def image(self):
        return {DRIVE: [version(1, OLD, ["sha-old"]), version(2, OLD + timedelta(seconds=2)),
                        version(3, OLD + timedelta(seconds=1))]}

    def test_should_delete_an_expired_index_before_its_manifests(self):
        github = FakeGitHub(self.image())
        self.assertEqual(clean(github, FakeRegistry({1: [2, 3]}))[0], 3)
        self.assertEqual(github.deleted, [(DRIVE, 1), (DRIVE, 2), (DRIVE, 3)])

    def test_should_keep_the_manifests_of_an_index_that_could_not_be_deleted(self):
        # Both the index and its manifests are past the retention, but the
        # index survives, so its tag must stay pullable.
        github = FakeGitHub(self.image(), refuse=[1])
        registry = FakeRegistry({1: [2, 3]})
        self.assertEqual(clean(github, registry)[0], 0)
        self.assertEqual(github.deleted, [])
        self.assertEqual(registry.read, [("dashpay/platform/ci/drive", "sha256:1")])

    def test_should_keep_an_old_manifest_that_a_recent_image_reuses(self):
        packages = self.image()
        packages[DRIVE].append(version(4, timedelta(hours=1), ["sha-new"]))
        github = FakeGitHub(packages)
        self.assertEqual(clean(github, FakeRegistry({1: [2, 3], 4: [2]}))[0], 2)
        self.assertEqual(github.deleted, [(DRIVE, 1), (DRIVE, 3)])

    def test_should_keep_untagged_versions_when_the_registry_cannot_be_read(self):
        packages = self.image()
        packages[DRIVE].append(version(4, timedelta(hours=1), ["sha-new"]))
        github = FakeGitHub(packages)
        count, log = clean(github, FakeRegistry({4: None}))
        self.assertEqual(count, 1)
        self.assertEqual(github.deleted, [(DRIVE, 1)])
        self.assertTrue(any(line.startswith("::warning::") and "untagged versions stay" in line for line in log))

    def test_should_keep_an_index_without_tag_information_and_its_manifests(self):
        # The packages API did not report the tags of the index, so it cannot
        # be told from an untagged version by its metadata.
        packages = self.image()
        packages[DRIVE][0]["metadata"] = None
        github = FakeGitHub(packages)
        registry = FakeRegistry({1: [2, 3]})
        count, log = clean(github, registry)
        self.assertEqual(count, 0)
        self.assertEqual(github.attempted, [])
        self.assertEqual(registry.read, [("dashpay/platform/ci/drive", "sha256:1")])
        self.assertTrue(any("without tag information" in line for line in log))

    def test_should_report_the_same_deletions_in_a_dry_run(self):
        github = FakeGitHub(self.image())
        self.assertEqual(clean(github, FakeRegistry({1: [2, 3]}), dry_run=True)[0], 3)
        self.assertEqual(github.attempted, [])


class CleanTests(unittest.TestCase):
    def packages(self, count=1):
        return {
            DRIVE: [version(10 + index, OLD, [f"sha-{index}"]) for index in range(count)],
            "platform/ci/rs-dapi": [version(5, OLD, ["sha-a"]), version(6, timedelta(hours=2), ["sha-b"])],
        }

    def test_should_delete_expired_versions_and_skip_a_package_that_does_not_exist_yet(self):
        github = FakeGitHub(self.packages())
        count, log = clean(github)
        self.assertEqual(count, 2)
        self.assertEqual(github.deleted, [(DRIVE, 10), ("platform/ci/rs-dapi", 5)])
        self.assertIn("platform/ci/dashmate-helper: no such package yet", log)

    def test_should_carry_on_when_a_version_cannot_be_deleted(self):
        github = FakeGitHub(self.packages(), refuse=[10])
        count, log = clean(github)
        self.assertEqual(count, 1)
        self.assertEqual(github.deleted, [("platform/ci/rs-dapi", 5)])
        self.assertTrue(any(line.startswith("::warning::") for line in log))

    def test_should_stop_at_the_attempt_limit_of_an_image_and_go_on_to_the_next(self):
        github = FakeGitHub(self.packages(count=cleanup.MAX_ATTEMPTS + 5))
        self.assertEqual(clean(github)[0], cleanup.MAX_ATTEMPTS + 1)
        self.assertEqual(len([item for item in github.deleted if item[0] == DRIVE]), cleanup.MAX_ATTEMPTS)

    def test_should_count_refused_deletions_against_the_attempt_limit(self):
        backlog = cleanup.MAX_ATTEMPTS + 50
        github = FakeGitHub(self.packages(count=backlog), refuse=range(10, 10 + backlog))
        self.assertEqual(clean(github)[0], 1)
        self.assertEqual(len([item for item in github.attempted if item[0] == DRIVE]), cleanup.MAX_ATTEMPTS)

    def test_should_end_the_sweep_when_github_rate_limits(self):
        github = FakeGitHub(self.packages(count=10), rate_limit_after=3)
        count, log = clean(github)
        self.assertEqual(count, 3)
        self.assertEqual(len(github.attempted), 3)
        self.assertTrue(any("stopped early" in line for line in log))


class HttpTests(unittest.TestCase):
    def error(self, code, headers=None, body=None):
        return urllib.error.HTTPError("https://api.github.invalid", code, "refused", headers or {},
                                      None if body is None else io.BytesIO(body))

    def test_should_tell_a_rate_limit_from_a_refusal(self):
        self.assertTrue(cleanup.rate_limited(self.error(429)))
        self.assertTrue(cleanup.rate_limited(self.error(403, {"Retry-After": "60"})))
        self.assertTrue(cleanup.rate_limited(self.error(403, {"X-RateLimit-Remaining": "0"})))
        self.assertFalse(cleanup.rate_limited(self.error(403, {"X-RateLimit-Remaining": "4000"})))
        self.assertFalse(cleanup.rate_limited(self.error(400)))

    def test_should_see_a_secondary_rate_limit_that_only_the_message_names(self):
        # No Retry-After header and primary quota left: the body is all there is.
        quota_left = {"X-RateLimit-Remaining": "4000"}
        secondary = b'{"message": "You have exceeded a secondary rate limit. Please wait a few minutes."}'
        self.assertTrue(cleanup.rate_limited(self.error(403, quota_left, secondary)))
        forbidden = b'{"message": "Resource not accessible by integration"}'
        self.assertFalse(cleanup.rate_limited(self.error(403, quota_left, forbidden)))
        http = cleanup.Http(time.monotonic() + 60)
        with mock.patch("urllib.request.urlopen", side_effect=self.error(403, quota_left, secondary)):
            with self.assertRaises(cleanup.Stop):
                http.json("DELETE", "https://api.github.invalid", {})

    def test_should_stop_on_a_rate_limit_and_pass_other_errors_on(self):
        http = cleanup.Http(time.monotonic() + 60)
        with mock.patch("urllib.request.urlopen", side_effect=self.error(429)):
            with self.assertRaises(cleanup.Stop):
                http.json("DELETE", "https://api.github.invalid", {})
        with mock.patch("urllib.request.urlopen", side_effect=self.error(400)):
            with self.assertRaises(urllib.error.HTTPError):
                http.json("DELETE", "https://api.github.invalid", {})

    def test_should_send_nothing_after_the_time_limit(self):
        http = cleanup.Http(time.monotonic() - 1)
        with mock.patch("urllib.request.urlopen") as urlopen:
            with self.assertRaises(cleanup.Stop):
                http.json("GET", "https://api.github.invalid", {})
        urlopen.assert_not_called()

    def test_should_exit_zero_whatever_goes_wrong(self):
        env = {"GITHUB_TOKEN": "token", "GITHUB_REPOSITORY": "dashpay/platform"}
        with mock.patch.dict("os.environ", env), mock.patch("builtins.print"), \
                mock.patch.object(cleanup, "clean", side_effect=AttributeError("'NoneType' object has no attribute 'get'")):
            self.assertEqual(cleanup.main([]), 0)


class RegistryTests(unittest.TestCase):
    def registry(self, manifest):
        calls = []

        class Http:
            def json(self, method, url, headers):
                calls.append(url)
                return {"token": "pull-token"} if "/token?" in url else manifest

        return cleanup.Registry(Http(), "token"), calls

    def test_should_list_what_an_image_index_points_to(self):
        registry, calls = self.registry({"manifests": [{"digest": "sha256:a"}, {"digest": "sha256:b"}]})
        self.assertEqual(registry.references("dashpay/platform/ci/drive", "sha256:1"), {"sha256:a", "sha256:b"})
        registry.references("dashpay/platform/ci/drive", "sha256:2")
        self.assertEqual(calls, [
            "https://ghcr.io/token?service=ghcr.io&scope=repository%3Adashpay%2Fplatform%2Fci%2Fdrive%3Apull",
            "https://ghcr.io/v2/dashpay/platform/ci/drive/manifests/sha256:1",
            "https://ghcr.io/v2/dashpay/platform/ci/drive/manifests/sha256:2",
        ])

    def test_should_find_no_references_in_a_plain_manifest(self):
        registry, _ = self.registry({"config": {}, "layers": []})
        self.assertEqual(registry.references("dashpay/platform/ci/drive", "sha256:1"), set())


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

    def test_should_run_outside_the_tests_workflow(self):
        # A registry or API problem must not change the result of a Tests run.
        self.assertNotIn("cleanup-ci-images", (ROOT / ".github/workflows/tests.yml").read_text())
        self.assertIn("cleanup-ci-images.py", (ROOT / ".github/workflows/cleanup-ci-images.yml").read_text())

    def test_should_give_a_pull_request_a_read_only_package_token(self):
        # A pull request runs its own version of the script, so its dry run
        # must not hold a token that can delete images.
        workflow = (ROOT / ".github/workflows/cleanup-ci-images.yml").read_text()
        sweep, dry_run = workflow.split("\njobs:\n", 1)[1].split("\n  dry-run:\n", 1)
        self.assertIn("if: ${{ github.event_name != 'pull_request' }}", sweep)
        self.assertNotIn("--dry-run", sweep)
        self.assertIn("if: ${{ github.event_name == 'pull_request' }}", dry_run)
        self.assertIn("packages: read", dry_run)
        self.assertIn("cleanup-ci-images.py --dry-run", dry_run)
        self.assertEqual(workflow.count("packages: write"), 1)
        self.assertNotIn("packages: write", dry_run)

    def test_should_let_only_the_image_build_write_packages_in_the_tests_workflow(self):
        workflow = (ROOT / ".github/workflows/tests.yml").read_text()
        default = workflow.split("\njobs:\n", 1)[0].split("\npermissions:\n", 1)[1].split("\n\n", 1)[0]
        self.assertIn("packages: read", default)
        self.assertNotIn("write", default)
        build = workflow.split("  build-images:\n", 1)[1].split("    uses: ./.github/workflows/tests-build-image.yml", 1)[0]
        self.assertIn("packages: write", build)
        self.assertEqual(workflow.count("packages: write"), 1)

    def test_should_address_a_package_whose_name_has_slashes(self):
        github = cleanup.GitHub(None, "token", "https://api.github.invalid/")
        github.request = lambda method, path: {"type": "Organization"}
        self.assertEqual(github.versions_path("dashpay", "platform/ci/drive"),
                         "/orgs/dashpay/packages/container/platform%2Fci%2Fdrive/versions")
        github.request = lambda method, path: {"type": "User"}
        self.assertEqual(github.versions_path("someone", "platform/ci/drive"),
                         "/users/someone/packages/container/platform%2Fci%2Fdrive/versions")


if __name__ == "__main__":
    unittest.main()
