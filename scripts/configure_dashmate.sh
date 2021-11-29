#!/usr/bin/env bash

set -e

CONFIG_NAME="local"

full_path=$(realpath "$0")
dir_path=$(dirname "$full_path")
root_path=$(dirname "$dir_path")
packages_path="$root_path/packages"

DAPI_REPO_PATH="${packages_path}/dapi"
DRIVE_REPO_PATH="${packages_path}/js-drive"

#yarn dashmate config:set --config=${CONFIG_NAME} platform.dapi.api.docker.build.path "$DAPI_REPO_PATH"
#yarn dashmate config:set --config=${CONFIG_NAME} platform.drive.abci.docker.build.path "$DRIVE_REPO_PATH"

# create tenderdash blocks every 10s to speed up test suite
yarn dashmate config:set --config=${CONFIG_NAME} platform.drive.tenderdash.consensus.createEmptyBlocksInterval "10s"
