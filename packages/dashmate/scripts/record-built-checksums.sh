#!/bin/bash

set -euo pipefail

cmd_usage="Usage: record-built-checksums.sh OUTPUT_PATH [DIST_PATH]

Records the sha256 of every package in DIST_PATH into OUTPUT_PATH/built.sha256, and the
file names they were taken from into OUTPUT_PATH/built-files.

Run after notarization and immediately before upload, so the hashes cover the exact
bytes that leave the packaging job. The checksums job compares them against what the
release actually serves, which is what makes SHA256SUMS evidence rather than a
restatement of whatever is attached to the release by the time it runs.

  OUTPUT_PATH  directory the two records are written to
  DIST_PATH    directory the packages were built into, default packages/dashmate/dist

  EXIT CODES:
    0  checksums recorded
    1  there was nothing to record
    2  wrong arguments
"

OUTPUT_PATH="${1:-}"

DIR_PATH=$(dirname "$(realpath "$0")")

DIST_PATH="${2:-${DIR_PATH}/../dist}"

if [ -z "${OUTPUT_PATH}" ]
then
  echo "$cmd_usage" >&2
  exit 2
fi

# The records are addressed from inside the dist directory, so their location has to
# survive the change of directory below.
OUTPUT_PATH=$(realpath "${OUTPUT_PATH}")

if command -v sha256sum > /dev/null; then
  hash_cmd=(sha256sum)
else
  hash_cmd=(shasum -a 256)
fi
cd "${DIST_PATH}"
# Hidden files are skipped to match the glob that uploads this
# directory, which does not match them either. Recording a file that
# never gets uploaded would fail the release for a package that was
# never meant to ship.
find . -type f -not -path '*/.*' | sed 's|^\./||' | LC_ALL=C sort > "${OUTPUT_PATH}/built-files"
if [ ! -s "${OUTPUT_PATH}/built-files" ]; then
  echo "::error::No built packages found to record"
  exit 1
fi
# Recorded under the name the file will carry on the release rather
# than its path in the build tree: a release has no directories, so
# the upload flattens every file to its basename. A record keyed by
# path could only ever be matched against a published asset by hash,
# which is what lets bytes built for one target be served under
# another target's name.
: > "${OUTPUT_PATH}/built.sha256"
while IFS= read -r file; do
  hash="$("${hash_cmd[@]}" "${file}" | cut -d' ' -f1)"
  printf '%s  %s\n' "${hash}" "${file##*/}" >> "${OUTPUT_PATH}/built.sha256"
done < "${OUTPUT_PATH}/built-files"
cat "${OUTPUT_PATH}/built.sha256"
