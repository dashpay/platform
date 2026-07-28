#!/bin/bash

set -e

cmd_usage="Usage: check_deb_version.sh NEW_VERSION PREVIOUS_VERSION

Exits successfully only when NEW_VERSION sorts strictly above PREVIOUS_VERSION under
dpkg's version comparison, which is what decides whether apt offers a release as an
upgrade at all.

Both arguments are Debian package versions ([EPOCH:]UPSTREAM[-REVISION]), not semver
tags. Translate a semver version first:

  scripts/check_deb_version.sh \\
    \"\$(node scripts/deb_version.js 4.1.0-rc.4)\" \\
    \"\$(node scripts/deb_version.js 4.1.0-rc.3)\"

  EXIT CODES:
    0  new version sorts above the previous one
    1  new version is equal to or below the previous one
    2  wrong arguments
    3  dpkg is unavailable, so the comparison could not be made
"

NEW_VERSION="$1"
PREVIOUS_VERSION="$2"

if [ -z "$NEW_VERSION" ] || [ -z "$PREVIOUS_VERSION" ]
then
  echo "$cmd_usage" >&2
  exit 2
fi

# dpkg is the only authority on its own ordering rules, and the rules are subtle enough
# (`~` below the empty string, digits and letters ordered differently) that guessing here
# would defeat the point of the check. Refuse to answer instead of answering wrongly.
if ! command -v dpkg > /dev/null 2>&1
then
  echo "check_deb_version.sh: dpkg not found, cannot compare Debian versions." >&2
  echo "Run this on a Debian based host or inside a container that has dpkg." >&2
  exit 3
fi

if dpkg --compare-versions "$NEW_VERSION" gt "$PREVIOUS_VERSION"
then
  echo "$NEW_VERSION sorts above $PREVIOUS_VERSION"
  exit 0
fi

if dpkg --compare-versions "$NEW_VERSION" eq "$PREVIOUS_VERSION"
then
  echo "$NEW_VERSION is the version that is already published." >&2
  echo "Set DASHMATE_DEB_REVISION to the next Debian revision to rebuild it." >&2
  exit 1
fi

echo "$NEW_VERSION sorts below $PREVIOUS_VERSION, so apt would refuse it as a downgrade." >&2
echo "Set DASHMATE_DEB_EPOCH to overtake a version published under a different scheme." >&2
exit 1
