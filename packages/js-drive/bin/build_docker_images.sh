#!/usr/bin/env bash

set -e

# ensure REPO_URL is set externally (e.g. via CI/CD)
if [ "x$REPO_URL" = "x" ]; then
  echo "error: REPO_URL is required to be set"
  exit 1
fi

IMAGE_NAME="dashevo/drive"
DRIVE_VERSION=$(node -p "require('./package.json').version")

docker build --build-arg NODE_ENV=development \
             --build-arg npm_token=$NPM_TOKEN \
             -t "${REPO_URL}/${IMAGE_NAME}:latest" \
             -t "${REPO_URL}/${IMAGE_NAME}:${DRIVE_VERSION}" \
             .

docker push "${REPO_URL}/${IMAGE_NAME}:latest"
docker push "${REPO_URL}/${IMAGE_NAME}:${DRIVE_VERSION}"
