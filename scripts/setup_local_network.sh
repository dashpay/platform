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
# CARGO_BUILD_PROFILE: temporarily on `dev` (debug) for snapshot e2e iteration.
# Previously set to `release` to make N=500_000 runtime seed survive
# tenderdash's InitChain timeout. With N=5000 + the snapshot-bake path
# landing soon, debug-profile seed in ~30-60s is acceptable, and dev
# profile cuts the docker image build from ~30 min to ~5-10 min.
# Flip back to `release` once we stop iterating on the shielded snapshot
# code path.
for i in $(seq 1 ${MASTERNODES_COUNT}); do
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA "true"
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.CARGO_BUILD_PROFILE "dev"
done
