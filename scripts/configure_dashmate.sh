#!/usr/bin/env bash

set -e

CONFIG_NAME="local"

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")

# build Drive, DAPI and Dashmate helper from sources
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.abci.dockerBuild.context "$ROOT_PATH"
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.abci.dockerBuild.dockerFilePath "$ROOT_PATH"

yarn dashmate config set --config=${CONFIG_NAME} platform.dapi.api.dockerBuild.context "$ROOT_PATH"
yarn dashmate config set --config=${CONFIG_NAME} platform.dapi.api.dockerBuild.dockerFilePath "$ROOT_PATH"

yarn dashmate config set --config=${CONFIG_NAME} platform.dapi.envoy.dockerBuild.context "$ROOT_PATH"
yarn dashmate config set --config=${CONFIG_NAME} platform.dapi.envoy.dockerBuild.dockerFilePath "$ROOT_PATH"

yarn dashmate config set --config=${CONFIG_NAME} dashmate.helper.dockerBuild.context "$ROOT_PATH"
yarn dashmate config set --config=${CONFIG_NAME} dashmate.helper.dockerBuild.dockerFilePath "$ROOT_PATH"

# create tenderdash blocks every 10s to speed up test suite
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.tenderdash.consensus.createEmptyBlocksInterval "10s"

# collect drive logs for bench suite
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.abci.log.jsonFile.level "trace"
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.abci.log.jsonFile.path "${ROOT_PATH}/logs/drive.json"
