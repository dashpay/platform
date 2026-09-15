#!/usr/bin/env bash

# Block until every publishable workspace package is resolvable from the npm
# registry at the version recorded in its package.json.
#
# `npm publish` returns as soon as the registry accepts the upload, but the
# registry can take several minutes before a new version is resolvable by
# installs. The dashmate packaging jobs install dashmate's freshly published
# workspace dependencies from the registry, so running them inside that
# window fails with "No candidates found". Run this script after the publish
# job and before any step that installs the published versions.
#
# Environment:
#   NPM_WAIT_TIMEOUT   seconds to wait before giving up (default 900)
#   NPM_WAIT_INTERVAL  seconds between polls (default 15)

set -euo pipefail

TIMEOUT="${NPM_WAIT_TIMEOUT:-900}"
INTERVAL="${NPM_WAIT_INTERVAL:-15}"

ROOT_PATH=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT_PATH"

# One "name@version" per line, matching the set published by
# `yarn workspaces foreach --all --no-private npm publish`.
pending=$(yarn workspaces list --no-private --json | node -e '
  const fs = require("fs");
  const lines = fs.readFileSync(0, "utf8").split("\n").filter(Boolean);
  for (const line of lines) {
    const { name, location } = JSON.parse(line);
    const { version } = JSON.parse(fs.readFileSync(location + "/package.json", "utf8"));
    console.log(name + "@" + version);
  }
')

if [ -z "$pending" ]; then
  echo "No publishable workspace packages found"
  exit 1
fi

echo "Waiting up to ${TIMEOUT}s for packages to become resolvable from the npm registry:"
echo "$pending"

deadline=$(( $(date +%s) + TIMEOUT ))

while true; do
  still_pending=""

  for spec in $pending; do
    # --prefer-online revalidates any cached packument so a stale local cache
    # cannot report a version as missing (or present) incorrectly.
    if npm view --prefer-online "$spec" version >/dev/null 2>&1; then
      echo "resolvable: $spec"
    else
      still_pending="${still_pending}${spec}"$'\n'
    fi
  done

  pending=$(printf '%s' "$still_pending")

  if [ -z "$pending" ]; then
    echo "All packages are resolvable"
    exit 0
  fi

  if [ "$(date +%s)" -ge "$deadline" ]; then
    echo "::error::Timed out after ${TIMEOUT}s waiting for npm packages to become resolvable:"
    echo "$pending"
    exit 1
  fi

  echo "Still waiting for:"
  echo "$pending"
  sleep "$INTERVAL"
done
