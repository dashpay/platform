#!/bin/bash

set -e

# Function to build the Docker image if it doesn't exist
build_docker_image() {
  local arch=$1
  local dockerfile=$2
  local image_name=$3

  if ! docker images | grep -q ${image_name}; then
    echo "Building Docker image ${image_name}..."
    docker build -t ${image_name} -f ${dockerfile} .
  else
    echo "Docker image ${image_name} already exists."
  fi
}

# Function to build and strip the binary for a specific target
build_and_strip() {
  local target=$1
  local output_name=$2
  local strip_tool=$3
  local image_name=$4

  echo "Building for target ${target}..."
  docker run --rm -v "$(pwd):/app" -w /app -u "$(id -u):$(id -g)" -e RUSTFLAGS='-C target-feature=+crt-static' ${image_name} cargo build --release --locked --target ${target} -p drive-abci

  echo "Renaming output binary to ${output_name}..."
  mv target/${target}/release/drive-abci ${output_name}

  echo "Stripping binary ${output_name}..."
  ${strip_tool} ${output_name}

  echo "Creating tar.gz archive for ${output_name}..."
  tar -czf ${output_name}.tar.gz ${output_name}
}

# Main script
main() {
  build_docker_image x86_64 Dockerfile.release.x86_64 rust-build-env-x86_64
  build_docker_image aarch64 Dockerfile.release.aarch64 rust-build-env-aarch64

  # Build and strip for x86_64
  build_and_strip x86_64-unknown-linux-gnu drive-abci-linux-gnu-x86_64 strip rust-build-env-x86_64

  # Build and strip for aarch64
  build_and_strip aarch64-unknown-linux-gnu drive-abci-linux-gnu-aarch64 aarch64-linux-gnu-strip rust-build-env-aarch64

  echo "Release process completed successfully."
}

main
