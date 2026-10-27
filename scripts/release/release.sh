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

gh pr create --base $CURRENT_BRANCH \
             --fill \
             --title "chore(release): update changelog and bump version to $NEW_PACKAGE_VERSION" \
             --body-file $DIR/pr_description.md \
             --milestone $MILESTONE

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
