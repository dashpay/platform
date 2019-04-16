#! /bin/sh

set -xe

# Update this whenever the latest Node.js LTS version changes (~ every year).
# Do not forget to add this version to .travis.yml config also.
LATEST_LTS_VERSION="10"

# We want this command to succeed whether or not the Node.js version is the
# latest (so that the build does not show as failed), but _only_ the latest
# should be used to publish the module.
if [ "$TRAVIS_NODE_VERSION" != "$LATEST_LTS_VERSION" ]; then
  echo "Node.js v$TRAVIS_NODE_VERSION is not latest LTS version -- will not deploy with this version."
  exit 0
fi

# Ensure the tag matches the one in package.json, otherwise abort.
PACKAGE_TAG=v"$(jq -r .version package.json)"
if [ "$PACKAGE_TAG" != "$TRAVIS_TAG" ]; then
  echo "Travis tag (\"$TRAVIS_TAG\") is not equal to package.json tag (\"$PACKAGE_TAG\"). Please push a correct tag and try again."
  exit 1
fi

# Now that checks have been passed, publish the module
npm publish
