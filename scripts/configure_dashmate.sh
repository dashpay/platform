#!/usr/bin/env bash

set -e

CONFIG_NAME="local"

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")

# build Drive, DAPI and Dashmate helper from sources

# TODO: Doesn't work
#yarn dashmate config set --config=${CONFIG_NAME} docker.baseImage.build.enabled true
#yarn dashmate config set --config=${CONFIG_NAME} docker.baseImage.build.target deps
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.abci.docker.build.enabled true
yarn dashmate config set --config=${CONFIG_NAME} platform.dapi.rsDapi.docker.build.enabled true
yarn dashmate config set --config=${CONFIG_NAME} dashmate.helper.docker.build.enabled true

# create tenderdash blocks every 10s to speed up test suite
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.tenderdash.consensus.createEmptyBlocksInterval "10s"

# collect drive logs for bench suite
yarn dashmate config set --config=${CONFIG_NAME} platform.drive.abci.logs.stdout.level "trace"
