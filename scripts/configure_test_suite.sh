#!/usr/bin/env bash

set -e

PATH_TO_SCRIPT=$(realpath $0)
PATH_TO_SCRIPTS_DIRECTORY=$(dirname $PATH_TO_SCRIPT)
PATH_TO_PROJECT_ROOT=$(dirname $PATH_TO_SCRIPTS_DIRECTORY)
PATH_TO_PACKAGES="${PATH_TO_PROJECT_ROOT}/packages"

TEST_SUITE_PATH="${PATH_TO_PACKAGES}/platform-test-suite"

CONFIG="local"

DPNS_CONTRACT_ID=$(yarn dashmate config:get --config="${CONFIG}_1" platform.dpns.contract.id)
DPNS_CONTRACT_BLOCK_HEIGHT=$(yarn dashmate config:get --config="${CONFIG}_1" platform.dpns.contract.blockHeight)
DPNS_TOP_LEVEL_IDENTITY_ID=$(yarn dashmate config:get --config="${CONFIG}_1" platform.dpns.ownerId)
DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY=$(grep -m 1 "HD private key:" ${PATH_TO_PROJECT_ROOT}/logs/setup.log | awk '{$1="";printf $5}')

FEATURE_FLAGS_IDENTITY_ID=$(yarn dashmate config:get --config="${CONFIG}_1" platform.featureFlags.ownerId)
FEATURE_FLAGS_CONTRACT_ID=$(yarn dashmate config:get --config="${CONFIG}_1" platform.featureFlags.contract.id)

echo "Mint 100 Dash to faucet address"

MINT_FILE_PATH=${PATH_TO_PROJECT_ROOT}/logs/mint.log

yarn dashmate wallet:mint --verbose --config=local_seed 100 | tee "${MINT_FILE_PATH}"
FAUCET_ADDRESS=$(grep -m 1 "Address:" "${MINT_FILE_PATH}" | awk '{printf $3}')
FAUCET_PRIVATE_KEY=$(grep -m 1 "Private key:" "${MINT_FILE_PATH}" | awk '{printf $4}')

# check variables are not empty
if [ -z "$FAUCET_ADDRESS" ] || \
    [ -z "$FAUCET_PRIVATE_KEY" ] || \
    [ -z "$DPNS_CONTRACT_ID" ] || \
    [ -z "$DPNS_CONTRACT_BLOCK_HEIGHT" ] || \
    [ -z "$DPNS_TOP_LEVEL_IDENTITY_ID" ] || \
    [ -z "$DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY" ] || \
    [ -z "$FEATURE_FLAGS_IDENTITY_ID" ] || \
    [ -z "$FEATURE_FLAGS_CONTRACT_ID" ]
then
  echo "Internal error. Some of the env variables are empty. Please check logs above."
  exit 1
fi

TEST_ENV_FILE_PATH=${TEST_SUITE_PATH}/.env
rm -f ${TEST_ENV_FILE_PATH}
touch ${TEST_ENV_FILE_PATH}

#cat << 'EOF' >> ${TEST_ENV_FILE_PATH}
echo "DAPI_SEED=127.0.0.1
FAUCET_ADDRESS=${FAUCET_ADDRESS}
FAUCET_PRIVATE_KEY=${FAUCET_PRIVATE_KEY}
DPNS_CONTRACT_ID=${DPNS_CONTRACT_ID}
DPNS_CONTRACT_BLOCK_HEIGHT=${DPNS_CONTRACT_BLOCK_HEIGHT}
DPNS_TOP_LEVEL_IDENTITY_ID=${DPNS_TOP_LEVEL_IDENTITY_ID}
DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY=${DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY}
FEATURE_FLAGS_IDENTITY_ID=${FEATURE_FLAGS_IDENTITY_ID}
FEATURE_FLAGS_CONTRACT_ID=${FEATURE_FLAGS_CONTRACT_ID}
NETWORK=regtest" >> ${TEST_ENV_FILE_PATH}
#EOF

echo "Platform test suite is configured. The config is written to ${TEST_ENV_FILE_PATH}"
