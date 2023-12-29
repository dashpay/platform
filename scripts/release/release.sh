#!/usr/bin/env bash

set -e

# get current dir
DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# get current version
PACKAGE_VERSION=$(cat $DIR/../../package.json|grep version|head -1|awk -F: '{ print $2 }'|sed 's/[", ]//g')

cmd_usage="Usage: yarn release [options]

  Options:
  -t          --type                                        - release, dev or alpha
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
    -c=*|--changelog-from=*)
      LATEST_TAG="${i#*=}"
    ;;
esac
done

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
yarn node $DIR/bump_version.js "$RELEASE_TYPE"

cargo metadata > /dev/null

NEW_PACKAGE_VERSION=$(cat $DIR/../../package.json|grep version|head -1|awk -F: '{ print $2 }'|sed 's/[", ]//g')

if [ -z "$LATEST_TAG" ]
then
  # get last tag for changelog
  LATEST_TAG=$(yarn node $DIR/find_latest_tag.js $NEW_PACKAGE_VERSION)
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

# switch back to base branch
git checkout -
