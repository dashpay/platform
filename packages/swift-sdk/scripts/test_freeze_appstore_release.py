"""Release-worker tests use synthetic stores, temporary Git repos, and mock APIs."""

import contextlib
import hashlib
import io
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest
from unittest import mock
import urllib.error

import freeze_appstore_release as worker


def git(directory, *args):
    result = subprocess.run(["git", *args], cwd=directory, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f"Test git {args[0]} failed: {result.stderr}")
    return result.stdout.strip()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.data = self.root / "data"
        self.data.mkdir()
        git(self.data, "init", "-b", worker.DATA_BRANCH)
        git(self.data, "config", "user.name", "Test")
        git(self.data, "config", "user.email", "test@example.invalid")
        git(self.data, "config", "commit.gpgsign", "false")
        git(self.data, "remote", "add", "origin", f"https://github.com/{worker.IOS_REPO}.git")
        self.store = self.root / "synthetic.store"
        with contextlib.closing(sqlite3.connect(self.store)) as database, database:
            database.execute("CREATE TABLE synthetic(value TEXT)")
            database.execute("INSERT INTO synthetic VALUES ('test')")
        self.fixture = self.store.read_bytes()
        digest = hashlib.sha256(self.fixture).hexdigest()
        self.manifest = {
            "format_version": 1, "bundle_id": "org.dash.wallet", "app_version": "2.0",
            "build_number": "21", "wallet_sha": "1" * 40, "platform_sha": "2" * 40,
            "schema": {"schema_version": "2.0.0", "model_checksum": "checksum",
                       "entity_hashes": {"Synthetic": "ab" * 16}, "indexes": []},
            "fixture_sha256": digest, "fixture_path": f"stores/{digest}.store",
        }
        self.proof = {
            "format_version": 1, "release_id": "release-21", "app_id": "123",
            "bundle_id": "org.dash.wallet", "app_version": "2.0", "build_number": "21",
            "build_id": "build-21", "manifest_path": "builds/org.dash.wallet/2.0/21/manifest.json",
            "observed_state": "READY_FOR_DISTRIBUTION", "observed_at": "2026-09-18T12:00:00Z",
        }
        self.commit = self.save()

    def save(self, expected_manifest_digest=None):
        manifest_path = self.data / self.proof["manifest_path"]
        write_json(manifest_path, self.manifest)
        self.proof["manifest_sha256"] = (expected_manifest_digest or
                                          hashlib.sha256(manifest_path.read_bytes()).hexdigest())
        write_json(self.data / f"releases/{self.proof['release_id']}.json", self.proof)
        fixture_path = self.data / self.manifest["fixture_path"]
        fixture_path.parent.mkdir(parents=True, exist_ok=True)
        fixture_path.write_bytes(self.fixture)
        git(self.data, "add", ".")
        git(self.data, "commit", "--allow-empty", "-m", "test data")
        commit = git(self.data, "rev-parse", "HEAD")
        git(self.data, "update-ref", f"refs/remotes/origin/{worker.DATA_BRANCH}", commit)
        return commit

    def validate(self, commit=None):
        return worker.validate_publication(self.data, commit or self.commit, "release-21")

    def test_uses_published_build_not_latest_testflight_build(self):
        newer = dict(self.manifest, build_number="22", platform_sha="9" * 40)
        write_json(self.data / "builds/org.dash.wallet/2.0/22/manifest.json", newer)
        commit = self.save()
        proof, manifest, fixture = self.validate(commit)
        self.assertEqual(proof["build_number"], "21")
        self.assertEqual(manifest["platform_sha"], "2" * 40)
        self.assertEqual(fixture, self.fixture)

    def test_rejects_nonpublication(self):
        self.proof["observed_state"] = "PENDING_DEVELOPER_RELEASE"
        with self.assertRaisesRegex(worker.ReleaseError, "not been published"):
            self.validate(self.save())

    def test_accepts_legacy_ready_for_sale_status(self):
        self.proof["observed_state"] = "READY_FOR_SALE"
        self.validate(self.save())

    def test_accepts_published_version_superseded_between_observer_runs(self):
        self.proof["observed_state"] = "REPLACED_WITH_NEW_VERSION"
        proof, manifest, _fixture = self.validate(self.save())
        self.assertEqual(proof["build_id"], "build-21")
        self.assertEqual(manifest["platform_sha"], "2" * 40)

    def test_manifest_checksum_is_verified(self):
        with self.assertRaisesRegex(worker.ReleaseError, "manifest checksum"):
            self.validate(self.save("0" * 64))

    def test_fixture_checksum_is_verified(self):
        self.fixture += b"changed"
        with self.assertRaisesRegex(worker.ReleaseError, "Fixture checksum"):
            self.validate(self.save())

    def test_cannot_substitute_another_build(self):
        self.manifest["build_number"] = "22"
        with self.assertRaisesRegex(worker.ReleaseError, "identity mismatch"):
            self.validate(self.save())

    def test_cannot_load_commit_outside_data_branch(self):
        git(self.data, "checkout", "--orphan", "unrelated")
        git(self.data, "commit", "-m", "unrelated")
        commit = git(self.data, "rev-parse", "HEAD")
        with self.assertRaises(worker.ReleaseError):
            self.validate(commit)

    def test_rejects_arbitrary_origin(self):
        git(self.data, "remote", "set-url", "origin", "https://github.com/other/repo.git")
        with self.assertRaisesRegex(worker.ReleaseError, "must come from"):
            self.validate()

    def test_rejects_path_traversal_before_git_lookup(self):
        with self.assertRaisesRegex(worker.ReleaseError, "release ID"):
            worker.validate_publication(self.data, self.commit, "../elsewhere")

    def test_rejects_symbolic_metadata_ref(self):
        with self.assertRaisesRegex(worker.ReleaseError, "metadata commit"):
            self.validate("HEAD")

    def test_rejects_fixture_symlink(self):
        fixture = self.data / self.manifest["fixture_path"]
        fixture.unlink()
        fixture.symlink_to("../outside")
        git(self.data, "add", ".")
        git(self.data, "commit", "-m", "symlink")
        commit = git(self.data, "rev-parse", "HEAD")
        git(self.data, "update-ref", f"refs/remotes/origin/{worker.DATA_BRANCH}", commit)
        with self.assertRaisesRegex(worker.ReleaseError, "regular data file"):
            self.validate(commit)

    def test_release_associations_are_idempotent_and_preserve_first_data_commit(self):
        registry = {"schemas": {"2.0.0": {}}, "releases": {}}
        entry = worker.release_entry(self.proof, self.manifest, self.commit)
        self.assertTrue(worker.associate_release(registry, "release-21", entry))
        newer_observation = dict(entry, data_commit="f" * 40)
        self.assertFalse(worker.associate_release(registry, "release-21", newer_observation))
        self.assertEqual(registry["releases"]["release-21"]["data_commit"], self.commit)

    def test_same_release_cannot_change_provenance(self):
        entry = worker.release_entry(self.proof, self.manifest, self.commit)
        registry = {"schemas": {"2.0.0": {}}, "releases": {"release-21": entry}}
        with self.assertRaisesRegex(worker.ReleaseError, "different provenance"):
            worker.associate_release(registry, "release-21", dict(entry, platform_sha="f" * 40))

    def test_association_requires_a_snapshot(self):
        with self.assertRaisesRegex(worker.ReleaseError, "without its generated snapshot"):
            worker.associate_release({"schemas": {}}, "release-21",
                                     worker.release_entry(self.proof, self.manifest, self.commit))


