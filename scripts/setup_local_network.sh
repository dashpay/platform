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
# TODO(temporary): the CARGO_BUILD_PROFILE=release pair below is a workaround
# for the shielded-pool seeder being unusable in debug profile at the default
# N=500_000 (Sinsemilla appends 20–50× slower → InitChain blows past
# tenderdash's timeout). Remove this line once any of these lands:
#   - the seeder is fast enough in debug for the default N (e.g. via
#     parallelised note generation or batched Sinsemilla), OR
#   - we adopt Option B from the perf doc (precomputed GroveDB snapshot
#     baked into the image — seeding cost goes to zero), OR
#   - the default N is dropped low enough that debug-profile seeding fits in
#     the tenderdash init window.
# See docs/shielded-seeder-performance.md.
for i in $(seq 1 ${MASTERNODES_COUNT}); do
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA "true"
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.CARGO_BUILD_PROFILE "release"
done
