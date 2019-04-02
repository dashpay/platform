#!/usr/bin/env bash

# Show script in output, and error if anything fails
set -xe

IMAGE_NAME="dashpay/drive"
VERSION=$(node -p "require('./package.json').version")

docker build --build-arg NODE_ENV=development \
             --build-arg npm_token=$NPM_TOKEN \
             -t "${REPO_URL}/${IMAGE_NAME}:latest" \
             -t "${REPO_URL}/${IMAGE_NAME}:${VERSION}" \
             .

# Login to Docker Hub
echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin

# Push images to the registry
docker push "${IMAGE_NAME}:latest"
docker push "${IMAGE_NAME}:${VERSION}"
