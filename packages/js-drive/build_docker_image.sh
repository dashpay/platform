#! /bin/bash

# 0. authenticate your Docker client to your registry:
eval $(aws ecr get-login --no-include-email)

# 0.5. set the current version:
VERSION="0.0.1"
REPO_URL="103738324493.dkr.ecr.us-west-2.amazonaws.com"
IMAGE_NAME="dashevo/dashdrive-api"

# 1. build image:
docker build -t "${IMAGE_NAME}:latest" -t "${IMAGE_NAME}:${VERSION}" .

# 2. After the build completes, tag your image so you can push the image to this repository:
docker tag "${IMAGE_NAME}:latest" "${REPO_URL}/${IMAGE_NAME}:latest"
docker tag "${IMAGE_NAME}:${VERSION}" "${REPO_URL}/${IMAGE_NAME}:${VERSION}"

# 3. Push to repository:
docker push "${REPO_URL}/${IMAGE_NAME}:latest"
docker push "${REPO_URL}/${IMAGE_NAME}:${VERSION}"
