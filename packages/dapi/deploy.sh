#! /bin/bash

# Show script in output, and error if anything fails
set -xe

VERSION=$(node -p "require('./package.json').version")
IMAGE_NAME="dashpay/dapi"

# 1. build image:
docker build -t "${IMAGE_NAME}:latest" -t "${IMAGE_NAME}:${VERSION}"

# Login to Docker Hub
echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin

# Push images to the registry
docker push "${IMAGE_NAME}:latest"
docker push "${IMAGE_NAME}:${VERSION}"
