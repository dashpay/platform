#!/usr/bin/env bash

MINING_INTERVAL_IN_SECONDS=30
MASTERNODES_COUNT=3

FULL_PATH=$(realpath $0)
DIR_PATH=$(dirname $FULL_PATH)
ROOT_PATH=$(dirname $DIR_PATH)
PACKAGES_PATH="$ROOT_PATH/packages"

DASHMATE_BIN="${PACKAGES_PATH}/dashmate/bin/dashmate"

"${DASHMATE_BIN}" update -v

"${DASHMATE_BIN}" setup local --verbose \
                              --debug-logs \
                              --miner-interval="${MINING_INTERVAL_IN_SECONDS}s" \
                              --node-count=${MASTERNODES_COUNT} | tee "${ROOT_PATH}"/logs/setup.log
