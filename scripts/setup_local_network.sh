#!/usr/bin/env bash

set -e

MINING_INTERVAL_IN_SECONDS=60
MASTERNODES_COUNT=3

FULL_PATH=$(realpath $0)
DIR_PATH=$(dirname $FULL_PATH)
ROOT_PATH=$(dirname $DIR_PATH)

yarn run dashmate setup local --verbose \
                          --debug-logs \
                          --miner-interval="${MINING_INTERVAL_IN_SECONDS}s" \
                          --node-count=${MASTERNODES_COUNT} | tee "${ROOT_PATH}"/logs/setup.log || exit 1

# enable insight
yarn dashmate config set core.insight.enabled true --config local_seed

# Enable SDK_TEST_DATA in drive-abci builds for each local masternode so the
# genesis shielded-pool seeder + identity/contract fixtures run at bring-up.
for i in $(seq 1 ${MASTERNODES_COUNT}); do
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA "true"
done
