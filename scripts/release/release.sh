#!/usr/bin/env bash

set -e

# get current dir
DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# get current version
PACKAGE_VERSION=$(cat $DIR/../../package.json|grep version|head -1|awk -F: '{ print $2 }'|sed 's/[", ]//g')

cmd_usage="Usage: yarn release [options]

  Options:
  -t          --type                                        - release, dev or alpha
  -v          --version                                     - explicitly set target version
  -c          --changelog-from                              - tag to build changelog from
  -h          --help                                        - show help
"

for i in "$@"
do
case ${i} in
    -h|--help)
        echo "$cmd_usage"
        exit 0
    ;;
    -t=*|--type=*)
      RELEASE_TYPE="${i#*=}"
    ;;
    -v=*|--version=*)
      TARGET_VERSION="${i#*=}"
    ;;
    -c=*|--changelog-from=*)
      LATEST_TAG="${i#*=}"
    ;;
esac
done

# if target version is provided but release type is not, infer release type from version
if [ -n "$TARGET_VERSION" ]; then
  if ! node -e "require.resolve('semver')" >/dev/null 2>&1; then
    echo "Error: 'semver' package not found. Run 'yarn add -D semver' in the repo root." >&2
    exit 1
  fi
  # validate target version
  if ! node -e "const semver=require('semver');process.exit(semver.valid('$TARGET_VERSION')?0:1)"; then
    echo "Error: TARGET_VERSION '$TARGET_VERSION' is not a valid semver." >&2
    exit 1
  fi
  if [ -z "$RELEASE_TYPE" ]; then
    RELEASE_TYPE=$(node -e "const semver=require('semver');const pr=semver.prerelease('$TARGET_VERSION');console.log(pr ? pr[0] : 'release');")
  fi
fi
# if parameter is empty, get release type from current version
if [ -z "$RELEASE_TYPE" ]
then
 if [[ $PACKAGE_VERSION == *-* ]]
 then
    RELEASE_TYPE=$(echo "$PACKAGE_VERSION" | awk -F[\-.] '{print $4}')
  else
    RELEASE_TYPE="release"
 fi
fi

UNCOMMITTED_FILES="$(git status -su)"
if [ -n "$UNCOMMITTED_FILES" ]
then
  echo "commit or stash your changes before running this script"
  exit 1
fi

# ensure github authentication
if ! gh auth status&> /dev/null; then
  gh auth login
fi

# bump version
if [ -n "$TARGET_VERSION" ]; then
  yarn node $DIR/bump_version.js "$RELEASE_TYPE" --target-version="$TARGET_VERSION"
else
  yarn node $DIR/bump_version.js "$RELEASE_TYPE"
fi

# Refresh bootstrap data for this version; failure aborts release preparation.
# REUSE_TENDERDASH_SEEDS=1 is the emergency path for a quorum-server outage: it
# carries the last validated snapshot forward, subject to the same age limit the
# publishing workflow enforces. See packages/dashmate/docs/tenderdash-seeds.md.
if [ "${REUSE_TENDERDASH_SEEDS:-}" = "1" ]; then
  node "$DIR/../../packages/dashmate/scripts/generate-tenderdash-seeds.js" --reuse-snapshot
else
  node "$DIR/../../packages/dashmate/scripts/generate-tenderdash-seeds.js"
fi

# Move the SDK's embedded quorum-proof checkpoints forward. Each new checkpoint
# is proven from the one it replaces. A network whose relay fails keeps its
# checkpoint, which stays valid; proofs from it are only longer.
# MAINNET_QUORUM_RELAY / TESTNET_QUORUM_RELAY replace a network's public relay.
# See packages/rs-sdk-trusted-context-provider/README.md.
# The per-network results go into the release PR's description.
CHECKPOINT_REPORT=$(mktemp)
if cargo run --quiet --package rs-sdk-trusted-context-provider --example refresh_checkpoints -- --report "$CHECKPOINT_REPORT"; then
  :
