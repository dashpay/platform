#!/usr/bin/env python3
"""Prepare a reviewed SwiftData snapshot from the iOS release-data branch.

The caller supplies a release ID and a metadata commit, never a Platform ref.
Only the iOS monitor has Apple credentials; its publication proof is trusted
only after proving it belongs to the fixed release-data branch. All Git writes
take place in a temporary clone. --dry-run generates and checks the patch but
does not commit, push, or write to the GitHub API.
"""

import argparse
import base64
import contextlib
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request


IOS_REPO = "dashpay/dashwallet-ios"
PLATFORM_REPO = "dashpay/platform"
BASE_BRANCH = "v4.2-dev"
DATA_BRANCH = "schema-release-data"
SDK = "packages/swift-sdk"
REGISTRY = f"{SDK}/schema-releases.json"
GENERATOR = f"{SDK}/scripts/freeze_schema_models.py"
GENERATED_TEST = f"{SDK}/SwiftTests/SwiftDashSDKTests/DashReleasedSchemaRegistry.generated.swift"
SHA = re.compile(r"[0-9a-f]{40}")
DIGEST = re.compile(r"[0-9a-f]{64}")
COMPONENT = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,254}")
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")


class ReleaseError(RuntimeError):
    pass


def run(directory, *args, env=None):
    result = subprocess.run(args, cwd=directory, env=env, capture_output=True)
    if result.returncode:
        # Never echo commands or credentials from subprocess diagnostics.
        raise ReleaseError(f"{args[0]} {args[1] if len(args) > 1 else ''} failed: "
                           f"{result.stderr.decode(errors='replace')[-2000:]}")
    return result.stdout


def git(directory, *args, env=None):
    return run(directory, "git", *args, env=env).decode().strip()


def require_string(value, pattern, label):
    if not isinstance(value, str) or not pattern.fullmatch(value):
        raise ReleaseError(f"Invalid {label}")
    return value


def read_blob(directory, commit, path):
    entry = git(directory, "ls-tree", commit, "--", path)
    if not entry.startswith("100644 blob "):
        raise ReleaseError(f"Missing regular data file: {path}")
    return run(directory, "git", "show", f"{commit}:{path}")


def json_object(data, label):
    try:
        value = json.loads(data)
    except (ValueError, UnicodeError) as error:
        raise ReleaseError(f"Invalid JSON: {label}") from error
    if not isinstance(value, dict) or value.get("format_version") != 1:
        raise ReleaseError(f"Unsupported format: {label}")
    return value


def verify_data_origin(directory):
    origin = git(directory, "remote", "get-url", "origin")
    if origin not in (f"https://github.com/{IOS_REPO}", f"https://github.com/{IOS_REPO}.git",
                      f"git@github.com:{IOS_REPO}.git"):
        raise ReleaseError("Release metadata must come from dashpay/dashwallet-ios")


