#!/usr/bin/env python3
"""Delete the CI Docker images that no test job can still need.

The Tests workflow builds Drive, RS-DAPI and the dashmate helper for a commit,
pushes them to GHCR under `<repository>/ci/` tagged `sha-<commit>`, and its
test jobs pull them back. GHCR keeps every image version until someone deletes
it, so this script deletes the old ones.

A tagged version is an image index. It is deleted RETENTION_DAYS after it was
last pushed. The untagged versions are the platform manifests and attestations
that the indexes point to, and two images share one when a build changed
nothing. An untagged version is deleted only when it is that old too and no
tagged version that is left points to it. If the registry cannot say what a
tagged version points to, the untagged versions of that image stay.

The script always exits 0: any problem is reported as a warning, and the next
run continues where this one stopped.
"""
import argparse
import base64
from datetime import datetime, timedelta, timezone
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request


# The `build-images` matrix in tests.yml; tests/test_cleanup_ci_images.py keeps the two in step.
IMAGES = ("drive", "rs-dapi", "dashmate-helper")
RETENTION_DAYS = 3
# Deletions tried per image and run, whether or not GitHub accepts them. A
# backlog is worked off over several runs instead of spending the API quota
# that the repository's workflows share.
MAX_ATTEMPTS = 100
# Well below the job timeout in cleanup-ci-images.yml.
TIME_LIMIT_SECONDS = 300
REQUEST_TIMEOUT_SECONDS = 20
MANIFEST_TYPES = ", ".join((
    "application/vnd.oci.image.index.v1+json",
    "application/vnd.docker.distribution.manifest.list.v2+json",
    "application/vnd.oci.image.manifest.v1+json",
    "application/vnd.docker.distribution.manifest.v2+json",
))


class Stop(Exception):
    """The sweep has to end now; the next run continues."""


def package_name(repository, image):
    """Return the GHCR package of a CI image, like `platform/ci/drive`."""
    return f"{repository.split('/', 1)[1].lower()}/ci/{image}"


def tags(version):
    return ((version.get("metadata") or {}).get("container") or {}).get("tags") or []


def expired(versions, now):
    """Return the versions past the retention, oldest first."""
    old = []
    for version in versions:
        pushed = datetime.fromisoformat(version["updated_at"].replace("Z", "+00:00"))
        if now - pushed > timedelta(days=RETENTION_DAYS):
            old.append((pushed, version))
    return [version for _, version in sorted(old, key=lambda item: item[0])]


def rate_limited(error):
    """Tell a rate limit apart from GitHub refusing one request."""
    headers = error.headers or {}
    return error.code == 429 or (
        error.code == 403 and (headers.get("Retry-After") is not None or headers.get("X-RateLimit-Remaining") == "0"))


class Http:
    """Send requests until the deadline or a rate limit; from then on raise Stop."""

    def __init__(self, deadline):
        self.deadline = deadline

    def json(self, method, url, headers):
        left = self.deadline - time.monotonic()
        if left <= 0:
            raise Stop(f"the time limit of {TIME_LIMIT_SECONDS} seconds is used up")
        request = urllib.request.Request(url, method=method, headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=min(REQUEST_TIMEOUT_SECONDS, left)) as response:
                body = response.read()
        except urllib.error.HTTPError as error:
            if rate_limited(error):
                raise Stop(f"rate limited (HTTP {error.code})") from error
            raise
        return json.loads(body) if body else None


class GitHub:
    def __init__(self, http, token, api_url):
        self.http = http
        self.token = token
        self.api_url = api_url.rstrip("/")

    def request(self, method, path):
        return self.http.json(method, self.api_url + path, {
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        })

    def versions_path(self, owner, package):
        """Return the API path of a package's versions for an organization or a user."""
        kind = "orgs" if self.request("GET", f"/users/{owner}")["type"] == "Organization" else "users"
        return f"/{kind}/{owner}/packages/container/{urllib.parse.quote(package, safe='')}/versions"

    def versions(self, path):
        found, page = [], 1
        while True:
            batch = self.request("GET", f"{path}?per_page=100&page={page}")
            found.extend(batch)
            if len(batch) < 100:
                return found
            page += 1


