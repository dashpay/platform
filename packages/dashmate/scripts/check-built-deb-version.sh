#!/bin/bash

set -euo pipefail

cmd_usage="Usage: check-built-deb-version.sh VALIDATED_VERSION [DIST_PATH] [WORK_PATH]

Checks that every deb in DIST_PATH carries VALIDATED_VERSION in its control field.

The version gate validates a version derived from the tag; nothing else proves the deb
that actually gets built carries it. Without this check the gate's verdict applies to a
prediction rather than to the bytes that ship.

  DIST_PATH  directory the packages were built into, default packages/dashmate/dist
  WORK_PATH  directory for intermediate files, default \$RUNNER_TEMP or \$TMPDIR

  EXIT CODES:
    0  every built deb carries the validated version
    1  a deb carries another version, or there was nothing to check
    2  wrong arguments
"

VALIDATED_VERSION="${1:-}"

DIR_PATH=$(dirname "$(realpath "$0")")

DIST_PATH="${2:-${DIR_PATH}/../dist}"
WORK_PATH="${3:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}}"

if [ -z "${VALIDATED_VERSION}" ]; then
  echo "::error::The version gate did not publish a validated version"
  echo "$cmd_usage" >&2
  exit 1
fi

debs="${WORK_PATH}/built-debs"
find "${DIST_PATH}" -type f -name '*.deb' > "${debs}"
if [ ! -s "${debs}" ]; then
  echo "::error::No deb was produced to check"
  exit 1
fi
while IFS= read -r deb; do
  built_version="$(dpkg-deb -f "${deb}" Version)"
  if [ "${built_version}" != "${VALIDATED_VERSION}" ]; then
    echo "::error::${deb} carries version ${built_version}, but the gate validated ${VALIDATED_VERSION}; the packaging scheme and the gate disagree"
    exit 1
  fi
  echo "${deb} carries the validated version ${built_version}"
done < "${debs}"
