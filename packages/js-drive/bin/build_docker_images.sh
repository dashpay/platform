#!/usr/bin/env bash

set -e

REPO_URL="103738324493.dkr.ecr.us-west-2.amazonaws.com"
IMAGE_NAME="dashevo/dashdrive"

DRIVE_VERSION=$(node -p "require('./package.json').version")

docker build --build-arg NODE_ENV=development \
             --build-arg npm_token=$NPM_TOKEN \
             -t "${REPO_URL}/${IMAGE_NAME}:latest" \
             -t "${REPO_URL}/${IMAGE_NAME}:${DRIVE_VERSION}" \
             .

docker push "${REPO_URL}/${IMAGE_NAME}:latest"
docker push "${REPO_URL}/${IMAGE_NAME}:${DRIVE_VERSION}"