def validate_publication(data_repo, data_commit, release_id):
    """Return verified publication proof, manifest, and fixture bytes."""
    require_string(data_commit, SHA, "metadata commit")
    require_string(release_id, COMPONENT, "Apple release ID")
    verify_data_origin(data_repo)
    git(data_repo, "merge-base", "--is-ancestor", data_commit, f"origin/{DATA_BRANCH}")
    proof = json_object(read_blob(data_repo, data_commit, f"releases/{release_id}.json"), "release proof")
    if proof.get("release_id") != release_id:
        raise ReleaseError("Release proof ID does not match the requested release")
    if proof.get("observed_state") not in (
        "READY_FOR_DISTRIBUTION", "READY_FOR_SALE", "REPLACED_WITH_NEW_VERSION"
    ):
        raise ReleaseError("The version has not been published in the App Store")
    for field in ("app_id", "bundle_id", "app_version", "build_number", "build_id"):
        require_string(proof.get(field), COMPONENT, field)
    expected_path = (f"builds/{proof['bundle_id']}/{proof['app_version']}/"
                     f"{proof['build_number']}/manifest.json")
    if proof.get("manifest_path") != expected_path:
        raise ReleaseError("Release proof does not point to its exact build manifest")
    manifest_bytes = read_blob(data_repo, data_commit, expected_path)
    require_string(proof.get("manifest_sha256"), DIGEST, "manifest digest")
    if hashlib.sha256(manifest_bytes).hexdigest() != proof["manifest_sha256"]:
        raise ReleaseError("Build manifest checksum mismatch")
    manifest = json_object(manifest_bytes, "build manifest")
    for field in ("bundle_id", "app_version", "build_number"):
        if manifest.get(field) != proof[field]:
            raise ReleaseError(f"Build identity mismatch: {field}")
    require_string(manifest.get("platform_sha"), SHA, "Platform commit")
    require_string(manifest.get("wallet_sha"), SHA, "wallet commit")
    schema = manifest.get("schema")
    if not isinstance(schema, dict):
        raise ReleaseError("Missing schema descriptor")
    require_string(schema.get("schema_version"), VERSION, "schema version")
    if not isinstance(schema.get("model_checksum"), str) or not schema["model_checksum"]:
        raise ReleaseError("Missing model checksum")
    hashes = schema.get("entity_hashes")
    if not isinstance(hashes, dict) or not hashes:
        raise ReleaseError("Missing entity hashes")
    for name, digest in hashes.items():
        require_string(name, COMPONENT, "entity name")
        require_string(digest, re.compile(r"(?:[0-9a-f]{2})+"), "entity hash")
    indexes = schema.get("indexes")
    if not isinstance(indexes, list) or not all(isinstance(x, str) for x in indexes):
        raise ReleaseError("Missing SQLite indexes")
    if len(set(indexes)) != len(indexes):
        raise ReleaseError("Duplicate SQLite index descriptors")
    require_string(manifest.get("fixture_sha256"), DIGEST, "fixture digest")
    fixture_path = f"stores/{manifest['fixture_sha256']}.store"
    if manifest.get("fixture_path") != fixture_path:
        raise ReleaseError("Fixture must use its content-addressed store path")
    fixture = read_blob(data_repo, data_commit, fixture_path)
    if hashlib.sha256(fixture).hexdigest() != manifest["fixture_sha256"]:
        raise ReleaseError("Fixture checksum mismatch")
    if not fixture.startswith(b"SQLite format 3\x00"):
        raise ReleaseError("Fixture is not a SQLite store")
    return proof, manifest, fixture


def release_entry(proof, manifest, data_commit):
    entry = {field: proof[field] for field in
             ("app_id", "bundle_id", "app_version", "build_number", "build_id", "manifest_sha256")}
    entry.update(schema_version=manifest["schema"]["schema_version"],
                 platform_sha=manifest["platform_sha"], data_commit=data_commit)
    return entry


def associate_release(registry, release_id, entry):
    """An observation at a newer metadata commit must not rewrite old provenance."""
    releases = registry.setdefault("releases", {})
    previous = releases.get(release_id)
    if previous:
        expected = dict(entry, data_commit=previous.get("data_commit"))
        if previous != expected:
            raise ReleaseError("This App Store release already has different provenance")
        return False
    if entry["schema_version"] not in registry.get("schemas", {}):
        raise ReleaseError("Cannot associate a release without its generated snapshot")
    releases[release_id] = entry
    return True


def permitted_change(path):
    return (path in (REGISTRY, GENERATED_TEST)
            or (path.startswith(f"{SDK}/Sources/SwiftDashSDK/Persistence/FrozenSchemas/")
                and path.endswith(".swift"))
            or (path.startswith(f"{SDK}/SwiftTests/SwiftDashSDKTests/Fixtures/SchemaStores/releases/")
                and path.endswith(".store")))


class GitHub:
    def __init__(self, token, opener=urllib.request.urlopen, sleep=time.sleep):
        self.token, self.opener, self.sleep = token, opener, sleep

    def request(self, method, path, payload=None):
        request = urllib.request.Request(
            f"https://api.github.com/repos/{PLATFORM_REPO}/{path}", method=method,
            data=None if payload is None else json.dumps(payload).encode(),
            headers={"Authorization": f"Bearer {self.token}", "Accept": "application/vnd.github+json",
                     "X-GitHub-Api-Version": "2022-11-28", "Content-Type": "application/json"})
        # Retry reads only. A POST may have succeeded before a connection failed;
        # the next workflow run reconciles against existing PRs instead.
        for attempt in range(4):
            try:
                with self.opener(request, timeout=45) as response:
                    return json.load(response)
            except urllib.error.HTTPError as error:
                if method == "GET" and error.code in (429, 500, 502, 503, 504) and attempt < 3:
                    self.sleep(min(30, 2 ** attempt))
                    continue
                raise ReleaseError(f"GitHub {method} failed with HTTP {error.code}") from error
            except urllib.error.URLError as error:
                if method == "GET" and attempt < 3:
                    self.sleep(2 ** attempt)
                    continue
                raise ReleaseError("GitHub request failed; retry the workflow to reconcile its state") from error

    def pull_requests(self, branch):
        results = []
        page = 1
        while True:
            query = urllib.parse.urlencode({"head": f"dashpay:{branch}", "base": BASE_BRANCH,
                                            "state": "all", "per_page": 100, "page": page})
            rows = self.request("GET", f"pulls?{query}")
            results.extend(rows)
            if len(rows) < 100:
                return results
            page += 1


