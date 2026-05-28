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

# Bake SDK_TEST_DATA=true into the drive-abci docker build for each masternode
# so the genesis shielded-pool seeder + identity/contract fixtures run on every
# local devnet bring-up. Production / release builds explicitly do NOT set this.
#
# Note: the seeder runs at `docker build` time inside the Dockerfile's bake
# stage, so a release-mode binary is highly recommended when SDK_TEST_DATA is
# on (see `docs/shielded-seeder-performance.md`). Set it per-invocation via
# `CARGO_BUILD_PROFILE=release yarn start`, or pin per config with
# `yarn dashmate config set --config=local_N
# platform.drive.abci.docker.build.buildArgs.CARGO_BUILD_PROFILE release`.
for i in $(seq 1 ${MASTERNODES_COUNT}); do
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA "true"
done
