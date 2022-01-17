#!/usr/bin/env bash

set -e

PATH_TO_SCRIPT=$(realpath $0)
PATH_TO_SCRIPTS_DIRECTORY=$(dirname $PATH_TO_SCRIPT)
PATH_TO_PROJECT_ROOT=$(dirname $PATH_TO_SCRIPTS_DIRECTORY)
PATH_TO_PACKAGES="${PATH_TO_PROJECT_ROOT}/packages"

TEST_SUITE_PATH="${PATH_TO_PACKAGES}/platform-test-suite"

CONFIG="local"

SETUP_FILE_PATH=${PATH_TO_PROJECT_ROOT}/logs/setup.log

DPNS_OWNER_PRIVATE_KEY=$(grep -m 1 "DPNS Private Key:" ${SETUP_FILE_PATH} | awk '{$1="";printf $5}')
FEATURE_FLAGS_OWNER_PRIVATE_KEY=$(grep -m 1 "Feature Flags Private Key:" ${SETUP_FILE_PATH} | awk '{$1="";printf $6}')
DASHPAY_OWNER_PRIVATE_KEY=$(grep -m 1 "Dashpay Private Key:" ${SETUP_FILE_PATH} | awk '{$1="";printf $5}')
MASTERNODE_REWARD_SHARES_OWNER_PRIVATE_KEY=$(grep -m 1 "Masternode Reward Shares Private Key:" "${SETUP_FILE_PATH}" | awk '{$1="";printf $7}')

MASTERNODE_REWARD_SHARES_OWNER_PRO_REG_TX_HASH=$(grep -m 1 "ProRegTx transaction ID:" "${SETUP_FILE_PATH}" | awk '{printf $5}')

echo "Mint 100 Dash to faucet address"

MINT_FILE_PATH=${PATH_TO_PROJECT_ROOT}/logs/mint.log

yarn dashmate wallet mint --verbose --config=local_seed 100 | tee "${MINT_FILE_PATH}"
FAUCET_ADDRESS=$(grep -m 1 "Address:" "${MINT_FILE_PATH}" | awk '{printf $3}')
FAUCET_PRIVATE_KEY=$(grep -m 1 "Private key:" "${MINT_FILE_PATH}" | awk '{printf $4}')

# check variables are not empty
if [ -z "$FAUCET_ADDRESS" ] || \
    [ -z "$FAUCET_PRIVATE_KEY" ] || \
    [ -z "$DPNS_OWNER_PRIVATE_KEY" ] || \
    [ -z "$FEATURE_FLAGS_OWNER_PRIVATE_KEY" ] || \
    [ -z "$DASHPAY_OWNER_PRIVATE_KEY" ] || \
    [ -z "$MASTERNODE_REWARD_SHARES_OWNER_PRO_REG_TX_HASH" ] || \
    [ -z "$MASTERNODE_REWARD_SHARES_OWNER_PRIVATE_KEY" ]
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
DPNS_OWNER_PRIVATE_KEY=${DPNS_OWNER_PRIVATE_KEY}
FEATURE_FLAGS_OWNER_PRIVATE_KEY=${FEATURE_FLAGS_OWNER_PRIVATE_KEY}
DASHPAY_OWNER_PRIVATE_KEY=${DASHPAY_OWNER_PRIVATE_KEY}
MASTERNODE_REWARD_SHARES_OWNER_PRO_REG_TX_HASH=${MASTERNODE_REWARD_SHARES_OWNER_PRO_REG_TX_HASH}
MASTERNODE_REWARD_SHARES_OWNER_PRIVATE_KEY=${MASTERNODE_REWARD_SHARES_OWNER_PRIVATE_KEY}
NETWORK=regtest" >> ${TEST_ENV_FILE_PATH}
#EOF

echo "Platform test suite is configured. The config is written to ${TEST_ENV_FILE_PATH}"