def git_environment(token):
    env = os.environ.copy()
    # This is process-local and never written to git config or command output.
    credential = base64.b64encode(f"x-access-token:{token}".encode()).decode()
    env.update(GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="http.https://github.com/.extraheader",
               GIT_CONFIG_VALUE_0=f"AUTHORIZATION: basic {credential}", GIT_TERMINAL_PROMPT="0")
    return env


def pr_body(proof, manifest, data_commit):
    schema = manifest["schema"]["schema_version"]
    return f"""## Issue being fixed or feature implemented
App Store version {proof['app_version']} (build {proof['build_number']}) shipped SwiftData schema {schema}.
Preserve its model definitions and synthetic migration fixture before changing that schema.

## What was done?
Generated the snapshot from Platform commit `{manifest['platform_sha']}` and associated Apple release `{proof['release_id']}`.
The publication proof and immutable build manifest are recorded in [iOS release metadata](https://github.com/{IOS_REPO}/blob/{data_commit}/releases/{proof['release_id']}.json).
This PR does not change the active runtime schema or invent a migration. If development has moved, reconcile the next schema version and migration before merging.

## How Has This Been Tested?
Metadata checksums and snapshot regeneration were checked by the freeze worker. Normal Swift SDK CI checks the stored schema, indexes and migrations.

## Breaking Changes
No runtime schema switch is included. This draft requires human review and a manual merge; no auto-merge is enabled.
"""


