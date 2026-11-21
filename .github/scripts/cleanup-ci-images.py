#!/usr/bin/env python3
"""Delete the CI Docker images that no test job can still need.

The Tests workflow builds Drive, RS-DAPI and the dashmate helper for a commit,
pushes them to GHCR under `<repository>/ci/` tagged `sha-<commit>`, and its
test jobs pull them back. GHCR keeps every image version until someone deletes
it, so this script deletes the old ones.

A version is deleted RETENTION_DAYS after it was last pushed. Untagged versions
are the platform manifests and attestations that a tagged image index points
to; they are pushed with it and stay one day longer, so an image never loses a
part before it loses its tag.

Nothing here may fail a Tests run: any problem is reported as a warning.
"""
import argparse
from datetime import datetime, timedelta, timezone
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request


# The `build-images` matrix in tests.yml; tests/test_cleanup_ci_images.py keeps the two in step.
IMAGES = ("drive", "rs-dapi", "dashmate-helper")
RETENTION_DAYS = 3
UNTAGGED_GRACE_DAYS = 1
# A backlog is worked off over several runs instead of spending the API quota
# that the repository's other workflows share.
MAX_DELETIONS = 300


def package_name(repository, image):
    """Return the GHCR package of a CI image, like `platform/ci/drive`."""
    return f"{repository.split('/', 1)[1].lower()}/ci/{image}"


def expired(versions, now):
    """Return the versions to delete, oldest first."""
    old = []
    for version in versions:
        tags = version.get("metadata", {}).get("container", {}).get("tags", [])
        keep_days = RETENTION_DAYS if tags else RETENTION_DAYS + UNTAGGED_GRACE_DAYS
        pushed = datetime.fromisoformat(version["updated_at"].replace("Z", "+00:00"))
        if now - pushed > timedelta(days=keep_days):
            old.append((pushed, version))
    return [version for _, version in sorted(old, key=lambda item: item[0])]


class GitHub:
    def __init__(self, token, api_url):
        self.token = token
        self.api_url = api_url.rstrip("/")

    def request(self, method, path):
        request = urllib.request.Request(self.api_url + path, method=method, headers={
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        })
        with urllib.request.urlopen(request, timeout=60) as response:
            body = response.read()
        return json.loads(body) if body else None

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


def clean(github, repository, now, dry_run=False, log=print):
    """Delete the expired versions of every CI image. Return how many were deleted."""
    owner = repository.split("/", 1)[0]
    deleted = 0
    for image in IMAGES:
        package = package_name(repository, image)
        path = github.versions_path(owner, package)
        try:
            versions = github.versions(path)
        except urllib.error.HTTPError as error:
            if error.code == 404:
                log(f"{package}: no such package yet")
                continue
            raise
        old = expired(versions, now)
        log(f"{package}: {len(versions)} versions, {len(old)} expired")
        for version in old:
            if deleted >= MAX_DELETIONS:
                log(f"Stopping at {MAX_DELETIONS} deletions; the next run continues")
                return deleted
            if not dry_run:
                try:
                    github.request("DELETE", f"{path}/{version['id']}")
                except urllib.error.HTTPError as error:
                    # Another run deleted it first, or GitHub refuses to delete it.
                    log(f"::warning::Could not delete {package} version {version['id']}: HTTP {error.code}")
                    continue
            deleted += 1
    return deleted


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dry-run", action="store_true", help="report what would be deleted and delete nothing")
    args = parser.parse_args()
    github = GitHub(os.environ["GITHUB_TOKEN"], os.environ.get("GITHUB_API_URL", "https://api.github.com"))
    try:
        deleted = clean(github, os.environ["GITHUB_REPOSITORY"], datetime.now(timezone.utc), args.dry_run)
    except (OSError, KeyError, ValueError) as error:
        print(f"::warning::CI image cleanup did not finish: {error}")
        return 0
    print(f"{'Would delete' if args.dry_run else 'Deleted'} {deleted} image versions")
    return 0


if __name__ == "__main__":
    sys.exit(main())
