#! /bin/bash

# ensure REPO_URL is set externally (e.g. via CI/CD)
if [ "x$REPO_URL" = "x" ]; then
  echo "error: REPO_URL is required to be set"
  exit 1
fi

echo "Docker deploy started"
# 0. authenticate your Docker client to your registry:
eval $(~/.local/bin/aws ecr get-login --no-include-email)

# 0.5. set the current version:
VERSION=$(node -p "require('./package.json').version")
IMAGE_NAME="dashevo/dapi"

echo "Building docker image"
# 1. build image:
docker build -t "${IMAGE_NAME}:latest" -t "${IMAGE_NAME}:${VERSION}" .
echo "Image built"

echo "Adding tags"
echo "${IMAGE_NAME}:${VERSION}" "${REPO_URL}/${IMAGE_NAME}:${VERSION}"
# 2. After the build completes, tag your image so you can push the image to this repository:
docker tag "${IMAGE_NAME}:latest" "${REPO_URL}/${IMAGE_NAME}:latest"
docker tag "${IMAGE_NAME}:${VERSION}" "${REPO_URL}/${IMAGE_NAME}:${VERSION}"
echo "Tags added"

echo "Pushing image to repo"
# 3. Push to repository:
docker push "${REPO_URL}/${IMAGE_NAME}:latest"
docker push "${REPO_URL}/${IMAGE_NAME}:${VERSION}"

echo "Image pushed. Docker deploy complete"
