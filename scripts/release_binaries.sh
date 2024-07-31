#!/bin/bash

set -e

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")

IMAGE_NAME="rust-build-env"

# Function to build the Docker image if it doesn't exist
build_docker_image() {
  echo "Building Docker image $IMAGE_NAME..."
  docker buildx build -t $IMAGE_NAME \
                         -f "$ROOT_PATH/Dockerfile.rust-build" \
                         --platform linux/amd64,linux/arm64 \
                         --load \
                         "$ROOT_PATH"
}

# Function to build and strip the binary for a specific target
build() {
  local target=$1
  local output_name=$2
  local arch=$3

  echo "Building Drive for target ${target}..."
  docker run --rm \
            -v "$ROOT_PATH:/app" \
            -w /app \
            -u "$(id -u):$(id -g)" \
            --platform "$arch" \
            "$IMAGE_NAME" \
            cargo build --release \
                        --locked \
                        --target "$target" \
                        -p drive-abci

  echo "Renaming output binary to ${output_name}..."
  mv "$ROOT_PATH/target/$target/release/drive-abci" "$ROOT_PATH/$output_name"

  echo "Creating tar.gz archive for $output_name..."
  tar -czf "$ROOT_PATH/$output_name.tar.gz" "$ROOT_PATH/$output_name"
}

# Main script
main() {
  build_docker_image

  # Build for x86_64
  build x86_64-unknown-linux-gnu drive-abci-linux-gnu-x86_64 linux/amd64

  # Build for aarch64
  build aarch64-unknown-linux-gnu drive-abci-linux-gnu-aarch64 linux/arm64

  echo "Release process completed successfully."
}

main
