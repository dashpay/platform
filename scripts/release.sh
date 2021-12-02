#!/usr/bin/env bash

set -e

# call bumper
#yarn run bump release

# generate changelog
#yarn run changelog

# get current version
PACKAGE_VERSION=$(cat ../package.json|grep version|head -1|awk -F: '{ print $2 }'|sed 's/[", ]//g')

# git
#git branch release_"$PACKAGE_VERSION"
#git checkout release_"$PACKAGE_VERSION"
git commit -am "chore(release): update changelog and bump version to $PACKAGE_VERSION"

# push

# create PR

