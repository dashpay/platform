#!/usr/bin/env bash

set -e

SCRIPT_PATH=$(realpath "$0")
SCRIPT_DIRECTORY_PATH=$(dirname "$SCRIPT_PATH")
PROJECT_ROOT_PATH=$(dirname "$SCRIPT_DIRECTORY_PATH")
PACKAGES_PATH="$PROJECT_ROOT_PATH/packages"
LOGS_PATH="$PROJECT_ROOT_PATH/logs"

CONFIG=local
DAPI_PATH="${PACKAGES_PATH}"/dapi
DRIVE_PATH="${PACKAGES_PATH}"/js-drive
SDK_PATH="${PACKAGES_PATH}"/js-dash-sdk
WALLET_LIB_PATH="${PACKAGES_PATH}"/wallet-lib

touch "${LOGS_PATH}"/mint.log

# DAPI:
cp "${DAPI_PATH}"/.env.example "${DAPI_PATH}"/.env

# JS-SDK:
FAUCET_ADDRESS=$(grep -m 1 "Address:" "${LOGS_PATH}"/mint.log | awk '{printf $3}')
FAUCET_PRIVATE_KEY=$(grep -m 1 "Private key:" "${LOGS_PATH}"/mint.log | awk '{printf $4}')
DPNS_CONTRACT_ID=$(yarn dashmate config:get --config="${CONFIG}_1" platform.dpns.contract.id)

SDK_ENV_FILE_PATH=${SDK_PATH}/.env
rm -f "${SDK_ENV_FILE_PATH}"
touch "${SDK_ENV_FILE_PATH}"

#cat << 'EOF' >> ${SDK_ENV_FILE_PATH}
echo "DAPI_SEED=127.0.0.1
FAUCET_ADDRESS=${FAUCET_ADDRESS}
FAUCET_PRIVATE_KEY=${FAUCET_PRIVATE_KEY}
DPNS_CONTRACT_ID=${DPNS_CONTRACT_ID}
NETWORK=regtest" >> "${SDK_ENV_FILE_PATH}"
#EOF

# DRIVE:
cp "${DRIVE_PATH}"/.env.example "${DRIVE_PATH}"/.env

# WALLET-LIB:
WALLET_LIB_ENV_FILE_PATH=${WALLET_LIB_PATH}/.env
rm -f "${WALLET_LIB_ENV_FILE_PATH}"
touch "${WALLET_LIB_ENV_FILE_PATH}"

#cat << 'EOF' >> ${SDK_ENV_FILE_PATH}
echo "DAPI_SEED=127.0.0.1
FAUCET_PRIVATE_KEY=${FAUCET_PRIVATE_KEY}
NETWORK=regtest" >> "${WALLET_LIB_ENV_FILE_PATH}"
#EOF
