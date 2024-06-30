#!/usr/bin/env bash

set -e

CONFIG_NAME="local"

FULL_PATH=$(realpath "$0")
DIR_PATH=$(dirname "$FULL_PATH")
ROOT_PATH=$(dirname "$DIR_PATH")

DASHMATE="$ROOT_PATH"/scripts/dashmate

# build Drive, DAPI and Dashmate helper from sources

# TODO: Doesn't work
#"$DASHMATE" config set --config=${CONFIG_NAME} docker.baseImage.build.enabled true
#"$DASHMATE" config set --config=${CONFIG_NAME} docker.baseImage.build.target deps
"$DASHMATE" config set --config=${CONFIG_NAME} platform.drive.abci.docker.build.enabled true
"$DASHMATE" config set --config=${CONFIG_NAME} platform.dapi.api.docker.build.enabled true
"$DASHMATE" config set --config=${CONFIG_NAME} dashmate.helper.docker.build.enabled true

# create tenderdash blocks every 10s to speed up test suite
"$DASHMATE" config set --config=${CONFIG_NAME} platform.drive.tenderdash.consensus.createEmptyBlocksInterval "10s"

# collect drive logs for bench suite
"$DASHMATE" config set --config=${CONFIG_NAME} platform.drive.abci.logs.stdout.level "trace"
