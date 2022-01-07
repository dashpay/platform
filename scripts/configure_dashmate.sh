#!/usr/bin/env bash

set -e

CONFIG_NAME="local"

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")
PACKAGES_PATH="$ROOT_PATH/packages"

DAPI_REPO_PATH="${PACKAGES_PATH}/dapi"
DRIVE_REPO_PATH="${PACKAGES_PATH}/js-drive"

# build Drive and DAPI from sources
yarn dashmate config set --config=${CONFIG_NAME} platform.sourcePath "$ROOT_PATH"

# create tenderdash blocks every 10s to speed up test suite
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.tenderdash.consensus.createEmptyBlocksInterval "10s"
