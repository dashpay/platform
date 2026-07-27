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

# Compile SDK_TEST_DATA=true into the drive-abci build for each masternode so
# the genesis shielded-pool seeder + identity/contract fixtures run on every
# local devnet bring-up. Production / release builds explicitly do NOT set this.
# The value is JSON-quoted because `dashmate config set` JSON-parses the value,
# and `buildArgs` entries must be strings (a bare `true` would parse to boolean
# and fail schema validation). Forwarded into `dynamic-compose.yml` build.args.
#
# CARGO_BUILD_PROFILE is intentionally NOT pinned here — operators choose it
# per-invocation (`dashmate config set ...buildArgs.CARGO_BUILD_PROFILE '"release"'`).
# Release is strongly recommended when SDK_TEST_DATA is on: the seeding runs at
# genesis (InitChain, runtime), and release-compiled Sinsemilla is ~10× faster
# than debug, so a `dev` binary can take hours to seed at large N.
for i in $(seq 1 ${MASTERNODES_COUNT}); do
    yarn dashmate config set --config=local_${i} \
        platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA '"true"'
done