class GitHubTests(unittest.TestCase):
    def test_retries_transient_reads(self):
        opener = mock.Mock(side_effect=[
            urllib.error.HTTPError("url", 503, "unavailable", {}, None),
            contextlib.closing(io.BytesIO(b'[]')),
        ])
        sleep = mock.Mock()
        self.assertEqual(worker.GitHub("secret", opener=opener, sleep=sleep).request("GET", "pulls"), [])
        self.assertEqual(opener.call_count, 2)

    def test_does_not_retry_ambiguous_post(self):
        opener = mock.Mock(side_effect=urllib.error.URLError("connection closed"))
        with self.assertRaisesRegex(worker.ReleaseError, "reconcile"):
            worker.GitHub("secret", opener=opener).request("POST", "pulls", {})
        self.assertEqual(opener.call_count, 1)

    def test_pr_listing_is_paginated(self):
        api = worker.GitHub("secret")
        api.request = mock.Mock(side_effect=[[{}] * 100, [{"number": 101}]])
        self.assertEqual(len(api.pull_requests("codex/freeze-swift-schema-v2.0.0")), 101)
        self.assertIn("page=2", api.request.call_args.args[1])


class WorkerIntegrationTests(unittest.TestCase):
    """The production worker pushes only to a temporary local bare repository."""

    save = PublicationTests.save

    def setUp(self):
        PublicationTests.setUp(self)
        self.platform = self.root / "platform"
        self.platform.mkdir()
        git(self.platform, "init", "-b", worker.BASE_BRANCH)
        git(self.platform, "config", "user.name", "Test")
        git(self.platform, "config", "user.email", "test@example.invalid")
        git(self.platform, "config", "commit.gpgsign", "false")
        write_json(self.platform / worker.REGISTRY, {"format_version": 1, "schemas": {}, "releases": {}})
        generator = self.platform / worker.GENERATOR
        generator.parent.mkdir(parents=True)
        generator.write_text('''import json, pathlib, sys
if "--check" in sys.argv:
    sys.exit(0)
m = json.loads(pathlib.Path(sys.argv[sys.argv.index("--release-manifest") + 1]).read_text())
p = pathlib.Path("packages/swift-sdk/schema-releases.json")
r = json.loads(p.read_text())
v = m["schema"]["schema_version"]
if v in r["schemas"] and r["schemas"][v]["schema"] != m["schema"]:
    sys.exit("Schema conflict")
r["schemas"].setdefault(v, {"schema": m["schema"], "platform_sha": m["platform_sha"]})
p.write_text(json.dumps(r))
''')
        git(self.platform, "add", ".")
        git(self.platform, "commit", "-m", "base")
        self.manifest["platform_sha"] = git(self.platform, "rev-parse", "HEAD")
        self.commit = self.save()
        self.remote = self.root / "platform.git"
        git(self.root, "clone", "--bare", str(self.platform), str(self.remote))
        self.api = mock.Mock()
        self.api.pull_requests.return_value = []
        self.api.request.return_value = {"html_url": "https://github.com/dashpay/platform/pull/1"}
        self.real_git = worker.git

    def redirected_git(self, directory, *args, **kwargs):
        if args[:3] == ("remote", "set-url", "origin"):
            args = (*args[:3], str(self.remote))
        return self.real_git(directory, *args, **kwargs)

    def prepare(self, dry_run=False):
        with mock.patch.object(worker, "git", side_effect=self.redirected_git), \
             mock.patch.object(worker, "GitHub", return_value=self.api), \
             contextlib.redirect_stdout(io.StringIO()):
            worker.prepare(self.platform, self.data, self.proof["release_id"], self.commit, "test-token", dry_run)

    def test_dry_run_does_not_publish_or_modify_checkout(self):
        before = git(self.remote, "show-ref")
        self.prepare(dry_run=True)
        self.assertEqual(git(self.remote, "show-ref"), before)
        self.api.request.assert_not_called()
        self.assertEqual(git(self.platform, "status", "--porcelain"), "")

    def test_fixture_check_closes_connection_on_success_or_failure(self):
        for corrupt in (False, True):
            database = sqlite3.connect(self.store)
            checked_database = mock.Mock(wraps=database)
            if corrupt:
                checked_database.execute.return_value.fetchone.return_value = ("corrupt",)
            with mock.patch.object(worker.sqlite3, "connect", return_value=checked_database):
                if corrupt:
                    with self.assertRaisesRegex(worker.ReleaseError, "corrupt or needs a WAL"):
                        self.prepare(dry_run=True)
                else:
                    self.prepare(dry_run=True)
            with self.assertRaises(sqlite3.ProgrammingError):
                database.execute("SELECT 1")

    def test_push_then_retry_reuses_draft_pr_and_commit(self):
        self.prepare()
        self.assertTrue(self.api.request.call_args.args[2]["draft"])
        branch = "codex/freeze-swift-schema-v2.0.0"
        before = git(self.remote, "rev-parse", branch)
        self.api.pull_requests.return_value = [{"state": "open", "html_url": "https://example.invalid/pr"}]
        self.api.request.reset_mock()
        self.prepare()
        self.assertEqual(git(self.remote, "rev-parse", branch), before)
        self.api.request.assert_not_called()

    def test_retry_recovers_push_succeeded_pr_creation_failed(self):
        self.api.request.side_effect = worker.ReleaseError("API unavailable")
        with self.assertRaisesRegex(worker.ReleaseError, "API unavailable"):
            self.prepare()
        self.api.request.side_effect = None
        self.api.request.reset_mock()
        self.prepare()
        self.api.request.assert_called_once()

    def test_human_edits_on_existing_branch_are_preserved(self):
        self.prepare()
        branch = "codex/freeze-swift-schema-v2.0.0"
        git(self.platform, "fetch", str(self.remote), branch)
        git(self.platform, "checkout", "-b", "human", "FETCH_HEAD")
        (self.platform / "human-review.txt").write_text("Keep this review change\n")
        git(self.platform, "add", "human-review.txt")
        git(self.platform, "commit", "-m", "human review")
        git(self.platform, "push", str(self.remote), f"HEAD:refs/heads/{branch}")
        self.api.pull_requests.return_value = [{"state": "open", "html_url": "https://example.invalid/pr"}]
        self.prepare()
        self.assertEqual(git(self.remote, "show", f"{branch}:human-review.txt"), "Keep this review change")

    def test_closed_unmerged_pr_requires_intervention(self):
        self.api.pull_requests.return_value = [{"state": "closed", "merged_at": None}]
        with self.assertRaisesRegex(worker.ReleaseError, "closed without merging"):
            self.prepare()
        self.api.request.assert_not_called()

    def test_already_merged_release_creates_no_commit_or_pr(self):
        self.prepare()
        branch = "codex/freeze-swift-schema-v2.0.0"
        git(self.remote, "update-ref", f"refs/heads/{worker.BASE_BRANCH}", git(self.remote, "rev-parse", branch))
        self.api.pull_requests.return_value = [{"state": "closed", "merged_at": "2026-09-18"}]
        self.api.request.reset_mock()
        before = git(self.remote, "show-ref")
        self.prepare()
        self.assertEqual(git(self.remote, "show-ref"), before)
        self.api.request.assert_not_called()

    def test_new_release_of_same_schema_appends_association_to_existing_pr(self):
        self.prepare()
        branch = "codex/freeze-swift-schema-v2.0.0"
        before = json.loads(git(self.remote, "show", f"{branch}:{worker.REGISTRY}"))
        self.manifest.update(app_version="2.1", build_number="22")
        self.proof.update(release_id="release-22", app_version="2.1", build_number="22",
                          build_id="build-22", manifest_path="builds/org.dash.wallet/2.1/22/manifest.json")
        self.commit = self.save()
        self.api.pull_requests.return_value = [{"state": "open", "html_url": "https://example.invalid/pr"}]
        self.api.request.reset_mock()
        self.prepare()
        after = json.loads(git(self.remote, "show", f"{branch}:{worker.REGISTRY}"))
        self.assertEqual(after["schemas"], before["schemas"])
        self.assertEqual(set(after["releases"]), {"release-21", "release-22"})
        self.api.request.assert_not_called()

    def test_schema_conflict_cannot_change_existing_branch(self):
        self.prepare()
        branch = "codex/freeze-swift-schema-v2.0.0"
        before = git(self.remote, "rev-parse", branch)
        self.manifest["schema"]["indexes"] = ["new index"]
        self.commit = self.save()
        self.api.pull_requests.return_value = [{"state": "open", "html_url": "https://example.invalid/pr"}]
        self.api.request.reset_mock()
        with self.assertRaisesRegex(worker.ReleaseError, "Schema conflict"):
            self.prepare()
        self.assertEqual(git(self.remote, "rev-parse", branch), before)
        self.api.request.assert_not_called()


if __name__ == "__main__":
    unittest.main()
