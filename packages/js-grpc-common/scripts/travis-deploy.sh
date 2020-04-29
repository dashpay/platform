#! /bin/bash

set -xe

# Update this whenever the latest Node.js LTS version changes (~ every year).
# Do not forget to add this version to .travis.yml config also.
LATEST_LTS_VERSION="12"

# We want this command to succeed whether or not the Node.js version is the
# latest (so that the build does not show as failed), but _only_ the latest
# should be used to publish the module.
if [ "$TRAVIS_NODE_VERSION" != "$LATEST_LTS_VERSION" ]; then
  echo "Node.js v$TRAVIS_NODE_VERSION is not latest LTS version -- will not deploy with this version."
  exit 0
fi

# Ensure the tag matches the one in package.json, otherwise abort.
PACKAGE_VERSION="$(jq -r .version package.json)"
PACKAGE_TAG="v${PACKAGE_VERSION}"
if [ "$PACKAGE_TAG" != "$TRAVIS_TAG" ]; then
  echo "Travis tag (\"$TRAVIS_TAG\") is not equal to package.json tag (\"$PACKAGE_TAG\"). Please push a correct tag and try again."
  exit 1
fi

MAJOR=$(awk -F. '{print $1}' <<< $PACKAGE_VERSION)
MINOR=$(awk -F. '{print $2}' <<< $PACKAGE_VERSION)

# Use regex pattern matching to check if "dev" exists in tag
NPM_TAG="latest"
if [[ $PACKAGE_TAG =~ dev ]]; then
  NPM_TAG="${MAJOR}.${MINOR}-dev"
fi

# Now that checks have been passed, publish the module
npm publish --tag $NPM_TAG