class Registry:
    """Read image manifests from GHCR; the packages API does not say what an index points to."""

    def __init__(self, http, token, url="https://ghcr.io"):
        self.http = http
        self.token = token
        self.url = url
        self.pull_tokens = {}

    def pull_token(self, repository):
        if repository not in self.pull_tokens:
            login = base64.b64encode(f"token:{self.token}".encode()).decode()
            scope = urllib.parse.quote(f"repository:{repository}:pull", safe="")
            reply = self.http.json("GET", f"{self.url}/token?service=ghcr.io&scope={scope}",
                                   {"Authorization": f"Basic {login}"})
            self.pull_tokens[repository] = reply["token"]
        return self.pull_tokens[repository]

    def references(self, repository, digest):
        """Return the digests an image index points to; a plain manifest points to none."""
        manifest = self.http.json("GET", f"{self.url}/v2/{repository}/manifests/{digest}", {
            "Authorization": f"Bearer {self.pull_token(repository)}",
            "Accept": MANIFEST_TYPES,
        })
        return {entry["digest"] for entry in manifest.get("manifests") or []}


def clean_package(github, registry, repository, image, now, dry_run, log, deleted):
    """Delete the expired versions of one CI image, adding their ids to `deleted`."""
    package = package_name(repository, image)
    path = github.versions_path(repository.split("/", 1)[0], package)
    try:
        versions = github.versions(path)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            log(f"{package}: no such package yet")
            return
        raise
    old = expired(versions, now)
    log(f"{package}: {len(versions)} versions, {len(old)} expired")
    attempts = 0

    def delete(candidates):
        """Return the ids of the candidates that are gone, or would be after a real run."""
        nonlocal attempts
        gone = set()
        for version in candidates:
            if attempts >= MAX_ATTEMPTS:
                log(f"{package}: stopping at {MAX_ATTEMPTS} deletions; the next run continues")
                break
            attempts += 1
            if not dry_run:
                try:
                    github.request("DELETE", f"{path}/{version['id']}")
                except urllib.error.HTTPError as error:
                    if error.code == 404:
                        # Another run deleted it first.
                        gone.add(version["id"])
                    else:
                        log(f"::warning::Could not delete {package} version {version['id']}: HTTP {error.code}")
                    continue
            gone.add(version["id"])
            deleted.append(version["id"])
        return gone

    # Indexes go first: a tag must never outlive a manifest it points to.
    gone = delete([version for version in old if tags(version)])
    untagged = [version for version in old if not tags(version)]
    if not untagged:
        return
    # Every tagged version still there protects what it points to, whether it
    # is recent or could not be deleted.
    referenced = set()
    try:
        for version in versions:
            if tags(version) and version["id"] not in gone:
                referenced |= registry.references(f"{repository.lower()}/ci/{image}", version["name"])
    except (OSError, ValueError, LookupError, TypeError, AttributeError) as error:
        log(f"::warning::{package}: cannot read what its images point to, so untagged versions stay: {error}")
        return
    delete([version for version in untagged if version["name"] not in referenced])


def clean(github, registry, repository, now, dry_run=False, log=print):
    """Delete the expired versions of every CI image. Return how many were deleted."""
    deleted = []
    try:
        for image in IMAGES:
            clean_package(github, registry, repository, image, now, dry_run, log, deleted)
    except Stop as stop:
        log(f"::warning::CI image cleanup stopped early, the next run continues: {stop}")
    return len(deleted)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dry-run", action="store_true", help="report what would be deleted and delete nothing")
    args = parser.parse_args(argv)
    try:
        http = Http(time.monotonic() + TIME_LIMIT_SECONDS)
        token = os.environ["GITHUB_TOKEN"]
        github = GitHub(http, token, os.environ.get("GITHUB_API_URL", "https://api.github.com"))
        deleted = clean(github, Registry(http, token), os.environ["GITHUB_REPOSITORY"],
                        datetime.now(timezone.utc), args.dry_run)
    except Exception as error:
        # Registry maintenance must not turn a run red, whatever went wrong.
        print(f"::warning::CI image cleanup did not finish: {error!r}")
        return 0
    print(f"{'Would delete' if args.dry_run else 'Deleted'} {deleted} image versions")
    return 0


if __name__ == "__main__":
    sys.exit(main())