def prepare(repo, data_repo, release_id, data_commit, token, dry_run=False):
    proof, manifest, fixture = validate_publication(data_repo, data_commit, release_id)
    branch = f"codex/freeze-swift-schema-v{manifest['schema']['schema_version']}"
    api = GitHub(token)
    pulls = api.pull_requests(branch)
    opened = [pr for pr in pulls if pr["state"] == "open"]
    if len(opened) > 1:
        raise ReleaseError("Multiple open snapshot pull requests need manual reconciliation")
    if not opened and pulls and not pulls[0].get("merged_at"):
        raise ReleaseError("The snapshot pull request was closed without merging; reopen it to retry")
    env = git_environment(token)
    with tempfile.TemporaryDirectory(prefix="appstore-schema-") as temporary:
        clone = Path(temporary) / "platform"
        run(None, "git", "clone", "--shared", "--no-checkout", str(repo), str(clone))
        git(clone, "remote", "set-url", "origin", f"https://github.com/{PLATFORM_REPO}.git")
        git(clone, "config", "user.name", "Dash schema release automation")
        git(clone, "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com")
        git(clone, "config", "commit.gpgsign", "false")
        git(clone, "fetch", "origin", f"{BASE_BRANCH}:refs/remotes/origin/{BASE_BRANCH}", env=env)
        git(clone, "checkout", "--detach", f"origin/{BASE_BRANCH}")
        merged_registry = json_object((clone / REGISTRY).read_bytes(), "merged snapshot registry")
        if release_id in merged_registry.get("releases", {}):
            associate_release(merged_registry, release_id, release_entry(proof, manifest, data_commit))
            print("This release is already present in the merged registry.")
            return
        remote_branch = git(clone, "ls-remote", "--heads", "origin", branch, env=env)
        if remote_branch:
            git(clone, "fetch", "origin", f"{branch}:refs/remotes/origin/{branch}", env=env)
            git(clone, "checkout", "-b", branch, f"origin/{branch}")
            # Merge with no commit so --dry-run never writes a commit. The
            # publication commit below includes the merge and generated patch.
            git(clone, "merge", "--no-commit", "--no-ff", f"origin/{BASE_BRANCH}")
        else:
            git(clone, "checkout", "-b", branch, f"origin/{BASE_BRANCH}")
        # A released SHA can be absent from current branch history after a
        # force-updated development branch. Fetch exactly the proven SHA.
        git(clone, "fetch", "origin", manifest["platform_sha"], env=env)
        run(clone, sys.executable, GENERATOR, "--check")
        before_registry = json_object((clone / REGISTRY).read_bytes(), "snapshot registry")
        immutable_files = {path: (clone / path).read_bytes()
                           for path in git(clone, "ls-files").splitlines()
                           if permitted_change(path) and path not in (REGISTRY, GENERATED_TEST)}
        manifest_file = Path(temporary) / "manifest.json"
        fixture_file = Path(temporary) / "fixture.store"
        manifest_file.write_text(json.dumps(manifest))
        fixture_file.write_bytes(fixture)
        with contextlib.closing(sqlite3.connect(f"file:{fixture_file}?immutable=1", uri=True)) as database:
            if database.execute("PRAGMA quick_check").fetchone() != ("ok",):
                raise ReleaseError("The release fixture is corrupt or needs a WAL file")
        run(clone, sys.executable, GENERATOR, "--release-manifest", str(manifest_file), "--fixture", str(fixture_file))
        if any(not (clone / path).is_file() or (clone / path).read_bytes() != content
               for path, content in immutable_files.items()):
            raise ReleaseError("Attempted to replace an existing snapshot or fixture")
        registry = json_object((clone / REGISTRY).read_bytes(), "snapshot registry")
        # The generator may append a schema, but existing snapshots are immutable.
        for key in ("schemas", "releases"):
            for identifier, value in before_registry.get(key, {}).items():
                if registry.get(key, {}).get(identifier) != value:
                    raise ReleaseError(f"Attempted to rewrite existing {key} entry: {identifier}")
        associate_release(registry, release_id, release_entry(proof, manifest, data_commit))
        (clone / REGISTRY).write_text(json.dumps(registry, indent=2, sort_keys=True) + "\n")
        run(clone, sys.executable, GENERATOR, "--check")
        # Inspect only unstaged changes: a non-conflicting base merge may have
        # staged unrelated source updates, which are preserved in the merge.
        changed = set(git(clone, "diff", "--name-only").splitlines())
        changed.update(git(clone, "ls-files", "--others", "--exclude-standard").splitlines())
        if any(not permitted_change(path) for path in changed):
            raise ReleaseError("The generator changed files outside the snapshot allowlist")
        print(json.dumps({"branch": branch, "release_id": release_id,
                          "files": sorted(changed), "dry_run": dry_run}, indent=2))
        if dry_run:
            return
        if changed:
            git(clone, "add", "--", *sorted(changed))
        staged = git(clone, "diff", "--cached", "--name-only")
        if staged or (clone / ".git/MERGE_HEAD").exists():
            git(clone, "commit", "-m", f"chore(swift-sdk): freeze App Store schema {manifest['schema']['schema_version']}")
            # Deliberately no force or force-with-lease. Concurrent human edits
            # cause a rejection; the next run starts from the new branch tip.
            git(clone, "push", "origin", f"HEAD:refs/heads/{branch}", env=env)
        # A previous push may have succeeded while PR creation failed. Even
        # when generation is a no-op, recover by creating the missing PR.
        if opened:
            print(f"Snapshot pull request: {opened[0]['html_url']}")
        else:
            created = api.request("POST", "pulls", {
                "title": f"chore(swift-sdk): freeze App Store schema {manifest['schema']['schema_version']}",
                "head": branch, "base": BASE_BRANCH, "draft": True,
                "body": pr_body(proof, manifest, data_commit)})
            print(f"Snapshot pull request: {created['html_url']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-id", required=True)
    parser.add_argument("--data-commit", required=True)
    parser.add_argument("--data-repo", required=True, type=Path)
    parser.add_argument("--repo", default=Path.cwd(), type=Path)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    token = os.environ.get("SCHEMA_RELEASE_TOKEN", "")
    if not token:
        parser.error("SCHEMA_RELEASE_TOKEN is required for read access, including dry runs")
    try:
        prepare(args.repo.resolve(), args.data_repo.resolve(), args.release_id,
                args.data_commit, token, args.dry_run)
    except (ReleaseError, OSError, sqlite3.Error) as error:
        print(f"Freeze stopped: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
