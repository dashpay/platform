#!/usr/bin/env bash

set -e

PATH_TO_SCRIPT=$(realpath $0)
PATH_TO_SCRIPTS_DIRECTORY=$(dirname $PATH_TO_SCRIPT)
PATH_TO_PROJECT_ROOT=$(dirname $PATH_TO_SCRIPTS_DIRECTORY)
PATH_TO_PACKAGES="${PATH_TO_PROJECT_ROOT}/packages"

TEST_SUITE_PATH="${PATH_TO_PACKAGES}/platform-test-suite"
BENCH_SUITE_PATH="${PATH_TO_PACKAGES}/bench-suite"

CONFIG="local"

SETUP_FILE_PATH=${PATH_TO_PROJECT_ROOT}/logs/setup.log

MASTERNODE_OWNER_PRO_REG_TX_HASH=$(grep -m 1 "ProRegTx transaction ID:" "${SETUP_FILE_PATH}" | awk '{printf $5}')
MASTERNODE_OWNER_MASTER_PRIVATE_KEY=$(grep -m 1 "Owner Private Key:" "${SETUP_FILE_PATH}" | awk '{printf $5}')

echo "Mint 50 Dash to the first faucet"

MINT_FILE_PATH=${PATH_TO_PROJECT_ROOT}/logs/mint.log

yarn dashmate wallet mint --verbose --config=local_seed 50 | tee "${MINT_FILE_PATH}"
FAUCET_1_ADDRESS=$(grep -m 1 "Address:" "${MINT_FILE_PATH}" | awk '{printf $3}')
FAUCET_1_PRIVATE_KEY=$(grep -m 1 "Private key:" "${MINT_FILE_PATH}" | awk '{printf $4}')

echo "Mint 50 Dash to the second faucet"

yarn dashmate wallet mint --verbose --config=local_seed 50 | tee "${MINT_FILE_PATH}"
FAUCET_2_ADDRESS=$(grep -m 1 "Address:" "${MINT_FILE_PATH}" | awk '{printf $3}')
FAUCET_2_PRIVATE_KEY=$(grep -m 1 "Private key:" "${MINT_FILE_PATH}" | awk '{printf $4}')

FAUCET_WALLET_USE_STORAGE=true

# check variables are not empty
if [ -z "$FAUCET_1_ADDRESS" ] || \
    [ -z "$FAUCET_1_PRIVATE_KEY" ] || \
    [ -z "$FAUCET_2_ADDRESS" ] || \
    [ -z "$FAUCET_2_PRIVATE_KEY" ] || \
    [ -z "$MASTERNODE_OWNER_PRO_REG_TX_HASH" ] || \
    [ -z "$MASTERNODE_OWNER_MASTER_PRIVATE_KEY" ]
then
  echo "Internal error. Some of the env variables are empty. Please check logs above."
  exit 1
fi

TEST_SUITE_ENV_FILE_PATH=${TEST_SUITE_PATH}/.env
rm -f ${TEST_SUITE_ENV_FILE_PATH}
touch ${TEST_SUITE_ENV_FILE_PATH}

#cat << 'EOF' >> ${TEST_SUITE_ENV_FILE_PATH}
echo "DAPI_SEED=127.0.0.1:2443:self-signed
FAUCET_1_ADDRESS=${FAUCET_1_ADDRESS}
FAUCET_1_PRIVATE_KEY=${FAUCET_1_PRIVATE_KEY}
FAUCET_2_ADDRESS=${FAUCET_2_ADDRESS}
FAUCET_2_PRIVATE_KEY=${FAUCET_2_PRIVATE_KEY}
FAUCET_WALLET_USE_STORAGE=${FAUCET_WALLET_USE_STORAGE}
FAUCET_WALLET_STORAGE_DIR="${PATH_TO_PROJECT_ROOT}/db"
MASTERNODE_OWNER_PRO_REG_TX_HASH=${MASTERNODE_OWNER_PRO_REG_TX_HASH}
MASTERNODE_OWNER_MASTER_PRIVATE_KEY=${MASTERNODE_OWNER_MASTER_PRIVATE_KEY}
NETWORK=regtest
LOG_LEVEL=info
BROWSER_TEST_BATCH_INDEX=0
BROWSER_TEST_BATCH_TOTAL=0" >> ${TEST_SUITE_ENV_FILE_PATH}
#EOF

BENCH_SUITE_ENV_FILE_PATH=${BENCH_SUITE_PATH}/.env
rm -f ${BENCH_SUITE_ENV_FILE_PATH}
touch ${BENCH_SUITE_ENV_FILE_PATH}

#cat << 'EOF' >> ${BENCH_SUITE_ENV_FILE_PATH}
echo "DAPI_SEED=127.0.0.1:2443:self-signed
FAUCET_ADDRESS=${FAUCET_ADDRESS}
FAUCET_PRIVATE_KEY=${FAUCET_PRIVATE_KEY}
DRIVE_LOG_PATH=${PATH_TO_PROJECT_ROOT}/logs/drive.json
NETWORK=regtest" >> ${BENCH_SUITE_ENV_FILE_PATH}
#EOF
