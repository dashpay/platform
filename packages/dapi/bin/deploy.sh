#!/usr/bin/env bash

# Show script in output, and error if anything fails
set -xe

# Update this whenever the latest Node.js LTS version changes (~ every year).
# Do not forget to add this version to .travis.yml config also.
LATEST_LTS_VERSION="12"

# We want this command to succeed whether or not the Node.js version is the
# latest (so that the build does not show as failed), but _only_ the latest
# should be used to publish the module.
if [[ "$TRAVIS_NODE_VERSION" != "$LATEST_LTS_VERSION" ]]; then
  echo "Node.js v$TRAVIS_NODE_VERSION is not latest LTS version -- will not deploy with this version."
  exit 0
fi

# Parse version and it's segments
VERSION="$(jq -r .version package.json)"
PACKAGE_TAG=v"$VERSION"

VERSION_NO_PRERELEASE=$(awk -F- '{print $1}' <<< $VERSION)
PRERELEASE=$(awk -F- '{print $2}' <<< $VERSION)

MAJOR=$(awk -F. '{print $1}' <<< $VERSION_NO_PRERELEASE)
MINOR=$(awk -F. '{print $2}' <<< $VERSION_NO_PRERELEASE)
PATCH=$(awk -F. '{print $3}' <<< $VERSION_NO_PRERELEASE)

# Ensure the tag matches the one in package.json, otherwise abort.
if [[ "$PACKAGE_TAG" != "$TRAVIS_TAG" ]]; then
  echo "Travis tag (\"$TRAVIS_TAG\") is not equal to package.json tag (\"$PACKAGE_TAG\"). Please push a correct tag and try again."
  exit 1
fi

IMAGE_NAME="dashpay/dapi"

# If prerelease is empty it is a stable release
# so we add latest tag, otherwise a full version tag
LAST_TAG="${MAJOR}.${MINOR}.${PATCH}-${PRERELEASE}"
TAG_POSTFIX="-dev"
if [[ -z "$PRERELEASE" ]]; then
  LAST_TAG="latest"
  TAG_POSTFIX=""
fi

# Build an image with multiple tags
docker build --build-arg NODE_ENV=development \
  -t "${IMAGE_NAME}:${MAJOR}${TAG_POSTFIX}" \
  -t "${IMAGE_NAME}:${MAJOR}.${MINOR}${TAG_POSTFIX}" \
  -t "${IMAGE_NAME}:${MAJOR}.${MINOR}.${PATCH}${TAG_POSTFIX}" \
  -t "${IMAGE_NAME}:${LAST_TAG}" \
  .

# Login to Docker Hub
echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin

# Push an image and all the tags
docker push "${IMAGE_NAME}:${MAJOR}${TAG_POSTFIX}"
docker push "${IMAGE_NAME}:${MAJOR}.${MINOR}${TAG_POSTFIX}"
docker push "${IMAGE_NAME}:${MAJOR}.${MINOR}.${PATCH}${TAG_POSTFIX}"
docker push "${IMAGE_NAME}:${LAST_TAG}"
