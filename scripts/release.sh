#!/usr/bin/env bash

set -e

# get current dir
DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# get current version
PACKAGE_VERSION=$(cat $DIR/../package.json|grep version|head -1|awk -F: '{ print $2 }'|sed 's/[", ]//g')

RELEASE_TYPE="$1"

# if parameter is empty, get release type from current version
if [ -z "$RELEASE_TYPE" ]
then
 if [[ $PACKAGE_VERSION == *-* ]]
 then
    RELEASE_TYPE="prerelease"
  else
    RELEASE_TYPE="release"
 fi
fi

if [[ $RELEASE_TYPE != "release" ]] && [[ $RELEASE_TYPE != "prerelease" ]]
then
  echo "release or prerelease are the only acceptable options"
  exit 1
fi

# check gh auth
if ! gh auth status&> /dev/null; then
    gh auth login
fi

# call bumper
$DIR/bump_version.sh "$RELEASE_TYPE"

# get last tag
LAST_TAG=$(git describe --tags --abbrev=0)

# generate changelog
$DIR/generate_changelog.sh $LAST_TAG

echo "New version is $PACKAGE_VERSION"

VERSION_WITHOUT_PRERELEASE=${PACKAGE_VERSION%-*}
MILESTONE="v${VERSION_WITHOUT_PRERELEASE%.*}.x"

if [[ $RELEASE_TYPE == "release" ]]
then
 BRANCH="master"
else
 BRANCH=$(git branch --show-current)
fi

# git
git branch release_"$PACKAGE_VERSION"
git checkout release_"$PACKAGE_VERSION"
git commit -am "chore(release): update changelog and version to $PACKAGE_VERSION"

# push
git push -u origin release_"$PACKAGE_VERSION"
# create PR
gh pr create --base $BRANCH --fill --title "chore(release): update changelog and version to $PACKAGE_VERSION" --body-file $DIR/utils/release.md --milestone $MILESTONE
