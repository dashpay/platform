#!/bin/bash

set -euo pipefail

cmd_usage="Usage: check-release-deb-version.sh CURRENT_TAG

Checks that the deb built for CURRENT_TAG sorts above the deb of the last release
operators were offered, which is what decides whether apt takes it as an upgrade.

The baseline release is chosen by deb-release-baseline.js, its version is read from
the published package's own control field, and the two are compared by
check-deb-version.sh.

  ENVIRONMENT:
    GITHUB_REPOSITORY  owner/name of the repository to read releases from
    GH_TOKEN           token the GitHub CLI authenticates with
    GITHUB_OUTPUT      step output file; the validated version is appended to it as
                       validated_version when set

  EXIT CODES:
    0  the new version sorts above the baseline, or there is no baseline
    1  the new version is equal to or below the baseline, or the check could not run
    2  wrong arguments
    3  dpkg is unavailable, so the comparison could not be made
"

CURRENT_TAG="${1:-}"

if [ -z "${CURRENT_TAG}" ]
then
  echo "$cmd_usage" >&2
  exit 2
fi

if [ -z "${GITHUB_REPOSITORY:-}" ]
then
  echo "check-release-deb-version.sh: GITHUB_REPOSITORY is not set." >&2
  echo "$cmd_usage" >&2
  exit 2
fi

# Both tools are needed before anything is downloaded: the baseline version is
# read with dpkg-deb and compared with dpkg. Checking here keeps a host without
# them reporting the documented "could not compare" status rather than dying on
# a missing command part way through.
for tool in dpkg dpkg-deb
do
  if ! command -v "$tool" > /dev/null 2>&1
  then
    echo "check-release-deb-version.sh: $tool not found, cannot compare Debian versions." >&2
    echo "Run this on a Debian based host or inside a container that has dpkg." >&2
    exit 3
  fi
done

DIR_PATH=$(dirname "$(realpath "$0")")

compare="${DIR_PATH}/check-deb-version.sh"
translate="${DIR_PATH}/deb-version.js"
select_baseline="${DIR_PATH}/deb-release-baseline.js"
if [ ! -x "${compare}" ] || [ ! -f "${translate}" ] || [ ! -f "${select_baseline}" ]; then
  echo "::error::${compare}, ${translate} or ${select_baseline} is missing"
  exit 1
fi
# The comparison runs on Debian versions, never on the semver tags:
# "4.1.0-1" is valid as both and means something different in each.
new_version="$(node "${translate}" "${CURRENT_TAG}")"
# Published so the packaging job can prove the deb it actually builds
# carries the version validated here, rather than both jobs
# independently predicting it from the tag.
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "validated_version=${new_version}" >> "${GITHUB_OUTPUT}"
fi

# Every release is fetched, not just the first page: the repository has
# several hundred of them, and a single page silently hides older
# lines. Which one becomes the baseline is decided by the script rather
# than by the API's default ordering, so the choice of predecessor is
# not an undocumented implementation detail.
baseline="$(gh api --paginate --slurp "repos/${GITHUB_REPOSITORY}/releases?per_page=100" \
  | node "${select_baseline}" "${CURRENT_TAG}")"

if [ -z "${baseline}" ]; then
  echo "::notice::No published deb to compare ${CURRENT_TAG} against"
  exit 0
fi

baseline_tag="${baseline%%$'\t'*}"
baseline_asset="${baseline#*$'\t'}"

# Read the baseline from the deb's own control field: the only version
# apt looks at, the only one a server-side rename of the asset cannot
# alter, and what operators actually installed rather than what the tag
# would produce today.
mkdir -p baseline-deb
gh release download "${baseline_tag}" --repo "${GITHUB_REPOSITORY}" \
  --pattern "${baseline_asset}" --dir baseline-deb --clobber
baseline_version="$(dpkg-deb -f "baseline-deb/${baseline_asset}" Version)"
if [ -z "${baseline_version}" ]; then
  echo "::error::Could not read the Version field from ${baseline_asset} in ${baseline_tag}"
  exit 1
fi

echo "Comparing ${CURRENT_TAG} (${new_version}) against ${baseline_asset} from ${baseline_tag} (${baseline_version})"
status=0
"${compare}" "${new_version}" "${baseline_version}" || status=$?
if [ "${status}" -eq 1 ]; then
  echo "::error::${new_version} does not sort above ${baseline_version}, so apt would refuse this release as a downgrade or report it as already the newest version. Rebuilding an already published version needs DASHMATE_DEB_REVISION; re-releasing a version whose predecessor carried a git sha in its upstream part needs DASHMATE_DEB_EPOCH=1. Both are read by packages/dashmate/scripts/deb-version.js and must be set for the packaging job as well."
fi
exit "${status}"
