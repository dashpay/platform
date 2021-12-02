#!/usr/bin/env bash

set -e

RELEASE_TYPE="$1"
if [ -z "$RELEASE_TYPE" ]
then
 RELEASE_TYPE="release"
fi

if [[ $RELEASE_TYPE != "release" ]] && [[ $RELEASE_TYPE != "release" ]]
then
  echo "release or prerelease are the only acceptable options"
  exit 1
fi

# check gh auth
if ! gh auth status&> /dev/null; then
    gh auth login
fi

DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# call bumper
yarn run bump "$RELEASE_TYPE"

# get last tag
LAST_TAG=$(git describe --tags --abbrev=0)

# generate changelog
yarn run changelog $LAST_TAG

# get current version
PACKAGE_VERSION=$(cat $DIR/../package.json|grep version|head -1|awk -F: '{ print $2 }'|sed 's/[", ]//g')

if [[ $RELEASE_TYPE == "release" ]]
then
 BRANCH="master"
else
 BRANCH=$(git branch --show-current)
fi

# git
git branch release_"$PACKAGE_VERSION"
git checkout release_"$PACKAGE_VERSION"
git commit -m "chore(release): update changelog and bump version to $PACKAGE_VERSION"

# push
git push -u origin release_"$PACKAGE_VERSION"
# create PR
gh pr create --base $BRANCH --fill --title "chore(release): update changelog and bump version to $PACKAGE_VERSION" --body-file $DIR/utils/release.md
