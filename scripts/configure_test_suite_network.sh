#!/usr/bin/env bash

if ! [ -x "$(command -v jq)" ]; then
  echo "Error: 'jq' not found in the system. Please follow installation guide https://stedolan.github.io/jq/download/"
  exit 1
fi

if ! [ -x "$(command -v yq)" ]; then
  echo "Error: 'yq' not found in the system. Please follow installation guide https://github.com/mikefarah/yq/#install"
  exit 1
fi

NETWORK_STRING=$1

PATH_TO_SCRIPT=$(realpath $0)
PATH_TO_SCRIPTS_DIRECTORY=$(dirname $PATH_TO_SCRIPT)
PATH_TO_PROJECT_ROOT=$(dirname $PATH_TO_SCRIPTS_DIRECTORY)
PATH_TO_PACKAGES="${PATH_TO_PROJECT_ROOT}/packages"
# 1 level above monorepo root
PATH_TO_ROOT=$(dirname $PATH_TO_PROJECT_ROOT)

# use path from the argument ($2), otherwise default to the ../dash-network-configs
DASH_NETWORK_CONFIGS_PATH="${2:-"$(realpath "$PATH_TO_ROOT/dash-network-configs")"}"

if [[ -z "$NETWORK_STRING" ]]; then
    echo "Network name must be specified"
    exit 1
fi

# if such network name could not be found in configs dir
if [ ! -f "$DASH_NETWORK_CONFIGS_PATH/$NETWORK_STRING.yml" ]; then
    echo "Dash network config '$NETWORK_STRING' was not found in the folder ($DASH_NETWORK_CONFIGS_PATH)"
    echo "Either place it in $DASH_NETWORK_CONFIGS_PATH or specify path via second script argument"
    exit 1
fi

set -ex

TEST_SUITE_PATH="${PATH_TO_PACKAGES}/platform-test-suite"
PATH_TO_CONFIGS=$(realpath $DASH_NETWORK_CONFIGS_PATH)

INVENTORY=${PATH_TO_CONFIGS}/${NETWORK_STRING}.inventory
CONFIG=${PATH_TO_CONFIGS}/${NETWORK_STRING}.yml

DAPI_SEED=$(awk -F '[= ]' '/^hp-masternode/ {print $5}' "$INVENTORY" | awk NF | shuf -n1)
DAPI_PORT=1443

echo "Running against node ${DAPI_SEED}"

FAUCET_ADDRESS=$(yq .faucet_address "$CONFIG")
FAUCET_PRIVATE_KEY=$(yq .faucet_privkey "$CONFIG")
DPNS_OWNER_PRIVATE_KEY=$(yq .dpns_hd_private_key "$CONFIG")
DASHPAY_OWNER_PRIVATE_KEY=$(yq .dashpay_hd_private_key "$CONFIG")
FEATURE_FLAGS_OWNER_PRIVATE_KEY=$(yq .feature_flags_hd_private_key "$CONFIG")

MASTERNODE_NAME=$(grep "$DAPI_SEED" "$INVENTORY" | awk '{print $1;}')

MASTERNODE_OWNER_PRO_REG_TX_HASH=$(grep "$DAPI_SEED" "$INVENTORY" | awk -F "=" '{print $6;}')
MASTERNODE_OWNER_MASTER_PRIVATE_KEY=$(yq .hp_masternodes."$MASTERNODE_NAME".owner.private_key "$CONFIG")

if [[ "$NETWORK_STRING" == "devnet"* ]]; then
  NETWORK=devnet
  INSIGHT_URL="http://insight.${NETWORK_STRING#devnet-}.networks.dash.org:3001/insight-api/sync"
  CERT_FLAG=":self-signed"
  ST_EXECUTION_INTERVAL=5000
else
  NETWORK=testnet
  INSIGHT_URL="https://testnet-insight.dashevo.org/insight-api/sync"
  CERT_FLAG=""
  ST_EXECUTION_INTERVAL=15000
fi
SKIP_SYNC_BEFORE_HEIGHT=$(curl -s $INSIGHT_URL | jq '.height - 200')

# check variables are not empty
if [ -z "$FAUCET_ADDRESS" ] || \
    [ -z "$FAUCET_PRIVATE_KEY" ] || \
    [ -z "$DPNS_OWNER_PRIVATE_KEY" ] || \
    [ -z "$FEATURE_FLAGS_OWNER_PRIVATE_KEY" ] || \
    [ -z "$DASHPAY_OWNER_PRIVATE_KEY" ] || \
    [ -z "$MASTERNODE_OWNER_MASTER_PRIVATE_KEY" ] || \
    [ -z "$NETWORK" ] || \
    [ -z "$SKIP_SYNC_BEFORE_HEIGHT" ]
then
  echo "Internal error. Some of the env variables are empty. Please check logs above."
  exit 1
fi

echo "DAPI_SEED=${DAPI_SEED}:${DAPI_PORT}${CERT_FLAG}
FAUCET_ADDRESS=${FAUCET_ADDRESS}
FAUCET_PRIVATE_KEY=${FAUCET_PRIVATE_KEY}
DPNS_OWNER_PRIVATE_KEY=${DPNS_OWNER_PRIVATE_KEY}
FEATURE_FLAGS_OWNER_PRIVATE_KEY=${FEATURE_FLAGS_OWNER_PRIVATE_KEY}
DASHPAY_OWNER_PRIVATE_KEY=${DASHPAY_OWNER_PRIVATE_KEY}
MASTERNODE_OWNER_PRO_REG_TX_HASH=${MASTERNODE_OWNER_PRO_REG_TX_HASH}
MASTERNODE_OWNER_MASTER_PRIVATE_KEY=${MASTERNODE_OWNER_MASTER_PRIVATE_KEY}
NETWORK=${NETWORK}
SKIP_SYNC_BEFORE_HEIGHT=${SKIP_SYNC_BEFORE_HEIGHT}
ST_EXECUTION_INTERVAL=${ST_EXECUTION_INTERVAL}" > "${TEST_SUITE_PATH}/.env"

echo "Configured .env in $TEST_SUITE_PATH for $NETWORK_STRING from $DASH_NETWORK_CONFIGS_PATH"
