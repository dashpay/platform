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
# CARGO_BUILD_PROFILE=release is mandatory at the current default N=1_000_000.
# Release-mode Sinsemilla is ~10× faster than debug; without it the bake
# stage during `docker build` would take hours. Apply at runtime InitChain
# is ~134 ms regardless of profile (single SST ingest, no Sinsemilla work).
#
# Drop back to `dev` only if you also lower
# `ShieldedSeedConfig::sdk_test_data().total_notes` to ~5k or below
# (debug bake fits in tenderdash's InitChain window only at small N).
for i in $(seq 1 ${MASTERNODES_COUNT}); do
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA '"true"'
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.CARGO_BUILD_PROFILE '"release"'
done