elif [ $? -eq 2 ]; then
  # The report no longer matches the checkpoint files, so the release PR
  # would misdescribe them.
  echo "ERROR: the checkpoint refresh could not write $CHECKPOINT_REPORT; stopping." >&2
  exit 1
else
  CHECKPOINTS_KEPT=1
  echo "WARNING: an embedded quorum-proof checkpoint was not refreshed; see above." >&2
fi

cargo metadata --format-version 1 > /dev/null

NEW_PACKAGE_VERSION=$(cat $DIR/../../package.json|grep version|head -1|awk -F: '{ print $2 }'|sed 's/[", ]//g')

if [ -z "$LATEST_TAG" ]
then
  # get last tag for changelog
  LATEST_TAG=$(yarn node $DIR/find_latest_tag.js $NEW_PACKAGE_VERSION)
fi

# Surface the changelog base tag for a human to verify: a wrong base makes
# conventional-changelog regenerate and duplicate existing CHANGELOG.md sections.
# Any WARNING printed above (from find_latest_tag) lists tags that would be duplicated.
echo ""
echo "----------------------------------------------------------------"
echo "Changelog base tag : $LATEST_TAG"
echo "New version        : $NEW_PACKAGE_VERSION"
if [ -n "${CHECKPOINTS_KEPT:-}" ]; then
  echo "Quorum-proof checkpoints: NOT all refreshed (warning above)"
fi
echo "Verify the base tag is the immediately-preceding release."
echo "If it is wrong, abort and re-run with:"
echo "  yarn release -v=$NEW_PACKAGE_VERSION -c=<correct-tag>"
echo "----------------------------------------------------------------"
if [ -t 0 ]; then
  read -r -p "Press Enter to generate the changelog and open the release PR, or Ctrl-C to abort... "
fi

# generate changelog
yarn node $DIR/generate_changelog.js $LATEST_TAG

echo "New version is $NEW_PACKAGE_VERSION"

VERSION_WITHOUT_PRERELEASE=${NEW_PACKAGE_VERSION%-*}
CURRENT_BRANCH=$(git branch --show-current)

# create branch
git checkout -b release_"$NEW_PACKAGE_VERSION"

# commit changes
git commit -am "chore(release): update changelog and version to $NEW_PACKAGE_VERSION"

# push changes
git push -u origin release_"$NEW_PACKAGE_VERSION"

# create PR
if [[ $RELEASE_TYPE == "release" ]]
then
  MILESTONE="v${VERSION_WITHOUT_PRERELEASE%.*}.x"
else
  MILESTONE="v${VERSION_WITHOUT_PRERELEASE%.*}.0"
fi

# The release PR's description, with each network's checkpoint result.
PR_BODY=$(mktemp)
{
  cat "$DIR/pr_description.md"
  echo ""
  echo "## Quorum-proof checkpoints"
  echo ""
  echo "Each moved checkpoint is proven from the one it replaces. Reviewers: check each new block hash at its height on a block explorer."
  echo ""
  if [ -s "$CHECKPOINT_REPORT" ]; then
    cat "$CHECKPOINT_REPORT"
  else
    echo "- **The refresh did not run**; both checkpoints are unchanged (see the release log)."
  fi
} > "$PR_BODY"

gh pr create --base $CURRENT_BRANCH \
             --fill \
             --title "chore(release): update changelog and bump version to $NEW_PACKAGE_VERSION" \
             --body-file "$PR_BODY" \
             --milestone $MILESTONE
rm -f "$CHECKPOINT_REPORT" "$PR_BODY"

echo ""
echo "----------------------------------------------------------------"
echo "Before publishing the GitHub release for $NEW_PACKAGE_VERSION:"
echo "If this release adds a NEW publishable @dashevo/* package, configure its"
echo "npm trusted publisher first (npmjs.com -> the package -> Access ->"
echo "Trusted Publisher: GitHub Actions, dashpay/platform, workflow release.yml)."
echo "OIDC cannot publish a package that has no trusted publisher, so the publish"
echo "job fails on it otherwise -- a one-off manual publish does NOT fix this."
echo "----------------------------------------------------------------"

# switch back to base branch
git checkout -
