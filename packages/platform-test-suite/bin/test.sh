#!/usr/bin/env bash

set -ea

cmd_usage="Run test suite

Usage: test <seed> [options]

  <seed> can be IP or IP:port (or pass via DAPI_SEED env)

  Options:
  -s=a,b,c    --scope=a,b,c                                 - test scope to run
  -k=key      --faucet-key=key                              - faucet private key string
  -n=network  --network=network                             - use regtest, devnet or testnet
              --skip-sync-before-height=H                   - start sync funding wallet from specific height
              --dpns-tld-identity-private-key=private_key   - top level identity private key
              --dpns-tld-identity-id=tld_identity_id        - top level identity id
              --dpns-contract-id=tld_contract_id            - dpns contract id
              --feature-flags-identity-id=ff_identity_id    - feature-flags contract id
              --feature-flags-contract-id=ff_contract_id    - feature-flags contract id
              --faucet-wallet-use-storage=true              - use persistent wallet storage for faucet
              --faucet-wallet-storage-dir=absolute_dir      - specify directory where faucet wallet persistent storage will be stored
  -t          --timeout                                     - test timeout in milliseconds
  -h          --help                                        - show help

  Possible scopes:
  e2e
  functional
  core
  platform
  e2e:dpns
  e2e:contacts
  functional:core
  functional:platform"

FIRST_ARG="$1"
DAPI_SEED="${DAPI_SEED:=$FIRST_ARG}"
network="testnet"

DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd "${DIR}/.."

for i in "$@"
do
case ${i} in
    -h|--help)
        echo "$cmd_usage"
        exit 0
    ;;
    -s=*|--scope=*)
    scope="${i#*=}"
    ;;
    -k=*|--faucet-key=*)
    faucet_key="${i#*=}"
    ;;
    -n=*|--network=*)
    network="${i#*=}"
    ;;
    --skip-sync-before-height=*)
    skip_sync_before_height="${i#*=}"
    ;;
    --dpns-tld-identity-private-key=*)
    identity_private_key="${i#*=}"
    ;;
    --dpns-tld-identity-id=*)
    tld_identity_id="${i#*=}"
    ;;
    --dpns-contract-id=*)
    tld_contract_id="${i#*=}"
    ;;
    --feature-flags-identity-id=*)
    ff_identity_id="${i#*=}"
    ;;
    --feature-flags-contract-id=*)
    ff_contract_id="${i#*=}"
    ;;
    -t=*|--timeout=*)
    timeout="${i#*=}"
    ;;
    --faucet-wallet-storage-dir=*)
    faucet_wallet_storage_dir="${i#*=}"
    ;;
    --faucet-wallet-use-storage=*)
    faucet_wallet_use_storage="${i#*=}"
    ;;
esac
done

if [ -z "$DAPI_SEED" ] || [[ $DAPI_SEED == -* ]]
then
  echo "Seed is not specified"
  echo ""
  echo "$cmd_usage"
  exit 1
fi

if [ -n "$timeout" ] && ! [[ $timeout =~ ^[0-9]+$ ]]
then
  echo "Timeout must be an integer"
  exit 1
fi

if [ -n "$scope" ]
then
  scope_dirs=""

  IFS=', ' read -r -a scopes <<< "$scope"

  for scope in "${scopes[@]}"
    do
      case $scope in
        e2e)
          scope_dirs="${scope_dirs} test/e2e/**/*.spec.js"
        ;;
        functional)
          scope_dirs="${scope_dirs} test/functional/**/*.spec.js"
        ;;
        core)
          scope_dirs="${scope_dirs} test/functional/core/**/*.spec.js test/e2e/**/*.spec.js"
        ;;
        platform)
          scope_dirs="${scope_dirs} test/functional/platform/**/*.spec.js test/e2e/**/*.spec.js"
        ;;
        e2e:dpns)
          scope_dirs="${scope_dirs} test/e2e/dpns.spec.js"
        ;;
        e2e:contacts)
          scope_dirs="${scope_dirs} test/e2e/contacts.spec.js"
        ;;
        functional:core)
          scope_dirs="${scope_dirs} test/functional/core/**/*.spec.js"
        ;;
        functional:platform)
          scope_dirs="${scope_dirs} test/functional/platform/**/*.spec.js"
        ;;
      *)
      echo "Unknown scope $scope"
      exit 1
      ;;
    esac
  done
else
  scope_dirs="test/functional/**/*.spec.js test/e2e/**/*.spec.js"
fi

cmd="DAPI_SEED=${DAPI_SEED}"

if [ -n "$faucet_key" ]
then
  cmd="${cmd} FAUCET_PRIVATE_KEY=${faucet_key}"
fi

if [ -n "$network" ]
then
  cmd="${cmd} NETWORK=${network}"
fi

if [ -n "$skip_sync_before_height" ]
then
  cmd="${cmd} SKIP_SYNC_BEFORE_HEIGHT=${skip_sync_before_height}"
fi

if [ -n "$tld_contract_id" ]
then
  cmd="${cmd} DPNS_CONTRACT_ID=${tld_contract_id}"
fi

if [ -n "$tld_identity_id" ]
then
  cmd="${cmd} DPNS_TOP_LEVEL_IDENTITY_ID=${tld_identity_id}"
fi

if [ -n "$ff_identity_id" ]
then
  cmd="${cmd} FEATURE_FLAGS_IDENTITY_ID=${ff_identity_id}"
fi

if [ -n "$ff_contract_id" ]
then
  cmd="${cmd} FEATURE_FLAGS_CONTRACT_ID=${ff_contract_id}"
fi

if [ -n "$identity_private_key" ]
then
  cmd="${cmd} DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY=${identity_private_key}"
fi

if [ -n "$faucet_wallet_use_storage" ]
then
  cmd="${cmd} FAUCET_WALLET_USE_STORAGE=${faucet_wallet_use_storage}"
fi

if [ -n "$faucet_wallet_storage_dir" ]
then
  cmd="${cmd} FAUCET_WALLET_STORAGE_DIR=${faucet_wallet_storage_dir}"
fi

if [ -n "$GITHUB_ACTIONS" ]
then
  cmd="${cmd} NODE_ENV=test node_modules/.bin/mocha -b ${scope_dirs}"
else
  echo $cmd
  cmd="${cmd} NODE_ENV=test yarn mocha --inspect-brk -b ${scope_dirs}"
fi

if [ -n "$timeout" ]
then
  cmd="${cmd} --timeout ${timeout}"
fi

eval $cmd
